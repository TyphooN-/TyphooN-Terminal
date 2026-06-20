use serde::{Deserialize, Serialize};

// Calmar, ulcer, variance-ratio, Amihud, Jarque-Bera, omega, DFA, Burke, seasonality, spread, and quant-risk-stat research types

/// CALMAR — Calmar ratio: annualized return / max drawdown.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// The canonical drawdown-adjusted return metric. Reports both
/// components (annualized return, max drawdown) plus the ratio.
/// Label classifies on the Calmar ratio value.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CalmarRatioSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub total_return_pct: f64,      // (last/first - 1) × 100
    pub annualized_return_pct: f64, // total × (252 / bars_used)
    pub max_drawdown_pct: f64,      // deepest peak-to-trough decline (positive number, %)
    pub calmar_ratio: f64,          // annualized_return / max_drawdown (signed)
    pub calmar_label: String, // "VERY_POOR" | "POOR" | "NEUTRAL" | "GOOD" | "EXCELLENT" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// ULCER — Ulcer index + Martin ratio (UPI).
/// Pure symbol-local HP stat over the trailing 253-session window.
/// `ulcer_index = sqrt(mean(dd²))` where `dd = (price - peak) / peak × 100`.
/// A continuous drawdown-weighted risk measure. Martin ratio = annualized
/// return / ulcer_index, the drawdown-analogue of Sharpe.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UlcerIndexSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ulcer_index: f64,       // sqrt(mean(dd_pct²))
    pub mean_drawdown_pct: f64, // mean of running dd series (always ≤ 0)
    pub max_drawdown_pct: f64,  // deepest dd point (always ≤ 0, most negative)
    pub pct_in_drawdown: f64,   // share of bars strictly below running peak (0-100)
    pub annualized_return_pct: f64,
    pub martin_ratio: f64,   // annualized_return / ulcer_index (UPI)
    pub ulcer_label: String, // "LOW_PAIN" | "MILD" | "MODERATE" | "HIGH" | "SEVERE" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// VARRATIO — Lo-MacKinlay variance ratio at multiple horizons.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// `VR(q) = Var(q-period returns) / (q × Var(1-period returns))`.
/// VR = 1 → random walk; VR > 1 → trending; VR < 1 → mean-reverting.
/// This is the formal random-walk hypothesis *test* (with z-statistics),
/// unlike HURST/AUTOCOR which are descriptive.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VarianceRatioSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub vr_2: f64,        // VR at horizon 2
    pub vr_5: f64,        // VR at horizon 5
    pub vr_10: f64,       // VR at horizon 10
    pub vr_20: f64,       // VR at horizon 20
    pub z_stat_2: f64,    // Lo-MacKinlay z-statistic at horizon 2
    pub z_stat_5: f64,    // Lo-MacKinlay z-statistic at horizon 5
    pub rw_label: String, // "STRONG_REVERT" | "MEAN_REVERT" | "RANDOM_WALK" | "TRENDING" | "STRONG_TREND" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// AMIHUD — Amihud illiquidity ratio.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// `ILLIQ = mean(|r_t| / dollar_volume_t) × 1e6`. The canonical
/// microstructure liquidity scalar — higher = less liquid = more
/// price impact per dollar traded. Uses close × volume for dollar volume.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AmihudIlliqSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,       // days with valid dollar volume > 0
    pub mean_illiq: f64,        // mean(|r| / dvol) × 1e6
    pub median_illiq: f64,      // median of daily ILLIQ × 1e6
    pub illiq_90th: f64,        // 90th percentile — worst liquidity day in 10
    pub avg_dollar_volume: f64, // average daily close × volume
    pub illiq_label: String, // "VERY_LIQUID" | "LIQUID" | "MODERATE" | "ILLIQUID" | "VERY_ILLIQUID" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// JBNORM — Jarque-Bera normality test.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// `JB = (n/6)(S² + K²/4)` where S = sample skewness and K = excess
/// kurtosis. Under H₀ (normality), JB ~ χ²(2). The p-value is exact:
/// `p = exp(-JB/2)` for χ²(2). Combines RETSKEW + RETKURT into a
/// single actionable "can we reject normality?" answer.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JarqueBeraSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub skewness: f64,        // sample skewness
    pub excess_kurtosis: f64, // sample excess kurtosis (normal = 0)
    pub jb_statistic: f64,    // (n/6)(S² + K²/4)
    pub jb_pvalue: f64,       // exp(-JB/2) for χ²(2)
    pub normal_label: String, // "NORMAL" | "MILD_DEPARTURE" | "MODERATE_DEPARTURE" | "NON_NORMAL" | "STRONGLY_NON_NORMAL" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── HP omega / DFA / Burke / monthly-seas / Roll-spread ──

