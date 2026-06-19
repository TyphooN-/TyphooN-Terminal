// Statistical indicator and technical-analysis research types

/// MCLEODLI — McLeod-Li test (Ljung-Box on squared returns for ARCH effects).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct McLeodLiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub lag_h: usize,           // portmanteau window on squared returns
    pub q_stat: f64,            // n(n+2) Σ ρ̂²(k)/(n-k) on squared returns
    pub df: usize,              // lag_h
    pub critical_95: f64,       // χ²(df) 95% critical value
    pub p_value: f64,           // Pr(χ²_df > q_stat)
    pub reject_null: bool,      // true if q_stat > critical_95
    pub mcleodli_label: String, // NO_ARCH / MILD_ARCH / STRONG_ARCH / INSUFFICIENT_DATA
    pub note: String,
}

/// OUFIT — Ornstein-Uhlenbeck mean-reversion fit on log-price.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct OuFitSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub theta: f64,          // mean-reversion speed (per bar)
    pub mu: f64,             // long-run mean log-price
    pub sigma: f64,          // diffusion scale (residual sd of discrete fit)
    pub half_life_bars: f64, // ln(2) / θ; +∞ if θ ≤ 0
    pub residual_sd: f64,    // sd of (x_{t+1} − â − b̂·x_t) residuals
    pub r_squared: f64,      // R² of the AR(1) fit on log-price
    pub oufit_label: String, // TRENDING / SLOW_REVERT / MODERATE_REVERT / FAST_REVERT / INSUFFICIENT_DATA
    pub note: String,
}

/// GPH — Geweke-Porter-Hudak log-periodogram long-memory d estimator.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct GphSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub m_freqs: usize,  // truncation m = floor(n^0.5)
    pub d_estimate: f64, // fractional integration order
    pub d_stderr: f64,   // √(π²/24m)
    pub t_stat: f64,     // d / stderr (H0: d=0)
    pub p_value_two_sided: f64,
    pub gph_label: String, // ANTIPERSISTENT / SHORT_MEMORY / LONG_MEMORY / NONSTATIONARY / INSUFFICIENT_DATA
    pub note: String,
}

/// BURGSPEC — Burg maximum-entropy AR-based spectral estimator.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BurgSpecSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ar_order: usize,           // p = min(20, n/4)
    pub dominant_freq: f64,        // cycles per bar at AR-spectrum peak
    pub dominant_period_bars: f64, // 1 / dominant_freq
    pub peak_power: f64,           // AR spectrum at peak
    pub mean_power: f64,           // mean AR-spectrum density over grid
    pub peak_to_mean_ratio: f64,   // peak_power / mean_power
    pub burgspec_label: String, // NO_AR_CYCLE / WEAK_AR_CYCLE / MODERATE_AR_CYCLE / STRONG_AR_CYCLE / INSUFFICIENT_DATA
    pub note: String,
}

/// KENDALLTAU — Kendall's tau lag-1 rank autocorrelation.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct KendallTauSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pair_count: usize, // n·(n-1)/2
    pub concordant: usize,
    pub discordant: usize,
    pub tau: f64,    // (C − D) / [n(n−1)/2]
    pub z_stat: f64, // τ / √(2(2n+5)/(9n(n−1)))
    pub p_value_two_sided: f64,
    pub kendalltau_label: String, // STRONG_POS / WEAK_POS / NO_RANK_AUTO / WEAK_NEG / STRONG_NEG / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 42 surfaces ─────────────────────────────────────────────

/// SQUEEZE — composite short-squeeze outlier score per symbol.
/// Fuses five orthogonal axes: short-float %, days-to-cover, 20d momentum,
/// relative volume, and IV-rank. Each axis is normalised to 0..100 and the
/// composite is the weighted mean.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SqueezeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub short_percent_of_float: f64, // raw % of float short
    pub days_to_cover: f64,          // raw days-to-cover (short / 20d vol)
    pub momentum_20d_pct: f64,       // (close_t / close_{t-20}) − 1, in %
    pub relvol_20d: f64,             // current volume / 20d avg
    pub iv_rank: f64,                // 0..100 from IvolSnapshot
    pub short_float_score: f64,      // 0..100 contribution
    pub days_to_cover_score: f64,    // 0..100 contribution
    pub momentum_score: f64,         // 0..100 contribution
    pub relvol_score: f64,           // 0..100 contribution
    pub iv_rank_score: f64,          // 0..100 contribution
    pub composite_score: f64,        // 0..100 weighted mean
    pub inputs_present: usize,       // how many of the 5 axes had data (0..5)
    pub squeeze_label: String, // NO_SQUEEZE / WATCH / ELEVATED / STRONG / EXTREME / INSUFFICIENT_DATA
    pub note: String,
}

