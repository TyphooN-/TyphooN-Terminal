# TyphooN-Terminal Indicator Reference

All indicators available in TyphooN-Terminal, organized by category. NNFX System
indicators are exact ports of MQL5 originals; Standard indicators follow textbook
definitions. All indicator math lives in `frontend/src/main.js` with hot-path
duplicates in `wasm-indicators/src/lib.rs` (Rust/WebAssembly).

---

## 1. NNFX System Indicators (9)

These are enabled by default in the indicator panel. Each is a faithful port of
an MQL5 `.mqh` source from the companion `MQL5-NNFX-Risk_Management_System`
repository.

### 1.1 MTF_MA (Multi-Timeframe Moving Average)

| Field | Value |
|---|---|
| **Checkbox** | `MTF_MA 200 SMA` (checked), `MTF_MA 100 SMA` (checked) |
| **MQL5 Source** | `Indicators/MTF_MA.mqh` |
| **Default Params** | period=200 or 100, type=SMA |
| **Price Type** | Close |
| **Colors** | Current TF 200: Yellow `#FFFF00`, 100: Magenta `#FF00FF`; HTF H1: Tomato `#FF6347`, H4/D1/W1: Magenta `#FF00FF` |
| **Algorithm** | Simple Moving Average computed on current chart data and on each higher timeframe (H1, H4, D1, W1). HTF values are projected onto the current chart via forward-fill of the most recent HTF bar's SMA value at each chart timestamp. |
| **MQL5 Parity** | Full parity. Uses `calcSMA()` identically; HTF projection matches MQL5 `iMA()` on higher-timeframe bars. |

### 1.2 MultiKAMA (Kaufman Adaptive Moving Average)

| Field | Value |
|---|---|
| **Checkbox** | `MultiKAMA (10/2/30)` (checked) |
| **MQL5 Source** | `Indicators/KAMA.mqh` |
| **Default Params** | period=10, fastEMA=2, slowEMA=30 |
| **Price Type** | Close |
| **Colors** | Current TF: White `#FFFFFF` (width 2); HTF: H1=Tomato `#FF6347`, H4/D1/W1=Magenta `#FF00FF` (width 2) |
| **Algorithm** | 1. Efficiency Ratio: `ER = abs(close[i] - close[i-period]) / sum(abs(close[j] - close[j-1]))` over period. 2. Smoothing constant: `SC = (ER * (fastSC - slowSC) + slowSC)^2` where `fastSC = 2/(fast+1)`, `slowSC = 2/(slow+1)`. 3. `KAMA[i] = SC * (close[i] - KAMA[i-1]) + KAMA[i-1]`. Multi-timeframe: computed on each HTF bar series and projected via forward-fill. |
| **MQL5 Parity** | Full parity. Algorithm matches `CalculateER()` and the `SSC^2` smoothing in `KAMA.mqh` exactly. |

### 1.3 Previous Candle Levels (MTF)

| Field | Value |
|---|---|
| **Checkbox** | `Prev Candle Levels (MTF)` (checked) |
| **MQL5 Source** | `Indicators/PreviousCandleLevels.mqh` |
| **Default Params** | None (uses H1/H4/D1/W1/MN1 timeframes) |
| **Price Type** | High/Low |
| **Colors** | H1/H4: White `#FFFFFF` (width 2, SOLID); D1/W1/MN1: Magenta `#FF00FF` (width 2, SOLID). Current D1/W1 Judas levels: Magenta. |
| **Algorithm** | For each higher timeframe, draws horizontal lines at the previous completed bar's high and low. Also draws the current D1/W1 bar's running high/low (Judas levels) when the chart timeframe is below H1. TF filtering mirrors MQL5: chart < H1 shows all HTF; H1-H4 shows D1+; D1 shows W1+; W1+ shows MN1 only. |
| **MQL5 Parity** | Full parity. Timeframe visibility rules match `DrawLines()` logic. Judas level recalculation on break of D1 high/low matches MQL5 intrabar update logic. |

### 1.4 ATR Projection (MTF)

