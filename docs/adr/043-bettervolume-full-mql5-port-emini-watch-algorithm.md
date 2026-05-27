# ADR-043: BetterVolume — Full MQL5 Port (Emini-Watch Algorithm)

**Status:** Implemented | **Date:** 2026-03-28

## Context

The BetterVolume indicator was using a simplified approximation with fixed ratio thresholds (vol > 2x avg = climax, etc.). This produced incorrect classifications, especially on crypto charts where volume patterns differ from forex. The MT5 BetterVolume.mqh uses a fundamentally different algorithm.

## Decision

Complete rewrite to match BetterVolume.mqh (Emini-Watch algorithm):

### Buy/Sell Pressure Estimation
Each bar's volume is split into estimated buy vs sell pressure based on candle body position within the range:
- Bullish bar: `buyVol = (range / (2*range + open - close)) * totalVol`
- Bearish bar: `buyVol = ((range + close - open) / (2*range + close - open)) * totalVol`
- Doji: 50/50 split

### Lookback Extreme Comparison (Adaptive, Not Fixed)
Instead of fixed thresholds (2x, 1.5x), each bar is compared against the highest/lowest values in the 20-bar lookback window:
- `buyVol * range` vs highest in lookback → Climax Up
- `sellVol * range` vs highest in lookback → Climax Down
- `totalVol / range` vs highest in lookback → Churn
- `totalVol` vs lowest in lookback → Low Volume

### 2-Bar Combined Analysis
Consecutive bar pairs are analyzed together for stronger pattern detection, with their own separate lookback extreme comparison.

### 6 Classifications (Priority Order)
1. **Climax+Churn** (Magenta) — both climax and churn detected
2. **Low Volume** (Yellow) — volume ≤ lowest in lookback
3. **Climax Up** (Red) — buying pressure at extreme, C > O
4. **Climax Down** (White) — selling pressure at extreme, C < O
5. **Churn** (Green) — high volume relative to range
6. **Normal** (SteelBlue) — none of the above

## GPU Status

CPU-only. Buy/sell pressure estimation requires open prices (not in GPU OHLC buffer) and lookback extreme comparison is inherently sequential per bar.

## Consequences

- **Pro**: 1:1 parity with MT5 BetterVolume.mqh
- **Pro**: Works correctly on crypto, forex, equities, commodities
- **Pro**: Adaptive thresholds (no fixed ratios that break on different asset classes)
- **Con**: CPU-only (no GPU acceleration)
- **Con**: Slightly more computation per bar (buy/sell estimation + 2-bar analysis)
