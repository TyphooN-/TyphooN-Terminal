# ADR-103 — Deeper Wiring Pass 4: Cached Scoped Fundamentals + Stat Arb + .to_uppercase Elimination

**Status:** Implemented
**Date:** 2026-04-12

## Context

Continuing the comprehensive optimization effort. Each pass deepens coverage
and finds new wins.

## Implemented

### PERF: Per-Frame Cached scoped_fundamentals_owned
- New fields: `cached_scoped_fundamentals: Vec<Fundamentals>`,
  `cached_scoped_fundamentals_frame: u64`
- Recomputed once per frame at start of `update()` (alongside scope cache,
  active_symbols cache).
- 3 callsites converted from `self.scoped_fundamentals_owned()` →
  `self.cached_scoped_fundamentals.clone()`:
  - **Sector Heatmap**, **Dividend Yield Screener**, **Outlier Scanner window**.
- For typical 200-symbol scope: avoids 3× Vec<Fundamentals> clone+filter
  per frame (~600 fundamentals struct clones eliminated).

### PERF: Eliminated Per-Row .to_uppercase() in EV Scanner Hot Path
- EV Scanner did 3× scope filter with `.to_uppercase()` per row per frame:
  1. Sparkline pre-fetch filter (200 rows)
  2. Inner sort filter (full fundamentals iter)
  3. Status bar count
- All 3 now use `cached_scoped_fundamentals` directly — scope filter applied
  once per frame, not 3× per render.
- Per 200-symbol render: **600 .to_uppercase() allocations eliminated** per frame
  on this window alone.

### UX3: Stat Arb Pairs Context Menus (now 18 surfaces)
- Stat arb pairs displayed as "SYM_A / SYM_B" in single label.
- Refactored to two clickable cells with separate symbol_label_with_menu().
- Both symbols in each pair now have right-click context menus.
- Cumulative: outliers, multi_outlier, ev_scanner, sec_filings, insider_agg,
  swap_harvest, radar, div_screen, unusual_vol, live_positions, tt_positions,
  live_orders, congress_grid, earnings_cal, div_cal, event_cal,
  insider_trades_window, **stat_arb_grid** + watchlist menu = **18 surfaces**.

## Tests

904 tests pass. Zero warnings. Zero production unwrap/expect violations.
