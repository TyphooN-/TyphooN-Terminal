// Fractal, jump, stationarity, and statistical-test research types

/// ROBVOL — Robust Volatility (MAD + IQR, outlier-resistant).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RobVolSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub classical_sigma: f64, // standard sample σ (annualised ×√252)
    pub mad_sigma: f64,       // MAD/0.6745 (annualised)
    pub iqr_sigma: f64,       // IQR/1.349 (annualised)
    pub mad_ratio: f64,       // mad_sigma / classical_sigma
    pub iqr_ratio: f64,       // iqr_sigma / classical_sigma
    pub robvol_label: String, // HEAVY_OUTLIERS / MODERATE_OUTLIERS / CLEAN / LIGHT_TAILS / INSUFFICIENT_DATA
    pub note: String,
}

/// RENYIENT — Rényi Entropy at α=2 (collision entropy).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RenyientSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub num_bins: usize,        // histogram bins (Sturges/Freedman-Diaconis)
    pub alpha: f64,             // 2.0
    pub renyi_raw: f64,         // −log₂(Σ pᵢ²)
    pub renyi_normalised: f64,  // H₂ / log₂(K) ∈ [0,1]
    pub collision_prob: f64,    // Σ pᵢ² — probability two samples share bin
    pub renyient_label: String, // CONCENTRATED / MODERATE / DISPERSED / HIGHLY_DISPERSED / INSUFFICIENT_DATA
    pub note: String,
}

/// RETQUANT — Return Quantile Profile (9-point P1..P99).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RetquantSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub p01_pct: f64,
    pub p05_pct: f64,
    pub p10_pct: f64,
    pub p25_pct: f64,
    pub p50_pct: f64,
    pub p75_pct: f64,
    pub p90_pct: f64,
    pub p95_pct: f64,
    pub p99_pct: f64,
    pub iqr_pct: f64,           // P75 − P25
    pub tail_asymmetry: f64,    // (P99 + P01) / (P99 − P01) — +ve ⇒ right-skew tails
    pub retquant_label: String, // LEFT_TAIL_HEAVY / SYMMETRIC / RIGHT_TAIL_HEAVY / WIDE_IQR / INSUFFICIENT_DATA
    pub note: String,
}

/// MSENT — Multiscale Entropy (Costa, Goldberger, Peng 2005).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MsentSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub embed_dim: usize, // m (default 2)
    pub tolerance: f64,   // r (default 0.2·σ of raw series)
    pub max_scale: usize, // τ_max = 5
    pub sampen_scale1: f64,
    pub sampen_scale2: f64,
    pub sampen_scale3: f64,
    pub sampen_scale4: f64,
    pub sampen_scale5: f64,
    pub msent_complexity_index: f64, // Σ SampEn(τ) — integrated complexity
    pub msent_label: String, // LONG_RANGE_REGULAR / DECAYING / SUSTAINED / INCREASING / INSUFFICIENT_DATA
    pub note: String,
}

/// EWMAVOL — RiskMetrics EWMA Volatility (λ=0.94).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct EwmaVolSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub lambda: f64,                 // decay factor (0.94 standard)
    pub ewma_variance: f64,          // final σ²_t
    pub ewma_sigma_daily: f64,       // √variance (daily)
    pub ewma_sigma_annual: f64,      // ×√252
    pub classical_sigma_annual: f64, // sample σ ×√252 for comparison
    pub ewma_to_classical: f64,      // ewma / classical — >1 ⇒ recent vol elevated
    pub ewmavol_label: String,       // ELEVATED / NORMAL / SUPPRESSED / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 36 ──────────────────────────────────────────────────────

/// KSNORM — Kolmogorov-Smirnov normality test (standardised returns vs N(0,1)).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct KsnormSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ks_statistic: f64,   // max |F_emp − Φ|
    pub critical_10pct: f64, // 1.22/√n
    pub critical_5pct: f64,  // 1.36/√n
    pub critical_1pct: f64,  // 1.63/√n
    pub reject_10pct: bool,
    pub reject_5pct: bool,
    pub reject_1pct: bool,
    pub mean: f64,            // sample mean (standardisation)
    pub sigma: f64,           // sample σ (standardisation)
    pub ksnorm_label: String, // NORMAL / MILD_DEVIATION / MODERATE_DEVIATION / STRONG_NON_NORMAL / INSUFFICIENT_DATA
    pub note: String,
}

