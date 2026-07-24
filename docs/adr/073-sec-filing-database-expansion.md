# ADR-073: SEC Filing Database Expansion

**Status:** Implemented
**Date:** 2026-04-11

## Context

The original SEC filing scanner (ADR-034, typhoon-engine/src/core/sec_filing.rs) tracked 30+ form
types with metadata storage, Form 4 insider trade parsing, and importance-scored alerts.
However it had a 100-filing display limit, 90-day insider trade window, no full-text search,
no filing content storage, and no broker scope filtering. Users needed a growing, searchable
SEC filing database comparable to external terminal's functionality.

## Changes

### Schema Expansion
- `sec_filing_content` table: indefinite plain-text storage of stripped HTML filings
- `sec_fts` FTS5 virtual table: full-text search with porter stemming + unicode tokenizer
- `sec_keyword_watchlist` table: user-defined keywords for proactive filing alerts
- `content_fetched`, `content_fetch_attempts`, `content_last_attempt_at`, and
  `content_last_error` columns on `sec_filings`: track stored content and retry
  state

### Engine Functions (sec_filing.rs)
- `get_all_filings()` / `get_all_insider_trades()`: unlimited queries for growing database
- `strip_html_to_text()`: reusable HTML→plain text conversion (was inline in app.rs)
- `store_filing_content()`: stores content + populates FTS5 index
- `search_filings_fts()`: FTS5 MATCH with optional ticker filter and rank ordering
- `get_unfetched_filings()` / `filing_content_stats()`: backfill support
- `add_keyword()` / `remove_keyword()` / `get_keywords()` / `check_keywords()`: watchlist CRUD
- `diff_filing_content()`: LCS paragraph-level diff for filing comparison
- `find_previous_filing()`: locate prior filing of same type for diff
- `get_filing_content()`: retrieve stored content for display/diff

### SEC Scrape Universe
- UI-triggered SEC scrapes are scope-derived. The top-bar `Scope` control is the
  authority: `ALL` means the currently enabled broker sources, and a broker scope
  means only that selected broker subset.
- Native code resolves the current scope to an explicit, sorted ticker list and
  passes it into `BrokerCmd::SecScrape`; the engine scrapes exactly that list
  after validating US-equity-looking symbols.
- The engine keeps its legacy self-discovery fallback only for non-UI callers.
  The UI path must not infer scope from stale `sec_scrape_index` history or broad
  bar-cache namespaces.
- `kraken-equities:*` bar-cache keys are intentionally not used as SEC scrape
  targets. They may represent the broad exchange cache, not user intent.

### Background Content Backfill
- BG thread spawns a low-priority worker on server/non-LAN-client sessions every
  10th lightweight BG cycle.
- Fetches up to 15 eligible filings per batch at 250ms/request (~4 req/sec), with
  EDGAR requests using the shared email-shaped `SEC_EDGAR_USER_AGENT`.
- Backfill priority is active-context first: symbols from active positions, open
  orders, watchlist rows, focused/open charts, and explicit user scopes are served
  before broad catalog/backlog filings. Treat active positions, orders, and
  watchlist as the same top-priority class; only ordering inside that class should
  be newest/importance-based.
- SEC filing and news/article sync budgets sit ahead of lower-priority bar/catalog
  backlog work. Historical bars may keep trickling, but not by starving currently
  held, ordered, watched, or charted symbols of filings/news.
- Eligibility excludes filings with stored content, recently failed fetches
  (6-hour cooldown), and rows that reached the permanent attempt cap (3 failed
  attempts).
- Selection is newest filing date first, then importance. Do not let old
  high-importance amendments monopolize the backfill queue ahead of current
  filings.
- The worker logs `stored` and `failed` separately. If it sees 3 consecutive HTTP
  403 responses, it pauses the current batch instead of burning through 15 rows
  every cycle; this usually means EDGAR rejected the User-Agent or temporarily
  blocked the app.
- Successful storage clears retry state, strips HTML, stores to
  `sec_filing_content`, populates FTS5, and checks keyword watchlists for
  KEYWORD_MATCH alerts.
- Over time, the filing database becomes full-text indexed without turning
  provider-blocked rows into front-of-line retry spam.

### SEC Scanner UI (4 tabs)
1. **Filings**: broker_scope filtering (replaces Active Only), instant text search box,
   form type checkboxes, sortable columns, pagination, document viewer with auto-storage
2. **Alerts**: keyword watchlist UI (add/remove as badges), dismiss actions, alert type
   explanations, KEYWORD_MATCH alerts from backfill