/// SQUEEZERANK — cross-symbol percentile rank of SQUEEZE composite scores.
/// Populated by a table-scan across all symbols with a SQUEEZE row.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SqueezeRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub composite_score: f64,      // mirror of SQUEEZE.composite_score
    pub peer_count: usize,         // symbols scanned
    pub rank: usize,               // 1 = highest composite
    pub percentile: f64,           // 0..100
    pub squeezerank_label: String, // TOP_1PCT / TOP_5PCT / TOP_10PCT / ABOVE_MEDIAN / BELOW_MEDIAN / INSUFFICIENT_DATA
    pub note: String,
}

/// BBSQUEEZE — Bollinger-Band squeeze detector.
/// Uses 20-bar SMA ±2σ; BB-width = (upper-lower)/mid. A "squeeze" is when
/// the current BB-width is in the low tail of its 120-bar history.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BbsqueezeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,         // 20
    pub bb_width_current: f64, // (upper - lower) / mid at last bar
    pub bb_width_min_120: f64,
    pub bb_width_max_120: f64,
    pub bb_width_percentile: f64, // 0..100 current rank in 120-bar history
    pub upper_band: f64,
    pub lower_band: f64,
    pub mid_band: f64,
    pub last_close: f64,
    pub bbsqueeze_label: String, // TIGHT_SQUEEZE / MODERATE_SQUEEZE / NORMAL / EXPANSION / INSUFFICIENT_DATA
    pub note: String,
}

/// DONCHIAN — Donchian-channel breakout detector (20-bar default).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DonchianSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,      // 20
    pub upper_channel: f64, // max(high) over period
    pub lower_channel: f64, // min(low) over period
    pub mid_channel: f64,   // (upper + lower)/2
    pub last_close: f64,
    pub channel_position_pct: f64, // 0..100, (close-lower)/(upper-lower)
    pub breakout_upper: bool,      // close ≥ prior upper
    pub breakout_lower: bool,      // close ≤ prior lower
    pub donchian_label: String, // BREAKOUT_UP / APPROACH_UP / NEUTRAL / APPROACH_DOWN / BREAKOUT_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// KAMA — Kaufman Adaptive Moving Average efficiency ratio.
/// Efficiency Ratio = |close_t - close_{t-n}| / Σ|close_i - close_{i-1}|.
/// High ER = trending; low ER = choppy.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct KamaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,         // 10
    pub efficiency_ratio: f64, // 0..1
    pub kama_value: f64,       // adaptive MA at last bar
    pub last_close: f64,
    pub kama_slope_pct: f64, // (kama_t / kama_{t-5} - 1) × 100
    pub kama_label: String, // STRONG_TREND / MODERATE_TREND / WEAK_TREND / CHOPPY / INSUFFICIENT_DATA
    pub note: String,
}

/// ICHIMOKU — Ichimoku Kinko Hyo five-line cloud system.
/// Tenkan 9, Kijun 26, Senkou A = (Tenkan+Kijun)/2 plotted +26, Senkou B = 52-bar
/// midpoint plotted +26, Chikou = close plotted −26. All midpoints use (H+L)/2.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct IchimokuSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub tenkan_sen: f64,    // 9-bar midpoint
    pub kijun_sen: f64,     // 26-bar midpoint
    pub senkou_span_a: f64, // (tenkan+kijun)/2 at t (projected +26)
    pub senkou_span_b: f64, // 52-bar midpoint at t (projected +26)
    pub chikou_span: f64,   // close plotted back −26
    pub cloud_top: f64,     // max(senkou_a, senkou_b)
    pub cloud_bottom: f64,  // min(senkou_a, senkou_b)
    pub last_close: f64,
    pub close_vs_cloud_pct: f64, // (close - cloud_mid) / cloud_mid × 100
    pub ichimoku_label: String, // STRONG_BULL / BULL / IN_CLOUD / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// SUPERTREND — ATR-based trailing-stop trend indicator.