/// OMEGA — Omega ratio at threshold 0.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// `Ω(τ) = E[max(r-τ, 0)] / E[max(τ-r, 0)]`. Uses the *full* return
/// distribution (not just mean + variance like Sharpe). At τ=0:
/// gains-sum / losses-sum (both in absolute terms). A moment-free
/// companion to SHARPR, DOWNVOL, CALMAR.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OmegaRatioSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub gains_sum: f64,      // Σ max(r, 0) over log-returns
    pub losses_sum: f64,     // Σ max(-r, 0) over log-returns
    pub gain_days: usize,    // count of positive return days
    pub loss_days: usize,    // count of negative return days
    pub omega_ratio: f64,    // gains_sum / losses_sum
    pub win_rate_pct: f64,   // gain_days / (gain_days + loss_days) × 100
    pub omega_label: String, // "VERY_POOR" | "POOR" | "NEUTRAL" | "GOOD" | "EXCELLENT" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// DFA — Detrended fluctuation analysis (Hurst alternative).
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Profile = cumulative sum of demeaned log-returns. For each window
/// size s in a geometric grid, split the profile into non-overlapping
/// boxes, detrend each by linear fit, and RMS the residuals → F(s).
/// log-log regress F(s) ~ s yields α (Hurst exponent). α ≈ 0.5
/// uncorrelated; α > 0.5 persistent; α < 0.5 anti-persistent. Robust
/// to non-stationarity; complementary to HURST (R/S) and VARRATIO.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DetrendedFluctuationSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub alpha: f64,        // DFA exponent (≈ Hurst)
    pub num_scales: usize, // distinct window sizes sampled
    pub r_squared: f64,    // quality of log-log fit
    pub dfa_label: String, // "ANTI_PERSISTENT" | "MEAN_REVERTING" | "RANDOM_WALK" | "PERSISTENT" | "STRONGLY_PERSISTENT" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// BURKE — Burke ratio (sum-of-squared drawdowns adjusted return).
/// Pure symbol-local HP stat over the trailing 253-session window.
/// `Burke = annualized_return / sqrt(Σ dd_i²)` over *trough events*
/// (local minima of the running drawdown series). Between CALMAR
/// (max-dd only) and ULCER (RMS of all dd), Burke weights by the
/// distinct drawdown events, emphasizing top-k worst episodes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BurkeRatioSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub annualized_return_pct: f64,
    pub dd_event_count: usize, // distinct peak-to-trough completed drawdown episodes
    pub sum_sq_drawdowns: f64, // Σ dd_i² in pct² units
    pub worst_event_dd_pct: f64, // deepest individual episode (positive %)
    pub burke_ratio: f64,      // ann_ret / sqrt(sum_sq_drawdowns)
    pub burke_label: String, // "VERY_POOR" | "POOR" | "NEUTRAL" | "GOOD" | "EXCELLENT" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// MONTHSEAS — monthly seasonality hit rate.
/// Pure symbol-local HP stat over *all* available bars (not just
/// trailing 253). For each calendar month (1..12), counts the number
/// of historical years where the month's close-to-close return was
/// positive, divided by the total years observed. The canonical
/// "January effect / Sell-in-May" axis — calendar-aware view that
/// no other packet surface captures.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MonthlySeasonalitySnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub years_covered: usize, // number of distinct years with at least one usable month pair
    pub month_hit_pct: [f64; 12], // share of positive months per Jan..Dec (0-100)
    pub month_mean_ret_pct: [f64; 12], // mean close-to-close % return per Jan..Dec
    pub best_month_idx: usize, // 0-based index of strongest month (0=Jan)
    pub worst_month_idx: usize, // 0-based index of weakest month
    pub best_month_hit_pct: f64,
    pub worst_month_hit_pct: f64,
    pub season_label: String, // "STRONG_SEASONAL" | "MILD_SEASONAL" | "NEUTRAL" | "INCONSISTENT" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// ROLLSPRD — Roll's (1984) implicit bid-ask spread.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// `spread = 2 · √(-Cov(Δp_t, Δp_{t-1}))` on close-to-close price
/// changes. Only valid when the first-lag price-change covariance
/// is *negative* (as bid/ask bounce implies). When covariance is
/// non-negative (trending series), Roll's model falls through to
/// INVALID_POSITIVE_COV. Microstructure companion to AMIHUD:
/// AMIHUD captures price impact per dollar; ROLLSPRD captures the
/// implicit bounce-driven effective spread in bps.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RollSpreadSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub first_lag_cov: f64,       // Cov(Δp_t, Δp_{t-1})
    pub mean_price: f64,          // mean close over window, for bps conversion
    pub implicit_spread: f64,     // 2 · √(-first_lag_cov), price units
    pub implicit_spread_bps: f64, // implicit_spread / mean_price × 1e4
    pub roll_label: String, // "TIGHT" | "NORMAL" | "WIDE" | "VERY_WIDE" | "INVALID_POSITIVE_COV" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── HP range-vol / Garman-Klass / Rogers-Satchell / CVaR / dow-effect ──

/// PARKINSON — Parkinson (1980) high-low range-based volatility estimator.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// `σ² = (1/(4·ln(2)·n)) · Σ (ln(H/L))²`. Uses only H and L — but
/// by virtue of being range-based is ~5.2× more statistically
/// efficient than close-to-close vol. Reported as annualized vol
/// percentage (daily σ × √252 × 100).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParkinsonVolSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub daily_vol_pct: f64,      // daily σ × 100
    pub annualized_vol_pct: f64, // daily σ × √252 × 100
    pub mean_hl_log_ratio: f64,  // mean of ln(H/L) across window
    pub vol_label: String, // "VERY_LOW" | "LOW" | "NORMAL" | "HIGH" | "VERY_HIGH" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// GKVOL — Garman-Klass (1980) OHLC volatility estimator.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// `σ² = (1/n) · Σ [0.5·(ln(H/L))² - (2ln2 - 1)·(ln(C/O))²]`.
