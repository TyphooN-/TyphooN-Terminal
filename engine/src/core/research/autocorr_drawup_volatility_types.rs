use serde::{Deserialize, Serialize};

// Autocorrelation, Hurst, hit-rate, gap, drawup, downside-volatility, Sharpe, efficiency, and wick-stat research types
//
// Five pure HP-local stat surfaces. All five compute over the
// trailing 253-session window of the existing `research_historical_price`
// cache and add zero new API dependencies. Each one fills a
// conceptually distinct gap vs Godel:
//
//  * AUTOCOR  — serial correlation of returns at lags 1/5/10/20,
//               the canonical momentum-vs-mean-reversion detector
//  * HURST    — long-memory exponent via rescaled-range (R/S)
//               analysis (H<0.5 mean-reverting, H≈0.5 random walk,
//               H>0.5 persistent / trending)
//  * HITRATE  — multi-horizon win rate: share of positive-return
//               bars over 5d/20d/60d/252d windows
//  * GLASYM   — gain/loss magnitude asymmetry: avg/median up-day
//               size vs avg/median down-day size + magnitude ratio
//  * VOLRATIO — accumulation/distribution hint from HP volume:
//               avg volume on up-days vs down-days + regime label

/// AUTOCOR — Autocorrelation of daily log returns at multiple lags.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Positive lag-1 ACF → momentum at the daily scale; negative → mean
/// reversion; near-zero → random-walk-like. Including longer lags
/// catches horizon-dependent regimes missed by lag-1 alone.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AutocorrelationSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize, // number of log returns used
    pub lag1_acf: f64,    // autocorrelation at lag 1
    pub lag5_acf: f64,    // autocorrelation at lag 5
    pub lag10_acf: f64,   // autocorrelation at lag 10
    pub lag20_acf: f64,   // autocorrelation at lag 20
    pub mean_log_return: f64,
    pub regime_label: String, // "MEAN_REVERTING" | "NEUTRAL" | "MOMENTUM" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// HURST — Hurst exponent via rescaled-range (R/S) analysis.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// H ∈ [0,1]: H<0.5 anti-persistent / mean-reverting,
/// H≈0.5 random walk, H>0.5 persistent / trending.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HurstSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub hurst_exponent: f64,
    pub scales_used: usize,   // number of R/S scales fit
    pub min_scale: usize,     // smallest chunk size
    pub max_scale: usize,     // largest chunk size
    pub memory_label: String, // "STRONG_MEAN_REVERT" | "MEAN_REVERT" | "RANDOM_WALK" | "PERSISTENT" | "STRONG_PERSISTENT" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// HITRATE — Multi-horizon hit rate.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Fraction of positive-return bars at 5d / 20d / 60d / 252d sliding
/// windows. Also reports the all-window share for context. Bullish
/// when every short-horizon window is above 55%; bearish when every
/// short-horizon window is below 45%.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HitRateSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub hitrate_5d: f64,   // positive share over last 5 bars
    pub hitrate_20d: f64,  // positive share over last 20 bars
    pub hitrate_60d: f64,  // positive share over last 60 bars
    pub hitrate_252d: f64, // positive share over last 252 bars
    pub up_days: usize,
    pub down_days: usize,
    pub flat_days: usize,
    pub hit_label: String, // "BEARISH" | "WEAK_BEARISH" | "NEUTRAL" | "WEAK_BULLISH" | "BULLISH" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// GLASYM — Gain/loss asymmetry.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Compares the magnitude of up-days vs down-days, independent of
/// count. Ratio > 1 → typical up-day bigger than typical down-day
/// (upside asymmetry); < 1 → downside asymmetry. Complements RETSKEW
/// (third-moment tail asymmetry) with an average-move view.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GainLossAsymmetrySnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub avg_up_pct: f64,      // mean |up-day return| (%)
    pub avg_down_pct: f64,    // mean |down-day return| (%)
    pub median_up_pct: f64,   // median |up-day return| (%)
    pub median_down_pct: f64, // median |down-day return| (%)
    pub magnitude_ratio: f64, // avg_up_pct / avg_down_pct
    pub up_days: usize,
    pub down_days: usize,
    pub asymmetry_label: String, // "DOWNSIDE_HEAVY" | "SLIGHT_DOWNSIDE" | "BALANCED" | "SLIGHT_UPSIDE" | "UPSIDE_HEAVY" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// VOLRATIO — Up-day volume vs down-day volume ratio.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Ratio > 1 → heavier volume on up-days than down-days
