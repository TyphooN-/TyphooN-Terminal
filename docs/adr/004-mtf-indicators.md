# ADR-004: Multi-Timeframe Indicator Support

**Status:** Accepted
**Date:** 2026-03-15
**Context:** The NNFX trading system relies on multi-timeframe indicators (MultiKAMA, ATR Projection, Previous Candle Levels) that display higher-timeframe data on the current chart.

## Decision

Backend fetches bars from M15/M30/H1/H4/D1/W1 via `get_multi_tf_bars` command. Frontend calculates indicators on each timeframe's data, then projects values onto the current chart's time axis.

## How MTF Projection Works

For line indicators (KAMA, SMA):
1. Calculate indicator on HTF bar data
2. `projectHTFToChartTime()`: for each chart bar, find the most recent HTF indicator value at or before that time
3. Draw as a line series on the main chart

For level indicators (Previous Candle Levels, ATR Projection):
1. Get the HTF's previous/current bar OHLC
2. Draw as horizontal line series spanning from the HTF bar's start time to the last chart bar

## MTF Filtering

Only show timeframes HIGHER than the current chart (matching MT5 behavior):
- H1 chart → shows H4, D1, W1 indicators
- D1 chart → shows W1 only
- W1 chart → shows nothing extra (no higher TF available)

Uses `TF_RANK` map: `1Min=0, 5Min=1, 15Min=2, 30Min=3, 1Hour=4, 4Hour=5, 1Day=6, 1Week=7, 1Month=8`

## MTF MA Grid

Dashboard panel showing bullish/bearish state (green/red dots) for SMA200, KAMA, and Fisher across M15/M30/H1/H4/D1/W1.

## Rate Limiting

300ms delay between each timeframe fetch to avoid Alpaca 429 errors. Automatic retry with 2s backoff on rate limit.

## MQL5 Color Defaults

From MTF_MA.mqh source:
- H1 200SMA: Tomato (#FF6347)
- H4/D1/W1 200SMA: Magenta (#FF00FF)
- Previous Candle Levels: White (H1/H4), Magenta (D1/W1)
- ATR Projection: Yellow (#FFFF00), solid, width 2
