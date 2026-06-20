use serde::{Deserialize, Serialize};

// Price-transform, directional-movement, rate-of-change, bands, regression, and volume-indicator research types
/// TA-Lib AVGPRICE — `(open + high + low + close) / 4`.
/// Simplest price-transform primitive: the four-component OHLC average.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AvgpriceSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub avgprice: f64, // (O+H+L+C) / 4
    pub avgprice_prev: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub delta_pct: f64,         // (avgprice - close) / close * 100
    pub avgprice_label: String, // ABOVE_CLOSE / NEAR_CLOSE / BELOW_CLOSE / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib MEDPRICE — `(high + low) / 2`.
/// Range-midpoint primitive — the simple median of the bar's range.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MedpriceSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub medprice: f64, // (H+L) / 2
    pub medprice_prev: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub delta_pct: f64,         // (medprice - close) / close * 100
    pub medprice_label: String, // ABOVE_MID / AT_MID / BELOW_MID / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib TYPPRICE — `(high + low + close) / 3`.
/// Typical price — used as the input for CCI and several VWAP variants.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TypPriceSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub typprice: f64, // (H+L+C) / 3
    pub typprice_prev: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub delta_pct: f64,         // (typprice - close) / close * 100
    pub typprice_label: String, // ABOVE_CLOSE / NEAR_CLOSE / BELOW_CLOSE / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib WCLPRICE — `(high + low + 2 × close) / 4`.
/// Weighted close — double-weights the close price for close-biased transforms.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WclPriceSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub wclprice: f64, // (H+L+2C) / 4
    pub wclprice_prev: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub delta_pct: f64,         // (wclprice - close) / close * 100
    pub wclprice_label: String, // ABOVE_CLOSE / NEAR_CLOSE / BELOW_CLOSE / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib VARIANCE — sample variance of close over N bars.
/// Statistical variance primitive: `σ² = Σ(x − μ)² / N` (population form, matching TA-Lib default).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VarianceSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 5 (TA-Lib default)
    pub mean: f64,
    pub variance: f64, // population variance Σ(x-μ)²/N
    pub variance_prev: f64,
    pub stddev: f64, // sqrt(variance)
    pub cv: f64,     // stddev / |mean| × 100 (coefficient of variation %)
    pub last_close: f64,
    pub variance_label: String, // HIGH_VOL / ELEVATED / NORMAL / LOW_VOL / INSUFFICIENT_DATA
    pub note: String,
}

// ── Research section ──
/// TA-Lib PLUS_DI — Wilder's Positive Directional Indicator.
/// `+DI = 100 × (Wilder-smoothed +DM) / ATR` over 14-bar default.
/// Measures upward directional movement strength: paired with −DI it
/// forms the crossover signal under Wilder's Directional Movement System
/// and feeds DX / ADX / ADXR.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PlusDiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 14
    pub plus_di: f64,
    pub plus_di_prev: f64,
    pub minus_di: f64, // for crossover context
    pub atr: f64,      // Wilder-smoothed true range
    pub last_close: f64,
    pub plus_di_label: String, // BULL_DOMINANT / BULL_LEAN / NEUTRAL / BEAR_LEAN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib MINUS_DI — Wilder's Negative Directional Indicator.
/// `−DI = 100 × (Wilder-smoothed −DM) / ATR` over 14-bar default.
/// Measures downward directional movement strength; mirror primitive of
/// +DI under Wilder's DM System. Distinct from +DI's bull framing.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MinusDiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 14
    pub minus_di: f64,
    pub minus_di_prev: f64,
    pub plus_di: f64, // for crossover context
    pub atr: f64,     // Wilder-smoothed true range
    pub last_close: f64,
    pub minus_di_label: String, // BEAR_DOMINANT / BEAR_LEAN / NEUTRAL / BULL_LEAN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib PLUS_DM — Wilder's raw Positive Directional Movement.