/// Combines the H-L range with the C-O drift to achieve ~7.4×
/// efficiency over close-to-close. The most commonly used
/// range-based vol estimator in practice.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GarmanKlassVolSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub daily_vol_pct: f64,
    pub annualized_vol_pct: f64,
    pub range_component: f64, // mean 0.5·(ln H/L)²
    pub co_component: f64,    // mean (2ln2-1)·(ln C/O)²
    pub vol_label: String, // "VERY_LOW" | "LOW" | "NORMAL" | "HIGH" | "VERY_HIGH" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// RSVOL — Rogers-Satchell (1991) drift-independent OHLC volatility estimator.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// `σ² = (1/n) · Σ [ln(H/C)·ln(H/O) + ln(L/C)·ln(L/O)]`.
/// Unlike Parkinson and Garman-Klass, Rogers-Satchell is **unbiased
/// under non-zero drift** — it correctly estimates variance even
/// when the underlying series has a non-zero mean log-return per bar.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RogersSatchellVolSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub daily_vol_pct: f64,
    pub annualized_vol_pct: f64,
    pub vol_label: String, // 5-bucket vol label (same scheme as PARKINSON/GKVOL)
    pub note: String,
}

/// CVAR — Conditional Value-at-Risk / Expected Shortfall at 5%.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Identifies the 5th percentile of daily log returns (VaR) and
/// reports the *mean* of returns that are ≤ that threshold (ES).
/// Distinct from TAILR (which reports the quantile ratio) and
/// DOWNVOL (variance of negative returns): CVaR answers
/// "given we're in the worst 5% of days, what's the *average* loss?"
/// — the coherent downside-risk measure preferred by Basel III and
/// most modern risk frameworks.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CVaRSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub var_5pct_ret_pct: f64,  // 5th percentile daily return, as %
    pub cvar_5pct_ret_pct: f64, // mean of returns ≤ VaR(5%), as %
    pub var_1pct_ret_pct: f64,  // 1st percentile daily return, as %
    pub cvar_1pct_ret_pct: f64, // mean of returns ≤ VaR(1%), as %
    pub tail_days_5pct: usize,  // count of days in the 5% tail
    pub tail_days_1pct: usize,
    pub cvar_label: String, // "MINIMAL" | "LOW" | "MODERATE" | "HIGH" | "EXTREME" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// DOWEFFECT — Day-of-week seasonality hit rate + mean return.
/// Pure symbol-local HP stat over the *full* HP cache (not 253-windowed).
/// For each weekday (Mon-Fri) reports hit rate (share of that weekday
/// which closed positive intraday, O→C) and mean intraday return %.
/// Calendar companion to MONTHSEAS: captures Monday-effect, Friday-rally,
/// Wednesday-weakness etc. that only a day-of-week lens can see.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DayOfWeekEffectSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub weeks_covered: usize,
    pub dow_hit_pct: [f64; 5], // hit rate per Mon..Fri, share of positive O→C
    pub dow_mean_ret_pct: [f64; 5], // mean intraday % return per Mon..Fri
    pub dow_sample_count: [usize; 5], // count of samples per weekday
    pub best_dow_idx: usize,   // 0=Mon..4=Fri
    pub worst_dow_idx: usize,
    pub best_dow_hit_pct: f64,
    pub worst_dow_hit_pct: f64,
    pub dow_label: String, // "STRONG_EFFECT" | "MILD_EFFECT" | "NEUTRAL" | "INCONSISTENT" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── HP Sterling / Kelly / Ljung-Box / runs test / zero-return ──

/// STERLING — Sterling ratio: annualized return / average of N worst drawdowns.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Distinct drawdown ratio family completion: CALMAR uses only max-dd,
/// ULCER is RMS of all continuous dd points, BURKE sums squared event
/// drawdowns — STERLING uses the *arithmetic mean* of the top-N (default 5)
/// worst distinct drawdown events. This gives the most directly interpretable
/// "average of my worst N drawdowns" reading.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SterlingRatioSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub annualized_return_pct: f64,
    pub worst_n: usize,         // N worst distinct dd events used (default 5)
    pub dd_event_count: usize,  // total distinct dd events in window
    pub mean_worst_dd_pct: f64, // mean of worst N event drawdowns, as %
    pub sterling_ratio: f64,    // annualized_return / mean_worst_dd (magnitudes)
    pub sterling_label: String, // "VERY_POOR" | "POOR" | "NEUTRAL" | "GOOD" | "EXCELLENT" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// KELLYF — Kelly fraction / optimal leverage.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Classical position-sizing scalar: `f* = (b·p − q) / b` where
/// p=win rate, q=1−p, b=avg_win/avg_loss. First packet surface in the
/// position-sizing axis — SHARPR/DOWNVOL/CALMAR etc all measure
/// realized risk-adjusted performance; KELLYF gives a forward-looking
/// optimal-stake scalar derived from the same return distribution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KellyFractionSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub win_rate: f64,       // p, fraction of positive-return days
    pub loss_rate: f64,      // q = 1 − p (positive-return days exclude zero)
    pub avg_win_pct: f64,    // mean of positive daily returns (%), 0 if none
    pub avg_loss_pct: f64,   // mean of |negative| daily returns (%), 0 if none
    pub win_loss_ratio: f64, // b = avg_win / avg_loss, ∞-handling emits 0.0
    pub kelly_fraction: f64, // f* = (b·p − q) / b; can be negative (skip) or capped
    pub half_kelly: f64,     // kelly/2, conservative practitioner default
    pub kelly_label: String, // "SKIP" | "MARGINAL" | "MODERATE" | "AGGRESSIVE" | "ALL_IN" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// LJUNGB — Ljung-Box Q-statistic at lag 10 (portmanteau autocorrelation test).
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Complements AUTOCOR (which reports individual-lag ACFs at 1/5/10/20)
/// with a *joint* test: `Q = n(n+2) · Σ(ρ_k²/(n−k))` for k=1..h, with
/// `Q ~ χ²(h)` under the null. Gives a single combined-lag p-value for
/// the "returns are white noise" hypothesis — the canonical
/// econometrics test for model adequacy.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LjungBoxSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub lag_h: usize,             // h, usually 10
    pub q_statistic: f64,         // Q
    pub p_value: f64,             // P(χ²(h) ≥ Q)
    pub reject_white_noise: bool, // p < 0.05
    pub ljungb_label: String, // "WHITE_NOISE" | "WEAK_DEP" | "MODERATE_DEP" | "STRONG_DEP" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// RUNSTEST — Wald-Wolfowitz runs test for randomness of the sign sequence.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Given the sequence of positive/negative daily returns, counts the