3. **Insiders**: cross-symbol insider trade aggregation, cluster detection (3+ trades
   within 14 days flagged), buy/sell color coding, officer/director badges
4. **Timeline**: monthly filing activity heatmap with proportional density bars,
   form type breakdown per month, scope-filtered

### Insider Trade Chart Overlay
- SEC Form 4 buy/sell markers rendered on price chart via `build_trade_overlay()`
- Green up-arrows for buys, red down-arrows for sells
- Cross-references `insider_trades[ticker]` with bar timestamps
- Labels show "SEC:InsiderName" on hover

### GPU Indicator Fallback Fixes (discovered during audit)
- RSI: `i <= period` → `i < period` (was losing 1st valid bar)
- ADX: separate DI+/- warmup (period) from ADX warmup (period*2)
- CCI: hardcoded `i < 20` → `i < 19` (period-1)
- Fisher: added index boundary alongside 0.0 sentinel check

## Tests

904 tests pass (216 typhoon-transpiler + 553 engine + 78 cli + 57 web-protocol). Zero warnings.
Zero production unwrap/expect violations (ADR-061 compliant).

## Update 2026-07-24 — empty Filings tab with a million rows stored

Reported symptom: the SEC Filing Scanner showed `Filings (0)` on every scope
while its own status line read `763270/1025508 indexed` and `Alerts (5825)`.
The database was fine — `SELECT COUNT(*) FROM sec_filings` returned 1,025,508
and `sec_filing_alerts` 5,830. The tab count is `bg.sec_filings.len()` under
scope `All`, so the background snapshot itself was empty.

Two defects combined:

1. **No index served the snapshot query.** `get_recent_filings(conn, None, n)`
   runs `ORDER BY filing_date DESC LIMIT n` with no ticker predicate, but the
   only date index was `idx_sec_ticker_date(ticker, filing_date DESC)` —
   ticker-leading, so unusable for a global sort. `EXPLAIN QUERY PLAN` confirmed
   `SCAN sec_filings ... USE TEMP B-TREE FOR ORDER BY`: a full scan plus a
   1M-row temp sort, re-run every background cycle. Added
   `idx_sec_filing_date ON sec_filings(filing_date DESC)`, which removes the
   temp B-tree entirely (`SCAN ... USING COVERING INDEX idx_sec_filing_date`).

2. **The failure was silent.** The snapshot did
   `get_recent_filings(...).unwrap_or_default()`, so any error — most plausibly
   `SQLITE_BUSY` while the broad EDGAR scraper holds the write lock, which is
   exactly the state the reporting screenshots were taken in ("Scraping…"
   active) — published an *empty* vector indistinguishable from "no filings".
   The UI then told the user to "Click Scrape Now to fetch from SEC EDGAR",
   pointing them at the one action guaranteed not to help. Both the filings and
   alerts snapshots now keep their previous contents and log a warning on error,
   and the empty-state label distinguishes three cases: nothing stored, rows
   stored but the snapshot is empty (read failed / not yet complete), and rows
   present but filtered out by scope / form filters / search.

The index is created by `create_sec_tables` with `IF NOT EXISTS`, so it lands on
the next start; on an existing 1M-row table that is a one-time build cost.

## Update 2026-07-24 (2) — Scope was broken two different ways

After the index/snapshot fix above the Filings tab populated, but switching
Scope still showed nothing for All and Kraken while Alpaca showed rows, and
scraping under Scope Alpaca logged "SEC EDGAR scrape skipped: Scope Alpaca has
no symbols". Two independent defects.

**1. Scope switches were swallowed by the rebuild gate.** The SEC tab cache has
an early-return that holds the last cache while a broad EDGAR scrape or heavy
sync runs, with an explicit carve-out so *user-driven* control changes still
rebuild ("otherwise the scanner controls look broken"). That carve-out keys off
`filings_controls_key`, which hashed filters, search and sort — but **not
scope**. Scope was in the data key (`filings_key`), so flipping it marked the
cache changed without marking the controls changed, and the gate returned
early. The previous scope's list stayed on screen for the duration of a scrape.
`sec_filings_controls_key` is now a named function that includes scope, pinned
by a test asserting all four `EventSource` values produce distinct keys.