/// ADTEST — Anderson-Darling normality test (tail-weighted, more powerful than KS).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AdtestSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ad_statistic: f64,   // A²
    pub ad_adjusted: f64,    // A²·(1 + 0.75/n + 2.25/n²)
    pub p_value_approx: f64, // Stephens approximation
    pub critical_10pct: f64, // 0.631
    pub critical_5pct: f64,  // 0.752
    pub critical_1pct: f64,  // 1.035
    pub reject_10pct: bool,
    pub reject_5pct: bool,
    pub reject_1pct: bool,
    pub adtest_label: String, // NORMAL / MILD_DEVIATION / MODERATE_DEVIATION / STRONG_NON_NORMAL / INSUFFICIENT_DATA
    pub note: String,
}

/// LMOM — Hosking 1990 L-moments (robust alternatives to classical moments).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LmomSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub l1_mean: f64,       // λ₁ = sample mean
    pub l2_scale: f64,      // λ₂ = 0.5·E|X₁−X₂|
    pub l3: f64,            // λ₃ (third L-moment)
    pub l4: f64,            // λ₄ (fourth L-moment)
    pub tau3_skew: f64,     // λ₃/λ₂ ∈ [−1,1]
    pub tau4_kurt: f64,     // λ₄/λ₂ ∈ [−0.25, 1]
    pub lmom_label: String, // HEAVY_LEFT / HEAVY_RIGHT / HEAVY_TAILS / LIGHT_TAILS / NEAR_SYMMETRIC / INSUFFICIENT_DATA
    pub note: String,
}

/// KYLELAM — Kyle's daily price-impact λ (regression |Δp| on volume).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct KylelamSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub kyle_lambda: f64,      // cov(|Δp|, V) / var(V)
    pub mean_abs_dp: f64,      // mean |Δp| ($ price change)
    pub mean_volume: f64,      // mean V (shares)
    pub correlation: f64,      // ρ(|Δp|, V)
    pub r_squared: f64,        // ρ²
    pub kylelam_label: String, // HIGH_IMPACT / MODERATE_IMPACT / LOW_IMPACT / NO_SIGNAL / INSUFFICIENT_DATA
    pub note: String,
}

/// PEAKOVER — Peaks-Over-Threshold (EVT/GPD foundation).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PeakoverSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub threshold_p95: f64,     // P95 of |returns|
    pub threshold_p99: f64,     // P99 of |returns|
    pub count_p95: usize,       // #|r| > P95
    pub count_p99: usize,       // #|r| > P99
    pub mean_excess_p95: f64,   // mean(|r|−P95 | |r|>P95)
    pub mean_excess_p99: f64,   // mean(|r|−P99 | |r|>P99)
    pub max_excess_p95: f64,    // max(|r|−P95)
    pub max_excess_p99: f64,    // max(|r|−P99)
    pub peakover_label: String, // EXTREME_TAIL / HEAVY_TAIL / MODERATE_TAIL / LIGHT_TAIL / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 37 ──────────────────────────────────────────────────────

/// HIGUCHI — Higuchi fractal dimension (Higuchi 1988).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HiguchiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub k_max: usize,          // max sub-sampling interval
    pub fractal_dim: f64,      // FD (slope of log L(k) vs log(1/k))
    pub r_squared: f64,        // linear-fit R²
    pub log_k_count: usize,    // #points used in regression
    pub higuchi_label: String, // SMOOTH / PERSISTENT / RANDOM / ROUGH / INSUFFICIENT_DATA
    pub note: String,
}