/// (accumulation); < 1 → heavier volume on down-days (distribution).
/// Uses the `volume` field of HP bars so it gracefully emits
/// INSUFFICIENT_DATA when the cache was populated without volume.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolumeRatioSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub avg_up_volume: f64,   // mean volume on up-days
    pub avg_down_volume: f64, // mean volume on down-days
    pub median_up_volume: f64,
    pub median_down_volume: f64,
    pub up_down_volume_ratio: f64, // avg_up_volume / avg_down_volume
    pub max_up_volume: f64,        // largest single up-day volume in window
    pub max_down_volume: f64,      // largest single down-day volume in window
    pub up_days: usize,
    pub down_days: usize,
    pub flow_label: String, // "DISTRIBUTION" | "SLIGHT_DISTRIBUTION" | "NEUTRAL" | "SLIGHT_ACCUMULATION" | "ACCUMULATION" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── Round 24 — HP drawup/gap/vol-cluster/close-placement/AR(1) stats ──

/// DRAWUP — Rally history (mirror of DDHIST).
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Tracks the running trough and each run from trough-to-peak: max
/// drawup, longest duration, and count of ≥5% / ≥10% rallies.
/// Complements DDHIST with the upside equivalent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawupHistorySnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub max_drawup_pct: f64, // deepest rally from a trough (positive)
    pub max_drawup_trough_date: String,
    pub max_drawup_peak_date: String,
    pub longest_drawup_days: usize, // sessions from trough to next failure or end of window
    pub rallies_5pct: usize,        // count of local-trough-to-peak advances ≥5%
    pub rallies_10pct: usize,       // count of local-trough-to-peak advances ≥10%
    pub current_drawup_pct: f64,    // latest close vs running trough (positive or 0)
    pub rally_label: String, // "MUTED" | "MILD" | "MEANINGFUL" | "STRONG" | "EXPLOSIVE" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// GAPSTATS — Overnight gap statistics.
/// Pure symbol-local HP stat. A "gap" is `(open_t - close_{t-1}) / close_{t-1}`.
/// Reports gap frequency and magnitude in both directions plus the single
/// largest gap up / down in the window. First surface in the packet to
/// read the bar.open field rather than close-only. Label classifies the
/// bias as UP_BIAS / NEUTRAL / DOWN_BIAS based on the average net gap.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GapStatsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub gap_up_count: usize,       // gap > +0.5%
    pub gap_down_count: usize,     // gap < -0.5%
    pub avg_gap_pct: f64,          // mean of all gap %s
    pub avg_gap_up_pct: f64,       // mean of up-gaps only
    pub avg_gap_down_pct: f64,     // mean of down-gaps only
    pub largest_gap_up_pct: f64,   // single largest gap up
    pub largest_gap_down_pct: f64, // single largest gap down (negative)
    pub gap_frequency_pct: f64,    // (gap_up + gap_down) / total_bars * 100
    pub bias_label: String, // "DOWN_BIAS" | "SLIGHT_DOWN" | "NEUTRAL" | "SLIGHT_UP" | "UP_BIAS" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// VOLCLUSTER — Volatility clustering autocorrelation.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// ACF of squared returns and |returns| — the canonical test for ARCH /
/// GARCH effects. High |r| autocorrelation at lag 1 means "big moves
/// follow big moves" (volatility clustering) even if AUTOCOR shows no
/// serial dependence in return sign. Label is bucketed from lag-1 ACF
/// of absolute returns because that's the most common reference metric.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolClusterSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub sq_acf_lag1: f64, // ACF of r² at lag 1
    pub sq_acf_lag5: f64,
    pub sq_acf_lag20: f64,
    pub abs_acf_lag1: f64, // ACF of |r| at lag 1
    pub abs_acf_lag5: f64,
    pub abs_acf_lag20: f64,
    pub cluster_label: String, // "NONE" | "MILD" | "MODERATE" | "STRONG" | "VERY_STRONG" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// CLOSEPLC — Close placement within daily range.
