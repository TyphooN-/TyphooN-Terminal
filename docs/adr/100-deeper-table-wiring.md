# ADR-100 — Deeper Table Wiring + Workspace Presets + Sparkline LRU

**Status:** Implemented
**Date:** 2026-04-12

## Context

After ADR-099 wired UX3/UX7 into 9 tables, the user requested deeper coverage:
sparklines in more tables, context menus in remaining symbol tables, built-in
workspace presets, and additional O(1) wins.

## Implemented

### UX7: Sparklines in 5 Tables (was 3)
- ev_scanner_grid (ADR-099)
- multi_outlier_grid (ADR-099)
- outliers_grid (ADR-099)
- **div_screen_grid** (NEW) — Dividend Yield Screener
- **unusual_vol_grid** (NEW) — Unusual Volume Scanner

Pre-fetch pattern: clone iteration target outside the closure, build a HashMap
of `symbol_upper → Vec<f64>` via `get_sparkline()`, look up inside the closure.

### UX3: Right-Click Menus in Live Positions/Orders (was 9 tables)
- **Live Alpaca positions** (right panel) — uses `deferred_symbol_action` field
  pattern since right panel render closures hold `&mut self` already.
- **Live tastytrade positions** — same pattern.
- New `deferred_symbol_action: SymbolAction` field on TyphooNApp.
- Applied at the end of `update()` via `std::mem::replace(...)` + `apply_symbol_action()`.

### UX4: Built-In Workspace Presets (4 named layouts)
- **TRADING**: alerts only, volume profile, no clutter — focused execution view
- **RESEARCH**: SEC + insider + fundamentals + outliers + earnings + dividends
  + sector heatmap + dividend screener + event calendar — full data analysis
- **DARWIN**: outliers + radar + swap harvest + browser + stress test + journal
- **COMPACT**: all windows closed + compact_mode = true
- `Self::builtin_workspace(name)` returns the JSON snapshot.
- `WORKSPACE_LOAD <name>` checks user-saved first, then falls back to builtin.
- `WORKSPACES` lists user-saved + 4 built-ins.

### MEM: Sparkline Cache Soft Cap
- `get_sparkline()` enforces 2000-entry soft cap (~480 KB max).
- When exceeded, drops 500 oldest entries (no LRU bookkeeping cost).
- Prevents unbounded growth on heavy fundamentals scans.

## Tests

904 tests pass (216 mql5-compiler + 553 engine + 78 cli + 57 web-protocol).
Zero warnings. Zero production unwrap/expect violations.

## Files Changed

- `native/src/app.rs` — sparkline wiring, position context menus, deferred
  symbol action field, builtin workspaces, sparkline cache cap