/// PICKANDS — Pickands 1975 tail-index estimator.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PickandsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub k_index: usize,         // order-statistic index used
    pub gamma_hat: f64,         // Pickands γ̂ = (1/ln2)·ln((x_k − x_2k)/(x_2k − x_4k))
    pub tail_index: f64,        // 1/γ̂ (Fréchet α, when γ̂ > 0)
    pub x_k: f64,               // k-th largest |r|
    pub x_2k: f64,              // 2k-th largest
    pub x_4k: f64,              // 4k-th largest
    pub pickands_label: String, // FRECHET_HEAVY / FRECHET_MODERATE / GUMBEL_EXPONENTIAL / WEIBULL_BOUNDED / INSUFFICIENT_DATA
    pub note: String,
}

/// KAPPA3 — Kaplan-Knowles 2004 Kappa-3 ratio (third-order LPM).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Kappa3Snapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub mar: f64,             // Minimum Acceptable Return (0 for simplicity)
    pub excess_mean: f64,     // μ − MAR (annualised)
    pub lpm3: f64,            // third lower partial moment E[max(MAR−r,0)³]
    pub lpm3_root: f64,       // LPM3^(1/3)
    pub kappa3: f64,          // (μ−MAR) / LPM3^(1/3)
    pub sortino_compare: f64, // (μ−MAR) / LPM2^(1/2) for reference
    pub kappa3_label: String, // STRONG / POSITIVE / NEUTRAL / NEGATIVE / INSUFFICIENT_DATA
    pub note: String,
}

/// LYAPUNOV — Rosenstein et al. 1993 largest Lyapunov exponent.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LyapunovSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub embed_dim: usize,       // m
    pub time_delay: usize,      // τ (=1 for daily returns)
    pub lambda_max: f64,        // largest Lyapunov exponent (per bar)
    pub r_squared: f64,         // fit quality of ln d(i) vs i
    pub steps_used: usize,      // number of i-steps in the regression
    pub lyapunov_label: String, // CHAOTIC / WEAKLY_CHAOTIC / PERIODIC / STABLE / INSUFFICIENT_DATA
    pub note: String,
}

/// RANKAC — Spearman rank autocorrelation at lags 1, 5, 10.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RankacSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub rho_lag1: f64,        // Spearman ρ at lag 1
    pub rho_lag5: f64,        // Spearman ρ at lag 5
    pub rho_lag10: f64,       // Spearman ρ at lag 10
    pub mean_abs_rho: f64,    // mean |ρ| across the 3 lags
    pub max_abs_rho: f64,     // max |ρ|
    pub rankac_label: String, // STRONG_DEPENDENCE / MODERATE_DEPENDENCE / WEAK_DEPENDENCE / INDEPENDENT / INSUFFICIENT_DATA
    pub note: String,
}

/// BNSJUMP — Barndorff-Nielsen & Shephard 2006 jump-detection Z-statistic.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BnsjumpSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub realized_variance: f64, // RV = Σ r_i²
    pub bipower_variance: f64,  // BV = (π/2) · Σ |r_i|·|r_{i-1}|
    pub jump_ratio: f64,        // (RV − BV) / RV  (zero if pure-diffusion)
    pub jump_z_stat: f64,       // (RV − BV) / sqrt(θ · Σ r_i⁴)  (standardised)
    pub p_value: f64,           // 1 − Φ(|z|) (approx)
    pub bnsjump_label: String, // STRONG_JUMP / MODERATE_JUMP / WEAK_JUMP / NO_JUMP / INSUFFICIENT_DATA
    pub note: String,
}

/// PPROOT — Phillips-Perron 1988 nonparametric unit-root test.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PprootSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub rho_hat: f64,          // OLS estimate of ρ in y_t = ρ·y_{t-1} + ε
    pub t_rho: f64,            // raw t-statistic for ρ = 1
    pub z_rho: f64,            // Phillips-Perron Z(ρ) statistic (newey-west corrected)
    pub z_t: f64,              // Phillips-Perron Z(t) statistic
    pub lag_truncation: usize, // bandwidth q for the long-run variance
    pub pproot_label: String, // STATIONARY_STRONG / STATIONARY_WEAK / BORDERLINE / UNIT_ROOT / INSUFFICIENT_DATA
    pub note: String,
}

