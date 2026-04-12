# ADR-101 — Deeper Wiring Pass 2: Live Orders, Congress, Fundamentals Sparkline, Hot-Path Clones

**Status:** Implemented
**Date:** 2026-04-12

## Context

Continuing the comprehensive wiring effort. Each pass deepens coverage of the
optimization items the user keeps requesting.

## Implemented

### UX3: Context Menus in 2 More Tables (was 11 tables, now 13)
- **Live Alpaca orders** (right panel) — uses `deferred_symbol_action` field.
- **Congressional Trades** (`congress_grid`) — wired with local pending action.
- Total tables with right-click symbol context menu: outliers_grid,
  multi_outlier_grid, ev_scanner_grid, sec_filings_grid, insider_agg_grid,
  swap_harvest_grid, radar_grid, div_screen_grid, unusual_vol_grid,
  live_positions, tt_positions, live_orders, congress_grid + watchlist menu.

### UX7: Sparklines in Fundamentals Window
- Added sparkline (80×18 px) inline next to "Fundamentals: TICKER" header.
- Pre-fetched outside closure for all active tickers.
- Total tables with sparklines: ev_scanner_grid, multi_outlier_grid,
  outliers_grid, div_screen_grid, unusual_vol_grid, fundamentals window.

### PERF: Eliminated Redundant Symbol Clones in OUTLIERS Handlers
- **OUTLIERS handler**: was cloning `f.symbol` 4× per row (symbols vec, ev_map,
  var_map, atr_map). Now clones once into `let sym = f.symbol.clone()` and
  reuses, with the last consumer taking ownership.
- **VAROUTLIER handler**: was cloning `f.symbol` 2× per row (var_data,
  industry_data). Now clones once and moves into the second push.
- For 1000-symbol scan: ~3000 String allocs eliminated per OUTLIERS scan,
  ~1000 per VAROUTLIER scan.

## Tests

904 tests pass. Zero warnings. Zero production unwrap/expect violations.