/// `+DM_t = max(0, H_t − H_{t−1})` only when that up-move exceeds
/// `L_{t−1} − L_t`. Wilder-smoothed via the standard recursion
/// `S_t = S_{t−1} − S_{t−1}/period + +DM_t`, the direct upstream of
/// +DI (divides by ATR to normalise to 0–100).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PlusDmSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,         // 14
    pub plus_dm_raw: f64,      // latest bar's raw +DM
    pub plus_dm_smoothed: f64, // Wilder-smoothed Σ(+DM)
    pub plus_dm_smoothed_prev: f64,
    pub up_move: f64,   // H_t − H_{t−1}
    pub down_move: f64, // L_{t−1} − L_t
    pub last_close: f64,
    pub plus_dm_label: String, // BULL_PRESSURE / BULL_SOFT / NEUTRAL / BEAR_PRESSURE / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib MINUS_DM — Wilder's raw Negative Directional Movement.
/// `−DM_t = max(0, L_{t−1} − L_t)` only when that down-move exceeds
/// `H_t − H_{t−1}`. Wilder-smoothed; direct upstream of −DI.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MinusDmSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,          // 14
    pub minus_dm_raw: f64,      // latest bar's raw −DM
    pub minus_dm_smoothed: f64, // Wilder-smoothed Σ(−DM)
    pub minus_dm_smoothed_prev: f64,
    pub up_move: f64,   // H_t − H_{t−1}
    pub down_move: f64, // L_{t−1} − L_t
    pub last_close: f64,
    pub minus_dm_label: String, // BEAR_PRESSURE / BEAR_SOFT / NEUTRAL / BULL_PRESSURE / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib DX — Wilder's Directional Movement Index.
/// `DX = 100 × |+DI − −DI| / (+DI + −DI)` — the unsmoothed directional
/// dispersion that feeds ADX and ADXR via a
/// further Wilder smoothing. DX alone is a raw directional-purity
/// indicator: high when +DI and −DI diverge, regardless of sign.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DxSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 14
    pub dx: f64,       // 100·|+DI − −DI|/(+DI + −DI)
    pub dx_prev: f64,
    pub plus_di: f64,
    pub minus_di: f64,
    pub last_close: f64,
    pub dx_label: String, // STRONG_DIR / DIR / WEAK_DIR / NO_DIR / INSUFFICIENT_DATA
    pub note: String,
}

// ── Research section ──
/// TA-Lib ROC — raw Rate of Change `close_t − close_{t−n}` (period 10).
/// Raw price delta; distinct from ROCP (percentage) and ROCR (ratio).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RocSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 10
    pub roc: f64,      // close_t − close_{t-n}
    pub roc_prev: f64,
    pub close_now: f64,
    pub close_lag: f64, // close_{t-n}
    pub last_close: f64,
    pub roc_label: String, // STRONG_UP / UP / NEUTRAL / DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib ROCP — Rate of Change Percentage `(close_t − close_{t−n}) / close_{t−n}`.
/// The "percentage change" form used widely in risk-return math.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RocpSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 10
    pub rocp: f64,     // (close_t − close_{t-n}) / close_{t-n}  (unitless)
    pub rocp_prev: f64,
    pub rocp_pct: f64, // rocp × 100 (percent display)
    pub close_now: f64,
    pub close_lag: f64,
    pub last_close: f64,
    pub rocp_label: String, // STRONG_UP / UP / NEUTRAL / DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib ROCR — Rate of Change Ratio `close_t / close_{t−n}` (period 10).
/// Ratio-form rate of change — 1.0 is no change, >1 up, <1 down.
/// Direct input for compounding return aggregations.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RocrSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 10
    pub rocr: f64,     // close_t / close_{t-n}
    pub rocr_prev: f64,
    pub close_now: f64,
    pub close_lag: f64,
    pub last_close: f64,
    pub rocr_label: String, // STRONG_UP / UP / NEUTRAL / DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib ROCR100 — Rate of Change Ratio ×100 `100 · close_t / close_{t−n}`.
/// 100 = no change, >100 up, <100 down. Scales ROCR to an index-like
/// band directly comparable to CCI / PPO / ADX with zero unit-mismatch.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Rocr100Snapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 10
    pub rocr100: f64,  // 100 · close_t / close_{t-n}
    pub rocr100_prev: f64,
    pub close_now: f64,
    pub close_lag: f64,
    pub last_close: f64,
    pub rocr100_label: String, // STRONG_UP / UP / NEUTRAL / DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CORREL — rolling Pearson correlation.