/// number of runs (blocks of consecutive same-signed returns) and
/// compares to its null distribution (mean = 2n₁n₂/n + 1,
/// variance = 2n₁n₂(2n₁n₂−n) / (n²(n−1))). Distinct from RUNLEN, which
/// is descriptive (longest/mean streak); RUNSTEST is inferential (z-stat +
/// p-value against the "sign sequence is random" null).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunsTestSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub positive_days: usize,
    pub negative_days: usize,
    pub runs_observed: usize,
    pub runs_expected: f64,
    pub runs_std: f64,
    pub z_statistic: f64,
    pub p_value: f64,            // two-sided
    pub reject_randomness: bool, // p < 0.05
    pub runs_label: String, // "RANDOM" | "SLIGHT_CLUST" | "MOD_CLUST" | "STRONG_CLUST" | "ANTI_CLUST" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// ZERORET — Lesmond-Ogden-Trzcinka zero-return-day fraction.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Fraction of bars with |log_return| < epsilon (default 1e-6, i.e.
/// exactly unchanged close) as a canonical academic liquidity proxy:
/// illiquid securities show more zero-return days (dealers don't update
/// the close because nobody traded). Distinct from AMIHUD (price
/// impact per $) and ROLLSPRD (implicit bid-ask spread) — ZERORET is
/// the third foundational microstructure scalar.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ZeroReturnSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub zero_day_count: usize,
    pub zero_day_pct: f64,          // 100 * zero_count / bars_used
    pub longest_zero_streak: usize, // longest run of consecutive zero-return bars
    pub epsilon: f64,               // threshold used (default 1e-6)
    pub zero_label: String, // "HIGHLY_LIQUID" | "LIQUID" | "MODERATE" | "ILLIQUID" | "VERY_ILLIQUID" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── PSR / ADF / MNKENDALL / BIPOWER / DDDUR ──────────────

/// PSR — Probabilistic Sharpe Ratio (Lopez de Prado 2012).
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Addresses the well-known critique that the classical Sharpe
/// ratio assumes normally-distributed returns. PSR(SR*) is the
/// *probability* that the true Sharpe exceeds some benchmark SR*,
/// computed as
/// `PSR = Φ((SR − SR*)·√(n−1) / √(1 − γ₃·SR + (γ₄−1)/4·SR²))`
/// where γ₃ = sample skewness, γ₄ = sample kurtosis (not excess).
/// Higher PSR at SR*=0 means the positive Sharpe is unlikely to be
/// a sampling fluke. First packet surface to correct a return-
/// quality ratio for higher-order moments.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProbabilisticSharpeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub sharpe: f64,       // observed annualized Sharpe (same convention as SHARPR)
    pub skewness: f64,     // sample γ₃
    pub kurtosis: f64,     // sample γ₄ (not excess)
    pub sr_benchmark: f64, // SR* used (default 0)
    pub psr: f64,          // PSR(SR*) ∈ [0, 1]
    pub psr_label: String, // "VERY_LOW" <0.50 / "LOW" <0.75 / "MODERATE" <0.90 / "HIGH" <0.95 / "VERY_HIGH" ≥0.95 / INSUFFICIENT_DATA
    pub note: String,
}

/// ADF — Augmented Dickey-Fuller unit-root / stationarity test.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Applied to **log prices** (not returns). Regresses
/// `Δlog(p)_t = α + β·log(p)_{t-1} + ε` and reports
/// `t-stat = β̂ / se(β̂)` against Dickey-Fuller critical values
/// (MacKinnon 1996 approximation). Rejection of H₀ (β=0) means
/// the log-price series is stationary. Complements Hurst (long-
/// memory exponent) and DFA (nonstationarity-robust persistence)
/// with a formal unit-root hypothesis test. Note: this is the
/// zero-lag DF test, not the augmented form — the lag-0 variant
/// is standard in trading literature.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DickeyFullerSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub beta: f64,              // OLS slope on lagged log-price
    pub se_beta: f64,           // standard error of β̂
    pub t_statistic: f64,       // β̂ / se(β̂)
    pub crit_1pct: f64,         // -3.43 (constant-only)
    pub crit_5pct: f64,         // -2.86
    pub crit_10pct: f64,        // -2.57
    pub reject_unit_root: bool, // t < crit_5pct
    pub adf_label: String, // "STATIONARY" / "BORDERLINE" / "NON_STATIONARY" / INSUFFICIENT_DATA
    pub note: String,
}