| Field | Value |
|---|---|
| **Checkbox** | `ATR Projection (MTF)` (checked) |
| **MQL5 Source** | `Indicators/ATR_Projection.mqh` |
| **Default Params** | period=14 |
| **Price Type** | True Range (H-L, abs(H-prevC), abs(L-prevC)) |
| **Colors** | Yellow `#FFFF00`, STYLE_DOT, width 2 |
| **Algorithm** | 1. True Range for each bar. 2. Wilder-smoothed ATR: `ATR = (prevATR * (period-1) + TR) / period`. 3. Projection: `currentOpen + ATR` (upper) and `currentOpen - ATR` (lower) drawn as horizontal dotted lines. Multi-timeframe: ATR computed on each HTF, then projects `HTF_open +/- HTF_ATR`. HTF lookbacks match MQL5: H1=12 bars, H4=11, D1=7, W1=4. |
| **MQL5 Parity** | Full parity. ATR smoothing, projection formula, and HTF lookback windows match `ATR_Projection.mqh` exactly. Font color escalation (Magenta when lower-TF ATR exceeds higher-TF) is displayed in the MTF grid, not as a separate indicator. |

### 1.5 Ehlers Fisher Transform

| Field | Value |
|---|---|
| **Checkbox** | `Ehlers Fisher (32)` (checked) |
| **MQL5 Source** | `Indicators/EhlersFisherTransform.mqh` |
| **Default Params** | period=32, calcMode=calc_no (exclude current bar from H/L range) |
| **Price Type** | Median `(high + low) / 2` (PRICE_MEDIAN) |
| **Colors** | Bullish (fisher > signal): MediumSeaGreen `#3CB371` (width 2); Bearish (fisher < signal): OrangeRed `#FF4500` (width 2); Neutral: DarkGray `#A9A9A9`. Signal line: DarkGray (width 1). |
| **Algorithm** | 1. Find highest high and lowest low over lookback period (excluding current bar's H/L in `calc_no` mode). 2. Normalize median price to 0-1, center to -1..+1: `os = 2 * ((price - minL) / range - 0.5)`. 3. Smooth: `smoothed = 0.5 * os + 0.5 * prevSmoothed`, clamped to (-0.999, 0.999). 4. Fisher Transform: `FT = 0.25 * ln((1 + smoothed) / (1 - smoothed)) + 0.5 * prevFisher`. 5. Signal = previous bar's Fisher value. 6. Color determined by Fisher vs Signal comparison. Rendered as a color-changing line in a separate pane, split into contiguous same-color segments. |
| **MQL5 Parity** | Full parity. Normalization, smoothing coefficients (0.5/0.5), Fisher log transform with 0.25/0.5 weights, and color logic match `EhlersFisherTransform.mqh`. The `calc_no` mode (excluding current bar H/L from range) is the default in both MQL5 and JS. |

### 1.6 BetterVolume

| Field | Value |
|---|---|
| **Checkbox** | `BetterVolume` (checked) |
| **MQL5 Source** | `Indicators/BetterVolume.mqh` |
| **Default Params** | lookback=20 |
| **Price Type** | OHLCV (all fields) |
| **Colors** | Yellow `#FFFF00` = Low Volume; Red `#FF0000` = Climax Up; White `#FFFFFF` = Climax Down; Green `#00FF00` = Churn; Magenta `#FF00FF` = Climax+Churn; SteelBlue `#4682B4` = Normal |
| **Algorithm** | Emini-Watch volume classification. 1. **Estimate buy/sell volume** from bar shape: bullish bars attribute more volume to buying via `buyVol = (range / (2*range + open - close)) * totalVol`. 2. Compute per-bar metrics: `buyRange`, `sellRange`, `volDivRange`, `sellDivRange`, `buyDivRange`. 3. Compare against lookback-period extremes. 4. Classification flags: LowVol (vol <= lowest), ClimaxUp (bullish + highest buyRange or lowest sellDivR), ClimaxDn (bearish + highest sellRange or lowest buyDivR), Churn (highest volDivRange). 5. Priority: Climax+Churn > LowVol > ClimaxUp > ClimaxDn > Churn > Normal. Rendered as a color-coded volume histogram. |
| **MQL5 Parity** | Partial parity. The JS port implements 1-bar analysis only. The MQL5 version also has optional 2-bar combined analysis (`InpUse2Bars`), average volume line (`InpShowAvg`), per-classification enable/disable toggles, and `VOLUME_REAL` vs `VOLUME_TICK` selection. These MQL5-only features are not yet ported. |

### 1.7 Supply/Demand Zones

| Field | Value |
|---|---|
| **Checkbox** | `Supply/Demand Zones` (checked) |
| **MQL5 Source** | `Indicators/SupplyDemand.mqh` (fractal-based zone detection) |
| **Default Params** | fractalLookback=5, backLimit=1000 |
| **Price Type** | OHLC |
| **Colors** | Supply zones: Red tones; Demand zones: Green tones. Opacity varies by strength (untested, tested, proven). Broken zones are filtered out. |
| **Algorithm** | 1. **Fractal detection**: A bar is a fractal high if its high exceeds all bars within `fractalLookback` on both sides. Similarly for fractal lows. 2. **Zone creation**: Supply zone at fractal high = `[min(close,open) .. high]`. Demand zone at fractal low = `[low .. max(close,open)]`. 3. **Zone testing**: Scan subsequent bars for overlap. If price closes beyond zone boundary, mark as "broken". Count touches for strength tiers (untested/tested/proven). 4. **Merge overlapping** same-type zones. 5. Filter out broken zones. Rendered as colored rectangular areas on the price chart. |
| **MQL5 Parity** | Full parity. Fractal detection, zone creation, testing, merging, and strength classification match MQL5 `IsFractalHigh`/`IsFractalLow`, `FindZones`, `TestZones`, and `MergeZones`. |

### 1.8 Auto Fibonacci

| Field | Value |
|---|---|
| **Checkbox** | `Auto Fibonacci` (checked) |
| **MQL5 Source** | Custom (fractal-based swing detection, no direct MQL5 equivalent) |
| **Default Params** | fractalLookback=10 |
| **Price Type** | High/Low |
| **Colors** | Retracement levels: semi-transparent horizontal lines. Extension levels: separate color. |
| **Algorithm** | 1. Detect fractal swing highs and lows (bars whose high/low exceeds all neighbors within lookback). 2. From the recent 60% of the chart, find the highest swing high and lowest swing low. 3. Determine trend: if swing low precedes swing high = bullish; otherwise bearish. 4. Compute standard Fibonacci levels from the swing range: retracements at 0%, 23.6%, 38.2%, 50%, 61.8%, 78.6%, 100%, and extensions at 127.2%, 161.8%, 200%, 261.8%, 361.8%, 423.6%. 5. Bull: retrace from high toward low, extend above high. Bear: retrace from low toward high, extend below low. |
| **MQL5 Parity** | N/A (terminal-original indicator, not a port). |

### 1.9 RVOL (Relative Volume)

| Field | Value |
|---|---|
| **Checkbox** | `RVOL (10)` (in Standard section, but ported from MQL5) |
| **MQL5 Source** | `Indicators/Retired/RVOL.mqh` |
| **Default Params** | avgDays=10 |
| **Price Type** | Volume |
| **Colors** | Green `#00FF00` = above average (RVOL > 1.25); Orange `#FFA500` = average (0.8-1.25); Red `#FF0000` = below average (< 0.8) |
| **Algorithm** | Sliding window average: `RVOL = currentVolume / mean(volume over previous avgDays bars)`. Color thresholds at 1.25 (above average) and 0.8 (below average). Rendered as a color-coded histogram. |
| **MQL5 Parity** | Full parity. Sliding window O(n) calculation and color thresholds (1.25/0.8) match `RVOL.mqh` exactly. The MQL5 version also supports `VOLUME_REAL` vs `VOLUME_TICK` selection (not yet ported). |

---

## 2. Standard Indicators (21)

These are disabled by default and use textbook-standard algorithms. Listed in UI
order.

### 2.1 EMA (Exponential Moving Average)

| Param | Default |
|---|---|
| Period | 50 or 200 |
| **Algorithm** | `EMA = close * k + prevEMA * (1 - k)` where `k = 2 / (period + 1)`. Seed: first bar's close. Output begins at bar `period - 1`. |
| **Colors** | EMA(50): Blue `#2196f3`; EMA(200): Orange `#ff9800` |

### 2.2 SMA (Simple Moving Average)

| Param | Default |
|---|---|
| Period | 50 |
| **Algorithm** | Arithmetic mean of the last `period` close prices. |
| **Colors** | SMA(200): Yellow `#FFFF00`; SMA(50): Blue `#2196f3` |

### 2.3 DEMA (Double Exponential Moving Average)

| Param | Default |
|---|---|
| Period | 21 |
| **Algorithm** | `DEMA = 2 * EMA(close, period) - EMA(EMA(close, period), period)`. Reduces lag compared to standard EMA. |
| **Color** | Green `#00e676` |

### 2.4 RSI (Relative Strength Index)

| Param | Default |
|---|---|
| Period | 14 |
| **Algorithm** | Wilder's smoothed RSI. `avgGain = (prevAvgGain * (period-1) + gain) / period`. `RS = avgGain / avgLoss`. `RSI = 100 - 100 / (1 + RS)`. Overbought/oversold levels drawn at 70/30. |
| **Color** | Purple `#ab47bc`; OB line: red, OS line: green (both dashed) |
| **Pane** | Separate price scale (`rsi`), bottom 18% of chart |

### 2.5 MACD (Moving Average Convergence Divergence)

| Param | Default |
|---|---|
| Fast/Slow/Signal | 12 / 26 / 9 |
| **Algorithm** | `MACD = EMA(12) - EMA(26)`. Signal = EMA(9) of MACD line. Histogram = MACD - Signal. |
| **Colors** | MACD line: Blue `#2196f3`; Signal: Orange `#ff9800`; Histogram: Teal `#26a69a` (positive), Red `#ef5350` (negative) |
| **Pane** | Separate price scale (`macd`), bottom 13% of chart |

### 2.6 Bollinger Bands

| Param | Default |
|---|---|
| Period | 20 |
| **Algorithm** | `Upper = SMA + 2 * StdDev(close, period)`. `Lower = SMA - 2 * StdDev`. Population standard deviation. |
| **Color** | Purple `#9c27b0` (dashed lines) |

### 2.7 ATR (Average True Range)

| Param | Default |
|---|---|
| Period | 14 |
| **Algorithm** | True Range = max(H-L, abs(H-prevC), abs(L-prevC)). Wilder smoothing: `ATR = (prevATR * (period-1) + TR) / period`. |
| **Color** | Deep orange `#ff5722` |
| **Pane** | Separate price scale (`atr`), bottom 13% of chart |

### 2.8 VWAP (Volume Weighted Average Price)

| Param | Default |
|---|---|
| (none) | Resets daily |
| **Algorithm** | `VWAP = cumulative(TP * volume) / cumulative(volume)` where `TP = (H + L + C) / 3`. Resets at each new calendar day. Falls back to volume=1 if volume data is missing. |
| **Color** | Pink `#ff4081` (width 2) |

### 2.9 Volume (Basic)

| Param | Default |
|---|---|
| (none) | N/A |
| **Algorithm** | Raw volume as a histogram. Green when close > open (up bar), red when close < open (down bar). |
| **Pane** | Separate price scale (`vol`) |

### 2.10 Stochastic Oscillator

| Param | Default |
|---|---|
| K Period / D Period / Smooth | 14 / 3 / 3 |
| **Algorithm** | `rawK = ((close - lowest_low) / (highest_high - lowest_low)) * 100` over K period. `%K` = SMA(rawK, smooth). `%D` = SMA(%K, D period). |
| **Colors** | %K: Blue `#2196f3`; %D: Orange `#ff9800` |
| **Pane** | Separate price scale (`stoch`) |

### 2.11 CCI (Commodity Channel Index)

| Param | Default |
|---|---|
| Period | 20 |
| **Algorithm** | `TP = (H + L + C) / 3`. `CCI = (TP - SMA(TP)) / (0.015 * MeanDeviation(TP))`. Standard Lambert constant 0.015. |
| **Color** | Teal `#009688` |
| **Pane** | Separate price scale (`cci`) |

### 2.12 ADX (Average Directional Index)

| Param | Default |
|---|---|
| Period | 14 |
| **Algorithm** | 1. Directional Movement: `+DM` = up move if positive and > down move, else 0. `-DM` = down move if positive and > up move, else 0. 2. Wilder-smoothed `+DI = smoothed(+DM) / smoothed(TR) * 100`. 3. `DX = abs(+DI - -DI) / (+DI + -DI) * 100`. 4. `ADX = Wilder_smooth(DX, period)`. |
| **Colors** | ADX: Yellow `#ffeb3b`; +DI: Green `#4caf50`; -DI: Red `#f44336` |
| **Pane** | Separate price scale (`adx`) |

### 2.13 Williams %R

| Param | Default |
|---|---|
| Period | 14 |
| **Algorithm** | `%R = ((highest_high - close) / (highest_high - lowest_low)) * -100`. Range: -100 to 0. |
| **Color** | Purple `#ab47bc` |
| **Pane** | Separate price scale (`williams`) |

### 2.14 Ichimoku Cloud

| Param | Default |
|---|---|
| Tenkan / Kijun / Senkou | 9 / 26 / 52 |
| **Algorithm** | `Tenkan-sen = (highest_high + lowest_low) / 2` over 9 periods. `Kijun-sen` = same over 26. `Senkou Span A = (Tenkan + Kijun) / 2`. `Senkou Span B = midpoint of 52-period H/L`. `Chikou Span = close displaced 26 bars back`. Cloud filled between Senkou A and B. |
| **Colors** | Tenkan: Red `#ef5350`; Kijun: Blue `#42a5f5`; Senkou A: Green `#66bb6a`; Senkou B: Red `#ef5350`; Cloud: green/red fill |

### 2.15 Parabolic SAR

| Param | Default |
|---|---|
| Step / MaxStep | 0.02 / 0.2 |
| **Algorithm** | Wilder's Parabolic SAR. `SAR = prevSAR + AF * (EP - prevSAR)`. AF starts at `step`, increments by `step` each new extreme, capped at `maxStep`. Reversal occurs when price crosses SAR. SAR constrained to prior two bars' extremes. |
| **Colors** | Green `#4caf50` (long), Red `#f44336` (short) |

### 2.16 OBV (On-Balance Volume)

| Param | Default |
|---|---|
| (none) | N/A |
| **Algorithm** | Cumulative: if close > prevClose, add volume; if close < prevClose, subtract volume; if equal, no change. |
| **Color** | White `#e0e0e0` |
| **Pane** | Separate price scale (`obv`) |

### 2.17 Momentum

| Param | Default |
|---|---|
| Period | 10 |
| **Algorithm** | `Momentum = close[i] - close[i - period]`. Simple price difference. |
| **Color** | Amber `#ffc107` |
| **Pane** | Separate price scale (`momentum`) |

### 2.18 WMA (Weighted Moving Average)

| Param | Default |
|---|---|
| Period | 20 |
| **Algorithm** | `WMA = sum(close[i] * weight[i]) / sum(weights)` where `weight = position + 1` (most recent bar has highest weight). Denominator = `period * (period + 1) / 2`. |
| **Color** | Cyan `#00bcd4` |

### 2.19 HMA (Hull Moving Average)

| Param | Default |
|---|---|
| Period | 20 |
| **Algorithm** | 1. `WMA_half = WMA(close, floor(period/2))`. 2. `WMA_full = WMA(close, period)`. 3. `diff = 2 * WMA_half - WMA_full`. 4. `HMA = WMA(diff, floor(sqrt(period)))`. Significantly reduced lag compared to SMA/EMA. |
| **Color** | Light cyan `#00e5ff` |

---

## 3. MT5 Parity Indicators (9)

These indicators match their MetaTrader 5 built-in counterparts. They are
implemented in `frontend/src/main.js` and disabled by default.

### 3.1 Alligator (Bill Williams)

| Param | Default |
|---|---|
| Jaw Period | 13 |
| Teeth Period | 8 |
| Lips Period | 5 |
| **Price Type** | Median `(high + low) / 2` |
| **Algorithm** | Three Smoothed Moving Averages (SMMA) on median price, each shifted forward by a fixed offset. Jaw = SMMA(13) shifted +8 bars. Teeth = SMMA(8) shifted +5 bars. Lips = SMMA(5) shifted +3 bars. SMMA: `S[i] = (S[i-1] * (period-1) + price[i]) / period`, seeded with the SMA of the first `period` values. |
| **Render** | Three overlay lines on the price chart |
| **MT5 Parity** | Full parity. SMMA smoothing and forward shift offsets match MT5 `iAlligator()`. |

### 3.2 Awesome Oscillator (Bill Williams)

| Param | Default |
|---|---|
| (none) | Fixed SMA periods 5 / 34 |
| **Price Type** | Median `(high + low) / 2` |
| **Algorithm** | `AO = SMA(5, median) - SMA(34, median)`. Output begins at bar 33. Bar color: green `#4caf50` when AO is rising (current > previous), red `#f44336` when falling. |
| **Render** | Color-coded histogram in a separate pane |
| **MT5 Parity** | Full parity. SMA periods and color logic match MT5 `iAO()`. |

### 3.3 MFI (Money Flow Index)

| Param | Default |
|---|---|
| Period | 14 |
| **Price Type** | Typical Price `(H + L + C) / 3` with Volume |
| **Algorithm** | Volume-weighted RSI. 1. `TP = (H + L + C) / 3`. 2. `MoneyFlow = TP * volume`. 3. Over the lookback period, sum positive money flow (when TP rises) and negative money flow (when TP falls). 4. `MFI = 100 - 100 / (1 + posFlow / negFlow)`. Falls back to `volume=1` when volume data is missing. |
| **Render** | Line in a separate pane (range 0-100) |
| **MT5 Parity** | Full parity. Algorithm matches MT5 `iMFI()`. |

### 3.4 Force Index

| Param | Default |
|---|---|
| Period | 13 |
| **Price Type** | Close with Volume |
| **Algorithm** | 1. `RawForce[i] = (close[i] - close[i-1]) * volume[i]`. 2. Apply EMA smoothing: `EMA = raw * k + prevEMA * (1 - k)` where `k = 2 / (period + 1)`. Output begins after `period` bars. Falls back to `volume=1` when volume data is missing. |
| **Render** | Line in a separate pane |
| **MT5 Parity** | Full parity. EMA smoothing of raw force matches MT5 `iForce()`. |

### 3.5 Envelopes

| Param | Default |
|---|---|
| Period | 20 |
| Deviation | 0.1 (10%) |
| **Price Type** | Close (via SMA) |
| **Algorithm** | `Upper = SMA(period) * (1 + deviation)`. `Lower = SMA(period) * (1 - deviation)`. Uses `calcSMA()` internally for the center line. |
| **Render** | Two overlay lines (upper/lower bands) on the price chart |
| **MT5 Parity** | Full parity. Percentage-based envelope around SMA matches MT5 `iEnvelopes()`. |

### 3.6 Standard Deviation

| Param | Default |
|---|---|
| Period | 20 |
| **Price Type** | Close |
| **Algorithm** | Population standard deviation over a rolling window: `StdDev = sqrt(sum(close^2) / period - mean^2)`. Uses the computational formula `sqrt(E[X^2] - (E[X])^2)`. |
| **Render** | Line in a separate pane |
| **MT5 Parity** | Full parity. Population StdDev calculation matches MT5 `iStdDev()`. |

### 3.7 Chaikin Oscillator

| Param | Default |
|---|---|
| Fast Period | 3 |
| Slow Period | 10 |
| **Price Type** | OHLCV (all fields) |
| **Algorithm** | 1. Accumulation/Distribution Line: `MFM = ((close - low) - (high - close)) / range`. `ADL[i] = ADL[i-1] + MFM * volume`. 2. Chaikin Oscillator = `EMA(fastP, ADL) - EMA(slowP, ADL)`. Falls back to `volume=1` when volume data is missing. |
| **Render** | Line in a separate pane |
| **MT5 Parity** | Full parity. A/D Line accumulation and dual-EMA difference match MT5 `iChaikin()`. |

### 3.8 DeMarker

| Param | Default |
|---|---|
| Period | 14 |
| **Price Type** | High/Low |
| **Algorithm** | 1. `DeMax = high[i] - high[i-1]` if positive, else 0. 2. `DeMin = low[i-1] - low[i]` if positive, else 0. 3. Sum DeMax and DeMin over the lookback period. 4. `DeM = DeMax_sum / (DeMax_sum + DeMin_sum)`. Returns 0.5 when denominator is zero. Range: 0 to 1. |
| **Render** | Line in a separate pane (range 0-1) |
| **MT5 Parity** | Full parity. DeMax/DeMin logic and summation match MT5 `iDeMarker()`. |

### 3.9 Fractals (Bill Williams)

| Param | Default |
|---|---|
| Lookback | 2 |
| **Price Type** | High/Low |
| **Algorithm** | A bar is a fractal high if its high strictly exceeds the highs of all bars within `lookback` distance on both sides. Similarly, a fractal low if its low is strictly below all neighbors' lows within `lookback`. Fractal highs are marked with a red `#ff5722` down-arrow above the bar. Fractal lows are marked with a green `#4caf50` up-arrow below the bar. |
| **Render** | Marker overlay on the price chart (above/below bars) |
| **MT5 Parity** | Full parity. Bilateral comparison with `lookback=2` matches MT5 `iFractals()` default (5-bar pattern = 2 bars each side). |

---

## 4. Wasm Implementation Status

The `wasm-indicators/` crate (`typhoon-indicators`) provides Rust/WebAssembly
implementations of select indicators for batch computation paths (optimizer grid
search, multi-symbol scanner). The JS implementations remain authoritative for
real-time chart rendering.

| Indicator | JS Function | Wasm Function | Wasm Status |
|---|---|---|---|
| SMA | `calcSMA()` | `wasm_sma()` | Available |
| EMA | `calcEMA()` | `wasm_ema()` | Available |
| KAMA | `calcKAMA()` | `wasm_kama()` | Available |
| RSI | `calcRSI()` | `wasm_rsi()` | Available |
| Ehlers Fisher | `calcEhlersFisher()` | `wasm_fisher()` | Available |
| ATR | `calcATR()` | `wasm_atr()` | Available |
| MACD | `calcMACD()` | `wasm_macd()` | Available |
| Bollinger Bands | `calcBollinger()` | `wasm_bollinger()` | Available |
| DEMA | `calcDEMA()` | -- | Not yet ported |
| VWAP | `calcVWAP()` | -- | Not yet ported |
| RVOL | `calcRVOL()` | -- | Not yet ported |
| Stochastic | `calcStochastic()` | -- | Not yet ported |
| CCI | `calcCCI()` | -- | Not yet ported |
| ADX | `calcADX()` | -- | Not yet ported |
| Williams %R | `calcWilliamsR()` | -- | Not yet ported |
| Ichimoku | `calcIchimoku()` | -- | Not yet ported |
| Parabolic SAR | `calcParabolicSAR()` | -- | Not yet ported |
| OBV | `calcOBV()` | -- | Not yet ported |
| Momentum | `calcMomentum()` | -- | Not yet ported |
| WMA | `calcWMA()` | -- | Not yet ported |
| HMA | `calcHMA()` | -- | Not yet ported |
| BetterVolume | `calcBetterVolume()` | -- | Not yet ported |
| Supply/Demand | `calcSupplyDemandZones()` | -- | Not yet ported |
| Auto Fibonacci | `calcAutoFibonacci()` | -- | Not yet ported |
| Prev Candle Levels | `calcPrevCandleLevels()` | -- | Not yet ported |
| ATR Projection | `calcATRProjection()` | -- | Not yet ported |
| Alligator | `calcAlligator()` | -- | Not yet ported |
| Awesome Oscillator | `calcAwesomeOscillator()` | -- | Not yet ported |
| MFI | `calcMFI()` | -- | Not yet ported |
| Force Index | `calcForceIndex()` | -- | Not yet ported |
| Envelopes | `calcEnvelopes()` | -- | Not yet ported |
| Standard Deviation | `calcStdDev()` | -- | Not yet ported |
| Chaikin Oscillator | `calcChaikin()` | -- | Not yet ported |
| DeMarker | `calcDeMarker()` | -- | Not yet ported |
| Fractals | `calcFractals()` | -- | Not yet ported |

The Wasm crate also provides batch utilities not listed above:

- `wasm_backtest_sma()` -- SMA crossover backtest on flat bar data
- `wasm_optimize_sma()` -- Grid-search SMA optimization (fast/slow period sweep)

**Build**: `cd wasm-indicators && wasm-pack build --target web --release`

**Data format**: Flat `f64` arrays with 5 fields per bar: `[open, high, low, close, volume, ...]`

---

## 5. Custom Plugin System

TyphooN-Terminal supports user-authored indicator plugins loaded at runtime from
the filesystem. Plugins are JavaScript files stored in:

```
~/.config/typhoon-terminal/indicators/
```

### Plugin Discovery

The backend exposes two Tauri commands:

- `list_custom_indicators` -- returns a JSON array of available plugin metadata
- `get_custom_indicator_source` -- returns the JS source of a named plugin

The frontend calls these on demand when the user clicks "Load Plugins" in the
indicator panel.

### Plugin Format

A plugin file must evaluate to a JavaScript object with a `calculate()` function.
Minimal example:

```javascript
// ~/.config/typhoon-terminal/indicators/my_sma_cross.js
({
  name: "SMA Cross",
  params: { fast: 10, slow: 50 },
  calculate(data, params) {
    // data = array of { time, open, high, low, close, volume }
    // Must return array of { time, value } for a single line series.
    const fast = [], slow = [];
    for (let i = params.slow - 1; i < data.length; i++) {
      let fSum = 0, sSum = 0;
      for (let j = 0; j < params.fast; j++) fSum += data[i - j].close;
      for (let j = 0; j < params.slow; j++) sSum += data[i - j].close;
      fast.push(fSum / params.fast);
      slow.push(sSum / params.slow);
    }
    // Return the fast SMA line
    return fast.map((v, idx) => ({
      time: data[idx + params.slow - 1].time,
      value: v,
    }));
  },
})
```

### Plugin Lifecycle

1. **Load**: User clicks "Load Plugins" in the indicator panel. The frontend
   calls `list_custom_indicators` to enumerate available plugins.
2. **Activate**: When a plugin checkbox is toggled on, `activateCustomPlugin()`
   fetches the source via `get_custom_indicator_source`, evaluates it in a
   sandboxed `new Function()` scope, and calls `plugin.calculate(data, params)`.
3. **Render**: If `calculate()` returns an array of `{ time, value }` objects,
   a line series is added to the chart with an auto-assigned color from the
   palette: `#e040fb`, `#40c4ff`, `#ffab40`, `#69f0ae`, `#ff5252`.
4. **Deactivate**: Toggling the checkbox off calls `removeCustomPlugin()`, which
   removes all chart series associated with the plugin.
5. **Re-apply**: Plugins are automatically re-applied when chart data changes
   (symbol switch, timeframe change).

### Plugin API Contract

| Field | Type | Required | Description |
|---|---|---|---|
| `name` | string | Yes | Display name in the indicator panel |
| `params` | object | No | Default parameters passed to `calculate()` |
| `calculate(data, params)` | function | Yes | Receives OHLCV bar array and params; must return `[{time, value}]` or `null` |

### Auto-Trade Plugins

Plugins can also export an `onSignal()` function for the `AUTOTRADE` command,
enabling strategy-driven live order placement through the same plugin system.
These are selected in the Auto-Trade dialog's plugin dropdown.