/// Per-symbol instantiation: lag-1 autocorrelation of close over 30
/// bars (`ρ(close_t, close_{t-1})`). Measures serial-dependence:
/// values near +1 indicate strong momentum (consecutive closes move
/// together), near 0 a random walk, near −1 mean-reversion.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CorrelSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 30
    pub correl: f64,   // Pearson correlation ∈ [-1, 1]
    pub correl_prev: f64,
    pub mean_x: f64, // mean(close_t)
    pub mean_y: f64, // mean(close_{t-1})
    pub stddev_x: f64,
    pub stddev_y: f64,
    pub last_close: f64,
    pub correl_label: String, // STRONG_MOMO / MOMO / RANDOM_WALK / MEAN_REVERT / STRONG_MEAN_REVERT / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib MIN — minimum of close over a rolling window (period 30).
/// The rolling-window support level. Combined with `last_close`, the
/// distance above the minimum gives a support-proximity label.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MinSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 30
    pub min_val: f64,  // min(close) over last period
    pub min_prev: f64, // min(close) ending at bar n-2
    pub max_ref: f64,  // max(close) in same window (for position pct)
    pub last_close: f64,
    pub position_pct: f64, // (close - min) / (max - min) · 100 ∈ [0, 100]
    pub min_label: String, // NEAR_LOW / MID / NEAR_HIGH / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib MAX — maximum of close over a rolling window (period 30).
/// The rolling-window resistance level. Distance below the maximum gives
/// a resistance-proximity label.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MaxSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 30
    pub max_val: f64,  // max(close) over last period
    pub max_prev: f64,
    pub min_ref: f64, // min(close) in same window
    pub last_close: f64,
    pub position_pct: f64, // (close - min) / (max - min) · 100
    pub max_label: String, // NEAR_HIGH / MID / NEAR_LOW / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib MINMAX — both endpoints of the rolling window in one snapshot,
/// plus `range_width` and `range_pct` which expose the regime (tight
/// range = consolidation, wide range = trending).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MinMaxSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 30
    pub min_val: f64,
    pub max_val: f64,
    pub range_width: f64, // max - min (price-space width)
    pub range_pct: f64,   // 100 · range_width / last_close (range-as-%-of-close)
    pub last_close: f64,
    pub position_pct: f64,    // (close - min) / range
    pub minmax_label: String, // RANGE_WIDE / RANGE_NORMAL / RANGE_TIGHT / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib MININDEX — bar index at which the window minimum occurred,
/// expressed as a recency (0 = most-recent bar, period-1 = oldest).
/// Labels capture "how fresh is the low?" — a useful lagging-signal for
/// exhaustion vs. continued weakness.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MinIndexSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 30
    pub min_val: f64,
    pub min_index_bars_ago: usize, // 0 = most recent bar is the low; period-1 = window start
    pub min_index_bars_ago_prev: usize,
    pub last_close: f64,
    pub min_index_label: String, // FRESH_LOW / RECENT_LOW / OLD_LOW / STALE_LOW / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib MAXINDEX — bar index at which the window maximum occurred,
/// expressed as a recency. Mirror of MININDEX.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MaxIndexSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 30
    pub max_val: f64,
    pub max_index_bars_ago: usize,
    pub max_index_bars_ago_prev: usize,
    pub last_close: f64,
    pub max_index_label: String, // FRESH_HIGH / RECENT_HIGH / OLD_HIGH / STALE_HIGH / INSUFFICIENT_DATA
    pub note: String,
}

// ── BBANDS / AD / ADOSC / SUM / LINEARREG_INTERCEPT ──

/// TA-Lib BBANDS — Bollinger Bands around a 20-bar SMA ± 2·σ.
/// Classic volatility-band oscillator. Position within the bands
/// (`pct_b`) and band width relative to mid (`bandwidth`) together
/// capture both where price is and how dynamic the band regime is.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BbandsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 20
    pub num_std: f64,  // 2.0
    pub upper: f64,
    pub middle: f64, // SMA_period(close)
    pub lower: f64,
    pub upper_prev: f64,
    pub middle_prev: f64,
    pub lower_prev: f64,
    pub last_close: f64,
    pub pct_b: f64,           // 100 · (close − lower) / (upper − lower)
    pub bandwidth: f64,       // 100 · (upper − lower) / middle
    pub bbands_label: String, // ABOVE_UPPER / UPPER_HALF / LOWER_HALF / BELOW_LOWER / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib AD — Chaikin Accumulation/Distribution Line.
