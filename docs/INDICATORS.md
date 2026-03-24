# TyphooN Terminal — Indicator Reference

All indicators are computed in pure Rust on `&[f64]` slices. No Web Workers, no WASM bridge.

## Overlay Indicators (on price chart)

| Indicator | Parameters | Color | Description |
|-----------|-----------|-------|-------------|
| **SMA** | 200, 100 | Yellow, Blue | Simple Moving Average |
| **EMA** | 21 | Orange | Exponential Moving Average |
| **KAMA** | 10, 2, 30 | Purple | Kaufman Adaptive Moving Average (NNFX core) |
| **WMA** | 20 | Light purple | Weighted Moving Average |
| **HMA** | 20 | Cyan | Hull Moving Average (triple-WMA smoothing) |
| **Bollinger Bands** | 20, 2.0 | Blue + fill | Middle ± 2 std dev, with cloud fill |
| **Ichimoku Cloud** | 9, 26, 52 | Multi-color | Tenkan, Kijun, Span A/B with bull/bear cloud fill |
| **Parabolic SAR** | 0.02, 0.2 | Yellow dots | Stop-and-reverse dots |
| **ATR Projection** | 14 | Yellow bands | Open ± ATR(14) bands (NNFX core) |

## Sub-Pane Indicators (separate panes below chart)

| Indicator | Parameters | Color | Range | Description |
|-----------|-----------|-------|-------|-------------|
| **RSI** | 14 | Yellow | 0-100 | Relative Strength Index, OB 70 / OS 30 |
| **Fisher Transform** | 10 | Green/Red bars + signal | Auto | Color histogram + signal line (NNFX core) |
| **MACD** | 12, 26, 9 | Blue line, orange signal | Auto | MACD + signal + histogram |
| **Stochastic** | 14, 3, 3 | Blue %K, Orange %D | 0-100 | OB 80 / OS 20 |
| **ADX** | 14 | Yellow ADX, Green DI+, Red DI- | 0-60 | Trend strength + directional |
| **CCI** | 20 | Orange | -200 to +200 | Commodity Channel Index |
| **Williams %R** | 14 | Purple | -100 to 0 | OB -20 / OS -80 |
| **OBV** | — | Green | Auto | On-Balance Volume |
| **Momentum** | 10 | Tan | Auto | Price momentum (close - close[n]) |
| **Better Volume** | 20 | Multi-color | Auto | NNFX-style: climax up (green), climax down (red), high vol (blue), low vol (yellow), churn (gray) |
| **Volume** | — | Green/Red | Auto | Standard volume bars |

## NNFX System Indicators

The core NNFX trading system uses:

1. **KAMA(10,2,30)** — trend direction (overlay)
2. **Fisher Transform(10)** — confirmation (sub-pane)
3. **ATR Projection(14)** — volatility bands (overlay)
4. **Better Volume** — volume analysis (sub-pane)
5. **SMA(200)** — baseline (overlay)

Enable all five via View → Indicators or the `~` command palette → `INDICATORS`.

## ATR(14)

ATR is computed internally and used by:
- ATR Projection bands
- Renko brick size
- Risk calculator (SL distance)
- Crosshair indicator values display

## Computation Performance

All 21 indicators compute in < 10ms on 10,000 bars. Indicators are pre-computed once on load and cached in `ChartState`. Re-computed only on symbol/timeframe change.