/// Period 10, multiplier 3. Flips on close crossing prior band.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SupertrendSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,   // 10
    pub multiplier: f64, // 3.0
    pub atr: f64,
    pub upper_band: f64,
    pub lower_band: f64,
    pub supertrend_value: f64, // active band (upper in downtrend, lower in up)
    pub trend_is_up: bool,
    pub last_close: f64,
    pub distance_pct: f64,        // (close - supertrend) / supertrend × 100
    pub bars_in_trend: usize,     // bars since last flip
    pub supertrend_label: String, // STRONG_UP / UP / FLAT / DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// KELTNER — Keltner Channels (EMA 20 ± multiplier × ATR 10).
/// Pairs with BBSQUEEZE for the TTM-squeeze (BB inside KC → volatility compression).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct KeltnerSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ema_period: usize, // 20
    pub atr_period: usize, // 10
    pub multiplier: f64,   // 2.0
    pub ema_value: f64,    // midline
    pub atr: f64,
    pub upper_channel: f64,
    pub lower_channel: f64,
    pub last_close: f64,
    pub channel_width: f64,        // upper - lower
    pub width_pct_of_mid: f64,     // (upper - lower) / ema × 100
    pub channel_position_pct: f64, // (close - lower) / (upper - lower) × 100
    pub ttm_squeeze_on: bool,      // true when BB fully inside KC (computed here using BB 20/2σ)
    pub keltner_label: String, // BREAKOUT_UP / NEAR_UPPER / IN_CHANNEL / NEAR_LOWER / BREAKOUT_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// FISHER — John Ehlers' Fisher Transform of normalised price.
/// Normalises close to [-1, 1] window, then applies 0.5·ln((1+x)/(1-x)).
/// Output distribution is approximately Gaussian; sharp turning points.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct FisherSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,         // 10
    pub fisher_value: f64,     // latest transform output
    pub fisher_signal: f64,    // prior bar's fisher_value (the "trigger")
    pub extreme_2_cross: bool, // crossed ±2 in last 3 bars (mean-reversion flag)
    pub peak_abs_10: f64,      // max |fisher| over last 10 bars
    pub last_close: f64,
    pub fisher_label: String, // STRONG_POS / POS / NEUTRAL / NEG / STRONG_NEG / INSUFFICIENT_DATA
    pub note: String,
}

/// AROON — Aroon Up / Aroon Down / Aroon Oscillator over 25 bars.
/// Aroon Up = 100 × (period − bars_since_high) / period
/// Aroon Down = 100 × (period − bars_since_low) / period
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AroonSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,         // 25
    pub aroon_up: f64,         // 0..100
    pub aroon_down: f64,       // 0..100
    pub aroon_oscillator: f64, // up − down, −100..100
    pub bars_since_high: usize,
    pub bars_since_low: usize,
    pub last_close: f64,
    pub aroon_label: String, // STRONG_UP / WEAK_UP / CONSOLIDATION / WEAK_DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// ADX — Wilder's Average Directional Index (period 14).
/// Reports +DI, -DI, ADX and directional-movement smoothed averages.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AdxSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 14
    pub plus_di: f64,
    pub minus_di: f64,
    pub adx: f64,
    pub dx: f64, // instantaneous |+DI − −DI|/(+DI + −DI) × 100
    pub atr: f64,
    pub last_close: f64,
    pub adx_label: String, // STRONG_TREND adx≥40 / TREND ≥25 / WEAK_TREND ≥15 / NO_TREND / INSUFFICIENT_DATA
    pub note: String,
}

/// CCI — Lambert's Commodity Channel Index (period 20).
/// CCI = (TP − SMA(TP)) / (0.015 × MD) where TP=(H+L+C)/3, MD is mean absolute deviation.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CciSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 20
    pub typical_price: f64,
    pub tp_sma: f64,
    pub mean_abs_dev: f64,
    pub cci_value: f64,
    pub last_close: f64,
    pub cci_label: String, // OVERBOUGHT >100 / BULL >0 / NEUTRAL / BEAR <0 / OVERSOLD <−100 / INSUFFICIENT_DATA
    pub note: String,
}

/// CMF — Chaikin Money Flow (period 20).
/// Σ(MFV)/Σ(volume) over window, where MFV = ((close−low) − (high−close))/(high−low) × volume.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CmfSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,  // 20
    pub cmf_value: f64, // −1..+1
    pub money_flow_volume_sum: f64,
    pub volume_sum: f64,
    pub last_close: f64,
    pub cmf_label: String, // STRONG_ACCUM >0.25 / ACCUM >0.05 / NEUTRAL / DIST <−0.05 / STRONG_DIST <−0.25 / INSUFFICIENT_DATA
    pub note: String,
}

