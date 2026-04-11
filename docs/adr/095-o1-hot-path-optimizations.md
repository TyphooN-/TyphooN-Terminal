# ADR-095 — O(1) Hot-Path Optimizations + Scope Regression Fix

**Status:** Implemented
**Date:** 2026-04-11

## Context

Performance audit identified O(quotes × charts) nested loops in the broker
message processing hot path (WatchlistQuotes, Mt5LiveQuotes), plus O(n × m)
linear scans in watchlist filtering and indices/forex routing. These run on
every quote tick (100s/second during market hours), causing unnecessary CPU
load proportional to `num_quotes × num_open_charts`.

Separately, a scope regression introduced in commit `3e73e0e` (broker_scope
default changed `All→Darwinex`) caused OUTLIERS, EVOUTLIERS, SECTOR_HEATMAP,
DIVIDENDS, and EV Scanner to silently return 0 results because
`darwinex_radar_data` was only populated by manual `DARWINEXRADAR` command.

## Changes

### O(1) Quote→Chart Routing
- **Mt5LiveQuotes** handler: Pre-build `HashMap<String, Vec<usize>>` mapping
  bare symbol → chart indices once per message batch. Each quote does O(1)
  HashMap lookup instead of O(charts) linear scan with per-chart string
  allocation.
- **WatchlistQuotes** handler: Same HashMap approach for exact matches.
  Partial-match fallback (`contains()`) preserved for edge cases (e.g.,
  "BTCUSD" matching "BTC") but only runs on non-exact-matched charts.

### O(1) Indices/Forex Routing
- Replaced `indices_syms.iter().any(|s| eq_ignore_ascii_case(s))` with
  `static LazyLock<HashSet<&str>>`. Single pass through rows classifies
  into indices + forex simultaneously. No per-tick Vec allocation.

### O(1) LAN Client Watchlist Filter
- Pre-build `HashSet<String>` from `user_watchlist` for O(1) exact match.
  Partial-match fallback preserved.

### O(1) MTF Grid Sort Key
- Replaced `.position()` linear search over 9-element array with `match`
  returning u8 ordinal directly.

### Darwinex Scope Regression Fix
- **BG thread auto-loads `darwinex_specs`** via `load_all_specs_parsed()`
  every 3s cycle. UI thread copies to `darwinex_radar_data` on each BG update.
- **Graceful fallback**: `broker_scope_symbols()` returns `None` (no filter)
  when `darwinex_radar_data` is empty, preventing silent 0-result filtering.
- **EVSCRAPE FORCE**: Now bypasses both 24h cache AND `scrape_failures`
  blocklist (was only bypassing cache).

## Performance Impact

| Path | Before | After |
|------|--------|-------|
| Mt5LiveQuotes (100 quotes, 10 charts) | O(1000) with 1000 string allocs | O(100) + 10 string allocs (once) |
| WatchlistQuotes (200 rows, 10 charts) | O(2000) with 2000 string allocs | O(200) + 10 string allocs (once) |
| Indices/Forex routing (200 rows) | O(200 × 26) + 2 Vec allocs | O(200) + 0 allocs (static sets) |
| LAN watchlist filter (500 rows, 50 watchlist) | O(25000) | O(500) exact + fallback |
| MTF grid sort | O(n × 9) | O(n × 1) |

## Tests

904 tests pass (216 mql5-compiler + 553 engine + 78 cli + 57 web-protocol).

## ADR Status Updates

ADRs 084-094 updated from "Accepted" → "Implemented" (all fully delivered).