**2. Alpaca scope meant "my open positions".** `broker_scope_symbols` returned
`live_positions` for `EventSource::Alpaca`, while `EventSource::Kraken`
returned the whole Kraken catalog and a separate `EventSource::Positions`
already exists for "what I hold". So Alpaca was a strict subset of Positions,
asymmetric with Kraken, and **empty whenever the account was flat** — which is
what produced the skipped scrape. Alpaca scope is now
`alpaca_scope_symbols()`: the tradable US-equity catalog from
`all_broker_assets` unioned with open positions (so a held symbol stays in
scope even before the asset list loads, and the pre-fetch state degrades to the
old positions-only behaviour rather than to empty). `sec_scrape_scope_symbols`
gained a matching `Alpaca` branch with active-context priority ordering,
mirroring the existing `All` and `Kraken` branches instead of falling through
to the positions-only `_` arm.

That second change required a cache-invalidation fix: `cached_scope_syms` is
keyed on `broker_scope_membership_signature`, which for Alpaca hashed only the
*positions* revision. Since Alpaca scope now also depends on the asset catalog,
a new `alpaca_scope_catalog_rev` (bumped in `handle_alpaca_all_assets`) feeds
the signature — otherwise the cached scope set would stay positions-only for
the whole session even after assets arrived. Tests assert the Alpaca signature
moves with the catalog revision and that Kraken's does not.

## Update 2026-07-24 (3) — Scope All showed 0 rows: the scope set lagged by a frame

Both fixes above were correct and both were live, and Scope **All** still
rendered `Filings (0)` against a 1000-row snapshot — while Alpaca and Kraken
looked fine. The remaining defect is not in the scope resolution at all; it is
a read-your-own-write ordering bug between eframe's `logic()` and `ui()`.

`rebuild_sec_caches` keys its caches on `self.broker_scope` (the enum) but
*filters* on `self.cached_scope_syms` (the resolved symbol set). The set was
resolved in exactly one place: the `logic()` pump, once per frame. `broker_scope`
is mutated from `ui()` — the menu-bar Scope chip, the scope window, the
scrape-status window, the `SCOPE` command. eframe runs `logic()` before `ui()`,
and the menu bar draws before the floating windows, so on the frame the user
cycles Scope the SEC window sees the **new enum** and the **old symbol set**.

That alone would be a one-frame flicker. What made it permanent is that the
rebuild stored the wrongly-filtered result under the *new* scope's key: the next
frame, with the set finally caught up, the key already matched and the guarded
rebuild was skipped. The lag latched for as long as the scope stayed put.

This is why the symptom looked scope-specific rather than universal — every
scope was rendering its predecessor in the cycle `All → Alpaca → Kraken`:

| Chip reads | Filter actually applied | Result |
|---|---|---|
| All | Kraken's ~159 xStock tickers | ~0 rows vs an unscoped recent-filings snapshot |
| Alpaca | All's "no filter" | every row — "works fine" |
| Kraken | Alpaca's ~12k us_equity catalog | plenty of rows — "works fine" |

Only the narrowest-into-widest transition produced a visibly empty table, so the
two scopes inheriting a wider filter masked the bug.

Two changes:

- The refresh is now `refresh_broker_scope_cache()`, called by the pump as
  before *and* at the top of `rebuild_sec_caches` (plus at the two sites that
  log a scope count immediately after mutating `broker_scope`, which had the
  same off-by-one-frame count). It is O(1) when nothing moved — the key compare
  is the whole cost — so calling it again at a point of use is free.
- The SEC data caches key on `sec_scope_identity_key(scope, membership_signature)`
  instead of the bare enum. The enum is not a sufficient identity: the same
  scope resolves to different sets across a session (Alpaca and Kraken both
  start positions-only and widen when the broker catalog lands), so an
  enum-keyed cache pinned the filtered result to whatever the set was the first
  time that scope rendered.

The general rule this establishes: **state mutated in `ui()` must not be read
through a cache refreshed only in `logic()`.** Refresh at the point of use, and
key derived caches on the resolved value rather than on the selector that
produced it.

## Update 2026-07-24 (4) — Kraken scope had no equities in it at all

With the frame-lag fix above the scope counts became truthful, and the truth
was: `Broker scope → Kraken (0 fundamentals in scope)` — every time, all
session, against `All (12338)` and `Alpaca (11961)`. Previously this was
invisible, because the logged count belonged to the *previous* scope.

