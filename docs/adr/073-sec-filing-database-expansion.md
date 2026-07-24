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

## Update 2026-07-24 (6) — the scanner was a 1000-row window over 1M filings

Reported as "why are there so few filings? impossible! we want at least the
past 1-3 years." Measured against the live corpus, the report was exactly right
and the data was never the problem:

| | |
|---|---|
| Rows in `sec_filings` | 1,039,956 |
| Date range stored | 1994-01-26 → 2026-07-24 |
| Rows the UI could see | **1,000** |
| What those 1,000 rows span | 2026-06-15 → 2026-07-24 (~5½ weeks) |
| Distinct tickers in them | **134** |
| SMCI filings stored | 975 (398 in the last 3 years) |
| SMCI filings visible | 0 — it is not one of the 134 |

`bg.sec_filings` is filled by exactly one call,
`get_recent_filings(conn, None, 1000)`, and that function opened with
`let limit = limit.min(1000)` — a silent clamp, so no caller could ask for more.
Scope, form filters and the search box then filtered *that window*. Searching a
symbol searched 134 tickers' worth of the last five weeks, not the table.

Widening the snapshot cannot fix this. Measured on the same corpus, 5,000 rows
reaches 2026-06-08, 20,000 reaches 2026-05-18, and 50,000 still only reaches
2026-03-31; "the last 3 years" is 415,279 rows, which at ~450 bytes each in
memory is ~200MB before the clone into the app — the OOM path the 1000-row cap
was introduced to avoid.

So the fix is the on-demand path the cap always implied ("deeper SEC
browsing/search must stay on-demand instead of living in every app snapshot")
but which was never built:

- **`BrokerCmd::SecFilingHistory`** — when the search box parses as ticker(s),
  query SQLite directly on `idx_sec_ticker_date` (a seek per ticker, not a
  scan), on its own thread, through a dedicated BG read connection so it never
  waits behind a bar-sync writer. 2,000 rows per ticker covers a symbol's whole
  history for essentially every issuer (SMCI's entire 975 filings since
  inception fit) for ~1MB.
- The filings cache, tab counts and grid all switch row source to those results
  while a symbol search is active, and the SEC data key includes the history so
  a landed query actually triggers a rebuild.
- A reply whose tickers no longer match the in-flight query is discarded, so a
  slow query cannot overwrite a newer one.
- The global browse window went 1,000 → 20,000 (~10MB, 1,182 tickers, back to
  2026-05-18). Still bounded; per-symbol depth is the query, not the snapshot.
- `MAX_FILING_QUERY_ROWS` replaces the bare `.min(1000)` so the ceiling is named
  and documented rather than a magic number that silently truncates callers.

A query only counts as a symbol search when **every** token is ticker-shaped.
Accepting the ticker-shaped subset meant "WESTERN DIGITAL CORP" dispatched a
query for `CORP` and replaced the snapshot rows the company-name filter was
about to match — turning a working search into an empty table.

## Update 2026-07-24 (7) — the Form 4 parser had never once seen XML

Applying update 6's "on-demand instead of snapshot" shape to the Insiders tab
started by measuring the table, which immediately said the diagnosis was wrong:

| | |
|---|---|
| Form 4 filings stored | **537,648** |
| Rows in `sec_insider_trades` | **0** |

Not a window problem. An ingest problem, and total.

EDGAR's `primaryDocument` for a Form 4 points at the **XSL-rendered view** —
`.../000000248826000117/xslF345X06/wk-form4_1784318998.xml`. Despite the `.xml`
suffix that path serves HTML. Fetched live, it contains `rptOwnerName` **zero**
times; every tag `insider_form4.rs` looks for is absent, so `extract_xml_value`
returned `Unknown`, `extract_transactions` returned an empty vec, and the parse
"succeeded" with nothing to insert. 537,451 of the 537,648 stored Form 4s
(99.96%) carry a render segment — `xslF345X02` through `xslF345X06`.

Dropping that one path segment yields the raw XML the filer submitted. Verified
against filings from the live corpus: WDC `0001266824-26-000160` →
`Tregillis Cynthia L`, 3 non-derivative + 1 derivative transaction; AXIA
`0001213900-26-079350` → `Batista de Lima Filho Pedro`, 2 + 2. Every one of
those was previously zero. The parser itself was always correct — its unit
tests pass on real XML shapes — it was simply never given XML.

The stored URL is deliberately left alone: the rendered view is the right thing
to open in a browser. Only the parse fetch is redirected, via `form4_xml_url`.

Three follow-ons, because the fix alone would not have surfaced the backlog:

- **Backfill.** Insider parsing only ever ran inline, over filings inserted
  during the current scrape pass. A Form 4 that failed was never revisited, so
  537k already-stored filings would have stayed unparsed forever. New
  `insider_parsed` / `insider_parse_attempts` / `insider_last_attempt_at`
  columns plus a partial index feed `get_unparsed_form4_filings`, drained
  newest-first at 15/cycle by a BG worker offset from the content backfill so
  the two never share a slot. Recent insider activity lands within minutes; the
  historical tail takes ~12 days at SEC's fair-use pacing.
- **Failures were invisible.** Every parse error logged at `debug!`, which is
  how a 100% failure rate across 537k filings went unnoticed. A wholesale
  failure now summarises at `warn!`.
- **Bounding the result.** `get_all_insider_trades` had a 5-year cutoff and no
  row cap — fine against an empty table, an unbounded ~1.5M-row `SELECT` cloned
  into every app snapshot once the backfill drains, i.e. exactly the OOM shape
  update 6 was about. Now capped by `MAX_INSIDER_SNAPSHOT_ROWS`; per-symbol
  depth is `get_insider_trades(conn, Some(ticker), days)` on `idx_insider_ticker`.

News was checked for the same defect and does not have it: `SearchNews` already
queries SQLite via FTS5 and `LoadCachedNews` accepts a symbol filter. (It does
use `cache.connection()` — the write mutex — rather than a BG read connection,
which is worth revisiting separately.)
