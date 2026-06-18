# ADR-073: SEC Filing Database Expansion

**Status:** Implemented
**Date:** 2026-04-11

## Context

The original SEC filing scanner (ADR-034, engine/src/core/sec_filing.rs) tracked 30+ form
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

904 tests pass (216 mql5-compiler + 553 engine + 78 cli + 57 web-protocol). Zero warnings.
Zero production unwrap/expect violations (ADR-061 compliant).