/// MFDFA — Multifractal Detrended Fluctuation Analysis at q ∈ {-2, 0, +2}.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MfdfaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub h_q_neg2: f64,       // generalised Hurst exponent at q = −2
    pub h_q_zero: f64,       // generalised Hurst exponent at q = 0 (median-walk)
    pub h_q_pos2: f64,       // generalised Hurst exponent at q = +2
    pub delta_h: f64,        // h(−2) − h(+2), width of the multifractal spectrum
    pub scales_used: usize,  // number of scales included in the fit
    pub mfdfa_label: String, // STRONG_MULTIFRACTAL / MODERATE_MULTIFRACTAL / WEAK_MULTIFRACTAL / MONOFRACTAL / INSUFFICIENT_DATA
    pub note: String,
}

/// HILLKS — Kolmogorov-Smirnov goodness-of-fit for the Hill-tail Pareto model.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HillksSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub k_order: usize,        // order-stat cutoff used by the Hill estimator
    pub alpha_hat: f64,        // Hill tail index α̂
    pub ks_statistic: f64,     // supremum |F_n − F_pareto| over the tail sample
    pub ks_critical_5pct: f64, // 1.36 / sqrt(k) conventional 5% critical value
    pub hillks_label: String,  // GOOD_FIT / ACCEPTABLE_FIT / POOR_FIT / REJECT / INSUFFICIENT_DATA
    pub note: String,
}

/// TSI — Blau 1991 True Strength Index (double-smoothed momentum oscillator).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TsiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ema_long: usize,       // long EMA period (default 25)
    pub ema_short: usize,      // short EMA period (default 13)
    pub tsi_value: f64,        // 100 × EMA_short(EMA_long(ΔP)) / EMA_short(EMA_long(|ΔP|))
    pub signal_value: f64,     // EMA_short(tsi_value)
    pub tsi_minus_signal: f64, // tsi − signal (momentum-of-momentum)
    pub tsi_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// GARCH11 — GARCH(1,1) conditional-volatility parameter fit.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Garch11Snapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub omega: f64,             // baseline variance ω
    pub alpha: f64,             // α (ARCH, shock weight)
    pub beta: f64,              // β (GARCH, persistence weight)
    pub persistence: f64,       // α + β
    pub unconditional_var: f64, // ω / (1 − α − β)
    pub half_life_bars: f64,    // ln(0.5) / ln(α+β)  (undefined if α+β ≥ 1)
    pub log_likelihood: f64,    // fitted log-likelihood
    pub garch11_label: String, // HIGH_PERSISTENCE / MODERATE_PERSISTENCE / LOW_PERSISTENCE / NEAR_INTEGRATED / INSUFFICIENT_DATA
    pub note: String,
}

/// SADF — Phillips-Wu-Yu 2011 Sup-ADF explosive-root / bubble test.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SadfSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub min_window: usize,      // r0 (minimum window size)
    pub adf_full: f64,          // standard ADF on the full sample
    pub sadf_stat: f64,         // sup over expanding windows of ADF-t
    pub sadf_argmax_end: usize, // end index of the argmax window
    pub critical_95: f64,       // approx 95% critical value for SADF at this n
    pub reject_null: bool,      // true if sadf_stat > critical_95
    pub sadf_label: String, // EXPLOSIVE_CONFIRMED / EXPLOSIVE_LIKELY / BORDERLINE / STABLE / INSUFFICIENT_DATA
    pub note: String,
}

/// CORDIM — Grassberger-Procaccia 1983 correlation dimension D2.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CordimSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub embed_dim: usize,     // m (embedding dim, default 3)
    pub radii_count: usize,   // number of ε values used in the fit
    pub d2: f64,              // correlation dimension D2
    pub r_squared: f64,       // fit quality of log C(ε) vs log ε
    pub cordim_label: String, // LOW_DIM / MODERATE_DIM / HIGH_DIM / STOCHASTIC / INSUFFICIENT_DATA
    pub note: String,
}

/// SKSPEC — Rolling-window skewness spectrum / stability.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SkspecSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub window_size: usize, // rolling window length (default 30)
    pub mean_skew: f64,     // mean of rolling skew values
    pub std_skew: f64,      // std of rolling skew values (skew stability)
    pub min_skew: f64,
    pub max_skew: f64,
    pub range_skew: f64,      // max − min
    pub skspec_label: String, // STABLE_POSITIVE / STABLE_NEGATIVE / DRIFTING / UNSTABLE / INSUFFICIENT_DATA
    pub note: String,
}