/// Cumulative `MF × volume` where `MF = ((close − low) − (high − close)) /
/// (high − low)`. A running total of volume-weighted close bias within
/// each bar's range — rising = net buying, falling = net distribution.
/// The scalar `ad_slope` is the 10-bar linear-regression slope of the
/// line for label classification.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AdSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ad: f64,       // cumulative A/D at last bar
    pub ad_prev: f64,  // cumulative A/D at bar n-2
    pub ad_delta: f64, // ad − ad_prev
    pub ad_slope: f64, // 10-bar slope of AD series
    pub last_close: f64,
    pub ad_label: String, // STRONG_ACCUM / ACCUM / FLAT / DIST / STRONG_DIST / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib ADOSC — Chaikin Accumulation/Distribution Oscillator.
/// `fast_EMA(AD) − slow_EMA(AD)` with default (fast=3, slow=10). A
/// zero-centred momentum oscillator on the AD line — signals
/// accumulation/distribution impulses that the raw AD slope can't
/// pick up.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AdoscSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub fast_period: usize, // 3
    pub slow_period: usize, // 10
    pub adosc: f64,
    pub adosc_prev: f64,
    pub last_close: f64,
    pub ad_ref: f64,         // underlying AD value at same bar (for cross-ref)
    pub adosc_label: String, // STRONG_BULL / BULL / FLAT / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib SUM — rolling sum of close over a 30-bar window. The raw
/// primitive SMA is built on top of (SMA = SUM / period) — distinct
/// because SUM is an absolute quantity useful for compounding
/// calculations, whereas SMA is an average. Label classifies whether
/// the sum is rising (momentum) or falling (decay) by comparing the
/// current sum to the sum one bar earlier.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SumSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 30
    pub sum: f64,
    pub sum_prev: f64,
    pub sum_delta: f64,
    pub sum_pct_change: f64, // 100 · (sum − sum_prev) / sum_prev
    pub last_close: f64,
    pub sum_label: String, // STRONG_UP / UP / FLAT / DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib LINEARREG_INTERCEPT — the `b` coefficient in `y = m·x + b`
/// where `y = close` and `x = bar_index` over a 14-bar window.
/// Complements the already-shipped LINEARREG (endpoint), LINEARREG_ANGLE,
/// LINEARREG_SLOPE, and TSF primitives. The intercept alone is not a
/// signal; it is the *level* the regression predicts at x=0 (the
/// oldest bar in the window). The informative scalar is
/// `intercept − last_close`, which says how far the regression
/// has walked from its oldest bar.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LinearRegInterceptSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 14
    pub intercept: f64,
    pub intercept_prev: f64,
    pub slope: f64, // for cross-ref with LINEARREG_SLOPE
    pub last_close: f64,
    pub drift: f64, // last_close − intercept (how far price is from regression base)
    pub drift_pct: f64, // 100 · drift / intercept
    pub linreg_intercept_label: String, // STRONG_ADVANCE / ADVANCE / FLAT / DECLINE / STRONG_DECLINE / INSUFFICIENT_DATA
    pub note: String,
}

// ── AROONOSC / MINMAXINDEX / MACDEXT / MACDFIX / MAVP ──

/// TA-Lib AROONOSC — Aroon Oscillator = AROON_UP − AROON_DOWN over a
/// 14-bar window. Complements the already-shipped AROON primitive by
/// surfacing the signed differential directly — values near +100 signal
/// strong uptrend (recent high very fresh, low stale), −100 strong
/// downtrend, near zero mixed/no-trend.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AroonoscSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 14
    pub aroonosc: f64, // aroon_up − aroon_down ∈ [-100, 100]
    pub aroonosc_prev: f64,
    pub aroon_up: f64, // for cross-ref
    pub aroon_down: f64,
    pub last_close: f64,
    pub aroonosc_label: String, // STRONG_BULL / BULL / FLAT / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib MINMAXINDEX — combined rolling-window MININDEX + MAXINDEX in