/// MFI — Quong & Soudack's Money Flow Index (period 14).
/// Volume-weighted RSI: uses typical price × volume as "money flow".
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MfiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,  // 14
    pub mfi_value: f64, // 0..100
    pub positive_mf_sum: f64,
    pub negative_mf_sum: f64,
    pub money_flow_ratio: f64,
    pub last_close: f64,
    pub mfi_label: String, // OVERBOUGHT >80 / BULL >50 / NEUTRAL / BEAR <50 / OVERSOLD <20 / INSUFFICIENT_DATA
    pub note: String,
}

/// PSAR — Wilder's Parabolic Stop-And-Reverse.
/// Initial AF 0.02, increment 0.02, cap 0.20. Flips when price crosses SAR.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PsarSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub af_start: f64,            // 0.02
    pub af_step: f64,             // 0.02
    pub af_max: f64,              // 0.20
    pub sar_value: f64,           // latest SAR
    pub extreme_point: f64,       // EP (highest high in long trend, lowest low in short)
    pub acceleration_factor: f64, // current AF
    pub trend_is_up: bool,
    pub bars_in_trend: usize,
    pub distance_pct: f64, // (close - sar) / sar × 100
    pub last_close: f64,
    pub psar_label: String, // STRONG_UP / UP / FLAT / DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// VORTEX — Botes & Siepman (2009) directional-movement alternative.
/// VI+ = Σ|H_t − L_{t−1}| / ΣTR, VI− = Σ|L_t − H_{t−1}| / ΣTR over period=14.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct VortexSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 14
    pub vi_plus: f64,
    pub vi_minus: f64,
    pub vi_diff: f64, // VI+ − VI−
    pub sum_tr: f64,
    pub sum_vm_plus: f64,
    pub sum_vm_minus: f64,
    pub last_close: f64,
    pub vortex_label: String, // BULL_CROSS / BULL / NEUTRAL / BEAR / BEAR_CROSS / INSUFFICIENT_DATA
    pub note: String,
}

/// CHOP — Bill Dreiss Choppiness Index (period=14).
/// CI = 100·log10(ΣTR / (maxH − minL)) / log10(N). Values > 61.8 = choppy, < 38.2 = trending.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ChopSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,   // 14
    pub chop_value: f64, // 0..100
    pub sum_tr: f64,
    pub range_high: f64, // max high over period
    pub range_low: f64,  // min low over period
    pub range_span: f64, // range_high - range_low
    pub last_close: f64,
    pub chop_label: String, // CHOP >61.8 / RANGING >50 / NEUTRAL / TRANSITIONAL <50 / TRENDING <38.2 / INSUFFICIENT_DATA
    pub note: String,
}

/// OBV — Granville (1963) On-Balance Volume cumulative + 20-bar slope.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ObvSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub slope_window: usize, // 20
    pub obv_value: f64,      // latest cumulative
    pub obv_slope: f64,      // linear-regression slope of last 20 OBV values
    pub obv_change_pct: f64, // (obv[N-1] - obv[N-20]) / |obv[N-20]| × 100 (or 0 if divisor≈0)
    pub obv_min_20: f64,
    pub obv_max_20: f64,
    pub last_close: f64,
    pub obv_label: String, // STRONG_UP / UP / NEUTRAL / DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// TRIX — Jack Hutson (1980s) triple-EMA momentum oscillator.