/// AUTOMI — Lag-1 auto-mutual-information (information-theoretic ACF).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AutomiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub num_bins: usize,      // k bins per marginal (default 8)
    pub mi_lag1: f64,         // I(X_t; X_{t-1})  (bits)
    pub mi_lag5: f64,         // I(X_t; X_{t-5})
    pub mi_lag10: f64,        // I(X_t; X_{t-10})
    pub h_marginal: f64,      // H(X) marginal entropy (bits) — baseline
    pub normalized_mi1: f64,  // MI(1) / H(X)  (0..1 fraction of marginal info shared)
    pub automi_label: String, // STRONG / MODERATE / WEAK / INDEPENDENT / INSUFFICIENT_DATA
    pub note: String,
}

/// DURBINWATSON — Durbin-Watson d statistic for AR(1) autocorrelation on returns.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DurbinWatsonSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub dw_stat: f64,      // d ∈ [0, 4], ~2 ⇒ no autocorr
    pub rho_estimate: f64, // implied ρ ≈ 1 − d/2
    pub dw_label: String, // STRONG_POS / WEAK_POS / NO_AUTOCORR / WEAK_NEG / STRONG_NEG / INSUFFICIENT_DATA
    pub note: String,
}

/// BDSTEST — Brock-Dechert-Scheinkman test for iid at embedding dim m=2.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BdsTestSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub embed_dim: usize,       // m (default 2)
    pub epsilon_mult: f64,      // ε = epsilon_mult × σ (default 0.7)
    pub bds_stat: f64,          // standardized test statistic (asymptotically N(0,1))
    pub p_value_two_sided: f64, // 2 × Φ(−|z|)
    pub reject_null: bool,      // p < 0.05
    pub bds_label: String, // IID_CONFIRMED / WEAK_DEPENDENCE / STRONG_DEPENDENCE / INSUFFICIENT_DATA
    pub note: String,
}

/// BREUSCHPAGAN — Breusch-Pagan LM test for heteroskedasticity.
/// Aux regression: squared residual on a simple trend regressor (bar index).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BreuschPaganSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub lm_stat: f64,      // n × R² from aux regression
    pub r_squared: f64,    // R² of aux regression
    pub df: usize,         // degrees of freedom (number of regressors, default 1)
    pub critical_95: f64,  // χ²(df) 95% critical value
    pub reject_null: bool, // true if lm_stat > critical_95
    pub bp_label: String,  // HOMOSKEDASTIC / MILD_HETERO / STRONG_HETERO / INSUFFICIENT_DATA
    pub note: String,
}

/// TURNPTS — Bartels turning-points randomness test on returns.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TurnPtsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub observed_turnpts: usize, // count of local minima + maxima
    pub expected_turnpts: f64,   // 2(n − 2) / 3 under iid
    pub variance_turnpts: f64,   // (16n − 29) / 90
    pub z_stat: f64,             // (obs − exp) / sqrt(var)
    pub p_value_two_sided: f64,
    pub reject_null: bool,
    pub turnpts_label: String, // RANDOM_IID / OVER_TURNING / UNDER_TURNING / INSUFFICIENT_DATA
    pub note: String,
}

/// PERIODOGRAM — discrete Fourier periodogram peak / spectral dominance.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PeriodogramSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub n_freqs: usize,            // number of positive Fourier frequencies tested
    pub dominant_freq: f64,        // cycle frequency (cycles per bar, 0..0.5)
    pub dominant_period_bars: f64, // 1 / dominant_freq
    pub dominant_power: f64,       // spectral power at the peak
    pub total_power: f64,
    pub dominant_power_ratio: f64, // dominant_power / total_power
    pub periodogram_label: String, // STRONG_CYCLE / MODERATE_CYCLE / WEAK_CYCLE / NO_CYCLE / INSUFFICIENT_DATA
    pub note: String,
}
