// Quant-stat research surface types

/// MODSHARPE — Pezier-White Adjusted Sharpe Ratio.
/// Classical Sharpe SR = √252 · E[r]/σ[r] assumes normal returns. The
/// Pezier-White (2006) adjustment corrects for higher moments:
///     ASR = SR · [1 + (S/6)·SR − ((K−3)/24)·SR²]
/// where S is skewness and K is kurtosis of bar-level returns. For
/// negatively-skewed fat-tailed distributions the adjustment reduces
/// the headline Sharpe; for positively-skewed returns it can boost it.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ModSharpeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub annualization_factor: f64, // 252 for daily bars
    pub mean_return_bar: f64,      // arithmetic mean of bar returns
    pub stdev_return_bar: f64,
    pub skewness: f64,           // 3rd standardised central moment
    pub excess_kurtosis: f64,    // K − 3
    pub sharpe_ratio: f64,       // annualised classical Sharpe
    pub adjusted_sharpe: f64,    // annualised Pezier-White ASR
    pub adjustment_factor: f64,  // ASR / SR
    pub modsharpe_label: String, // STRONG_POS / MODERATE_POS / WEAK / MODERATE_NEG / STRONG_NEG / INSUFFICIENT_DATA
    pub note: String,
}

/// HSIEHTEST — Hsieh (1989) third-moment nonlinearity test.
/// Fits AR(1) residuals ε_t = r_t − μ − φ·r_{t-1}, then probes the
/// standardised third cross-moment T(i,j) = E[ε_{t−i} ε_{t−j} ε_t]/σ³.
/// Under linearity, T(i,j) = 0 for all (i,j). We test lags (1,1) and
/// (2,2); |z| > 1.96 indicates statistically detectable nonlinearity.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HsiehTestSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ar_order: usize, // 1
    pub t_11: f64,       // standardised third moment at lag (1,1)
    pub t_22: f64,       // at (2,2)
    pub z_11: f64,       // asymptotic z-stat of T(1,1) · √n / √6
    pub z_22: f64,
    pub max_abs_z: f64,      // max(|z_11|, |z_22|)
    pub critical_95: f64,    // 1.96
    pub reject_null: bool,   // max_abs_z > 1.96
    pub hsieh_label: String, // LINEAR / MILD_NONLIN / STRONG_NONLIN / INSUFFICIENT_DATA
    pub note: String,
}

/// CHOWBREAK — Chow (1960) mean-shift structural break F-test.
/// Splits the return series at n/2 and compares the pooled-mean
/// RSS to the sum of within-group RSS. F = [(RSS_p − RSS_u)/k] /
/// [RSS_u/(n−2k)] with k=1 regressor (constant). Large F ⇒ reject
/// "no break at n/2". Useful as a quick structural-change screen.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ChowBreakSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub break_point_idx: usize, // n/2
    pub rss_pooled: f64,
    pub rss_unrestricted: f64, // RSS_1 + RSS_2 with separate means
    pub mean_pre: f64,
    pub mean_post: f64,
    pub k_regressors: usize, // 1 (constant-only model)
    pub f_stat: f64,
    pub df_num: usize,
    pub df_den: usize,
    pub critical_95: f64, // ≈ 3.84 for k=1, large n
    pub reject_null: bool,
    pub chowbreak_label: String, // NO_BREAK / MILD_BREAK / STRONG_BREAK / INSUFFICIENT_DATA
    pub note: String,
}

/// DRIFTBURST — Christensen-Oomen-Renò (2018) drift-burst hypothesis test.
/// Scans the return series with a Gaussian kernel to compute a
/// rolling drift-to-volatility ratio T(t) = √h · μ̂(t)/σ̂(t). Large
/// |T(t)| is a local "drift burst" — a period where the trend
/// dominates the volatility scale. Reports the maximum over the
/// window and the number of excursions above |T|>3 (approx 99%
/// pointwise critical value).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DriftBurstSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub kernel_bandwidth_bars: f64, // σ of Gaussian kernel (half-width)
    pub max_abs_statistic: f64,     // max_t |T(t)|
    pub max_stat_signed: f64,       // signed T at argmax |T|
    pub max_at_offset: usize,       // bars before series end (0 = latest)
    pub excursions_gt_3: usize,     // count of t with |T(t)| > 3
    pub critical_99_approx: f64,    // 3.0 (pointwise)
    pub driftburst_label: String,   // NO_BURST / MILD_BURST / STRONG_BURST / INSUFFICIENT_DATA
    pub note: String,
}

