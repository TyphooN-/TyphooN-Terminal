# ADR-004: Multi-Timeframe Indicator Support

**Status:** Implemented
**Date:** 2026-03-15
**Context:** The NNFX trading system relies on multi-timeframe indicators (MultiKAMA, ATR Projection, Previous Candle Levels) that display higher-timeframe data on the current chart.

## Decision

Higher-timeframe bars come from the local SQLite bar cache — the sync lanes keep
every enabled timeframe current, so MTF never issues its own provider fetches.
Indicators are calculated on each timeframe's data, then projected onto the
current chart's time axis.

## How MTF Projection Works

For line indicators (KAMA, SMA):
1. Calculate indicator on HTF bar data. When a cached HTF row is missing, the
   chart derives it in-process with
   `typhoon_chart_ui::indicators::aggregate_bars_to_htf(bars, htf_minutes)`.
2. For each chart bar, find the most recent HTF indicator value at or before
   that time (the projection lands in `ChartState::mtf_sma` and the MTF overlay
   render path).
3. Draw as a line series on the main chart

For level indicators (Previous Candle Levels, ATR Projection):
1. Get the HTF's previous/current bar OHLC
2. Draw as horizontal line series spanning from the HTF bar's start time to the last chart bar

## MTF Filtering

Only show timeframes HIGHER than the current chart (matching MT5 behavior):
- H1 chart → shows H4, D1, W1 indicators
- D1 chart → shows W1 only
- W1 chart → shows nothing extra (no higher TF available)

Uses `mtf_timeframe_rank` (`typhoon-native/src/app/chart_ops.rs`), exhaustive over
`Timeframe`: `M1=0, M5=1, M15=2, M30=3, H1=4, H4=5, D1=6, W1=7, MN1=8`.

Modern depth profile bins and L3 per-order data also respect MTF projection where applicable.

## MTF MA Grid

Right-panel section showing bullish/bearish state (green/red dots) for SMA200,
KAMA, and Fisher per timeframe — `render_right_panel_mtf_grid_section` +
`mtf_dot_color` (`typhoon-native/src/app/app_runtime_right_panel_mtf_grid.rs`),
with per-cell values typed as `MtfCellValues`
(`(close, sma200, kama, fisher, fisher_signal)`).

## Pacing

MTF reads the cache, so it inherits no provider rate limit. The grid fill
(`compute_mtf_grid_status`) runs off the render thread, loads at most
`MTF_GRID_FILL_PER_BATCH` (256) cells per pass, and is re-triggered only on
active-symbol change, an open/close/retimeframe of a chart, or a 1s throttle —
so it self-terminates once every cell is warm. Provider pacing for the
underlying bars belongs to the sync lanes (ADR-087/094/095/112).

## MQL5 Color Defaults

From MTF_MA.mqh source:
- H1 200SMA: Tomato (#FF6347)
- H4/D1/W1 200SMA: Magenta (#FF00FF)
- Previous Candle Levels: White (H1/H4), Magenta (D1/W1)
- ATR Projection: Yellow (#FFFF00), solid, width 2