/// MNKENDALL — Mann-Kendall nonparametric trend test.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Applied to log prices. Counts sign(x_j − x_i) over all i<j to
/// form `S = Σᵢ<ⱼ sign(x_j − x_i)`. Under H₀ (no trend), S is
/// approximately normal with mean 0 and variance
/// `n(n−1)(2n+5)/18` (no ties correction). Z-statistic and
/// two-sided p-value via standard normal CDF. Distribution-free
/// (does not assume linearity or normality) — complements
/// Hurst/DFA (persistence) and ADF (stationarity) with a
/// formal trend-presence test.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MannKendallSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub s_statistic: i64, // Kendall S sum
    pub variance: f64,    // Var(S) under null
    pub z_statistic: f64,
    pub p_value: f64,          // two-sided
    pub tau: f64,              // Kendall τ = S / (n·(n-1)/2)
    pub reject_no_trend: bool, // p < 0.05
    pub mk_label: String, // "STRONG_UP" / "UP" / "NO_TREND" / "DOWN" / "STRONG_DOWN" / INSUFFICIENT_DATA
    pub note: String,
}

/// BIPOWER — Bipower variation and realized-jump ratio.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Barndorff-Nielsen & Shephard (2004) decomposition: the realized
/// variance `RV = Σr_t²` includes both continuous (diffusive)
/// volatility and jumps. Bipower variation
/// `BPV = (π/2)·Σ|r_t|·|r_{t-1}|` converges to the integrated
/// variance of the continuous component *only*, under mild
/// conditions. Jump ratio `1 − BPV/RV` ∈ [0, 1] estimates the
/// share of realized variance attributable to jumps. Large
/// jump ratio ⇒ returns are dominated by discrete events;
/// small ⇒ classic diffusive behaviour. Distinct from the
/// vol-level estimators (CLOSEVOL/PARKINSON/GKVOL/RSVOL) — this
/// is a *composition* metric, not a magnitude.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BipowerVariationSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub realized_var: f64,           // Σ r_t²
    pub bipower_var: f64,            // BPV
    pub continuous_vol_ann_pct: f64, // √(BPV · 252/n) × 100
    pub realized_vol_ann_pct: f64,   // √(RV · 252/n) × 100 (close-to-close annualized)
    pub jump_ratio: f64,             // max(0, 1 − BPV/RV), clamped to [0, 1]
    pub jump_pct: f64,               // 100 × jump_ratio
    pub jump_label: String, // "NO_JUMPS" <0.05 / "MILD_JUMPS" <0.20 / "NOTABLE_JUMPS" <0.40 / "HEAVY_JUMPS" ≥0.40 / INSUFFICIENT_DATA
    pub note: String,
}

/// DDDUR — Drawdown duration statistics.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Walks the closing-price series with a running-max tracker and
/// records, for each *closed* drawdown event, the number of bars
/// from peak to recovery. Complements the *magnitude*-focused
/// CALMAR (single worst dd) / BURKE (sum-of-squares) / STERLING
/// (mean of N worst) family with a *duration* axis: "how long
/// am I underwater?" Reports max/mean/median event durations,
/// total bars underwater in the window, % of time underwater, and
/// (if a drawdown is still open at window end) a `currently_underwater`
/// flag with `current_dd_duration`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawdownDurationSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub dd_event_count: usize, // closed drawdowns recorded
    pub max_dd_duration_bars: usize,
    pub mean_dd_duration_bars: f64,
    pub median_dd_duration_bars: f64,
    pub total_bars_underwater: usize, // including ongoing
    pub pct_time_underwater: f64,     // 100 × total / bars_used
    pub currently_underwater: bool,
    pub current_dd_duration_bars: usize,
    pub dddur_label: String, // "MOSTLY_DRY" <20% / "FREQUENT_DD" <40% / "PERSISTENT_DD" <60% / "DEEP_WATER" ≥60% / INSUFFICIENT_DATA
    pub note: String,
}

/// HILLTAIL — Hill tail-index estimator.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// For order statistics X_(1) ≥ X_(2) ≥ … ≥ X_(n) of |r_t|, the
/// Hill estimator `α̂ = k / Σᵢ₌₁ᵏ log(X_(i) / X_(k+1))` estimates
/// the Pareto-tail index assuming `P(|R| > x) ≈ c·x^(−α)`. Small α
/// ⇒ heavy power-law tails (α ≤ 2 ⇒ infinite variance in the
/// underlying Pareto); large α ⇒ tails decay fast (α > 4 ≈ Gaussian-
/// like). Complements JBNORM (joint normality *test*) and KURT
/// (fourth-moment magnitude) with a *nonparametric power-law
/// exponent*. Separate estimates on left-tail (negative-return
/// magnitudes) and right-tail (positive-return magnitudes) expose
/// tail asymmetry invisible to KURT.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HillTailSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub k_order_stats: usize,  // top-k used (10% of n, min 10)
    pub threshold_abs: f64,    // X_(k+1) from |r| ordering
    pub hill_alpha_abs: f64,   // tail index on |r|
    pub hill_alpha_left: f64,  // tail index on negative-return magnitudes
    pub hill_alpha_right: f64, // tail index on positive-return magnitudes
    pub tail_label: String, // "GAUSSIAN_LIKE" α>4 / "LIGHT_TAIL" α>3 / "MODERATE_TAIL" α>2 / "HEAVY_TAIL" α>1 / "VERY_HEAVY_TAIL" α≤1 / INSUFFICIENT_DATA
    pub note: String,
}