/// HLVCLUST — Parkinson high-low volatility clustering (Ljung-Box on
/// log-range series). The Parkinson range estimator is
/// σ̂²_P(t) = (1/(4 ln 2)) · ln(H_t/L_t)². We form v_t = ln(σ̂_P(t))
/// (or equivalently 0.5·ln(ln(H/L)²) up to a constant) and apply
/// Ljung-Box to lag h=10. Rejecting white noise on v_t confirms
/// volatility clustering even without return-based GARCH machinery.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HlvClustSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub lag_h: usize,                  // 10
    pub parkinson_vol_bar: f64,        // mean σ̂_P per bar
    pub parkinson_vol_annualised: f64, // × √252
    pub ac_lag1: f64,                  // lag-1 autocorrelation of log-range series
    pub ac_lag5: f64,                  // lag-5
    pub lb_q_stat: f64,                // Ljung-Box Q at lag h
    pub critical_95: f64,              // χ²(h) 95%
    pub p_value: f64,
    pub reject_null: bool,
    pub hlvclust_label: String, // NO_CLUST / MILD_CLUST / STRONG_CLUST / INSUFFICIENT_DATA
    pub note: String,
}

/// YANGZHANG — Yang-Zhang (2000) three-component range volatility estimator.
/// σ²_YZ = σ²_O + k·σ²_C + (1-k)·σ²_RS, where σ²_O is overnight open-vs-prev-close
/// variance, σ²_C is close-to-close variance, σ²_RS is the Rogers-Satchell intraday
/// component, and k = 0.34 / (1.34 + (n+1)/(n-1)) minimises variance under a
/// drift-free Brownian assumption. Asymptotically the most efficient of the
/// range-based estimators that use OHLC data.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct YangZhangVolSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub overnight_var: f64,         // σ²_O
    pub open_to_close_var: f64,     // σ²_C (open-to-close close variant)
    pub rs_component: f64,          // σ²_RS
    pub k_weight: f64,              // 0.34 / (1.34 + (n+1)/(n-1))
    pub yz_vol_bar: f64,            // √σ²_YZ per bar
    pub yz_vol_annualised_pct: f64, // yz_vol_bar × √252 × 100
    pub cc_vol_annualised_pct: f64, // close-to-close comparison σ × √252 × 100
    pub efficiency_vs_close: f64,   // cc_vol / yz_vol (higher = YZ more efficient)
    pub yangzhang_label: String, // VERY_LOW / LOW / MODERATE / HIGH / VERY_HIGH / INSUFFICIENT_DATA
    pub note: String,
}

/// KUIPER — Kuiper (1960) two-sided empirical CDF goodness-of-fit statistic
/// against standard normal. V = D⁺ + D⁻ where D⁺ = max(F_n(x) − F(x)) and
/// D⁻ = max(F(x) − F_n(x)). More sensitive to tail departures than
/// Kolmogorov-Smirnov. Uses Stephens (1970) finite-n modification
/// V* = V · (√n + 0.155 + 0.24/√n); reject normality at 95% if V* > 1.747.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct KuiperSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub mean: f64,            // sample μ of returns (for standardisation)
    pub stdev: f64,           // sample σ
    pub d_plus: f64,          // max(F_n − F)
    pub d_minus: f64,         // max(F − F_n)
    pub v_stat: f64,          // D⁺ + D⁻
    pub v_stat_adj: f64,      // Stephens-modified V*
    pub critical_95: f64,     // 1.747 for standard normal
    pub p_value_approx: f64,  // Stephens 1970 approximation
    pub reject_null: bool,    // V* > 1.747
    pub kuiper_label: String, // NORMAL / MILD_DEPART / STRONG_DEPART / INSUFFICIENT_DATA
    pub note: String,
}

