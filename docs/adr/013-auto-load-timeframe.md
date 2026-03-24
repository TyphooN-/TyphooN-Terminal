# ADR-013: Auto-Load on Timeframe/Bar Count Change

**Status:** Implemented
**Date:** 2026-03-15 (updated 2026-03-24)

## Context

The UI had a "Load" button that users had to click after changing the timeframe. Every other trading terminal (MT5, TradingView, thinkorswim) auto-loads the chart when timeframe changes. The Load button was unnecessary friction.

## Decision

Auto-trigger chart load when:
- Timeframe button clicked in the toolbar
- Symbol entered via Enter key in the symbol input
- Watchlist item clicked in the right panel
- Screener "Load" button clicked

## Implementation

1. Toolbar timeframe buttons (M1–MN1) call `reload_symbol()` directly on click
2. Symbol input `lost_focus + Enter` triggers `reload_symbol()`
3. Watchlist click loads from `cache.get_bars_raw()` → `chart.compute_indicators()`
4. All paths go through `ChartState::load()` → SQLite cache → indicator recompute

No separate "Load" button exists. Selection triggers load immediately.

## Consequences

- **Pro**: Matches user expectation from MT5/TradingView — select timeframe, chart loads instantly
- **Pro**: Fewer UI elements — cleaner toolbar
- **Pro**: Pre-cached timeframes (from BarCacheWriter) appear near-instantly
- **Con**: Accidental timeframe changes trigger a load (acceptable — data is cached)