`kraken_scope_symbols()` — the membership set behind the fundamentals filter
and every SEC cache — was built from `kr_positions` + `kraken_pairs` +
`kraken_futures_symbols`. Equities were supposed to come out of `kraken_pairs`
via `kraken_xstock_fundamental_symbol`, which requires a `.EQ` suffix. That
suffix appears on *private* balances and the iapi catalog; the **public**
AssetPairs feed this list is loaded from exposes tokenized equities as
`{SYM}x/USD`, and the helper deliberately refuses to infer an equity from an
`x`/`X` suffix (it cannot tell `AAPLx` from `AVAX`, `FLUX`, `CVX` without a
catalog). So the derivation never fired: 875 pairs loaded, zero equities
extracted, and a scope set of pure crypto/futures tickers that shares nothing
with the equity-keyed `all_fundamentals` and `sec_filings.ticker`.

The catalog the helper lacks is `kraken_equity_universe_symbols`, and
`kraken_sec_scrape_scope_symbols` — the **scrape** path — has always used it.
The two Kraken scope derivations disagreed: the scraper fetched filings for
Kraken's tradable equities and the display filter then discarded every one of
them. That is the shape of the earlier Alpaca defect (Update 2 above) in the
opposite direction — there the scrape target was too narrow, here the
membership set was.

`kraken_scope_membership_symbols` is now a named function that unions the
equity universe in (bare, uppercase, `.EQ`-stripped, matching
`Fundamentals::symbol` / `SecFiling::ticker`), keeping crypto pairs and futures
for the non-equity views. A test asserts every symbol the Kraken scrape path
targets is present in the membership set — the two may not drift apart again.

Matching invalidation, exactly as `alpaca_scope_catalog_rev` needed in Update 2:
`kraken_scope_catalog_rev` was bumped in `handle_kraken_pairs` and
`handle_kraken_futures_instruments` but **not** where the universe digest is
applied. The digest bumps `bg_rev`, which is enough for `cached_scope_syms`
(bg_rev is in its key) but not for the SEC caches, which key on
`broker_scope_membership_signature()` alone. Without the bump the Filings tab
would keep the pre-universe (empty) filter result after the catalog landed.

## Update 2026-07-24 (5) — News had no auto-scrape at all

SEC and fundamentals both auto-start at startup and retry themselves when a
broker universe lands. News had every equivalent piece —
`news_scrape_scope_symbols`, `BrokerCmd::NewsScrapeSymbols`, a
`research_news_scrape_index` freshness table, a full multi-provider fetch — and
**no caller**. Every path into it was a button. The corpus therefore only grew
for symbols the user manually fetched, which is what "news does not sync unless
I force it on select pairs" describes.

`app/news_auto_scrape.rs` adds the missing scheduler. It is a *rotating sweep*
rather than a copy of the SEC one-shot, because the two have opposite
requirements: SEC auto-scrape is capped at 512 symbols and never repeats
(filings are not time-sensitive within a session), whereas news is worthless
stale and the universe is 10k+ symbols, so a single bounded pass would cover a
few percent of it once and never again. A cursor advances one batch per tick,
keeping per-tick cost bounded while still reaching the whole universe.

Per-frame cost is the design constraint. The steady state is four scalar
compares before any allocation — enabled, scrape already in flight, heavy sync,
interval elapsed. Only a firing tick allocates, and even then the 10k+ scope
expansion is cached behind the scope membership signature, so a firing tick is
O(batch), not O(universe). The News window already refuses to expand ALL per
frame; this holds the same line.

Sizing: 128 symbols per batch against the broker's 500ms inter-symbol pacing is
a ≤64s worst-case run inside a 10-minute default interval, so the sweep never
overlaps itself or pins `news_loading` (and therefore `heavy_sync_in_progress`)
on. Half the batch is reserved for the active set (watchlist / positions / MTF
grid / charts) and half for the rotation — a fixed share, not "whatever the
active set did not use", or a user holding 128+ active symbols would starve the
cursor and the broad universe would never be reached. Rate limiting is
server-side and already existed: `fresh_news_symbols` skips anything scraped
within 30 minutes, so re-listing a symbol costs a skip, not a fetch.

`NEWSAUTO [ON|OFF|<minutes>]` toggles and re-paces it at runtime.

While wiring this up, `news_scrape_scope_symbols` turned out to carry the same
`_ =>` fall-through that `sec_scrape_scope_symbols` had before Update 2: Scope
Alpaca and Scope Kraken both collapsed to the active set, so a background sweep
could never reach the broad universe for anyone not parked on Scope ALL. Both
now have real branches — Alpaca's `us_equity` catalog, and Kraken's equity
universe plus spot/FX pairs (crypto belongs in a *news* scope in a way it does
not for filings: the pipeline has CryptoPanic/CoinDesk providers and dedups
fetches by base asset).