/// DAGOSTINO — D'Agostino-Pearson (1973) K² omnibus normality test.
/// Transforms sample skewness via D'Agostino (1970) to z_skew and
/// sample kurtosis via Anscombe-Glynn (1983) to z_kurt; combined
/// K² = z_skew² + z_kurt² is asymptotically χ²(2) under H0: normal.
/// Complements Jarque-Bera by exposing whether skew or kurt dominates
/// the departure. Reject at 95% if K² > 5.991.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DagostinoSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub skewness: f64,        // sample b1
    pub excess_kurtosis: f64, // sample b2 − 3
    pub z_skew: f64,          // D'Agostino transformed skew z-stat
    pub z_kurt: f64,          // Anscombe-Glynn transformed kurt z-stat
    pub k2_stat: f64,         // z_skew² + z_kurt²
    pub critical_95: f64,     // 5.991
    pub p_value: f64,
    pub reject_null: bool,
    pub dagostino_label: String, // NORMAL / SKEW_DOMINANT / KURT_DOMINANT / BOTH_DEPART / INSUFFICIENT_DATA
    pub note: String,
}

/// BAIPERRON — Bai-Perron (1998) sup-F structural-break test with
/// interior search over [0.15n, 0.85n] (Andrews 1993 trimming).
/// Extends CHOWBREAK by searching *where* the break is rather than
/// assuming n/2. Reports sup-F over the trimmed interior and the
/// argmax break index. Rejects H0 "no break" at 95% if sup-F exceeds
/// the Andrews (1993) critical value (≈8.58 for 15% trim, k=1).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BaiPerronSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub trim_fraction: f64,    // 0.15
    pub search_lo: usize,      // ⌈0.15n⌉
    pub search_hi: usize,      // ⌊0.85n⌋
    pub best_break_idx: usize, // argmax over search range
    pub sup_f_stat: f64,
    pub mean_pre: f64,
    pub mean_post: f64,
    pub rss_no_break: f64,
    pub rss_at_best: f64,
    pub critical_95: f64,    // Andrews 1993: ≈8.58 for 15% trim, k=1
    pub p_value_approx: f64, // Hansen (1997) F-asymptotic approx
    pub reject_null: bool,
    pub baiperron_label: String, // NO_BREAK / MILD_BREAK / STRONG_BREAK / INSUFFICIENT_DATA
    pub note: String,
}

/// KUPIECPOF — Kupiec (1995) Proportion-of-Failures VaR backtest.
/// Builds a rolling historical-VaR_{α=0.95} from the first `rolling_window`
/// bars and counts exceedances in the remaining test window. Likelihood
/// ratio: LR_POF = −2·ln[((1−α)^{T_ok}·α^{T_fail}) / ((1−p̂)^{T_ok}·p̂^{T_fail})]
/// where p̂ is the realised exceedance rate. LR_POF is asymptotically χ²(1)
/// under H0: realised exceedance rate equals nominal α.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct KupiecPofSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub confidence_level: f64,        // 0.95
    pub nominal_exceedance_rate: f64, // 1 − confidence_level = 0.05
    pub rolling_window: usize,        // 60
    pub test_window: usize,           // bars_used − rolling_window
    pub var_latest_bar: f64, // latest VaR estimate (positive number; -VaR is the threshold)
    pub n_exceedances: usize,
    pub expected_exceedances: f64,
    pub realised_exceedance_rate: f64,
    pub lr_pof_stat: f64,
    pub critical_95: f64, // 3.841 = χ²_95(1)
    pub p_value: f64,
    pub reject_null: bool,
    pub kupiec_label: String, // GOOD_FIT / OVER_ESTIMATED / UNDER_ESTIMATED / INSUFFICIENT_DATA
    pub note: String,
}
