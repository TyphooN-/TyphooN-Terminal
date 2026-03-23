# ADR-042: MTF Grid — Unified Indicator Pipeline

**Status:** Implemented
**Date:** 2026-03-23

> **Note:** Supersedes the previous grid-specific indicator rendering in `loadMTFCellData()`.

## Context

The MTF Grid previously used a completely separate rendering path from the main chart. Each grid cell had ~170 lines of inline indicator code in `loadMTFCellData()` that manually applied a hardcoded subset of indicators (SMA, KAMA, ATR Projection, Fisher, BetterVolume, S/D Zones, Auto Fibonacci). This caused:

- Missing indicators: RSI, MACD, Bollinger, Stochastic, ADX, Ichimoku, Parabolic SAR, Alligator, CCI, Williams %R, OBV, Momentum, Envelopes, Fractals
- Missing price labels and series titles
- Every indicator bug fix had to be duplicated in two code paths
- Grid cells looked visually different from the main chart
- New indicators added to the registry were silently absent from the grid

## Decision

**Reuse the exact same `applyIndicators()` function for both single chart and grid cells**, matching MT5's architecture where each chart window has its own full indicator set.

### Implementation

1. `applyIndicators(chartData, ctx?)` now accepts an optional context parameter:
   - `ctx.chart` — the cell's chart instance (overrides global `chart`)
   - `ctx.fisherChart` — the cell's Fisher pane
   - `ctx.volumeChart` — the cell's volume pane

2. All internal references use `_chart`, `_fisherChart`, `_volumeChart`, `_indicatorSeries`, `_fisherSeries`, `_volumeSeries` locals that resolve to either globals (single mode) or ctx (grid mode).

3. MTF projection indicators (KAMA HTF, prev-levels, ATR HTF, MTF MA) are skipped in grid mode via `_isGridCell` guard — each grid cell IS a specific timeframe, so projecting higher TFs onto it is redundant.

4. `loadMTFCellData()` now calls:
   ```javascript
   applyIndicators(chartData, {
     chart: cellInfo.chart,
     fisherChart: cellInfo.fisherChart,
     volumeChart: cellInfo.volumeChart,
   });
   ```

### What Changed
- **Removed:** ~170 lines of duplicated inline indicator code from `loadMTFCellData()`
- **Added:** 15 lines of context parameter handling at the top of `applyIndicators()`
- **Added:** `_isGridCell` guards on 4 MTF projection blocks
- **Net:** Bundle size decreased by 2.3 KB

## Consequences

- **Pro**: Grid cells now render identically to the main chart — whatever indicators are checked, they appear on all cells
- **Pro**: Single code path means all indicator fixes apply everywhere automatically
- **Pro**: New indicators added to the registry automatically appear in grid cells
- **Pro**: Price labels, series titles, and all visual options work in grid cells
- **Con**: Grid cells do slightly more work per load (computing all checked indicators vs. the previous hardcoded subset)
- **Mitigation**: Grid cells already limit to 500 bars, so the additional computation is negligible