/// one snapshot plus `age_diff` (bars between the two extrema) and
/// `extrema_order` (HIGH_FIRST / LOW_FIRST / SAME_BAR) which together
/// describe the window's directional signature. Completes the
/// rolling-extrema family.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MinMaxIndexSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 30
    pub min_index_bars_ago: usize,
    pub max_index_bars_ago: usize,
    pub age_diff: i64, // min_idx − max_idx (signed — positive means max is fresher)
    pub extrema_order: String, // HIGH_FIRST / LOW_FIRST / SAME_BAR
    pub last_close: f64,
    pub minmaxindex_label: String, // FRESH_HIGH / FRESH_LOW / MID / OLD_EXTREMA / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib MACDEXT — MACD with *configurable MA types* for fast, slow,
/// and signal. This snapshot pins all three to SMA (the classic "simple
/// MACD" textbook form) to give agents a deterministic alternative to
/// the default EMA-based MACD. Complements the existing MACD snapshot
/// without replacing it.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MacdextSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub fast_period: usize,   // 12
    pub slow_period: usize,   // 26
    pub signal_period: usize, // 9
    pub ma_type: String,      // "SMA" for this snapshot
    pub macd: f64,            // fast_MA − slow_MA
    pub macd_prev: f64,
    pub signal: f64, // MA(macd)
    pub signal_prev: f64,
    pub hist: f64, // macd − signal
    pub hist_prev: f64,
    pub last_close: f64,
    pub macdext_label: String, // STRONG_BULL / BULL / FLAT / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib MACDFIX — MACD with *hardcoded* 12/26 fast/slow (fix = fixed)
/// and configurable signal (9 default). Historically the most-widely
/// used MACD form — this snapshot surfaces the canonical 12/26/9
/// EMA-based MACD. Distinct from the existing MACD snapshot in
/// that it exposes the hardcoded-fast/slow as an explicit constraint,
/// useful for agents wanting to verify textbook parameters.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MacdfixSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub fast_period: usize,   // 12 (fixed)
    pub slow_period: usize,   // 26 (fixed)
    pub signal_period: usize, // 9
    pub macd: f64,            // EMA12(close) − EMA26(close)
    pub macd_prev: f64,
    pub signal: f64, // EMA9(macd)
    pub signal_prev: f64,
    pub hist: f64, // macd − signal
    pub hist_prev: f64,
    pub last_close: f64,
    pub macdfix_label: String, // STRONG_BULL / BULL / FLAT / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib MAVP — Moving Average with Variable Period. Unlike SMA/EMA/WMA
/// which all use a single fixed period, MAVP takes a per-bar period
/// array — the MA at bar t is computed with a t-specific lookback. This
/// snapshot uses a linear ramp (5 at start → 30 at end) to exercise the
/// polymorphic behaviour and emit a single scalar at the last bar
/// (last-bar period = 30). Label classifies sign of mavp_delta.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MavpSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub min_period: usize,      // 5 (ramp start)
    pub max_period: usize,      // 30 (ramp end)
    pub last_bar_period: usize, // period used at final bar (== max_period)
    pub mavp: f64,
    pub mavp_prev: f64,
    pub mavp_delta: f64,
    pub last_close: f64,
    pub mavp_label: String, // STRONG_UP / UP / FLAT / DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternDOJI — Doji candlestick pattern. A doji is a single-bar
/// pattern where open ≈ close (body very small relative to range),
/// signalling indecision. TA-Lib convention: pattern_value is 100 for
/// bullish match, -100 for bearish match, 0 for no match. Doji is
/// directionally neutral by nature, so we emit 100 when present and
/// classify as NEUTRAL_PATTERN.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlDojiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // 100 if doji on last bar, 0 otherwise
    pub pattern_value_prev: i32,
    pub body_pct_range: f64, // |close-open| / (high-low) as percent
    pub upper_shadow_pct: f64,
    pub lower_shadow_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize, // 0 if last bar matches, else bars since last match within bars_used window
    pub last_close: f64,
    pub cdl_doji_label: String, // DOJI_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternHAMMER — Hammer pattern. Single-bar bullish reversal