/// ARCHLM — Engle (1982) ARCH Lagrange-multiplier test.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Regresses squared mean-residuals ε_t² on intercept +
/// ε²_{t-1}, …, ε²_{t-q} (here q=5) and reports
/// `LM = n·R² ~ χ²(q)` under H₀ (no conditional heteroskedasticity).
/// Critical values χ²₀.₀₅(5)=11.07, χ²₀.₀₁(5)=15.09 (hardcoded).
/// Complements VOLOFVOL (descriptive rolling-σ scatter) with a
/// formal hypothesis test for volatility clustering, which is the
/// canonical stylized fact of financial returns.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArchLmSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub q_lags: usize,              // 5
    pub r_squared: f64,             // R² of ε² regression
    pub lm_statistic: f64,          // n·R²
    pub p_value: f64,               // via Wilson-Hilferty transform to Φ
    pub crit_5pct_chi2: f64,        // 11.07
    pub crit_1pct_chi2: f64,        // 15.09
    pub reject_homoskedastic: bool, // LM > crit_5pct_chi2
    pub arch_label: String,         // "NO_ARCH" / "WEAK_ARCH" / "STRONG_ARCH" / INSUFFICIENT_DATA
    pub note: String,
}

/// PAINRATIO — Pain index and pain ratio (Zephyr/FIBA).
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Pain Index = arithmetic mean of |dd_t| (%) across every bar of
/// the window, where dd_t = (close_t − peak_t)/peak_t · 100.
/// Pain Ratio = annualized_return / pain_index — the drawdown-
/// averaged analogue of Sharpe/Calmar/Burke/Ulcer/Sterling. Distinct
/// denominators: CALMAR=max, BURKE=√Σdd², STERLING=mean of worst N,
/// ULCER=√mean(dd²) (RMS), PAIN=mean|dd| (L¹). Pain treats every
/// bar equally, not just the worst ones.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PainRatioSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pain_index_pct: f64, // mean |dd_t| in %
    pub annualized_return_pct: f64,
    pub pain_ratio: f64,    // ann_return / pain_index
    pub max_dd_pct: f64,    // companion magnitude
    pub pain_label: String, // "LOW_PAIN" <1 / "MILD_PAIN" <3 / "MODERATE_PAIN" <7 / "HIGH_PAIN" <15 / "SEVERE_PAIN" ≥15 / INSUFFICIENT_DATA
    pub note: String,
}

/// CUSUM — Brown-Durbin-Evans (1975) OLS CUSUM test for mean stability.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Builds standardized cumulative sum
/// `S_t = Σ_{s=1..t} (r_s − r̄) / σ̂` and reports the scaled
/// test statistic `D = max_t |S_t| / √n`, which under H₀ (mean
/// stability) has the Kolmogorov-Smirnov limiting distribution with
/// critical values {10%=1.22, 5%=1.36, 1%=1.63}. Rejection signals
/// a structural break in the return mean. Pairs with ADF
/// (stationarity of levels), LJUNGB (joint autocorrelation), and
/// RUNSTEST (randomness of signs) as the fourth inferential
/// diagnostic and the first structural-break test in the packet.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CusumBreakSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub max_abs_cusum: f64,       // max_t |S_t|
    pub test_statistic: f64,      // max_abs_cusum / √n
    pub max_abs_bar: usize,       // index t where max is achieved
    pub direction_at_max: String, // "UP" if S_t>0 at max, "DOWN" if <0, "NONE" otherwise
    pub crit_10pct: f64,          // 1.22
    pub crit_5pct: f64,           // 1.36
    pub crit_1pct: f64,           // 1.63
    pub reject_stability: bool,   // test_statistic > crit_5pct
    pub cusum_label: String, // "STABLE" / "MARGINAL" / "BREAK_DETECTED" / "STRONG_BREAK" / INSUFFICIENT_DATA
    pub note: String,
}

/// CFVAR — Cornish-Fisher modified Value-at-Risk.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Applies the Cornish-Fisher (1938) expansion
/// `z* = z + (z²−1)·γ₃/6 + (z³−3z)·γ₄/24 − (2z³−5z)·γ₃²/36`
/// to the standard-normal quantile, then reports
/// `CF-VaR = μ + z*·σ`. This corrects the Gaussian VaR quantile
/// for sample skewness (γ₃) and *excess* kurtosis (γ₄). Complements
/// historical CVAR (fully nonparametric tail) with a parametric
/// skew/kurt-aware VaR, useful when an agent wants a smooth
/// analytical quantile rather than an empirical one. Reports both
/// the Gaussian and CF-adjusted quantiles at 5% and 1%, and the
/// dominance of skew-term vs kurt-term in driving any deviation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CornishFisherSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub mean_ret_pct: f64,           // daily mean × 100
    pub sigma_ret_pct: f64,          // daily std × 100
    pub skewness: f64,               // γ₃
    pub excess_kurtosis: f64,        // γ₄ (excess — subtract 3)
    pub gauss_var_5pct_pct: f64,     // μ + (−1.645)·σ × 100
    pub cf_var_5pct_pct: f64,        // μ + z*(5%)·σ × 100
    pub gauss_var_1pct_pct: f64,     // μ + (−2.326)·σ × 100
    pub cf_var_1pct_pct: f64,        // μ + z*(1%)·σ × 100
    pub cf_adjustment_5pct_pct: f64, // cf_var_5pct − gauss_var_5pct
    pub skew_term_5pct: f64,         // (z²−1)·γ₃/6 − (2z³−5z)·γ₃²/36 at z=-1.645
    pub kurt_term_5pct: f64,         // (z³−3z)·γ₄/24 at z=-1.645
    pub cfvar_label: String, // "BENIGN" / "SKEW_DRIVEN" / "KURT_DRIVEN" / "EXTREME_DEVIATION" / INSUFFICIENT_DATA
    pub note: String,
}