/// EMA3 = EMA(EMA(EMA(close, N), N), N); TRIX = 100·(EMA3_t/EMA3_{t−1} − 1); signal = EMA(TRIX, 9).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TrixSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,        // 15
    pub signal_period: usize, // 9
    pub trix_value: f64,      // % change
    pub signal_value: f64,    // EMA(TRIX, 9)
    pub histogram: f64,       // trix − signal
    pub ema3_value: f64,      // final triple-smoothed EMA level
    pub last_close: f64,
    pub trix_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// HMA — Alan Hull (2005) weighted-moving-average combo: HMA = WMA(2·WMA(n/2) − WMA(n), √n).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HmaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,         // 20
    pub half_period: usize,    // 10
    pub sqrt_period: usize,    // 4 (floor(sqrt(20)))
    pub hma_value: f64,        // latest HMA
    pub hma_slope_pct: f64,    // (hma[N-1] - hma[N-6]) / hma[N-6] × 100
    pub hma_vs_close_pct: f64, // (close - hma) / hma × 100
    pub last_close: f64,
    pub hma_label: String, // STRONG_UP / UP / NEUTRAL / DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// PPO — Gerald Appel Percentage Price Oscillator: 100·(EMA_fast − EMA_slow)/EMA_slow.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PpoSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub fast_period: usize,   // 12
    pub slow_period: usize,   // 26
    pub signal_period: usize, // 9
    pub ema_fast: f64,
    pub ema_slow: f64,
    pub ppo_value: f64,    // 100·(ema_fast − ema_slow)/ema_slow
    pub signal_value: f64, // EMA(ppo, 9)
    pub histogram: f64,    // ppo − signal
    pub last_close: f64,
    pub ppo_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// DPO — Detrended Price Oscillator: close − SMA(close, N) shifted back (N/2 + 1) bars.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DpoSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,  // 20
    pub shift: usize,   // N/2 + 1 = 11
    pub sma_value: f64, // SMA at shifted index
    pub dpo_value: f64, // close[t - shift] − sma[t]
    pub dpo_pct: f64,   // dpo / sma × 100
    pub last_close: f64,
    pub dpo_label: String, // PEAK_HIGH / BULL / NEUTRAL / BEAR / PEAK_LOW / INSUFFICIENT_DATA
    pub note: String,
}

/// KST — Martin Pring Know Sure Thing: weighted sum of four ROCs smoothed by SMA.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct KstSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub rcma1: f64,        // SMA(ROC(10), 10)
    pub rcma2: f64,        // SMA(ROC(15), 10)
    pub rcma3: f64,        // SMA(ROC(20), 10)
    pub rcma4: f64,        // SMA(ROC(30), 15)
    pub kst_value: f64,    // 1·rcma1 + 2·rcma2 + 3·rcma3 + 4·rcma4
    pub signal_value: f64, // SMA(kst, 9)
    pub histogram: f64,    // kst − signal
    pub last_close: f64,
    pub kst_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// ULTOSC — Larry Williams Ultimate Oscillator: weighted 3-period BP/TR ratio.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct UltoscSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period_short: usize, // 7
    pub period_mid: usize,   // 14
    pub period_long: usize,  // 28
    pub avg_short: f64,      // ΣBP_7 / ΣTR_7
    pub avg_mid: f64,        // ΣBP_14 / ΣTR_14
    pub avg_long: f64,       // ΣBP_28 / ΣTR_28
    pub ultosc_value: f64,   // 100·(4·avg_short + 2·avg_mid + avg_long) / 7
    pub last_close: f64,
    pub ultosc_label: String, // OVERBOUGHT >70 / BULL >50 / NEUTRAL / BEAR <50 / OVERSOLD <30 / INSUFFICIENT_DATA
    pub note: String,
}

/// WILLR — Larry Williams %R: (highest_high − close) / (highest_high − lowest_low) · −100.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct WillrSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 14
    pub highest_high: f64,
    pub lowest_low: f64,
    pub willr_value: f64, // ∈ [−100, 0]
    pub last_close: f64,
    pub willr_label: String, // OVERBOUGHT >-20 / BULL >-50 / NEUTRAL / BEAR <-50 / OVERSOLD <-80 / INSUFFICIENT_DATA
    pub note: String,
}

/// MASS — Donald Dorsey (1992) Mass Index: reversal-detection from H-L range expansion.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MassSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ema_period: usize, // 9
    pub sum_period: usize, // 25
    pub mass_value: f64,   // Σ(EMA₉(H−L) / EMA₉(EMA₉(H−L))) over 25 bars
    pub single_ratio: f64, // latest per-bar ratio
    pub last_close: f64,
    pub mass_label: String, // REVERSAL_BULGE >27 crossing back <26.5 (sentinel NEAR) / WATCH >25 / NEUTRAL / INSUFFICIENT_DATA
    pub note: String,
}

/// CHAIKOSC — Marc Chaikin Oscillator: MACD (fast-slow EMA) of the Accumulation/Distribution line.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ChaikoscSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub fast_period: usize, // 3
    pub slow_period: usize, // 10
    pub ad_last: f64,       // latest A/D line
    pub ema_fast_ad: f64,
    pub ema_slow_ad: f64,
    pub chaikosc_value: f64, // ema_fast_ad − ema_slow_ad
    pub last_close: f64,
    pub chaikosc_label: String, // STRONG_ACCUM / ACCUM / NEUTRAL / DIST / STRONG_DIST / INSUFFICIENT_DATA
    pub note: String,
}

