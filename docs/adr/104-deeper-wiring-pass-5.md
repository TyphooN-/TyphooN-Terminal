# ADR-104 — Deeper Wiring Pass 5: O(1) Active Filter, Filing Truncation, Backfill Menu

**Status:** Implemented
**Date:** 2026-04-12

## Context

5th iteration of the comprehensive optimization effort.

## Implemented

### PERF: O(n²) → O(1) Active Symbol Filter
- 5 windows did `vol_active.iter().any(|s| s.eq_ignore_ascii_case(sym))`
  per row — that's O(N × M) where N = rows, M = active symbols.
- New field: `cached_active_symbols_set: HashSet<String>` — built once per
  frame from `cached_active_symbols`.
- 5 callsites converted: Unusual Volume, Congress Trades, EV Scanner,
  Earnings Calendar, Dividend Calendar.
- For typical 100-row × 30-active scan: **3000 string comparisons → 100 hash
  lookups per frame** on each window.

### MEM: Filing Content Truncation Cap
- Multi-MB SEC filings (large 10-Ks) now capped at 500KB plain text before
  zstd compression.
- Truncation marker `[Truncated at 500KB — original X bytes]` appended.
- UTF-8 char-boundary safe truncation.
- FTS5 index also receives truncated content (same source).
- Result: bounded DB growth even on extreme filings.

### UX3: Crypto Backfill Grid Context Menu (now 19 surfaces)
- `backfill_grid` symbol cells wired with `symbol_label_with_menu()`.
- Cumulative wired: outliers, multi_outlier, ev_scanner, sec_filings,
  insider_agg, swap_harvest, radar, div_screen, unusual_vol, live_positions,
  tt_positions, live_orders, congress_grid, earnings_cal, div_cal,
  event_cal, insider_trades_window, stat_arb_grid, **backfill_grid** +
  watchlist menu = **19 surfaces**.

## Tests

904 tests pass. Zero warnings. Zero production unwrap/expect violations.
