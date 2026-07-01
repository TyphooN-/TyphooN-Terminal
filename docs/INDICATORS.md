# TyphooN Terminal — Indicator Reference (46+ Indicators)

All indicators computed in pure Rust on `&[f64]` slices. No Web Workers, no WASM bridge. 40+ run on the GPU via wgpu compute shaders with CPU fallback. See ADR-079 for the chartable parity bundle (CMO / QStick / Disparity / BOP / StdDev) and the broader research + indicator parity surface.

## NNFX System (Baseline + Confirmation + Volume + Exit)

| Category | Indicator | Parameters | Color | Description |
|----------|-----------|-----------|-------|-------------|
| **Baseline** | SMA | 200 | Yellow | Simple Moving Average — trend direction |
| **Baseline** | KAMA | 10, 2, 30 | White | Kaufman Adaptive MA — NNFX core trend (MT5 match) |
| **Confirmation 1** | Fisher Transform | 32 | Green/Red bars | Ehlers Fisher — NNFX confirmation (MT5-style / computational match) |
| **Volume** | Better Volume | 20 | Multi-color | Climax up (green), climax down (red), high (blue), low (yellow), churn (gray) |
| **Exit** | ATR Projection | 14 | Yellow bands | Open ± ATR(14) — volatility bands |
| **Support** | Previous Candle Levels | H1/H4/D1/W1/MN1 | White/Magenta | Previous bar high/low per timeframe |
| **Support** | Supply/Demand Zones | Auto | Green/Red fill | Auto-detected from impulse candles |
| **Support** | Fractals | 5-bar | Green ▲ / Red ▼ | Bill Williams swing points |
| **Pattern** | Harmonic Patterns | Auto | Cyan XABCD | Gartley, Butterfly, Bat, Crab, Shark, Cypher, 5-0, Alt Bat, Deep Crab, Three Drives |

## Overlay Indicators (on price chart)

| Indicator | Parameters | Color | Description |
|-----------|-----------|-------|-------------|
| MTF SMA | H1/H4/D1/W1 200, W1/MN1 100 | Tomato (H1), Magenta (H4+) | Multi-timeframe SMA overlay (matching MT5) |
| ATR Projection MTF | 14 | Yellow bands | Open ± ATR on M15/H1/H4/D1/W1/MN1 horizontal levels |
| SMA | 200, 100 | Yellow, Blue | Simple Moving Average |
| EMA | 21 | Orange | Exponential Moving Average |
| KAMA | 10, 2, 30 | White | Kaufman Adaptive Moving Average (MT5 match) |
| WMA | 20 | Light purple | Weighted Moving Average |
| HMA | 20 | Cyan | Hull Moving Average |
| Bollinger Bands | 20, 2.0 | Blue + fill | Middle ± 2 std dev |
| Ichimoku Cloud | 9, 26, 52 | Multi-color | Tenkan, Kijun, Span A/B with cloud fill |
| Parabolic SAR | 0.02, 0.2 | Yellow dots | Stop-and-reverse |
| ATR Projection | 14 | Yellow bands | Open ± ATR(14) |
| Pivot Points | Daily | Multi-color | P (white), R1/R2 (red), S1/S2 (green) |

## Ehlers Indicators (John F. Ehlers DSP)

### Overlay
| Indicator | Parameters | Color | Description |
|-----------|-----------|-------|-------------|
| Super Smoother | 10 | Cyan | 2-pole recursive noise filter |
| Decycler | 20 | Orange | Trend filter (removes cycle component) |
| Instantaneous Trendline | Auto | Yellow-green | Adaptive trend following |
| MAMA / FAMA | 0.5, 0.05 | Pink / Light blue | Mesa Adaptive Moving Average + Following AMA |

### Sub-Pane
| Indicator | Parameters | Color | Range | Description |
|-----------|-----------|-------|-------|-------------|
| Even Better Sinewave | 40 | Teal | -1 to 1 | Cycle mode indicator |
| Cyber Cycle | Auto | Purple | Auto | Dominant cycle extraction |
| CG Oscillator | 10 | Orange | Auto | Center of Gravity |
| Roofing Filter | 10, 48 | Green | Auto | Bandpass for cycle isolation |

## Standard Sub-Pane Indicators

| Indicator | Parameters | Color | Range | Description |
|-----------|-----------|-------|-------|-------------|
| RSI | 14 | Yellow | 0-100 | Relative Strength Index, OB 70 / OS 30 |
| Fisher Transform | 32 (smoothing 0.5/0.5, coefficient 0.25) | Green/Red bars + signal | Auto | NNFX confirmation (MT5-style / computational match) |
| MACD | 12, 26, 9 | Blue line, orange signal | Auto | MACD + signal + histogram |
| Stochastic | 14, 3, 3 | Blue %K, Orange %D | 0-100 | OB 80 / OS 20 |
| ADX | 14 | Yellow ADX, Green DI+, Red DI- | 0-60 | Trend strength + directional |
| CCI | 20 | Orange | -200 to +200 | Commodity Channel Index |
| Williams %R | 14 | Purple | -100 to 0 | OB -20 / OS -80 |
| OBV | — | Green | Auto | On-Balance Volume |
| Momentum | 10 | Tan | Auto | Price momentum |
| Better Volume | 20 | Multi-color | Auto | NNFX volume analysis |
| Volume | — | Green/Red | Auto | Standard volume bars |

## Pattern Detection

| Pattern | Type | Description |
|---------|------|-------------|
| Gartley | Harmonic XABCD | AB=0.618 XA, XD=0.786 XA |
| Butterfly | Harmonic XABCD | AB=0.786 XA, XD=1.27 XA |
| Bat | Harmonic XABCD | AB=0.382-0.50 XA, XD=0.886 XA |
| Crab | Harmonic XABCD | AB=0.382-0.618 XA, XD=1.618 XA |
| Shark | Harmonic XABCD | AB=1.13-1.618 XA, XD=0.886 XA |
| Cypher | Harmonic XABCD | AB=0.382-0.618 XA, BC=1.13-1.414 AB |
| 5-0 | Harmonic XABCD | AB=1.13-1.618 XA, XD=0.50 BC |
| Alt Bat | Harmonic XABCD | AB=0.382 XA, XD=1.13 XA |
| Deep Crab | Harmonic XABCD | AB=0.886 XA, XD=1.618 XA |
| Three Drives | Harmonic XABCD | AB=0.618-0.786 XA, CD=0.618-0.786 BC |
| Supply/Demand Zones | Price action | Auto-detected from impulse candles (>2x ATR) |
| Fractals | Bill Williams | 5-bar swing high/low arrows |

## Performance

All 46+ indicators compute in < 20ms on 10,000 bars. Indicators pre-computed once on load, cached in ChartState. Session-persistent toggles saved to `~/.config/typhoon-terminal/session.json`.