/// KLINGER — Stephen Klinger Volume Oscillator: volume-force EMA spread with signal.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct KlingerSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub fast_period: usize,   // 34
    pub slow_period: usize,   // 55
    pub signal_period: usize, // 13
    pub ema_fast_vf: f64,
    pub ema_slow_vf: f64,
    pub kvo_value: f64,    // ema_fast_vf − ema_slow_vf
    pub signal_value: f64, // EMA(kvo, 13)
    pub histogram: f64,    // kvo − signal
    pub last_close: f64,
    pub klinger_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// STOCHRSI — Tushar Chande (1994) Stochastic RSI: Stochastic applied to the RSI series.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct StochRsiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub rsi_period: usize,   // 14
    pub stoch_period: usize, // 14
    pub k_period: usize,     // 3 (%K smoothing)
    pub d_period: usize,     // 3 (%D smoothing)
    pub rsi_value: f64,      // underlying RSI
    pub rsi_min: f64,        // min RSI over stoch_period
    pub rsi_max: f64,        // max RSI over stoch_period
    pub stoch_rsi_raw: f64,  // (RSI − min) / (max − min) · 100
    pub k_value: f64,        // SMA(raw, 3)
    pub d_value: f64,        // SMA(%K, 3)
    pub last_close: f64,
    pub stochrsi_label: String, // OVERBOUGHT >80 / BULL >50 / NEUTRAL / BEAR <50 / OVERSOLD <20 / INSUFFICIENT_DATA
    pub note: String,
}

/// AWESOME — Bill Williams Awesome Oscillator: SMA5(hl2) − SMA34(hl2).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AwesomeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub fast_period: usize, // 5
    pub slow_period: usize, // 34
    pub sma_fast: f64,      // SMA(hl2, 5)
    pub sma_slow: f64,      // SMA(hl2, 34)
    pub ao_value: f64,      // fast − slow
    pub ao_prev: f64,       // prior bar AO (for color signal)
    pub ao_color_up: bool,  // true if ao_value > ao_prev
    pub last_close: f64,
    pub awesome_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// EFI — Alexander Elder (1993) Force Index: volume-weighted close change, smoothed by EMA13.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct EfiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ema_period: usize, // 13
    pub raw_efi: f64,      // volume * (close − prev_close) latest bar
    pub efi_value: f64,    // EMA13 of raw_efi
    pub efi_prev: f64,     // prior bar EFI (zero-cross detection)
    pub last_close: f64,
    pub efi_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// EMV — Richard Arms (1980s) Ease of Movement: distance-moved / box-ratio smoothed by SMA14.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct EmvSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub sma_period: usize, // 14
    pub volume_scale: f64, // 100_000_000 (100 M)
    pub raw_emv: f64,      // distance_moved / box_ratio latest bar
    pub emv_value: f64,    // SMA14 of raw_emv
    pub last_close: f64,
    pub emv_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// NVI — Paul Dysart / Norman Fosback Negative Volume Index: updates only on down-volume days.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NviSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub signal_period: usize, // 255 (EMA of NVI line)
    pub nvi_value: f64,       // latest NVI (starts at 1000)
    pub signal_value: f64,    // EMA255 of NVI
    pub last_close: f64,
    pub nvi_label: String, // BULL (nvi > signal) / NEUTRAL / BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// PVI — Paul Dysart / Norman Fosback Positive Volume Index: updates only on up-volume days.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PviSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub signal_period: usize, // 255
    pub pvi_value: f64,       // latest PVI (starts at 1000)
    pub signal_value: f64,    // EMA255 of PVI
    pub last_close: f64,
    pub pvi_label: String, // BULL / NEUTRAL / BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// COPPOCK — E.S.C. Coppock (1962) Coppock Curve: WMA10(ROC14 + ROC11) long-term momentum.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CoppockSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub roc_fast: usize,    // 11
    pub roc_slow: usize,    // 14
    pub wma_period: usize,  // 10
    pub coppock_value: f64, // current reading
    pub coppock_prev: f64,  // prior bar
    pub last_close: f64,
    pub coppock_label: String, // BUY_CROSS (prev<0, now>0) / BULL / NEUTRAL / BEAR / SELL_CROSS / INSUFFICIENT_DATA
    pub note: String,
}

