# ADR-102 — Deeper Wiring Pass 3: Calendars + Insider Window + active_symbols Cache

**Status:** Implemented
**Date:** 2026-04-12

## Context

Continuing the comprehensive wiring effort, this pass extends UX3/UX7
coverage to all calendar windows and the insider trades detail view, plus
adds another per-frame cache for `active_symbols()` (used by 5+ "Active Only"
filters).

## Implemented

### UX3: Context Menus in 4 More Tables (now 17 total)
- **earnings_cal_grid** — Earnings Calendar
- **div_cal_grid** — Dividend Calendar
- **event_cal_grid** — Event Calendar (earnings/ex-div/dividend payment)
- **Insider Trades window** header — adds menu to active symbol display

Cumulative: outliers, multi_outlier, ev_scanner, sec_filings, insider_agg,
swap_harvest, radar, div_screen, unusual_vol, live_positions, tt_positions,
live_orders, congress_grid, **earnings_cal**, **div_cal**, **event_cal**,
**insider_trades_window** + watchlist menu = **17 wired tables/windows**.

### UX7: Sparkline in Insider Trades Window
- Pre-fetched 30-day closes for active chart symbol.
- Inline 100×18 px sparkline next to symbol header.
- Total UX7 coverage: ev_scanner, multi_outlier, outliers, div_screen,
  unusual_vol, fundamentals, **insider_trades** = **7 wired surfaces**.

### PERF: Per-Frame Cached active_symbols + O(1) Dedup
- New fields: `cached_active_symbols: Vec<String>`, `cached_active_symbols_frame: u64`
- Recomputed once per frame at start of `update()` (alongside scope cache).
- 6 callsites converted from `self.active_symbols()` → `self.cached_active_symbols.clone()`:
  - Unusual Volume, Congress, Fundamentals, EV Scanner, Earnings Calendar,
    Dividend Calendar.
- **Inner O(n²) → O(n)**: `active_symbols()` itself was using `Vec::contains` for
  dedup (O(n²) for N symbols). Refactored to use `HashSet::insert` (O(1) per add).
- For typical 30-symbol active set: **6× recomputation per frame eliminated**,
  AND each recomputation is now linear instead of quadratic.

## Tests

904 tests pass. Zero warnings. Zero production unwrap/expect violations.