// ── structs ──

/// ENTROPY — Shannon entropy of the return distribution.
/// H = −Σ pᵢ log₂(pᵢ) over a histogram of daily log-returns
/// (bins = ceil(√n)). Low H → concentrated/predictable returns;
/// high H → dispersed/unpredictable.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct EntropySnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub num_bins: usize,
    pub entropy_bits: f64,       // H in bits (log base 2)
    pub max_entropy_bits: f64,   // log₂(num_bins) — uniform distribution
    pub normalised_entropy: f64, // H / H_max ∈ [0,1]
    pub entropy_label: String, // LOW_ENTROPY / MODERATE_ENTROPY / HIGH_ENTROPY / VERY_HIGH_ENTROPY / INSUFFICIENT_DATA
    pub note: String,
}

/// RACHEV — Rachev ratio = ES_α(+R) / ES_α(−R).
/// Compares right-tail expected gain to left-tail expected loss
/// at matching confidence levels (5% and 1%). Rachev > 1 ⇒
/// upside tail outweighs downside tail.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct RachevSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub es_right_5pct: f64, // mean of top 5% returns (×100 ⇒ pct)
    pub es_left_5pct: f64,  // mean of bottom 5% returns (×100 ⇒ pct, negative)
    pub rachev_5pct: f64,   // |es_right_5pct| / |es_left_5pct|
    pub es_right_1pct: f64,
    pub es_left_1pct: f64,
    pub rachev_1pct: f64,
    pub rachev_label: String, // STRONG_LEFT_TAIL / LEFT_HEAVY / SYMMETRIC / RIGHT_HEAVY / STRONG_RIGHT_TAIL / INSUFFICIENT_DATA
    pub note: String,
}

/// GPR — Gain-to-Pain Ratio (Schwager).
/// GPR = Σ rₜ / Σ |min(rₜ, 0)|. Also reports Profit Factor =
/// Σ max(rₜ,0) / Σ |min(rₜ, 0)| = GPR + 1.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct GprSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub sum_all_returns_pct: f64, // Σ rₜ × 100
    pub sum_losses_pct: f64,      // Σ |min(rₜ, 0)| × 100 (positive number)
    pub sum_gains_pct: f64,       // Σ max(rₜ, 0) × 100
    pub gain_to_pain: f64,        // GPR = sum_all / sum_losses
    pub profit_factor: f64,       // PF = sum_gains / sum_losses = GPR + 1
    pub win_count: usize,
    pub loss_count: usize,
    pub gpr_label: String, // DEEP_PAIN / NEGATIVE / MODEST / GOOD / EXCELLENT / INSUFFICIENT_DATA
    pub note: String,
}

/// PACF — Partial autocorrelation function at lags 1-5.
/// Uses the Durbin-Levinson recursion to compute PACF from
/// the sample autocorrelation function. Reports individual lag
/// values plus Bartlett 95% critical band ±1.96/√n.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PacfSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pacf_lag1: f64,
    pub pacf_lag2: f64,
    pub pacf_lag3: f64,
    pub pacf_lag4: f64,
    pub pacf_lag5: f64,
    pub bartlett_crit_95: f64,   // ±1.96/√n
    pub significant_lags: usize, // count of lags where |PACF| > crit
    pub max_abs_pacf: f64,
    pub max_abs_lag: usize, // lag number of max |PACF|
    pub pacf_label: String, // NO_STRUCTURE / LAG1_DOMINANT / LAG_STRUCTURE / STRONG_STRUCTURE / INSUFFICIENT_DATA
    pub note: String,
}

/// APEN — Approximate entropy (Pincus 1991).
/// Measures regularity/predictability of a time series.
/// Low ApEn → regular, self-similar patterns; high ApEn →
/// irregular, complex dynamics. Parameters: m=2, r=0.2·σ.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ApenSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub embed_dim: usize,   // m (always 2)
    pub tolerance: f64,     // r = 0.2 × std
    pub phi_m: f64,         // Φ^m(r)
    pub phi_m1: f64,        // Φ^{m+1}(r)
    pub apen: f64,          // Φ^m − Φ^{m+1}
    pub apen_label: String, // REGULAR / MODERATE / COMPLEX / HIGHLY_COMPLEX / INSUFFICIENT_DATA
    pub note: String,
}

// ── structs ──

/// UPR — Upside Potential Ratio (Sortino & van der Meer 1991).
/// UPR = E[max(r−MAR,0)] / √E[min(r−MAR,0)²] where MAR=0.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct UprSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub upm1: f64,         // upper partial moment of order 1
    pub lpm2: f64,         // lower partial moment of order 2
    pub downside_dev: f64, // √LPM₂
    pub upr: f64,          // UPM₁ / downside_dev
    pub upr_label: String, // LOW_UPSIDE / MODERATE_UPSIDE / BALANCED / HIGH_UPSIDE / VERY_HIGH_UPSIDE / INSUFFICIENT_DATA
    pub note: String,
}

/// LEVEREFF — Leverage effect (Black 1976, Christie 1982).
/// Measures asymmetric volatility: negative returns tend to
/// increase future volatility more than positive returns.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct LeverEffSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub corr_r_nextsq: f64,      // corr(rₜ, rₜ₊₁²)
    pub mean_vol_after_neg: f64, // mean |rₜ₊₁| after rₜ < 0 (×100)
    pub mean_vol_after_pos: f64, // mean |rₜ₊₁| after rₜ > 0 (×100)
    pub asym_ratio: f64,         // mean_vol_after_neg / mean_vol_after_pos
    pub lever_label: String, // STRONG_LEVERAGE / MILD_LEVERAGE / SYMMETRIC / REVERSE_LEVERAGE / INSUFFICIENT_DATA
    pub note: String,
}

