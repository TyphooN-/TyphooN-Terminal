# Indicator Porting: MQL5 → TyphooN-Terminal (Lessons Learned)

## Core Differences: MQL5 vs lightweight-charts

### 1. No Per-Bar Line Coloring

**MQL5**: `DRAW_COLOR_LINE` changes the line color on every bar via a color index buffer.

**lightweight-charts**: Line series have a single color. Workaround: split the data into contiguous same-color segments, each as its own line series. The transition bar is included in both adjacent segments for visual continuity.

**Applied to**: Ehlers Fisher Transform (green bullish / red bearish / gray neutral).

### 2. No Native Rectangles

**MQL5**: `OBJ_RECTANGLE` draws a filled rectangle between two price/time coordinates.

**lightweight-charts**: No rectangle primitive. Workaround: use `addBaselineSeries()` with `baseValue: { type: "price", price: bottomLevel }` — this fills between the series data (top) and the baseline price (bottom), creating a bounded filled rectangle.

**Applied to**: Supply/Demand zones.

### 3. No True Sub-Windows

**MQL5**: Indicators can declare `indicator_separate_window` and get their own price scale, axis, and independent vertical space.

**lightweight-charts**: No sub-windows. Workaround: create separate `createChart()` instances in stacked DOM elements, then sync their time scales via `subscribeVisibleLogicalRangeChange()` and crosshair via `subscribeCrosshairMove()`.

**Applied to**: Ehlers Fisher Transform pane, BetterVolume pane.

### 4. No Multi-Timeframe Built-In

**MQL5**: `iCustom()` and `CopyBuffer()` can fetch indicator values from any timeframe. The terminal manages history across all timeframes.

**lightweight-charts**: Single-timeframe only. Workaround: backend fetches bars from multiple timeframes via `get_multi_tf_bars` Tauri command, frontend projects HTF values onto current chart's time axis using `projectHTFToChartTime()`.

**Applied to**: MultiKAMA, ATR Projection (MTF), Previous Candle Levels (MTF).

### 5. Price Lines vs Line Series for Horizontal Levels

**MQL5**: `OBJ_HLINE` and `OBJ_TREND` (horizontal) draw persistent lines at fixed prices.

**lightweight-charts**: Two options:
- `candleSeries.createPriceLine()` — horizontal line at a fixed price, persists until removed. Good for levels that don't change (SL/TP, HTF levels).
- `chart.addLineSeries()` with constant values — line series where every data point has the same value. Good for levels that need to show/hide with bar data.

**Lesson**: Price lines create axis labels by default — set `axisLabelVisible: false` and `title: ""` to avoid label spam on the price axis.

**Applied to**: MTF Previous Candle Levels, MTF ATR Projection, SL/TP lines.

### 6. PRICE_OPEN vs close

**MQL5 KAMA**: Applied to `PRICE_OPEN` by default (the open price of each bar).

**JavaScript**: Must explicitly use `data[i].open` instead of `data[i].close`. Easy to miss — most indicator libraries default to close price.

### 7. Area Series Fills to Chart Bottom

**lightweight-charts `addAreaSeries()`**: Fills from the line value to the bottom of the visible chart area. There is NO way to fill between two arbitrary price levels with a plain area series.

**Fix**: Use `addBaselineSeries()` instead — it supports `baseValue: { type: "price", price: X }` which fills between the data line and a specific price level.

### 8. Indicator Data Must Be Clipped

**MQL5**: Indicators automatically stop drawing at the last calculated bar.

**lightweight-charts**: If indicator calculation produces data beyond the last candle's timestamp (e.g., from padding or projection), the line extends into empty space on the right side of the chart.

**Fix**: Filter all indicator data with `clip()` — `data.filter(d => d.time <= lastBarTime)`.

### 9. Volume in Sub-Pane vs Overlay

**MQL5 BetterVolume**: Separate window with its own price scale.

**lightweight-charts overlay approach**: Using `priceScaleId: "volume"` with `scaleMargins: { top: 0.85, bottom: 0 }` puts volume at the bottom of the main chart as a tiny overlay. This doesn't match MT5's separate window.

**Fix**: Use a separate `createChart()` instance for volume (same as Fisher pane).

### 10. GlobalVariables → JavaScript State

**MQL5**: `GlobalVariableSet()` / `GlobalVariableGet()` for cross-indicator communication (e.g., `IsAbove_KAMA_H1_SYMBOL`, `FisherBias_SYMBOL`).

**JavaScript**: Use module-level variables or a shared state object. The MTF MA grid reads indicator state from the calculated data directly instead of storing intermediate flags.

## Visual Parity Checklist

| Element | MQL5 | TyphooN-Terminal | Match? |
|---|---|---|---|
| Background | Black (#000000) | Black (#000000) | ✓ |
| Candles up | Filled green (#00FF00) | Filled green (#00FF00) | ✓ |
| Candles down | Filled red (#FF0000) | Filled red (#FF0000) | ✓ |
| Grid | Dotted gray | Dotted gray (#333, style 3) | ✓ |
| KAMA line | White, width 2 | White, width 2 | ✓ |
| 200 SMA | Yellow | Yellow (#FFFF00) | ✓ |
| ATR Projection | Yellow dotted, width 2 | Yellow dotted, width 2 | ✓ |
| Prev Candle H1/H4 | White solid, width 2 | White solid, width 2 | ✓ |
| Prev Candle D1/W1 | Magenta solid, width 2 | Magenta (#FF00FF), width 2 | ✓ |
| Fisher bullish | MediumSeaGreen (#3CB371) | #3CB371 | ✓ |
| Fisher bearish | OrangeRed (#FF4500) | #FF4500 | ✓ |
| Fisher signal | DarkGray (#A9A9A9) | #A9A9A9 | ✓ |
| BetterVolume colors | G/R/C/M/Y per bar | G/R/C/M/Y per bar | ✓ |
| S/D zones | Filled rectangles | Baseline series fill | ✓ |
| Sub-panes | Separate windows | Separate chart instances | ✓ |
| Axis labels | Minimal | Disabled on overlays | ✓ |

## Known Limitations

1. **Fisher color transitions** are per-segment, not per-bar. Each contiguous bullish/bearish section is its own line series. Visually identical but uses more series objects.

2. **Supply/Demand detection** is algorithmic (body ratio + preceding small candle). The MQL5 version may use a different detection algorithm — needs comparison testing.

3. **MultiKAMA on very low timeframes** (1m, 5m) requires many HTF bars to project meaningfully. Monthly KAMA on a 1-minute chart needs significant lookback.

4. **BetterVolume classification** (climax/churn/high/low) uses simplified heuristics. The original BetterVolume indicator by Emilio Tomasini has more sophisticated logic.