/// CMO — Tushar Chande (1994) Momentum Oscillator: raw gain/loss spread on [-100, +100].
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CmoSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,  // 9
    pub sum_up: f64,    // Σ positive close changes over period
    pub sum_dn: f64,    // Σ |negative close changes| over period
    pub cmo_value: f64, // 100 · (sum_up − sum_dn) / (sum_up + sum_dn)
    pub last_close: f64,
    pub cmo_label: String, // OVERBOUGHT >50 / BULL >0 / NEUTRAL / BEAR <0 / OVERSOLD <−50 / INSUFFICIENT_DATA
    pub note: String,
}

/// QSTICK — Tushar Chande (1995) Q-Stick: SMA of candle body (close − open).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct QstickSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,     // 14
    pub qstick_value: f64, // SMA(close − open, 14)
    pub qstick_prev: f64,  // prior bar
    pub last_close: f64,
    pub qstick_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// DISPARITY — Steve Nison popularised (Japanese origin) Disparity Index: % deviation from SMA.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DisparitySnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 14
    pub sma_value: f64,
    pub disparity_value: f64, // (close / SMA − 1) · 100
    pub last_close: f64,
    pub disparity_label: String, // STRONG_BULL >3 / BULL >0 / NEUTRAL / BEAR <0 / STRONG_BEAR <−3 / INSUFFICIENT_DATA
    pub note: String,
}

/// BOP — Igor Livshin Balance of Power: (close − open) / (high − low), smoothed by SMA.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BopSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,  // 14
    pub raw_bop: f64,   // latest bar raw BOP
    pub bop_value: f64, // SMA14 of raw BOP
    pub last_close: f64,
    pub bop_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// SCHAFF — Doug Schaff (2008) Schaff Trend Cycle: stochastic-of-MACD, double-smoothed.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SchaffSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ema_fast: usize, // 23
    pub ema_slow: usize, // 50
    pub cycle: usize,    // 10
    pub stc_value: f64,  // current STC in [0, 100]
    pub stc_prev: f64,   // prior bar (for direction)
    pub last_close: f64,
    pub schaff_label: String, // OVERBOUGHT / BULL / NEUTRAL / BEAR / OVERSOLD / INSUFFICIENT_DATA
    pub note: String,
}

/// STOCH — Lane's classic Stochastic Oscillator (%K fast + %D slow).
/// Distinct from STOCHRSI which applies the stochastic to RSI instead of price.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct StochSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub k_period: usize,  // 14
    pub d_period: usize,  // 3
    pub smoothing: usize, // 3 (slow %K smoothing)
    pub percent_k: f64,   // current slow %K in [0, 100]
    pub percent_d: f64,   // current %D (SMA of %K)
    pub last_close: f64,
    pub stoch_label: String, // OVERBOUGHT / BULL / NEUTRAL / BEAR / OVERSOLD / INSUFFICIENT_DATA
    pub note: String,
}

/// MACD — Gerald Appel (1979) Moving Average Convergence Divergence.
/// 12/26 EMAs, 9-period signal EMA, histogram = MACD - signal.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MacdSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub fast_period: usize,   // 12
    pub slow_period: usize,   // 26
    pub signal_period: usize, // 9
    pub macd_value: f64,      // fast_ema - slow_ema
    pub signal_value: f64,    // 9-EMA of macd_value
    pub histogram: f64,       // macd_value - signal_value
    pub histogram_prev: f64,  // previous bar's histogram (for direction)
    pub last_close: f64,
    pub macd_label: String, // BULL_CROSS / BULL / NEUTRAL / BEAR / BEAR_CROSS / INSUFFICIENT_DATA
    pub note: String,
}

/// VWAP — Volume Weighted Average Price computed over a rolling window.
/// VWAP = Σ(typical_price × volume) / Σ(volume) where typical_price = (H+L+C)/3.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct VwapSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub window: usize,   // 20
    pub vwap_value: f64, // current VWAP over `window` bars
    pub last_close: f64,
    pub deviation_pct: f64, // (last_close - vwap) / vwap × 100
    pub vwap_label: String, // STRONG_ABOVE / ABOVE / AT / BELOW / STRONG_BELOW / INSUFFICIENT_DATA
    pub note: String,
}

