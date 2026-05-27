# ADR-073: SEC Filing Database Expansion

**Status:** Implemented
**Date:** 2026-04-11

## Context

The original SEC filing scanner (ADR-034, engine/src/core/sec_filing.rs) tracked 30+ form
types with metadata storage, Form 4 insider trade parsing, and importance-scored alerts.
However it had a 100-filing display limit, 90-day insider trade window, no full-text search,
no filing content storage, and no broker scope filtering. Users needed a growing, searchable
SEC filing database comparable to Godel terminal's functionality.

## Changes

### Schema Expansion
- `sec_filing_content` table: indefinite plain-text storage of stripped HTML filings
- `sec_fts` FTS5 virtual table: full-text search with porter stemming + unicode tokenizer
- `sec_keyword_watchlist` table: user-defined keywords for proactive filing alerts
- `content_fetched` column on `sec_filings`: tracks which filings have stored content

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

### Background Content Backfill
- BG thread spawns a low-priority thread every ~30s (10th BG cycle)
- Fetches 5 unfetched filings per batch at 250ms rate limit (~4 req/sec)
- Strips HTML, stores to `sec_filing_content`, populates FTS5 index
- Checks keyword watchlist during backfill, creates KEYWORD_MATCH alerts
- Over time, entire filing database becomes full-text indexed

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
