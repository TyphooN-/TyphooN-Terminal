# ADR-099 — Full Table Wiring Pass: Context Menus, Sparklines, Arc\<str\> Interning

**Status:** Implemented
**Date:** 2026-04-12

## Context

ADR-097 and ADR-098 documented the optimization audit and implementation. ADR-098
wired UX3/UX7/PERF5 as proof-of-concept in one window each. The user explicitly
requested full wiring across **every** symbol-bearing table. This ADR documents
the complete pass.

## Implemented

### UX3: Right-Click Symbol Context Menu — Wired into 9 Tables

| Table | Window | Notes |
|-------|--------|-------|
| `outliers_grid` | Outlier Scanner | Single-dim outliers, with auto-scroll |
| `multi_outlier_grid` | Outlier Scanner | Multi-dimensional composite outliers |
| `ev_scanner_grid` | EV Scanner | Enterprise value scanner |
| `sec_filings_grid` | SEC Filing Scanner | Filings tab — replaces click-to-select |
| `insider_agg_grid` | SEC Filing Scanner | Insiders tab cross-symbol aggregation |
| `swap_harvest_grid` | SwapHarvest | Positive swap symbols |
| `radar_grid` | Darwinex Radar | All MT5 symbols |
| `div_screen_grid` | Dividend Yield Screener | Dividend stocks |
| `unusual_vol_grid` | Unusual Volume Scanner | Volume spike detection |

Plus the **watchlist context menu** is extended with View fundamentals / SEC /
insider trades alongside its existing chart/move/remove options.

Each window pre-declares `pending_action: SymbolAction`, the inner closure
captures actions via `symbol_label_with_menu()`, and `apply_symbol_action()` is
called after the window closure releases its borrow.

### UX7: Inline Sparklines — Wired into 3 Outlier Tables + EV Scanner

- **EV Scanner**: pre-fetches sparklines for visible 200 symbols, shows 60×14
  px line chart in dedicated `30d` column.
- **Multi-Outlier Scanner**: pre-fetches 200 + 200 outlier symbols, shows
  50×12 px sparklines in dedicated column.
- **Single-Dim Outlier table**: same sparkline cache used, 8-column grid.
- Pre-fetching outside the closure avoids `&mut self` borrow conflicts.
- `sparkline_cache` is a HashMap<String, Vec<f64>> populated lazily from
  bar_cache (mt5/alpaca keys, 30 daily closes).

### PERF5: Arc\<str\> in detect_outliers

- **Internal change** to `detect_outliers()` in `engine/src/core/var.rs`:
  - `sector_intern: HashMap<&str, Arc<str>>` deduplicates sector strings.
  - `by_sector: HashMap<Arc<str>, Vec<(&str, f64)>>` keyed by Arc — `Arc::clone`
    is a refcount bump, not an allocation.
  - References instead of clones for symbol/sector during grouping.
  - `tier` changed from `String::to_string()` to `&'static str` literal.
  - `sector_str` materialized once per group, cloned only for outlier rows.
- **Public API unchanged**: `OutlierResult.sector` stays `String` for serde
  stability across LAN sync, web protocol, and KV cache.
- **Win**: O(N) String allocations during sector grouping → O(unique_sectors)
  Arc allocations. For typical 1000-symbol scan with ~50 sectors:
  1000 String allocs → 50 Arc allocs (~20× reduction).

## Tests

904 tests pass (216 mql5-compiler + 553 engine + 78 cli + 57 web-protocol).
Zero warnings. Zero production unwrap/expect violations (ADR-082 compliant).

## Files Changed

- `engine/src/core/var.rs` — `detect_outliers` Arc\<str\> interning
- `native/src/app.rs` — context menu wiring across 9 tables, sparkline wiring
  across 3 outlier tables + EV scanner, watchlist menu extension