/// signal: small body in upper third of range, long lower shadow
/// (≥ 2× body), minimal upper shadow. TA-Lib emits 100 on match
/// (always treated as bullish in TA-Lib's reference implementation).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlHammerSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // 100 if hammer on last bar, 0 otherwise
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,
    pub upper_shadow_pct: f64,
    pub lower_shadow_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_hammer_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternSHOOTINGSTAR — Shooting Star pattern. Mirror of hammer:
/// small body in lower third of range, long upper shadow (≥ 2× body),
/// minimal lower shadow. Bearish reversal signal. TA-Lib emits -100
/// on match.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlShootingStarSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 if shooting star, 0 otherwise
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,
    pub upper_shadow_pct: f64,
    pub lower_shadow_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_shooting_star_label: String, // BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternENGULFING — Engulfing pattern. Two-bar reversal signal:
/// current bar's body fully engulfs prior bar's body AND the direction
/// is opposite (prior red → current green = bullish engulfing, prior
/// green → current red = bearish engulfing). TA-Lib emits 100 for
/// bullish, -100 for bearish, 0 for no match.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlEngulfingSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub body_size_ratio: f64, // current body / prior body (>1.0 means current engulfs)
    pub prior_body_pct_range: f64,
    pub current_body_pct_range: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_engulfing_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternHARAMI — Harami pattern. Two-bar reversal signal
/// (inside-bar): current bar's body fully contained within prior
/// bar's body AND direction is opposite. TA-Lib emits 100 for
/// bullish harami (prior red, current green inside), -100 for
/// bearish harami (prior green, current red inside), 0 for no match.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlHaramiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub body_size_ratio: f64, // current body / prior body (<1.0 means current contained)
    pub prior_body_pct_range: f64,
    pub current_body_pct_range: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_harami_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternMORNINGSTAR — Morning Star (3-bar bullish reversal).
/// Bar 0 = large red body, bar 1 = small star (gapped or near
/// bar-0 close), bar 2 = large green body closing above bar-0
/// midpoint. Emits +100 when all three conditions hold.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlMorningStarSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, 0 none
    pub pattern_value_prev: i32,
    pub penetration_pct: f64, // 100 · (bar2_close - bar0_midpoint) / bar0_body  (> 0 when bullish)
    pub star_body_pct_range: f64, // middle bar body % of range
    pub first_body_pct_range: f64,
    pub last_body_pct_range: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_morning_star_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternEVENINGSTAR — Evening Star (3-bar bearish reversal).
/// Mirror of morning star: bar 0 = large green body, bar 1 = small
/// star, bar 2 = large red body closing below bar-0 midpoint.
/// Emits -100 on match.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlEveningStarSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub penetration_pct: f64, // 100 · (bar0_midpoint - bar2_close) / bar0_body  (> 0 when bearish)
    pub star_body_pct_range: f64,
    pub first_body_pct_range: f64,
    pub last_body_pct_range: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_evening_star_label: String, // BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick pattern3BLACKCROWS — Three Black Crows (3-bar bearish
/// continuation). Three consecutive red bars, each closing below
/// the prior close AND opening within the prior body. TA-Lib emits
/// -100 on match.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlThreeBlackCrowsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub avg_body_pct_range: f64, // average body % of range across the 3 bars
    pub total_close_decline_pct: f64, // 100 · (bar2_close - bar0_open) / bar0_open
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_three_black_crows_label: String, // BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick pattern3WHITESOLDIERS — Three White Soldiers (3-bar bullish
/// continuation). Three consecutive green bars, each closing above
/// the prior close AND opening within the prior body. TA-Lib emits
/// +100 on match.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlThreeWhiteSoldiersSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, 0 none
    pub pattern_value_prev: i32,
    pub avg_body_pct_range: f64,
    pub total_close_advance_pct: f64, // 100 · (bar2_close - bar0_open) / bar0_open
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_three_white_soldiers_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// candlestick patternDARKCLOUDCOVER — Dark Cloud Cover (2-bar bearish
/// reversal). Prior bar green with large body; current bar red,
/// opens above prior high, closes below prior midpoint (standard
/// TA-Lib penetration threshold 0.5). Emits -100 on match.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlDarkCloudCoverSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub penetration_pct: f64, // 100 · (prior_close - current_close) / prior_body
    pub prior_body_pct_range: f64,
    pub current_body_pct_range: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_dark_cloud_cover_label: String, // BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}