/// Pure symbol-local HP stat. For each bar: `pos = (close - low) / (high - low)`
/// (∈ [0, 1]). Averaged over the window, this captures bar "anatomy":
/// near 1.0 → closes typically pin near the high (buyers in control),
/// near 0.0 → closes near the low (sellers in control). Reports the
/// share of bars that closed in the top 20% of the range ("near high")
/// and bottom 20% ("near low") alongside the mean and median positions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClosePlacementSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,        // bars with high > low
    pub avg_placement: f64,      // mean pos ∈ [0, 1]
    pub median_placement: f64,   // median pos ∈ [0, 1]
    pub latest_placement: f64,   // latest bar's pos
    pub pct_near_high: f64,      // % of bars with pos > 0.8
    pub pct_near_low: f64,       // % of bars with pos < 0.2
    pub placement_label: String, // "STRONG_BEAR" | "BEAR" | "NEUTRAL" | "BULL" | "STRONG_BULL" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// MRHL — Mean-reversion half-life via AR(1) fit.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Fits `r_t = α + β r_{t-1} + ε` to log returns, then reports
/// half-life = -ln(2) / ln(|β|) for 0 < β < 1 (persistent regime with
/// finite memory decay). β ≤ 0 → same-period mean reversion (label
/// FAST_REVERT, half-life undefined). β ≥ 1 → explosive (shouldn't
/// happen on stationary log returns). Complements AUTOCOR (lag ACF)
/// and HURST (multi-scale persistence) with the explicit "how many
/// days until a shock decays" view.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MeanReversionHalfLifeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub beta: f64,            // AR(1) slope
    pub alpha: f64,           // AR(1) intercept
    pub half_life_days: f64,  // -ln(2) / ln(|β|) for β ∈ (0, 1); else 0
    pub r_squared: f64,       // goodness-of-fit
    pub regime_label: String, // "FAST_REVERT" | "MEAN_REVERTING" | "NEUTRAL" | "PERSISTENT" | "STRONG_PERSISTENT" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── Round 25 — HP downside-vol / Sharpe / efficiency / wick / vol-of-vol ──
//
// Five more pure symbol-local HP surfaces computed from the trailing 253-
// session window. DOWNVOL and SHARPR are classical return-distribution risk
// metrics that AUTOCOR/HURST/GLASYM don't cover. EFFRATIO is Kaufman's
// efficiency ratio — a clean "trend vs noise" signal complementary to HURST.
// WICKBIAS pairs with CLOSEPLC on the bar-anatomy axis (wicks instead of
// body placement). VOLOFVOL captures "is the vol regime stable?" and is
// the textbook companion to VOLCLUSTER.

/// DOWNVOL — Downside deviation + Sortino ratio.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Semi-deviation uses only negative log returns: `sqrt(mean(min(r,0)²))`.
/// Sortino = `mean(r) / downside_dev` (dimensionless, same sign as mean
/// return). Complements the full-stdev view in RSTATS by isolating
/// "scary vol" from total vol. Label classifies Sortino into the
/// standard VERY_POOR / POOR / NEUTRAL / GOOD / EXCELLENT bands.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DownsideVolSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub mean_log_return: f64,       // mean r over window
    pub downside_dev: f64,          // sqrt(mean(min(r,0)²))
    pub downside_dev_ann: f64,      // downside_dev × √252
    pub upside_dev: f64,            // sqrt(mean(max(r,0)²))
    pub sortino_ratio: f64,         // mean(r) / downside_dev
    pub sortino_ratio_ann: f64,     // (mean × 252) / downside_dev_ann
    pub downside_pct_of_total: f64, // downside_dev² / total_var × 100
    pub sortino_label: String, // "VERY_POOR" | "POOR" | "NEUTRAL" | "GOOD" | "EXCELLENT" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// SHARPR — Sharpe ratio snapshot.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Classical Sharpe = `(mean_return - rf) / stdev_return`. We use `rf = 0`