/// DRAWDAR — Drawdown-at-Risk + Conditional DaR.
/// Quantile-based drawdown risk measure (Chekhlov et al. 2005).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DrawDaRSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub dar_5pct: f64,         // 95th percentile of drawdown distribution (%)
    pub cdar_5pct: f64,        // mean dd given dd > DaR(5%) — conditional DaR (%)
    pub dar_1pct: f64,         // 99th percentile (%)
    pub cdar_1pct: f64,        // conditional DaR at 1% (%)
    pub max_dd_pct: f64,       // max drawdown (%)
    pub mean_dd_pct: f64,      // mean of all non-zero drawdowns (%)
    pub drawdar_label: String, // LOW_DD_RISK / MODERATE_DD_RISK / HIGH_DD_RISK / SEVERE_DD_RISK / INSUFFICIENT_DATA
    pub note: String,
}

/// VARHALF — Volatility half-life (vol persistence).
/// AR(1) on rolling 20d realized vol → half-life = −ln(2)/ln(β).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct VarHalfSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub vol_obs: usize,        // number of rolling-vol observations
    pub ar1_beta: f64,         // AR(1) coefficient
    pub ar1_alpha: f64,        // AR(1) intercept
    pub ar1_r2: f64,           // R² of AR(1) fit
    pub half_life_days: f64,   // −ln(2)/ln(β)
    pub varhalf_label: String, // FAST_REVERT / MODERATE_PERSIST / SLOW_PERSIST / VERY_PERSISTENT / INSUFFICIENT_DATA
    pub note: String,
}

/// GINI — Gini coefficient of |returns|.
/// Measures concentration/inequality of absolute return magnitudes.
/// Gini = 0 → all |returns| equal; Gini = 1 → one return dominates.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct GiniSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub gini_coeff: f64,            // Gini on |returns| ∈ [0,1]
    pub mean_abs_return_pct: f64,   // mean |r| × 100
    pub median_abs_return_pct: f64, // median |r| × 100
    pub gini_label: String, // LOW_CONCENTRATION / MODERATE_CONCENTRATION / HIGH_CONCENTRATION / VERY_HIGH_CONCENTRATION / INSUFFICIENT_DATA
    pub note: String,
}

// ── structs ──

/// SAMPEN — Sample Entropy (Richman & Moorman 2000).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SampenSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub embed_dim: usize,     // m (default 2)
    pub tolerance: f64,       // r (default 0.2·σ)
    pub a_count: usize,       // template matches length m+1 (excl self)
    pub b_count: usize,       // template matches length m (excl self)
    pub sampen: f64,          // −ln(A/B)
    pub sampen_label: String, // REGULAR / MODERATE / COMPLEX / HIGHLY_COMPLEX / INSUFFICIENT_DATA / UNDEFINED
    pub note: String,
}

/// PERMEN — Permutation Entropy (Bandt & Pompe 2002).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PermenSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub embed_dim: usize,         // m (default 3 → 6 ordinal patterns)
    pub patterns_observed: usize, // distinct ordinal patterns seen
    pub patterns_possible: usize, // m! = 6
    pub permen_raw: f64,          // Shannon entropy of pattern distribution
    pub permen_normalised: f64,   // H / log₂(m!) ∈ [0,1]
    pub permen_label: String, // REGULAR / MODERATE / COMPLEX / HIGHLY_COMPLEX / INSUFFICIENT_DATA
    pub note: String,
}

/// RECFACT — Recovery Factor.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RecfactSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub cum_return_pct: f64,   // (last/first − 1) × 100
    pub max_drawdown_pct: f64, // absolute max drawdown × 100
    pub recovery_factor: f64,  // cum_return / |max_drawdown|
    pub recfact_label: String, // DEEP_LOSS / NEGATIVE / RECOVERING / GOOD / EXCELLENT / INSUFFICIENT_DATA
    pub note: String,
}

/// KPSS — Kwiatkowski-Phillips-Schmidt-Shin stationarity test.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct KpssSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub kpss_stat: f64,          // η_μ = Σ S_t² / (n² · s²_ℓ)
    pub lag_truncation: usize,   // ℓ = floor(4·(n/100)^(2/9))
    pub crit_10: f64,            // 0.347
    pub crit_5: f64,             // 0.463
    pub crit_1: f64,             // 0.739
    pub reject_stationary: bool, // true if η_μ > crit_5
    pub kpss_label: String, // STATIONARY / WEAKLY_NONSTATIONARY / NONSTATIONARY / INSUFFICIENT_DATA
    pub note: String,
}

/// SPECENT — Spectral Entropy via DFT.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SpecentSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub num_freqs: usize,           // N/2 frequency bins
    pub spectral_entropy_raw: f64,  // −Σ pₖ log₂(pₖ) on normalised PSD
    pub spectral_entropy_norm: f64, // H / log₂(N/2) ∈ [0,1]
    pub peak_freq_idx: usize,       // index of max PSD bin
    pub peak_power_share: f64,      // fraction of total power at peak
    pub specent_label: String, // PERIODIC / MODERATE_PERIODICITY / BROAD_SPECTRUM / NOISE_LIKE / INSUFFICIENT_DATA
    pub note: String,
}