/// MCGD — John McGinley (1997) McGinley Dynamic adaptive moving average.
/// MD[i] = MD[i-1] + (P - MD[i-1]) / (N × (P/MD[i-1])^4). Designed to resist whipsaws.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct McgdSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,   // 14
    pub mcgd_value: f64, // current McGinley Dynamic
    pub mcgd_prev: f64,  // prior bar (for slope direction)
    pub last_close: f64,
    pub deviation_pct: f64, // (last_close - mcgd) / mcgd × 100
    pub mcgd_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// RWI — Michael Poulos (1991) Random Walk Index.
/// Measures how far price has moved vs a random-walk expectation. Ratios > 1
/// mean the move is larger than what random noise would produce at that horizon.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RwiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 14 (max lookback used for ratios)
    pub rwi_high: f64, // max ratio over 2..=length
    pub rwi_low: f64,  // max ratio over 2..=length
    pub last_close: f64,
    pub rwi_label: String, // TRENDING_UP / TRENDING_DOWN / RANGE_BOUND / INSUFFICIENT_DATA
    pub note: String,
}

/// DEMA — Patrick Mulloy (1994) Double Exponential MA.
/// DEMA = 2·EMA(N) − EMA(EMA(N)). Reduces the lag of a plain EMA.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DemaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 20
    pub dema_value: f64,
    pub dema_prev: f64,     // prior-bar DEMA (for slope)
    pub deviation_pct: f64, // (last_close - dema) / dema × 100
    pub last_close: f64,
    pub dema_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// TEMA — Patrick Mulloy (1994) Triple Exponential MA.
/// TEMA = 3·EMA(N) − 3·EMA(EMA(N)) + EMA(EMA(EMA(N))). Even less lag than DEMA.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TemaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 20
    pub tema_value: f64,
    pub tema_prev: f64,
    pub deviation_pct: f64,
    pub last_close: f64,
    pub tema_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// LINREG — OLS linear regression on last N closes (slope + R² + channel bounds).
/// Slope in price/bar units; R² bounded [0, 1] (fit quality); channel = ±1σ of residuals.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LinregSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,  // 20
    pub slope: f64,     // fit line slope (price units per bar)
    pub intercept: f64, // fit line value at bar 0 of window
    pub r_squared: f64, // coefficient of determination [0, 1]
    pub sigma: f64,     // standard error of residuals
    pub last_close: f64,
    pub fit_value: f64,       // regression line at the final bar
    pub channel_upper: f64,   // fit_value + 2σ
    pub channel_lower: f64,   // fit_value − 2σ
    pub linreg_label: String, // STRONG_UP_TREND / UP_TREND / RANGE / DOWN_TREND / STRONG_DOWN_TREND / INSUFFICIENT_DATA
    pub note: String,
}

/// PIVOTS — classic floor-trader daily pivot levels computed from the prior bar.
/// PP = (H + L + C) / 3; R1 = 2·PP − L; S1 = 2·PP − H; R2 = PP + (H − L); S2 = PP − (H − L).
/// Header labels where the current close sits relative to the levels.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PivotsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pp: f64, // central pivot point
    pub r1: f64,
    pub r2: f64,
    pub s1: f64,
    pub s2: f64,
    pub last_close: f64,
    pub prior_high: f64,      // the H used
    pub prior_low: f64,       // the L used
    pub prior_close: f64,     // the C used
    pub pivots_label: String, // ABOVE_R2 / BETWEEN_R1_R2 / BETWEEN_PP_R1 / AT_PP / BETWEEN_S1_PP / BETWEEN_S2_S1 / BELOW_S2 / INSUFFICIENT_DATA
    pub note: String,
}

/// HEIKIN — Heikin Ashi candle sentiment tracker.
/// HA_close = (O+H+L+C)/4; HA_open = (prior_HA_open + prior_HA_close)/2;
/// HA_high = max(H, HA_open, HA_close); HA_low = min(L, HA_open, HA_close).
/// Tracks consecutive same-color count and current body/wick geometry.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HeikinSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ha_open: f64,
    pub ha_high: f64,
    pub ha_low: f64,
    pub ha_close: f64,
    pub body_abs: f64,                 // |HA_close − HA_open|
    pub upper_wick: f64,               // HA_high − max(HA_open, HA_close)
    pub lower_wick: f64,               // min(HA_open, HA_close) − HA_low
    pub consecutive_same_color: usize, // count of bars in current run (inclusive of current)
    pub last_close: f64,
    pub heikin_label: String, // STRONG_BULL_RUN / BULL / DOJI / BEAR / STRONG_BEAR_RUN / INSUFFICIENT_DATA
    pub note: String,
}