/// because the HP cache doesn't carry a risk-free series and most
/// single-stock Sharpe conversations use the excess-above-zero
/// formulation. Both raw and annualized forms are reported. Label
/// classifies into POOR / BELOW_AVG / NEUTRAL / GOOD / EXCELLENT per
/// the standard buckets on annualized Sharpe.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SharpeRatioSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub mean_log_return: f64,
    pub stdev_log_return: f64,
    pub sharpe_ratio: f64,     // raw daily
    pub sharpe_ratio_ann: f64, // × √252
    pub mean_return_ann: f64,  // mean × 252
    pub stdev_return_ann: f64, // stdev × √252
    pub sharpe_label: String, // "POOR" | "BELOW_AVG" | "NEUTRAL" | "GOOD" | "EXCELLENT" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// EFFRATIO — Kaufman's efficiency ratio.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// `ER = |close_N - close_1| / Σ |close_t - close_{t-1}|`. Measures
/// the "directness" of a price move — 1.0 → straight line, 0.0 → all
/// chop with zero net movement. Complements HURST (multi-scale
/// persistence) and MRHL (shock decay) with a cleaner single-number
/// "signal-to-noise in price travel" view. Label: CHOP (<0.1) /
/// NOISY (<0.25) / MIXED (<0.4) / TRENDING (<0.6) / STRONG_TREND (≥0.6).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EfficiencyRatioSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub start_close: f64,
    pub end_close: f64,
    pub net_change: f64,          // signed end - start
    pub net_change_pct: f64,      // (end/start - 1) × 100
    pub sum_abs_changes: f64,     // Σ |close_t - close_{t-1}|
    pub efficiency_ratio: f64,    // |net| / sum_abs (signed direction separate)
    pub signed_efficiency: f64,   // efficiency_ratio × sign(net_change)
    pub efficiency_label: String, // "CHOP" | "NOISY" | "MIXED" | "TRENDING" | "STRONG_TREND" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// WICKBIAS — Upper vs lower wick asymmetry.
/// Pure symbol-local HP stat. For each bar with `high > low`:
/// `upper_wick = (high - max(open, close)) / (high - low)`
/// `lower_wick = (min(open, close) - low) / (high - low)`.
/// Averaged over the window, this captures who rejected price at the
/// extremes: long upper wicks = sellers rejecting the high; long
/// lower wicks = buyers defending the low. Reports means, medians,
/// and a bias score = `avg_lower - avg_upper` (positive = buyers).
/// Complements CLOSEPLC (where the bar closes within its range) on
/// the wick side (how far the bar traveled outside its body).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WickBiasSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,    // bars with high > low
    pub avg_upper_wick: f64, // mean upper wick share
    pub avg_lower_wick: f64, // mean lower wick share
    pub median_upper_wick: f64,
    pub median_lower_wick: f64,
    pub avg_body_share: f64,  // 1 - upper - lower
    pub wick_bias_score: f64, // avg_lower - avg_upper
    pub bias_label: String, // "SELLER_REJECT" | "SELLER_LEAN" | "NEUTRAL" | "BUYER_LEAN" | "BUYER_DEFEND" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// VOLOFVOL — Standard deviation of rolling 20-day realized volatility.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// For each trailing window of 20 bars, compute realized vol = stdev of
/// log returns; then report mean and stdev of the resulting series. This
/// captures "is the vol regime stable, or does vol itself bounce?" —
/// a name with high vol-of-vol has unpredictable risk even if its
/// average vol is moderate. Label classifies `stdev(rv20) / mean(rv20)`
/// (coefficient of variation) into STABLE / MILD / MODERATE /
/// UNSTABLE / CHAOTIC buckets.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolOfVolSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize, // bars with valid rv20 values
    pub mean_rv20: f64,   // mean of rolling 20d vol (daily)
    pub stdev_rv20: f64,  // stdev of rolling 20d vol
    pub min_rv20: f64,
    pub max_rv20: f64,
    pub latest_rv20: f64,
    pub cv_rv20: f64,     // stdev_rv20 / mean_rv20 (coefficient of variation)
    pub cv_label: String, // "STABLE" | "MILD" | "MODERATE" | "UNSTABLE" | "CHAOTIC" | "INSUFFICIENT_DATA"
    pub note: String,
}
