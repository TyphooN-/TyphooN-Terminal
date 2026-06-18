//! Return-distribution and risk-statistical research snapshot computations.

use super::*;

mod autocorr_regime;
mod distribution_shape;
mod downside_efficiency;
mod drawdown_liquidity_normality;
mod drawup_gap_range;
mod seasonality_spread;
mod volatility_estimators;
mod performance_runs_tests;
pub use autocorr_regime::*;
use autocorr_regime::acf_at_lag;
pub use distribution_shape::*;
pub use downside_efficiency::*;
pub use drawdown_liquidity_normality::*;
pub use drawup_gap_range::*;
pub use seasonality_spread::*;
pub use volatility_estimators::*;
pub use performance_runs_tests::*;

// Shared helpers for return-distribution and risk-statistical compute modules.

/// Shared helper: collect trailing 253 bars sorted oldest-first and
/// compute log returns. Returns (sorted_bars, log_returns).
pub(crate) fn trailing_log_returns(
    bars: &[HistoricalPriceRow],
) -> (Vec<&HistoricalPriceRow>, Vec<f64>) {
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let window: Vec<&HistoricalPriceRow> = sorted.iter().rev().take(253).rev().copied().collect();
    let mut log_rets: Vec<f64> = Vec::with_capacity(window.len());
    for w in window.windows(2) {
        let prev = w[0].close;
        let curr = w[1].close;
        if prev > 0.0 && curr > 0.0 {
            log_rets.push((curr / prev).ln());
        }
    }
    (window, log_rets)
}

// PSR, ADF, Mann-Kendall, bipower, and drawdown-duration computes

pub fn compute_psr_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> ProbabilisticSharpeSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return ProbabilisticSharpeSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            psr_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let nf = n as f64;
    let mean = log_rets.iter().sum::<f64>() / nf;
    let centered: Vec<f64> = log_rets.iter().map(|r| r - mean).collect();
    let var = centered.iter().map(|d| d * d).sum::<f64>() / nf;
    if var < f64::EPSILON {
        return ProbabilisticSharpeSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            psr_label: "INSUFFICIENT_DATA".into(),
            note: "zero variance".into(),
            ..Default::default()
        };
    }
    let std = var.sqrt();
    // Annualized Sharpe (zero risk-free assumption, 252 days/yr).
    let sharpe = (mean / std) * (252.0_f64).sqrt();
    let m3 = centered.iter().map(|d| d.powi(3)).sum::<f64>() / nf;
    let m4 = centered.iter().map(|d| d.powi(4)).sum::<f64>() / nf;
    let skew = m3 / (var.powi(3).sqrt());
    let kurt = m4 / (var * var); // NOT excess — PSR uses γ₄ directly
    let sr_star = 0.0_f64;
    // Sharpe used in PSR formula must be in same units as skew/kurtosis of the
    // per-period returns. Convert annualized back to per-period SR for the
    // inside of the formula.
    let sr_per = mean / std;
    let denom_sq = 1.0 - skew * sr_per + (kurt - 1.0) / 4.0 * sr_per * sr_per;
    let psr = if denom_sq > 0.0 && n > 1 {
        let z = (sr_per - sr_star / (252.0_f64).sqrt()) * ((nf - 1.0).sqrt()) / denom_sq.sqrt();
        std_normal_cdf(z)
    } else {
        0.0
    };
    let label = if psr < 0.50 {
        "VERY_LOW"
    } else if psr < 0.75 {
        "LOW"
    } else if psr < 0.90 {
        "MODERATE"
    } else if psr < 0.95 {
        "HIGH"
    } else {
        "VERY_HIGH"
    };
    ProbabilisticSharpeSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        sharpe,
        skewness: skew,
        kurtosis: kurt,
        sr_benchmark: sr_star,
        psr,
        psr_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_adf_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DickeyFullerSnapshot {
    let sym = symbol.to_uppercase();
    let window: Vec<&HistoricalPriceRow> = bars
        .iter()
        .rev()
        .take(253)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if window.len() < 30 {
        return DickeyFullerSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            crit_1pct: -3.43,
            crit_5pct: -2.86,
            crit_10pct: -2.57,
            adf_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 bars, got {}", window.len()),
            ..Default::default()
        };
    }
    // Use log(close) to avoid scale/trend dependency issues.
    let logp: Vec<f64> = window
        .iter()
        .filter_map(|b| {
            if b.close > 0.0 {
                Some(b.close.ln())
            } else {
                None
            }
        })
        .collect();
    if logp.len() < 30 {
        return DickeyFullerSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            crit_1pct: -3.43,
            crit_5pct: -2.86,
            crit_10pct: -2.57,
            adf_label: "INSUFFICIENT_DATA".into(),
            note: "not enough positive closes".into(),
            ..Default::default()
        };
    }
    // Regression: Δp_t = α + β · p_{t-1} + ε
    let n = logp.len() - 1;
    let nf = n as f64;
    let x: Vec<f64> = logp[..logp.len() - 1].to_vec();
    let dy: Vec<f64> = (1..logp.len()).map(|i| logp[i] - logp[i - 1]).collect();
    let x_mean = x.iter().sum::<f64>() / nf;
    let y_mean = dy.iter().sum::<f64>() / nf;
    let sxx: f64 = x.iter().map(|v| (v - x_mean).powi(2)).sum();
    let sxy: f64 = x
        .iter()
        .zip(dy.iter())
        .map(|(xi, yi)| (xi - x_mean) * (yi - y_mean))
        .sum();
    if sxx < f64::EPSILON {
        return DickeyFullerSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            crit_1pct: -3.43,
            crit_5pct: -2.86,
            crit_10pct: -2.57,
            adf_label: "INSUFFICIENT_DATA".into(),
            note: "zero predictor variance".into(),
            ..Default::default()
        };
    }
    let beta = sxy / sxx;
    let alpha = y_mean - beta * x_mean;
    let residuals: Vec<f64> = x
        .iter()
        .zip(dy.iter())
        .map(|(xi, yi)| yi - alpha - beta * xi)
        .collect();
    let k = 2.0; // parameters: intercept + slope
    let rss: f64 = residuals.iter().map(|r| r * r).sum();
    let sigma2 = rss / (nf - k);
    let se_beta = (sigma2 / sxx).sqrt();
    let t_stat = if se_beta < f64::EPSILON {
        0.0
    } else {
        beta / se_beta
    };
    let crit_5 = -2.86_f64;
    let crit_1 = -3.43_f64;
    let crit_10 = -2.57_f64;
    let reject = t_stat < crit_5;
    let label = if t_stat < crit_1 {
        "STATIONARY"
    } else if t_stat < crit_5 {
        "STATIONARY"
    } else if t_stat < crit_10 {
        "BORDERLINE"
    } else {
        "NON_STATIONARY"
    };
    DickeyFullerSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: logp.len(),
        beta,
        se_beta,
        t_statistic: t_stat,
        crit_1pct: crit_1,
        crit_5pct: crit_5,
        crit_10pct: crit_10,
        reject_unit_root: reject,
        adf_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_mnkendall_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MannKendallSnapshot {
    let sym = symbol.to_uppercase();
    let window: Vec<&HistoricalPriceRow> = bars
        .iter()
        .rev()
        .take(253)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if window.len() < 30 {
        return MannKendallSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            mk_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 bars, got {}", window.len()),
            ..Default::default()
        };
    }
    let x: Vec<f64> = window
        .iter()
        .filter_map(|b| {
            if b.close > 0.0 {
                Some(b.close.ln())
            } else {
                None
            }
        })
        .collect();
    let n = x.len();
    if n < 30 {
        return MannKendallSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            mk_label: "INSUFFICIENT_DATA".into(),
            note: "not enough positive closes".into(),
            ..Default::default()
        };
    }
    let mut s: i64 = 0;
    for i in 0..n - 1 {
        for j in (i + 1)..n {
            let d = x[j] - x[i];
            if d > 0.0 {
                s += 1;
            } else if d < 0.0 {
                s -= 1;
            }
        }
    }
    let nf = n as f64;
    let var = nf * (nf - 1.0) * (2.0 * nf + 5.0) / 18.0;
    let z = if s > 0 {
        (s as f64 - 1.0) / var.sqrt()
    } else if s < 0 {
        (s as f64 + 1.0) / var.sqrt()
    } else {
        0.0
    };
    let p = 2.0 * (1.0 - std_normal_cdf(z.abs()));
    let reject = p < 0.05;
    let pairs = nf * (nf - 1.0) / 2.0;
    let tau = if pairs > 0.0 { s as f64 / pairs } else { 0.0 };
    let label = if !reject {
        "NO_TREND"
    } else if z > 0.0 {
        if p < 0.001 { "STRONG_UP" } else { "UP" }
    } else {
        if p < 0.001 { "STRONG_DOWN" } else { "DOWN" }
    };
    MannKendallSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        s_statistic: s,
        variance: var,
        z_statistic: z,
        p_value: p,
        tau,
        reject_no_trend: reject,
        mk_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_bipower_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> BipowerVariationSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return BipowerVariationSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            jump_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let rv: f64 = log_rets.iter().map(|r| r * r).sum();
    let mut bpv: f64 = 0.0;
    for i in 1..n {
        bpv += log_rets[i].abs() * log_rets[i - 1].abs();
    }
    bpv *= std::f64::consts::FRAC_PI_2;
    let cont_var_ann = bpv * 252.0 / n as f64;
    let rv_ann = rv * 252.0 / n as f64;
    let cont_vol_ann_pct = cont_var_ann.max(0.0).sqrt() * 100.0;
    let rv_vol_ann_pct = rv_ann.max(0.0).sqrt() * 100.0;
    let jump_ratio = if rv < f64::EPSILON {
        0.0
    } else {
        (1.0 - bpv / rv).max(0.0).min(1.0)
    };
    let jump_pct = jump_ratio * 100.0;
    let label = if jump_ratio < 0.05 {
        "NO_JUMPS"
    } else if jump_ratio < 0.20 {
        "MILD_JUMPS"
    } else if jump_ratio < 0.40 {
        "NOTABLE_JUMPS"
    } else {
        "HEAVY_JUMPS"
    };
    BipowerVariationSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        realized_var: rv,
        bipower_var: bpv,
        continuous_vol_ann_pct: cont_vol_ann_pct,
        realized_vol_ann_pct: rv_vol_ann_pct,
        jump_ratio,
        jump_pct,
        jump_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_dddur_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DrawdownDurationSnapshot {
    let sym = symbol.to_uppercase();
    let window: Vec<&HistoricalPriceRow> = bars
        .iter()
        .rev()
        .take(253)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if window.len() < 30 {
        return DrawdownDurationSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            dddur_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 bars, got {}", window.len()),
            ..Default::default()
        };
    }
    let n = window.len();
    let mut durations: Vec<usize> = Vec::new();
    let mut peak: f64 = window[0].close;
    let mut in_dd = false;
    let mut dd_start: usize = 0;
    let mut total_underwater: usize = 0;
    for (i, b) in window.iter().enumerate() {
        let c = b.close;
        if c > peak {
            if in_dd {
                // recovery
                durations.push(i - dd_start);
                in_dd = false;
            }
            peak = c;
        } else if c < peak {
            if !in_dd {
                in_dd = true;
                dd_start = i;
            }
            total_underwater += 1;
        } else if in_dd {
            total_underwater += 1;
        }
    }
    let currently = in_dd;
    let current_dur = if in_dd { n - dd_start } else { 0 };
    let dd_event_count = durations.len();
    let max_dur = durations.iter().copied().max().unwrap_or(0);
    let mean_dur = if dd_event_count == 0 {
        0.0
    } else {
        durations.iter().copied().sum::<usize>() as f64 / dd_event_count as f64
    };
    let median_dur = if durations.is_empty() {
        0.0
    } else {
        let mut sorted = durations.clone();
        sorted.sort_unstable();
        let mid = sorted.len() / 2;
        if sorted.len() % 2 == 0 {
            (sorted[mid - 1] + sorted[mid]) as f64 / 2.0
        } else {
            sorted[mid] as f64
        }
    };
    let pct_under = total_underwater as f64 / n as f64 * 100.0;
    let label = if pct_under < 20.0 {
        "MOSTLY_DRY"
    } else if pct_under < 40.0 {
        "FREQUENT_DD"
    } else if pct_under < 60.0 {
        "PERSISTENT_DD"
    } else {
        "DEEP_WATER"
    };
    DrawdownDurationSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        dd_event_count,
        max_dd_duration_bars: max_dur,
        mean_dd_duration_bars: mean_dur,
        median_dd_duration_bars: median_dur,
        total_bars_underwater: total_underwater,
        pct_time_underwater: pct_under,
        currently_underwater: currently,
        current_dd_duration_bars: current_dur,
        dddur_label: label.into(),
        note: String::new(),
    }
}

// Hill-tail, ARCH-LM, pain-ratio, CUSUM, and Cornish-Fisher VaR computes

pub fn compute_hilltail_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> HillTailSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 50 {
        return HillTailSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            tail_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥50 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    fn hill_of(xs: &[f64]) -> (f64, usize, f64) {
        // xs already positive magnitudes
        let mut v: Vec<f64> = xs.iter().copied().filter(|x| *x > 0.0).collect();
        if v.len() < 20 {
            return (0.0, 0, 0.0);
        }
        v.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        let k = ((v.len() as f64 * 0.10).floor() as usize)
            .max(10)
            .min(v.len() - 1);
        let threshold = v[k];
        if threshold <= 0.0 {
            return (0.0, k, 0.0);
        }
        let sum_log: f64 = v[..k].iter().map(|x| (x / threshold).ln()).sum();
        let alpha = if sum_log > f64::EPSILON {
            k as f64 / sum_log
        } else {
            0.0
        };
        (alpha, k, threshold)
    }
    let abs_mags: Vec<f64> = log_rets.iter().map(|r| r.abs()).collect();
    let left_mags: Vec<f64> = log_rets.iter().filter(|r| **r < 0.0).map(|r| -r).collect();
    let right_mags: Vec<f64> = log_rets.iter().filter(|r| **r > 0.0).copied().collect();
    let (alpha_abs, k_abs, thresh_abs) = hill_of(&abs_mags);
    let (alpha_left, _, _) = hill_of(&left_mags);
    let (alpha_right, _, _) = hill_of(&right_mags);
    let label = if alpha_abs <= 0.0 {
        "INSUFFICIENT_DATA"
    } else if alpha_abs > 4.0 {
        "GAUSSIAN_LIKE"
    } else if alpha_abs > 3.0 {
        "LIGHT_TAIL"
    } else if alpha_abs > 2.0 {
        "MODERATE_TAIL"
    } else if alpha_abs > 1.0 {
        "HEAVY_TAIL"
    } else {
        "VERY_HEAVY_TAIL"
    };
    HillTailSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: log_rets.len(),
        k_order_stats: k_abs,
        threshold_abs: thresh_abs,
        hill_alpha_abs: alpha_abs,
        hill_alpha_left: alpha_left,
        hill_alpha_right: alpha_right,
        tail_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_archlm_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> ArchLmSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    let q: usize = 5;
    if log_rets.len() < q + 30 {
        return ArchLmSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            q_lags: q,
            crit_5pct_chi2: 11.0705,
            crit_1pct_chi2: 15.0863,
            arch_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} returns, got {}", q + 30, log_rets.len()),
            ..Default::default()
        };
    }
    let n_r = log_rets.len();
    let mean = log_rets.iter().sum::<f64>() / n_r as f64;
    let eps2: Vec<f64> = log_rets.iter().map(|r| (r - mean).powi(2)).collect();
    // Build design matrix: rows t from q..n_r of [1, eps2[t-1], ..., eps2[t-q]] regressing eps2[t]
    let n = n_r - q;
    let nf = n as f64;
    let y: Vec<f64> = (q..n_r).map(|t| eps2[t]).collect();
    // Build sums for normal equations: X'X is (q+1)x(q+1), X'Y is (q+1)x1.
    let p = q + 1;
    let mut xtx = vec![0.0_f64; p * p];
    let mut xty = vec![0.0_f64; p];
    let y_mean = y.iter().sum::<f64>() / nf;
    let tss: f64 = y.iter().map(|yi| (yi - y_mean).powi(2)).sum();
    for t in q..n_r {
        // row = [1, eps2[t-1], eps2[t-2], ..., eps2[t-q]]
        let mut row = vec![1.0_f64; p];
        for lag in 1..=q {
            row[lag] = eps2[t - lag];
        }
        for i in 0..p {
            for j in 0..p {
                xtx[i * p + j] += row[i] * row[j];
            }
            xty[i] += row[i] * y[t - q];
        }
    }
    // Solve via simple Gaussian elimination on (p x p) matrix. p=6 is tiny.
    let mut a = xtx.clone();
    let mut b = xty.clone();
    let mut ok = true;
    for col in 0..p {
        let mut pivot = col;
        for r in col + 1..p {
            if a[r * p + col].abs() > a[pivot * p + col].abs() {
                pivot = r;
            }
        }
        if a[pivot * p + col].abs() < 1e-12 {
            ok = false;
            break;
        }
        if pivot != col {
            for k in 0..p {
                a.swap(col * p + k, pivot * p + k);
            }
            b.swap(col, pivot);
        }
        let inv = 1.0 / a[col * p + col];
        for r in col + 1..p {
            let factor = a[r * p + col] * inv;
            for k in col..p {
                a[r * p + k] -= factor * a[col * p + k];
            }
            b[r] -= factor * b[col];
        }
    }
    let mut coef = vec![0.0_f64; p];
    if ok {
        for i in (0..p).rev() {
            let mut sum = b[i];
            for j in i + 1..p {
                sum -= a[i * p + j] * coef[j];
            }
            coef[i] = sum / a[i * p + i];
        }
    }
    let rss: f64 = (q..n_r)
        .map(|t| {
            let mut yhat = coef[0];
            for lag in 1..=q {
                yhat += coef[lag] * eps2[t - lag];
            }
            (y[t - q] - yhat).powi(2)
        })
        .sum();
    // Near-constant ε² (e.g. deterministic oscillating returns) makes X'X singular; that's
    // equivalent to "no conditional heteroskedasticity" — treat as NO_ARCH with LM=0.
    let r2 = if tss > f64::EPSILON && ok {
        (1.0 - rss / tss).max(0.0).min(1.0)
    } else {
        0.0
    };
    let lm = nf * r2;
    // Wilson-Hilferty chi-squared to normal: z = ((LM/q)^(1/3) - (1 - 2/(9q))) / √(2/(9q))
    let qf = q as f64;
    let z = ((lm / qf).powf(1.0 / 3.0) - (1.0 - 2.0 / (9.0 * qf))) / (2.0 / (9.0 * qf)).sqrt();
    let p_val = (1.0 - std_normal_cdf(z)).max(0.0).min(1.0);
    let crit5 = 11.0705_f64;
    let crit1 = 15.0863_f64;
    let reject = lm > crit5;
    let label = if lm < crit5 {
        "NO_ARCH"
    } else if lm < crit1 {
        "WEAK_ARCH"
    } else {
        "STRONG_ARCH"
    };
    ArchLmSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n_r,
        q_lags: q,
        r_squared: r2,
        lm_statistic: lm,
        p_value: p_val,
        crit_5pct_chi2: crit5,
        crit_1pct_chi2: crit1,
        reject_homoskedastic: reject,
        arch_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_painratio_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> PainRatioSnapshot {
    let sym = symbol.to_uppercase();
    let window: Vec<&HistoricalPriceRow> = bars
        .iter()
        .rev()
        .take(253)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if window.len() < 30 {
        return PainRatioSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            pain_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 bars, got {}", window.len()),
            ..Default::default()
        };
    }
    let n = window.len();
    let mut peak: f64 = window[0].close;
    let mut sum_abs_dd: f64 = 0.0;
    let mut max_abs_dd: f64 = 0.0;
    for b in window.iter() {
        if b.close > peak {
            peak = b.close;
        }
        let dd = if peak > 0.0 {
            (b.close - peak) / peak * 100.0
        } else {
            0.0
        };
        let abs_dd = (-dd).max(0.0); // dd ≤ 0 by construction; take magnitude
        sum_abs_dd += abs_dd;
        if abs_dd > max_abs_dd {
            max_abs_dd = abs_dd;
        }
    }
    let pain_index = sum_abs_dd / n as f64;
    // Annualized return: total log return × (252/n)
    let first = window.first().map(|b| b.close).unwrap_or(0.0);
    let last = window.last().map(|b| b.close).unwrap_or(0.0);
    let ann_ret_pct = if first > 0.0 && last > 0.0 {
        ((last / first).ln() * 252.0 / n as f64) * 100.0
    } else {
        0.0
    };
    let pain_ratio = if pain_index > f64::EPSILON {
        ann_ret_pct / pain_index
    } else {
        0.0
    };
    let label = if pain_index < 1.0 {
        "LOW_PAIN"
    } else if pain_index < 3.0 {
        "MILD_PAIN"
    } else if pain_index < 7.0 {
        "MODERATE_PAIN"
    } else if pain_index < 15.0 {
        "HIGH_PAIN"
    } else {
        "SEVERE_PAIN"
    };
    PainRatioSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pain_index_pct: pain_index,
        annualized_return_pct: ann_ret_pct,
        pain_ratio,
        max_dd_pct: max_abs_dd,
        pain_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cusum_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CusumBreakSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    let n = log_rets.len();
    if n < 30 {
        return CusumBreakSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            crit_10pct: 1.22,
            crit_5pct: 1.36,
            crit_1pct: 1.63,
            direction_at_max: "NONE".into(),
            cusum_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", n),
            ..Default::default()
        };
    }
    let nf = n as f64;
    let mean = log_rets.iter().sum::<f64>() / nf;
    let var = log_rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (nf - 1.0).max(1.0);
    let std = var.sqrt();
    if std < f64::EPSILON {
        return CusumBreakSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            crit_10pct: 1.22,
            crit_5pct: 1.36,
            crit_1pct: 1.63,
            direction_at_max: "NONE".into(),
            cusum_label: "INSUFFICIENT_DATA".into(),
            note: "zero variance".into(),
            ..Default::default()
        };
    }
    let mut cum = 0.0_f64;
    let mut max_abs = 0.0_f64;
    let mut max_bar = 0_usize;
    let mut max_signed = 0.0_f64;
    for (t, r) in log_rets.iter().enumerate() {
        cum += (r - mean) / std;
        let a = cum.abs();
        if a > max_abs {
            max_abs = a;
            max_bar = t;
            max_signed = cum;
        }
    }
    let stat = max_abs / nf.sqrt();
    let crit10 = 1.22_f64;
    let crit5 = 1.36_f64;
    let crit1 = 1.63_f64;
    let reject = stat > crit5;
    let label = if stat < crit10 {
        "STABLE"
    } else if stat < crit5 {
        "MARGINAL"
    } else if stat < crit1 {
        "BREAK_DETECTED"
    } else {
        "STRONG_BREAK"
    };
    let dir = if max_signed > 0.0 {
        "UP"
    } else if max_signed < 0.0 {
        "DOWN"
    } else {
        "NONE"
    };
    CusumBreakSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        max_abs_cusum: max_abs,
        test_statistic: stat,
        max_abs_bar: max_bar,
        direction_at_max: dir.into(),
        crit_10pct: crit10,
        crit_5pct: crit5,
        crit_1pct: crit1,
        reject_stability: reject,
        cusum_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cfvar_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CornishFisherSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return CornishFisherSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cfvar_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let nf = n as f64;
    let mean = log_rets.iter().sum::<f64>() / nf;
    let centered: Vec<f64> = log_rets.iter().map(|r| r - mean).collect();
    let var = centered.iter().map(|d| d * d).sum::<f64>() / nf;
    if var < f64::EPSILON {
        return CornishFisherSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cfvar_label: "INSUFFICIENT_DATA".into(),
            note: "zero variance".into(),
            ..Default::default()
        };
    }
    let std = var.sqrt();
    let m3 = centered.iter().map(|d| d.powi(3)).sum::<f64>() / nf;
    let m4 = centered.iter().map(|d| d.powi(4)).sum::<f64>() / nf;
    let skew = m3 / var.powf(1.5);
    let kurt_excess = m4 / (var * var) - 3.0;
    fn cf_z(z: f64, skew: f64, kurt_excess: f64) -> f64 {
        z + (z * z - 1.0) * skew / 6.0 + (z.powi(3) - 3.0 * z) * kurt_excess / 24.0
            - (2.0 * z.powi(3) - 5.0 * z) * skew * skew / 36.0
    }
    fn cf_skew_term(z: f64, skew: f64) -> f64 {
        (z * z - 1.0) * skew / 6.0 - (2.0 * z.powi(3) - 5.0 * z) * skew * skew / 36.0
    }
    fn cf_kurt_term(z: f64, kurt_excess: f64) -> f64 {
        (z.powi(3) - 3.0 * z) * kurt_excess / 24.0
    }
    let z5 = -1.6448536269514722_f64; // one-tailed 5%
    let z1 = -2.3263478740408408_f64; // one-tailed 1%
    let z5_cf = cf_z(z5, skew, kurt_excess);
    let z1_cf = cf_z(z1, skew, kurt_excess);
    let g5 = (mean + z5 * std) * 100.0;
    let g1 = (mean + z1 * std) * 100.0;
    let c5 = (mean + z5_cf * std) * 100.0;
    let c1 = (mean + z1_cf * std) * 100.0;
    let adj5 = c5 - g5;
    let skew_t5 = cf_skew_term(z5, skew);
    let kurt_t5 = cf_kurt_term(z5, kurt_excess);
    let rel_dev = if g5.abs() > f64::EPSILON {
        adj5.abs() / g5.abs()
    } else {
        0.0
    };
    let label = if rel_dev > 0.50 {
        "EXTREME_DEVIATION"
    } else if rel_dev < 0.10 {
        "BENIGN"
    } else if skew_t5.abs() >= kurt_t5.abs() {
        "SKEW_DRIVEN"
    } else {
        "KURT_DRIVEN"
    };
    CornishFisherSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        mean_ret_pct: mean * 100.0,
        sigma_ret_pct: std * 100.0,
        skewness: skew,
        excess_kurtosis: kurt_excess,
        gauss_var_5pct_pct: g5,
        cf_var_5pct_pct: c5,
        gauss_var_1pct_pct: g1,
        cf_var_1pct_pct: c1,
        cf_adjustment_5pct_pct: adj5,
        skew_term_5pct: skew_t5,
        kurt_term_5pct: kurt_t5,
        cfvar_label: label.into(),
        note: String::new(),
    }
}

// Entropy, Rachev, gain-pain, PACF, and approximate-entropy computes

/// ENTROPY compute: Shannon entropy over a histogram of daily log-returns.
pub fn compute_entropy_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> EntropySnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return EntropySnapshot {
            symbol: sym,
            as_of: as_of.into(),
            entropy_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let num_bins = (n as f64).sqrt().ceil() as usize;
    let min_r = log_rets.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_r = log_rets.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max_r - min_r;
    if range < f64::EPSILON {
        return EntropySnapshot {
            symbol: sym,
            as_of: as_of.into(),
            entropy_label: "INSUFFICIENT_DATA".into(),
            note: "zero range".into(),
            ..Default::default()
        };
    }
    let bin_width = range / num_bins as f64;
    let mut counts = vec![0usize; num_bins];
    for &r in &log_rets {
        let idx = ((r - min_r) / bin_width).floor() as usize;
        let idx = idx.min(num_bins - 1);
        counts[idx] += 1;
    }
    let nf = n as f64;
    let mut h = 0.0_f64;
    for &c in &counts {
        if c > 0 {
            let p = c as f64 / nf;
            h -= p * p.log2();
        }
    }
    let h_max = (num_bins as f64).log2();
    let norm = if h_max > f64::EPSILON { h / h_max } else { 0.0 };
    let label = if norm < 0.50 {
        "LOW_ENTROPY"
    } else if norm < 0.70 {
        "MODERATE_ENTROPY"
    } else if norm < 0.85 {
        "HIGH_ENTROPY"
    } else {
        "VERY_HIGH_ENTROPY"
    };
    EntropySnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        num_bins,
        entropy_bits: h,
        max_entropy_bits: h_max,
        normalised_entropy: norm,
        entropy_label: label.into(),
        note: String::new(),
    }
}

/// RACHEV compute: right-tail ES / left-tail ES at 5% and 1%.
pub fn compute_rachev_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> RachevSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return RachevSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            rachev_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let mut sorted = log_rets.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    fn tail_es(sorted: &[f64], frac: f64, right: bool) -> f64 {
        let k = ((sorted.len() as f64 * frac).ceil() as usize).max(1);
        if right {
            let start = sorted.len() - k;
            sorted[start..].iter().sum::<f64>() / k as f64
        } else {
            sorted[..k].iter().sum::<f64>() / k as f64
        }
    }
    let esr5 = tail_es(&sorted, 0.05, true) * 100.0;
    let esl5 = tail_es(&sorted, 0.05, false) * 100.0;
    let esr1 = tail_es(&sorted, 0.01, true) * 100.0;
    let esl1 = tail_es(&sorted, 0.01, false) * 100.0;
    let r5 = if esl5.abs() > f64::EPSILON {
        esr5.abs() / esl5.abs()
    } else {
        0.0
    };
    let r1 = if esl1.abs() > f64::EPSILON {
        esr1.abs() / esl1.abs()
    } else {
        0.0
    };
    let label = if r5 < 0.5 {
        "STRONG_LEFT_TAIL"
    } else if r5 < 0.8 {
        "LEFT_HEAVY"
    } else if r5 <= 1.2 {
        "SYMMETRIC"
    } else if r5 <= 2.0 {
        "RIGHT_HEAVY"
    } else {
        "STRONG_RIGHT_TAIL"
    };
    RachevSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        es_right_5pct: esr5,
        es_left_5pct: esl5,
        rachev_5pct: r5,
        es_right_1pct: esr1,
        es_left_1pct: esl1,
        rachev_1pct: r1,
        rachev_label: label.into(),
        note: String::new(),
    }
}

/// GPR compute: Gain-to-Pain Ratio = Σ rₜ / Σ |min(rₜ,0)|.
pub fn compute_gpr_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> GprSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return GprSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            gpr_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let mut sum_all = 0.0_f64;
    let mut sum_gains = 0.0_f64;
    let mut sum_losses = 0.0_f64;
    let mut wins = 0usize;
    let mut losses = 0usize;
    for &r in &log_rets {
        sum_all += r;
        if r > 0.0 {
            sum_gains += r;
            wins += 1;
        } else if r < 0.0 {
            sum_losses += r.abs();
            losses += 1;
        }
    }
    let gpr = if sum_losses > f64::EPSILON {
        sum_all / sum_losses
    } else {
        0.0
    };
    let pf = if sum_losses > f64::EPSILON {
        sum_gains / sum_losses
    } else {
        0.0
    };
    let label = if gpr < -0.5 {
        "DEEP_PAIN"
    } else if gpr < 0.0 {
        "NEGATIVE"
    } else if gpr < 0.5 {
        "MODEST"
    } else if gpr < 1.5 {
        "GOOD"
    } else {
        "EXCELLENT"
    };
    GprSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        sum_all_returns_pct: sum_all * 100.0,
        sum_losses_pct: sum_losses * 100.0,
        sum_gains_pct: sum_gains * 100.0,
        gain_to_pain: gpr,
        profit_factor: pf,
        win_count: wins,
        loss_count: losses,
        gpr_label: label.into(),
        note: String::new(),
    }
}

/// PACF compute: partial autocorrelation at lags 1-5 via Durbin-Levinson.
pub fn compute_pacf_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> PacfSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return PacfSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            pacf_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let nf = n as f64;
    let mean = log_rets.iter().sum::<f64>() / nf;
    let centered: Vec<f64> = log_rets.iter().map(|r| r - mean).collect();
    let c0: f64 = centered.iter().map(|d| d * d).sum::<f64>() / nf;
    if c0 < f64::EPSILON {
        return PacfSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            pacf_label: "INSUFFICIENT_DATA".into(),
            note: "zero variance".into(),
            ..Default::default()
        };
    }
    let max_lag = 5usize;
    let mut acf = vec![0.0_f64; max_lag + 1];
    for k in 1..=max_lag {
        let mut s = 0.0;
        for t in k..n {
            s += centered[t] * centered[t - k];
        }
        acf[k] = s / (nf * c0);
    }
    // Durbin-Levinson recursion
    let mut pacf_vals = vec![0.0_f64; max_lag + 1];
    let mut phi: Vec<Vec<f64>> = vec![vec![0.0; max_lag + 1]; max_lag + 1];
    phi[1][1] = acf[1];
    pacf_vals[1] = acf[1];
    for k in 2..=max_lag {
        let mut num = acf[k];
        for j in 1..k {
            num -= phi[k - 1][j] * acf[k - j];
        }
        let mut den = 1.0;
        for j in 1..k {
            den -= phi[k - 1][j] * acf[j];
        }
        if den.abs() < f64::EPSILON {
            break;
        }
        phi[k][k] = num / den;
        pacf_vals[k] = phi[k][k];
        for j in 1..k {
            phi[k][j] = phi[k - 1][j] - phi[k][k] * phi[k - 1][k - j];
        }
    }
    let crit = 1.96 / nf.sqrt();
    let pacfs = [
        pacf_vals[1],
        pacf_vals[2],
        pacf_vals[3],
        pacf_vals[4],
        pacf_vals[5],
    ];
    let sig_count = pacfs.iter().filter(|p| p.abs() > crit).count();
    let (max_abs, max_lag_idx) = pacfs
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.abs().partial_cmp(&b.abs()).unwrap())
        .map(|(i, v)| (v.abs(), i + 1))
        .unwrap_or((0.0, 0));
    let label = if sig_count == 0 {
        "NO_STRUCTURE"
    } else if sig_count == 1 && pacfs[0].abs() > crit {
        "LAG1_DOMINANT"
    } else if max_abs > 2.0 * crit {
        "STRONG_STRUCTURE"
    } else {
        "LAG_STRUCTURE"
    };
    PacfSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pacf_lag1: pacfs[0],
        pacf_lag2: pacfs[1],
        pacf_lag3: pacfs[2],
        pacf_lag4: pacfs[3],
        pacf_lag5: pacfs[4],
        bartlett_crit_95: crit,
        significant_lags: sig_count,
        max_abs_pacf: max_abs,
        max_abs_lag: max_lag_idx,
        pacf_label: label.into(),
        note: String::new(),
    }
}

/// APEN compute: approximate entropy (Pincus 1991), m=2, r=0.2·σ.
pub fn compute_apen_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> ApenSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return ApenSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            apen_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let nf = n as f64;
    let mean = log_rets.iter().sum::<f64>() / nf;
    let var = log_rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / nf;
    if var < f64::EPSILON {
        return ApenSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            apen_label: "INSUFFICIENT_DATA".into(),
            note: "zero variance".into(),
            ..Default::default()
        };
    }
    let std = var.sqrt();
    let m = 2usize;
    let r = 0.2 * std;
    fn phi_func(data: &[f64], m: usize, r: f64) -> f64 {
        let n = data.len();
        let nm = n - m + 1;
        if nm == 0 {
            return 0.0;
        }
        let mut sum = 0.0_f64;
        for i in 0..nm {
            let mut count = 0usize;
            for j in 0..nm {
                let mut matched = true;
                for k in 0..m {
                    if (data[i + k] - data[j + k]).abs() > r {
                        matched = false;
                        break;
                    }
                }
                if matched {
                    count += 1;
                }
            }
            sum += (count as f64 / nm as f64).ln();
        }
        sum / nm as f64
    }
    let phi_m = phi_func(&log_rets, m, r);
    let phi_m1 = phi_func(&log_rets, m + 1, r);
    let apen = (phi_m - phi_m1).max(0.0);
    let label = if apen < 0.3 {
        "REGULAR"
    } else if apen < 0.7 {
        "MODERATE"
    } else if apen < 1.2 {
        "COMPLEX"
    } else {
        "HIGHLY_COMPLEX"
    };
    ApenSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        embed_dim: m,
        tolerance: r,
        phi_m,
        phi_m1,
        apen,
        apen_label: label.into(),
        note: String::new(),
    }
}

// Upside-potential, leverage-effect, drawdown-at-risk, VaR-half-life, and Gini computes

/// UPR compute: Upside Potential Ratio = E[max(r,0)] / √E[min(r,0)²] (MAR=0).
pub fn compute_upr_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> UprSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return UprSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            upr_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let nf = n as f64;
    let upm1 = log_rets.iter().map(|&r| r.max(0.0)).sum::<f64>() / nf;
    let lpm2 = log_rets.iter().map(|&r| r.min(0.0).powi(2)).sum::<f64>() / nf;
    let dd = lpm2.sqrt();
    let upr = if dd > f64::EPSILON { upm1 / dd } else { 0.0 };
    let label = if upr < 0.5 {
        "LOW_UPSIDE"
    } else if upr < 1.0 {
        "MODERATE_UPSIDE"
    } else if upr < 1.5 {
        "BALANCED"
    } else if upr < 2.5 {
        "HIGH_UPSIDE"
    } else {
        "VERY_HIGH_UPSIDE"
    };
    UprSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        upm1,
        lpm2,
        downside_dev: dd,
        upr,
        upr_label: label.into(),
        note: String::new(),
    }
}

/// LEVEREFF compute: leverage effect corr(rₜ, rₜ₊₁²) + asymmetric vol ratio.
pub fn compute_levereff_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> LeverEffSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return LeverEffSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            lever_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let pairs: Vec<(f64, f64)> = (0..n - 1)
        .map(|i| (log_rets[i], log_rets[i + 1] * log_rets[i + 1]))
        .collect();
    let np = pairs.len() as f64;
    let mean_r = pairs.iter().map(|(r, _)| r).sum::<f64>() / np;
    let mean_s = pairs.iter().map(|(_, s)| s).sum::<f64>() / np;
    let cov = pairs
        .iter()
        .map(|(r, s)| (r - mean_r) * (s - mean_s))
        .sum::<f64>()
        / np;
    let var_r = pairs.iter().map(|(r, _)| (r - mean_r).powi(2)).sum::<f64>() / np;
    let var_s = pairs.iter().map(|(_, s)| (s - mean_s).powi(2)).sum::<f64>() / np;
    let corr = if var_r > f64::EPSILON && var_s > f64::EPSILON {
        cov / (var_r.sqrt() * var_s.sqrt())
    } else {
        0.0
    };
    let mut sum_vol_neg = 0.0_f64;
    let mut cnt_neg = 0usize;
    let mut sum_vol_pos = 0.0_f64;
    let mut cnt_pos = 0usize;
    for i in 0..n - 1 {
        let next_abs = log_rets[i + 1].abs();
        if log_rets[i] < 0.0 {
            sum_vol_neg += next_abs;
            cnt_neg += 1;
        } else if log_rets[i] > 0.0 {
            sum_vol_pos += next_abs;
            cnt_pos += 1;
        }
    }
    let mvn = if cnt_neg > 0 {
        sum_vol_neg / cnt_neg as f64 * 100.0
    } else {
        0.0
    };
    let mvp = if cnt_pos > 0 {
        sum_vol_pos / cnt_pos as f64 * 100.0
    } else {
        0.0
    };
    let asym = if mvp > f64::EPSILON { mvn / mvp } else { 0.0 };
    let label = if corr < -0.15 {
        "STRONG_LEVERAGE"
    } else if corr < -0.05 {
        "MILD_LEVERAGE"
    } else if corr <= 0.05 {
        "SYMMETRIC"
    } else {
        "REVERSE_LEVERAGE"
    };
    LeverEffSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        corr_r_nextsq: corr,
        mean_vol_after_neg: mvn,
        mean_vol_after_pos: mvp,
        asym_ratio: asym,
        lever_label: label.into(),
        note: String::new(),
    }
}

/// DRAWDAR compute: Drawdown-at-Risk + Conditional DaR at 5% and 1%.
pub fn compute_drawdar_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DrawDaRSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 30 {
        return DrawDaRSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            drawdar_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 bars, got {}", bars.len()),
            ..Default::default()
        };
    }
    let n = bars.len();
    let mut peak = bars[0].close;
    let mut dds: Vec<f64> = Vec::with_capacity(n);
    for b in bars {
        if b.close > peak {
            peak = b.close;
        }
        let dd = if peak > f64::EPSILON {
            (peak - b.close) / peak * 100.0
        } else {
            0.0
        };
        dds.push(dd);
    }
    let mut sorted = dds.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let quantile = |q: f64| -> f64 {
        let idx = ((n as f64 * q).ceil() as usize).min(n) - 1;
        sorted[idx]
    };
    let dar5 = quantile(0.95);
    let dar1 = quantile(0.99);
    let cdar5 = {
        let tail: Vec<f64> = sorted.iter().filter(|&&d| d >= dar5).cloned().collect();
        if tail.is_empty() {
            dar5
        } else {
            tail.iter().sum::<f64>() / tail.len() as f64
        }
    };
    let cdar1 = {
        let tail: Vec<f64> = sorted.iter().filter(|&&d| d >= dar1).cloned().collect();
        if tail.is_empty() {
            dar1
        } else {
            tail.iter().sum::<f64>() / tail.len() as f64
        }
    };
    let max_dd = sorted.last().cloned().unwrap_or(0.0);
    let nonzero: Vec<f64> = dds.iter().filter(|&&d| d > f64::EPSILON).cloned().collect();
    let mean_dd = if nonzero.is_empty() {
        0.0
    } else {
        nonzero.iter().sum::<f64>() / nonzero.len() as f64
    };
    let label = if dar5 < 3.0 {
        "LOW_DD_RISK"
    } else if dar5 < 7.0 {
        "MODERATE_DD_RISK"
    } else if dar5 < 15.0 {
        "HIGH_DD_RISK"
    } else {
        "SEVERE_DD_RISK"
    };
    DrawDaRSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        dar_5pct: dar5,
        cdar_5pct: cdar5,
        dar_1pct: dar1,
        cdar_1pct: cdar1,
        max_dd_pct: max_dd,
        mean_dd_pct: mean_dd,
        drawdar_label: label.into(),
        note: String::new(),
    }
}

/// VARHALF compute: volatility half-life via AR(1) on rolling 20d realized vol.
pub fn compute_varhalf_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> VarHalfSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 50 {
        return VarHalfSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            varhalf_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥50 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let window = 20usize;
    let mut vols: Vec<f64> = Vec::new();
    for i in window..=n {
        let slice = &log_rets[i - window..i];
        let m = slice.iter().sum::<f64>() / window as f64;
        let v = slice.iter().map(|r| (r - m).powi(2)).sum::<f64>() / window as f64;
        vols.push(v.sqrt());
    }
    if vols.len() < 10 {
        return VarHalfSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            varhalf_label: "INSUFFICIENT_DATA".into(),
            note: "too few vol observations".into(),
            ..Default::default()
        };
    }
    let nv = vols.len();
    let pairs: Vec<(f64, f64)> = (0..nv - 1).map(|i| (vols[i], vols[i + 1])).collect();
    let np = pairs.len() as f64;
    let mx = pairs.iter().map(|(x, _)| x).sum::<f64>() / np;
    let my = pairs.iter().map(|(_, y)| y).sum::<f64>() / np;
    let sxy = pairs.iter().map(|(x, y)| (x - mx) * (y - my)).sum::<f64>();
    let sxx = pairs.iter().map(|(x, _)| (x - mx).powi(2)).sum::<f64>();
    let beta = if sxx > f64::EPSILON { sxy / sxx } else { 0.0 };
    let alpha = my - beta * mx;
    let ss_res = pairs
        .iter()
        .map(|(x, y)| (y - alpha - beta * x).powi(2))
        .sum::<f64>();
    let ss_tot = pairs.iter().map(|(_, y)| (y - my).powi(2)).sum::<f64>();
    let r2 = if ss_tot > f64::EPSILON {
        1.0 - ss_res / ss_tot
    } else {
        0.0
    };
    let hl = if beta > f64::EPSILON && beta < 1.0 {
        -(2.0_f64.ln()) / beta.ln()
    } else if beta >= 1.0 {
        f64::INFINITY
    } else {
        0.0
    };
    let label = if hl.is_infinite() || hl > 60.0 {
        "VERY_PERSISTENT"
    } else if hl > 30.0 {
        "SLOW_PERSIST"
    } else if hl > 10.0 {
        "MODERATE_PERSIST"
    } else {
        "FAST_REVERT"
    };
    VarHalfSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        vol_obs: nv,
        ar1_beta: beta,
        ar1_alpha: alpha,
        ar1_r2: r2,
        half_life_days: hl,
        varhalf_label: label.into(),
        note: String::new(),
    }
}

/// GINI compute: Gini coefficient of |returns|.
pub fn compute_gini_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> GiniSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return GiniSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            gini_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let nf = n as f64;
    let mut abs_rets: Vec<f64> = log_rets.iter().map(|r| r.abs()).collect();
    abs_rets.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let total = abs_rets.iter().sum::<f64>();
    if total < f64::EPSILON {
        return GiniSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            gini_label: "INSUFFICIENT_DATA".into(),
            note: "zero total |returns|".into(),
            ..Default::default()
        };
    }
    let weighted_sum: f64 = abs_rets
        .iter()
        .enumerate()
        .map(|(i, &v)| (i as f64 + 1.0) * v)
        .sum();
    let gini = (2.0 * weighted_sum) / (nf * total) - (nf + 1.0) / nf;
    let mean_abs = total / nf * 100.0;
    let median_abs = abs_rets[n / 2] * 100.0;
    let label = if gini < 0.30 {
        "LOW_CONCENTRATION"
    } else if gini < 0.45 {
        "MODERATE_CONCENTRATION"
    } else if gini < 0.60 {
        "HIGH_CONCENTRATION"
    } else {
        "VERY_HIGH_CONCENTRATION"
    };
    GiniSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        gini_coeff: gini,
        mean_abs_return_pct: mean_abs,
        median_abs_return_pct: median_abs,
        gini_label: label.into(),
        note: String::new(),
    }
}

// Sample-entropy, permutation-entropy, recurrence-factor, KPSS, and spectral-entropy computes

/// SAMPEN compute: Sample Entropy (Richman & Moorman 2000).
pub fn compute_sampen_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> SampenSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return SampenSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            sampen_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let m = 2usize;
    let sigma = {
        let mean = log_rets.iter().sum::<f64>() / n as f64;
        (log_rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n as f64).sqrt()
    };
    let r = 0.2 * sigma;
    if r < f64::EPSILON {
        return SampenSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            sampen_label: "INSUFFICIENT_DATA".into(),
            note: "zero stdev".into(),
            ..Default::default()
        };
    }
    let mut b_count = 0usize;
    let mut a_count = 0usize;
    for i in 0..n - m {
        for j in (i + 1)..n - m {
            let match_m = (0..m).all(|k| (log_rets[i + k] - log_rets[j + k]).abs() <= r);
            if match_m {
                b_count += 1;
                if i + m < n && j + m < n && (log_rets[i + m] - log_rets[j + m]).abs() <= r {
                    a_count += 1;
                }
            }
        }
    }
    let (sampen, label) = if b_count == 0 {
        (0.0, "UNDEFINED")
    } else if a_count == 0 {
        (f64::INFINITY, "HIGHLY_COMPLEX")
    } else {
        let se = -(a_count as f64 / b_count as f64).ln();
        let l = if se < 0.3 {
            "REGULAR"
        } else if se < 0.7 {
            "MODERATE"
        } else if se < 1.2 {
            "COMPLEX"
        } else {
            "HIGHLY_COMPLEX"
        };
        (se, l)
    };
    SampenSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        embed_dim: m,
        tolerance: r,
        a_count,
        b_count,
        sampen,
        sampen_label: label.into(),
        note: String::new(),
    }
}

/// PERMEN compute: Permutation Entropy (Bandt & Pompe 2002).
pub fn compute_permen_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> PermenSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return PermenSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            permen_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let m = 3usize;
    let factorial_m = 6usize; // 3! = 6
    let mut pattern_counts = std::collections::HashMap::<Vec<usize>, usize>::new();
    let num_patterns = n - m + 1;
    for i in 0..num_patterns {
        let window = &log_rets[i..i + m];
        let mut indices: Vec<usize> = (0..m).collect();
        indices.sort_by(|&a, &b| {
            window[a]
                .partial_cmp(&window[b])
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        *pattern_counts.entry(indices).or_insert(0) += 1;
    }
    let num_p = num_patterns as f64;
    let h_raw: f64 = pattern_counts
        .values()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = c as f64 / num_p;
            -p * p.log2()
        })
        .sum();
    let h_max = (factorial_m as f64).log2();
    let h_norm = if h_max > 0.0 { h_raw / h_max } else { 0.0 };
    let label = if h_norm < 0.50 {
        "REGULAR"
    } else if h_norm < 0.70 {
        "MODERATE"
    } else if h_norm < 0.85 {
        "COMPLEX"
    } else {
        "HIGHLY_COMPLEX"
    };
    PermenSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        embed_dim: m,
        patterns_observed: pattern_counts.len(),
        patterns_possible: factorial_m,
        permen_raw: h_raw,
        permen_normalised: h_norm,
        permen_label: label.into(),
        note: String::new(),
    }
}

/// RECFACT compute: Recovery Factor.
pub fn compute_recfact_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> RecfactSnapshot {
    let sym = symbol.to_uppercase();
    let usable: Vec<&HistoricalPriceRow> = bars.iter().filter(|b| b.close > 0.0).collect();
    if usable.len() < 20 {
        return RecfactSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            recfact_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥20 bars, got {}", usable.len()),
            ..Default::default()
        };
    }
    let n = usable.len();
    let first_close = usable[0].close;
    let last_close = usable[n - 1].close;
    let cum_return = (last_close / first_close) - 1.0;
    let mut peak = usable[0].close;
    let mut max_dd = 0.0f64;
    for b in usable.iter() {
        if b.close > peak {
            peak = b.close;
        }
        let dd = (peak - b.close) / peak;
        if dd > max_dd {
            max_dd = dd;
        }
    }
    let (rf, label) = if max_dd < 1e-10 {
        if cum_return >= 0.0 {
            (f64::INFINITY, "EXCELLENT")
        } else {
            (0.0, "DEEP_LOSS")
        }
    } else {
        let r = cum_return / max_dd;
        let l = if r < -1.0 {
            "DEEP_LOSS"
        } else if r < 0.0 {
            "NEGATIVE"
        } else if r < 1.0 {
            "RECOVERING"
        } else if r < 3.0 {
            "GOOD"
        } else {
            "EXCELLENT"
        };
        (r, l)
    };
    RecfactSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        cum_return_pct: cum_return * 100.0,
        max_drawdown_pct: max_dd * 100.0,
        recovery_factor: rf,
        recfact_label: label.into(),
        note: String::new(),
    }
}

/// KPSS compute: Kwiatkowski-Phillips-Schmidt-Shin stationarity test.
pub fn compute_kpss_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> KpssSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return KpssSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            kpss_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let nf = n as f64;
    let mean = log_rets.iter().sum::<f64>() / nf;
    let residuals: Vec<f64> = log_rets.iter().map(|r| r - mean).collect();
    let mut partial_sums = vec![0.0f64; n];
    partial_sums[0] = residuals[0];
    for i in 1..n {
        partial_sums[i] = partial_sums[i - 1] + residuals[i];
    }
    let lag_trunc = ((4.0 * (nf / 100.0).powf(2.0 / 9.0)).floor()) as usize;
    let lag_trunc = lag_trunc.max(1);
    let sigma2 = residuals.iter().map(|e| e * e).sum::<f64>() / nf;
    let mut s2_long = sigma2;
    for l in 1..=lag_trunc {
        let gamma_l: f64 = (0..n - l)
            .map(|t| residuals[t] * residuals[t + l])
            .sum::<f64>()
            / nf;
        let w = 1.0 - (l as f64 / (lag_trunc as f64 + 1.0));
        s2_long += 2.0 * w * gamma_l;
    }
    if s2_long < f64::EPSILON {
        return KpssSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            kpss_label: "INSUFFICIENT_DATA".into(),
            note: "zero long-run variance".into(),
            ..Default::default()
        };
    }
    let eta = partial_sums.iter().map(|s| s * s).sum::<f64>() / (nf * nf * s2_long);
    let crit_10 = 0.347;
    let crit_5 = 0.463;
    let crit_1 = 0.739;
    let reject = eta > crit_5;
    let label = if eta <= crit_10 {
        "STATIONARY"
    } else if eta <= crit_5 {
        "WEAKLY_NONSTATIONARY"
    } else {
        "NONSTATIONARY"
    };
    KpssSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        kpss_stat: eta,
        lag_truncation: lag_trunc,
        crit_10,
        crit_5,
        crit_1,
        reject_stationary: reject,
        kpss_label: label.into(),
        note: String::new(),
    }
}

/// SPECENT compute: Spectral Entropy via DFT.
pub fn compute_specent_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> SpecentSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return SpecentSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            specent_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let mean = log_rets.iter().sum::<f64>() / n as f64;
    let centered: Vec<f64> = log_rets.iter().map(|r| r - mean).collect();
    let num_freqs = n / 2;
    let mut psd = vec![0.0f64; num_freqs];
    let pi2 = 2.0 * std::f64::consts::PI;
    for k in 1..=num_freqs {
        let mut re = 0.0f64;
        let mut im = 0.0f64;
        for (t, &x) in centered.iter().enumerate() {
            let angle = pi2 * k as f64 * t as f64 / n as f64;
            re += x * angle.cos();
            im -= x * angle.sin();
        }
        psd[k - 1] = (re * re + im * im) / n as f64;
    }
    let total_power: f64 = psd.iter().sum();
    if total_power < f64::EPSILON {
        return SpecentSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            specent_label: "INSUFFICIENT_DATA".into(),
            note: "zero spectral power".into(),
            ..Default::default()
        };
    }
    let norm_psd: Vec<f64> = psd.iter().map(|p| p / total_power).collect();
    let h_raw: f64 = norm_psd
        .iter()
        .filter(|&&p| p > 0.0)
        .map(|&p| -p * p.log2())
        .sum();
    let h_max = (num_freqs as f64).log2();
    let h_norm = if h_max > 0.0 { h_raw / h_max } else { 0.0 };
    let (peak_idx, peak_share) = norm_psd
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(i, &v)| (i, v))
        .unwrap_or((0, 0.0));
    let label = if h_norm < 0.50 {
        "PERIODIC"
    } else if h_norm < 0.70 {
        "MODERATE_PERIODICITY"
    } else if h_norm < 0.85 {
        "BROAD_SPECTRUM"
    } else {
        "NOISE_LIKE"
    };
    SpecentSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        num_freqs,
        spectral_entropy_raw: h_raw,
        spectral_entropy_norm: h_norm,
        peak_freq_idx: peak_idx,
        peak_power_share: peak_share,
        specent_label: label.into(),
        note: String::new(),
    }
}

// Robust-volatility, Renyi-entropy, return-quantile, market-sentiment, and EWMA-volatility computes

/// ROBVOL compute: Robust Volatility via MAD and IQR.
pub fn compute_robvol_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> RobVolSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return RobVolSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            robvol_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let nf = n as f64;
    let mean = log_rets.iter().sum::<f64>() / nf;
    let classical = (log_rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / nf).sqrt();
    if classical < f64::EPSILON {
        return RobVolSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            robvol_label: "INSUFFICIENT_DATA".into(),
            note: "zero classical sigma".into(),
            ..Default::default()
        };
    }
    let mut sorted = log_rets.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = if n % 2 == 0 {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    } else {
        sorted[n / 2]
    };
    let mut abs_dev: Vec<f64> = log_rets.iter().map(|r| (r - median).abs()).collect();
    abs_dev.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mad = if n % 2 == 0 {
        (abs_dev[n / 2 - 1] + abs_dev[n / 2]) / 2.0
    } else {
        abs_dev[n / 2]
    };
    let mad_sigma_daily = mad / 0.6745;
    let q = |p: f64| -> f64 {
        let idx = (p * (n as f64 - 1.0)).clamp(0.0, (n - 1) as f64);
        let lo = idx.floor() as usize;
        let hi = idx.ceil() as usize;
        let frac = idx - lo as f64;
        sorted[lo] + frac * (sorted[hi] - sorted[lo])
    };
    let iqr = q(0.75) - q(0.25);
    let iqr_sigma_daily = iqr / 1.349;
    let ann = (252.0_f64).sqrt();
    let classical_ann = classical * ann;
    let mad_ann = mad_sigma_daily * ann;
    let iqr_ann = iqr_sigma_daily * ann;
    let mad_ratio = mad_ann / classical_ann;
    let iqr_ratio = iqr_ann / classical_ann;
    let avg_ratio = (mad_ratio + iqr_ratio) / 2.0;
    let label = if avg_ratio < 0.60 {
        "HEAVY_OUTLIERS"
    } else if avg_ratio < 0.80 {
        "MODERATE_OUTLIERS"
    } else if avg_ratio < 1.10 {
        "CLEAN"
    } else {
        "LIGHT_TAILS"
    };
    RobVolSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        classical_sigma: classical_ann,
        mad_sigma: mad_ann,
        iqr_sigma: iqr_ann,
        mad_ratio,
        iqr_ratio,
        robvol_label: label.into(),
        note: String::new(),
    }
}

/// RENYIENT compute: Rényi Entropy at α=2 (collision entropy).
pub fn compute_renyient_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> RenyientSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return RenyientSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            renyient_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let k = (((n as f64).log2()).ceil() as usize + 1).max(4);
    let (mn, mx) = log_rets
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &r| {
            (a.min(r), b.max(r))
        });
    let range = mx - mn;
    if range < f64::EPSILON {
        return RenyientSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            renyient_label: "INSUFFICIENT_DATA".into(),
            note: "zero range".into(),
            ..Default::default()
        };
    }
    let width = range / k as f64;
    let mut counts = vec![0usize; k];
    for &r in log_rets.iter() {
        let idx = (((r - mn) / width).floor() as usize).min(k - 1);
        counts[idx] += 1;
    }
    let nf = n as f64;
    let p2_sum: f64 = counts
        .iter()
        .map(|&c| {
            let p = c as f64 / nf;
            p * p
        })
        .sum();
    if p2_sum < f64::EPSILON {
        return RenyientSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            renyient_label: "INSUFFICIENT_DATA".into(),
            note: "zero collision prob".into(),
            ..Default::default()
        };
    }
    let h_raw = -p2_sum.log2();
    let h_max = (k as f64).log2();
    let h_norm = if h_max > 0.0 { h_raw / h_max } else { 0.0 };
    let label = if h_norm < 0.50 {
        "CONCENTRATED"
    } else if h_norm < 0.70 {
        "MODERATE"
    } else if h_norm < 0.85 {
        "DISPERSED"
    } else {
        "HIGHLY_DISPERSED"
    };
    RenyientSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        num_bins: k,
        alpha: 2.0,
        renyi_raw: h_raw,
        renyi_normalised: h_norm,
        collision_prob: p2_sum,
        renyient_label: label.into(),
        note: String::new(),
    }
}

/// RETQUANT compute: 9-point Return Quantile Profile.
pub fn compute_retquant_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> RetquantSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return RetquantSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            retquant_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let mut sorted = log_rets.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let q = |p: f64| -> f64 {
        let idx = (p * (n as f64 - 1.0)).clamp(0.0, (n - 1) as f64);
        let lo = idx.floor() as usize;
        let hi = idx.ceil() as usize;
        let frac = idx - lo as f64;
        sorted[lo] + frac * (sorted[hi] - sorted[lo])
    };
    let p01 = q(0.01);
    let p05 = q(0.05);
    let p10 = q(0.10);
    let p25 = q(0.25);
    let p50 = q(0.50);
    let p75 = q(0.75);
    let p90 = q(0.90);
    let p95 = q(0.95);
    let p99 = q(0.99);
    let iqr = p75 - p25;
    let span = p99 - p01;
    let tail_asymm = if span.abs() < f64::EPSILON {
        0.0
    } else {
        (p99 + p01) / span
    };
    let label = if tail_asymm < -0.30 {
        "LEFT_TAIL_HEAVY"
    } else if tail_asymm > 0.30 {
        "RIGHT_TAIL_HEAVY"
    } else if iqr > 0.04 {
        "WIDE_IQR"
    } else {
        "SYMMETRIC"
    };
    RetquantSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        p01_pct: p01 * 100.0,
        p05_pct: p05 * 100.0,
        p10_pct: p10 * 100.0,
        p25_pct: p25 * 100.0,
        p50_pct: p50 * 100.0,
        p75_pct: p75 * 100.0,
        p90_pct: p90 * 100.0,
        p95_pct: p95 * 100.0,
        p99_pct: p99 * 100.0,
        iqr_pct: iqr * 100.0,
        tail_asymmetry: tail_asymm,
        retquant_label: label.into(),
        note: String::new(),
    }
}

/// MSENT compute: Multiscale Entropy (Costa, Goldberger, Peng 2005).
pub fn compute_msent_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MsentSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 100 {
        return MsentSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            msent_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥100 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let m = 2usize;
    let mean = log_rets.iter().sum::<f64>() / n as f64;
    let sigma = (log_rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n as f64).sqrt();
    let r = 0.2 * sigma;
    if r < f64::EPSILON {
        return MsentSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            msent_label: "INSUFFICIENT_DATA".into(),
            note: "zero stdev".into(),
            ..Default::default()
        };
    }
    let max_scale = 5usize;
    let mut sampens = [0.0f64; 5];
    for tau in 1..=max_scale {
        let len = n / tau;
        if len < 20 {
            sampens[tau - 1] = f64::NAN;
            continue;
        }
        let mut coarse = Vec::with_capacity(len);
        for j in 0..len {
            let s = j * tau;
            let block = &log_rets[s..s + tau];
            coarse.push(block.iter().sum::<f64>() / tau as f64);
        }
        let mut a = 0usize;
        let mut b = 0usize;
        if coarse.len() > m {
            for i in 0..coarse.len() - m {
                for j in (i + 1)..coarse.len() - m {
                    let match_m = (0..m).all(|k| (coarse[i + k] - coarse[j + k]).abs() <= r);
                    if match_m {
                        b += 1;
                        if i + m < coarse.len()
                            && j + m < coarse.len()
                            && (coarse[i + m] - coarse[j + m]).abs() <= r
                        {
                            a += 1;
                        }
                    }
                }
            }
        }
        sampens[tau - 1] = if b == 0 {
            f64::NAN
        } else if a == 0 {
            0.0
        } else {
            -(a as f64 / b as f64).ln()
        };
    }
    let finite: Vec<f64> = sampens.iter().filter(|v| v.is_finite()).copied().collect();
    let complexity_index = finite.iter().sum::<f64>();
    let label = if finite.len() < 3 {
        "INSUFFICIENT_DATA"
    } else {
        let first = sampens[0];
        let last = *finite.last().unwrap();
        let all_low = finite.iter().all(|&v| v < 0.3);
        if all_low {
            "LONG_RANGE_REGULAR"
        } else if last < first * 0.7 {
            "DECAYING"
        } else if last > first * 1.3 {
            "INCREASING"
        } else {
            "SUSTAINED"
        }
    };
    MsentSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        embed_dim: m,
        tolerance: r,
        max_scale,
        sampen_scale1: sampens[0],
        sampen_scale2: sampens[1],
        sampen_scale3: sampens[2],
        sampen_scale4: sampens[3],
        sampen_scale5: sampens[4],
        msent_complexity_index: complexity_index,
        msent_label: label.into(),
        note: String::new(),
    }
}

/// EWMAVOL compute: RiskMetrics EWMA Volatility (λ=0.94).
pub fn compute_ewmavol_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> EwmaVolSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return EwmaVolSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            ewmavol_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let nf = n as f64;
    let mean = log_rets.iter().sum::<f64>() / nf;
    let classical_var = log_rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / nf;
    let lambda = 0.94f64;
    let mut var_t = classical_var;
    for &r in log_rets.iter() {
        let dev = r - mean;
        var_t = lambda * var_t + (1.0 - lambda) * dev * dev;
    }
    if var_t < f64::EPSILON {
        return EwmaVolSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            ewmavol_label: "INSUFFICIENT_DATA".into(),
            note: "zero ewma variance".into(),
            ..Default::default()
        };
    }
    let ewma_sigma_daily = var_t.sqrt();
    let ann = (252.0_f64).sqrt();
    let ewma_ann = ewma_sigma_daily * ann;
    let classical_ann = classical_var.sqrt() * ann;
    let ratio = if classical_ann > f64::EPSILON {
        ewma_ann / classical_ann
    } else {
        1.0
    };
    let label = if ratio > 1.20 {
        "ELEVATED"
    } else if ratio < 0.80 {
        "SUPPRESSED"
    } else {
        "NORMAL"
    };
    EwmaVolSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        lambda,
        ewma_variance: var_t,
        ewma_sigma_daily,
        ewma_sigma_annual: ewma_ann,
        classical_sigma_annual: classical_ann,
        ewma_to_classical: ratio,
        ewmavol_label: label.into(),
        note: String::new(),
    }
}

// KS-normality, Anderson-Darling, L-moment, Kyle-lambda, and peak-over-threshold computes

/// Standard normal CDF via Abramowitz-Stegun 7.1.26 approximation.
fn norm_cdf_as(z: f64) -> f64 {
    let a1 = 0.254829592_f64;
    let a2 = -0.284496736_f64;
    let a3 = 1.421413741_f64;
    let a4 = -1.453152027_f64;
    let a5 = 1.061405429_f64;
    let p = 0.3275911_f64;
    let sign = if z < 0.0 { -1.0 } else { 1.0 };
    let x = (z / std::f64::consts::SQRT_2).abs();
    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();
    0.5 * (1.0 + sign * y)
}

/// KSNORM compute: Kolmogorov-Smirnov normality test.
pub fn compute_ksnorm_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> KsnormSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return KsnormSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            ksnorm_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let nf = n as f64;
    let mean = log_rets.iter().sum::<f64>() / nf;
    let var = log_rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / nf;
    let sigma = var.sqrt();
    if sigma < f64::EPSILON {
        return KsnormSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            ksnorm_label: "INSUFFICIENT_DATA".into(),
            note: "zero stdev".into(),
            ..Default::default()
        };
    }
    let mut z: Vec<f64> = log_rets.iter().map(|r| (r - mean) / sigma).collect();
    z.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut d_stat = 0.0_f64;
    for (i, &zi) in z.iter().enumerate() {
        let f_emp_hi = (i as f64 + 1.0) / nf;
        let f_emp_lo = i as f64 / nf;
        let f_theor = norm_cdf_as(zi);
        let d1 = (f_emp_hi - f_theor).abs();
        let d2 = (f_theor - f_emp_lo).abs();
        if d1 > d_stat {
            d_stat = d1;
        }
        if d2 > d_stat {
            d_stat = d2;
        }
    }
    let sqrt_n = nf.sqrt();
    let c10 = 1.22 / sqrt_n;
    let c5 = 1.36 / sqrt_n;
    let c1 = 1.63 / sqrt_n;
    let r10 = d_stat > c10;
    let r5 = d_stat > c5;
    let r1 = d_stat > c1;
    let label = if !r10 {
        "NORMAL"
    } else if !r5 {
        "MILD_DEVIATION"
    } else if !r1 {
        "MODERATE_DEVIATION"
    } else {
        "STRONG_NON_NORMAL"
    };
    KsnormSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        ks_statistic: d_stat,
        critical_10pct: c10,
        critical_5pct: c5,
        critical_1pct: c1,
        reject_10pct: r10,
        reject_5pct: r5,
        reject_1pct: r1,
        mean,
        sigma,
        ksnorm_label: label.into(),
        note: String::new(),
    }
}

/// ADTEST compute: Anderson-Darling normality test (tail-weighted).
pub fn compute_adtest_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> AdtestSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return AdtestSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            adtest_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let nf = n as f64;
    let mean = log_rets.iter().sum::<f64>() / nf;
    let var = log_rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (nf - 1.0).max(1.0);
    let sigma = var.sqrt();
    if sigma < f64::EPSILON {
        return AdtestSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            adtest_label: "INSUFFICIENT_DATA".into(),
            note: "zero stdev".into(),
            ..Default::default()
        };
    }
    let mut z: Vec<f64> = log_rets.iter().map(|r| (r - mean) / sigma).collect();
    z.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut sum = 0.0_f64;
    let eps = 1e-12_f64;
    for (i, &zi) in z.iter().enumerate() {
        let fi = norm_cdf_as(zi).clamp(eps, 1.0 - eps);
        let j = n - 1 - i;
        let fj = norm_cdf_as(z[j]).clamp(eps, 1.0 - eps);
        let w = (2.0 * (i as f64 + 1.0) - 1.0) / nf;
        sum += w * (fi.ln() + (1.0 - fj).ln());
    }
    let a2 = -nf - sum;
    let a2_adj = a2 * (1.0 + 0.75 / nf + 2.25 / (nf * nf));
    // Stephens (1986) p-value approximation for N(μ̂,σ̂²) case
    let p_value = if a2_adj >= 0.600 {
        (1.2937 - 5.709 * a2_adj + 0.0186 * a2_adj * a2_adj).exp()
    } else if a2_adj >= 0.340 {
        (0.9177 - 4.279 * a2_adj - 1.38 * a2_adj * a2_adj).exp()
    } else if a2_adj >= 0.200 {
        1.0 - (-8.318 + 42.796 * a2_adj - 59.938 * a2_adj * a2_adj).exp()
    } else {
        1.0 - (-13.436 + 101.14 * a2_adj - 223.73 * a2_adj * a2_adj).exp()
    };
    let p_value = p_value.clamp(0.0, 1.0);
    let c10 = 0.631_f64;
    let c5 = 0.752_f64;
    let c1 = 1.035_f64;
    let r10 = a2_adj > c10;
    let r5 = a2_adj > c5;
    let r1 = a2_adj > c1;
    let label = if !r10 {
        "NORMAL"
    } else if !r5 {
        "MILD_DEVIATION"
    } else if !r1 {
        "MODERATE_DEVIATION"
    } else {
        "STRONG_NON_NORMAL"
    };
    AdtestSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        ad_statistic: a2,
        ad_adjusted: a2_adj,
        p_value_approx: p_value,
        critical_10pct: c10,
        critical_5pct: c5,
        critical_1pct: c1,
        reject_10pct: r10,
        reject_5pct: r5,
        reject_1pct: r1,
        adtest_label: label.into(),
        note: String::new(),
    }
}

/// LMOM compute: Hosking 1990 L-moments (unbiased PWM estimators).
pub fn compute_lmom_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> LmomSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return LmomSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            lmom_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let nf = n as f64;
    let mut x: Vec<f64> = log_rets.clone();
    x.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    // Unbiased PWMs: b_r = (1/n) Σ_{i=1..n} C(i-1,r)/C(n-1,r) · x_(i)
    let mut b0 = 0.0_f64;
    let mut b1 = 0.0_f64;
    let mut b2 = 0.0_f64;
    let mut b3 = 0.0_f64;
    for (k, &xi) in x.iter().enumerate() {
        let i = k as f64 + 1.0;
        b0 += xi;
        if n >= 2 {
            b1 += (i - 1.0) / (nf - 1.0) * xi;
        }
        if n >= 3 {
            b2 += (i - 1.0) * (i - 2.0) / ((nf - 1.0) * (nf - 2.0)) * xi;
        }
        if n >= 4 {
            b3 += (i - 1.0) * (i - 2.0) * (i - 3.0) / ((nf - 1.0) * (nf - 2.0) * (nf - 3.0)) * xi;
        }
    }
    b0 /= nf;
    b1 /= nf;
    b2 /= nf;
    b3 /= nf;
    let l1 = b0;
    let l2 = 2.0 * b1 - b0;
    let l3 = 6.0 * b2 - 6.0 * b1 + b0;
    let l4 = 20.0 * b3 - 30.0 * b2 + 12.0 * b1 - b0;
    if l2.abs() < f64::EPSILON {
        return LmomSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            lmom_label: "INSUFFICIENT_DATA".into(),
            note: "zero L-scale".into(),
            ..Default::default()
        };
    }
    let tau3 = l3 / l2;
    let tau4 = l4 / l2;
    let label = if tau3 < -0.30 {
        "HEAVY_LEFT"
    } else if tau3 > 0.30 {
        "HEAVY_RIGHT"
    } else if tau4 > 0.30 {
        "HEAVY_TAILS"
    } else if tau4 < 0.05 {
        "LIGHT_TAILS"
    } else {
        "NEAR_SYMMETRIC"
    };
    LmomSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        l1_mean: l1,
        l2_scale: l2,
        l3,
        l4,
        tau3_skew: tau3,
        tau4_kurt: tau4,
        lmom_label: label.into(),
        note: String::new(),
    }
}

/// KYLELAM compute: Kyle's daily price-impact λ (|Δp| on V regression).
pub fn compute_kylelam_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> KylelamSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let window: Vec<&HistoricalPriceRow> = sorted.iter().rev().take(253).rev().copied().collect();
    if window.len() < 30 {
        return KylelamSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            kylelam_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 bars, got {}", window.len()),
            ..Default::default()
        };
    }
    let mut abs_dp: Vec<f64> = Vec::with_capacity(window.len());
    let mut vol: Vec<f64> = Vec::with_capacity(window.len());
    for w in window.windows(2) {
        let dp = (w[1].close - w[0].close).abs();
        let v = w[1].volume;
        if v > 0.0 {
            abs_dp.push(dp);
            vol.push(v);
        }
    }
    let n = abs_dp.len();
    if n < 30 {
        return KylelamSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            kylelam_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 valid pairs, got {}", n),
            ..Default::default()
        };
    }
    let nf = n as f64;
    let mean_dp = abs_dp.iter().sum::<f64>() / nf;
    let mean_v = vol.iter().sum::<f64>() / nf;
    let mut cov = 0.0_f64;
    let mut var_v = 0.0_f64;
    let mut var_dp = 0.0_f64;
    for i in 0..n {
        let ddp = abs_dp[i] - mean_dp;
        let dv = vol[i] - mean_v;
        cov += ddp * dv;
        var_v += dv * dv;
        var_dp += ddp * ddp;
    }
    cov /= nf;
    var_v /= nf;
    var_dp /= nf;
    if var_v < f64::EPSILON || var_dp < f64::EPSILON {
        return KylelamSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            kylelam_label: "INSUFFICIENT_DATA".into(),
            note: "zero variance".into(),
            ..Default::default()
        };
    }
    let lambda = cov / var_v;
    let corr = cov / (var_dp.sqrt() * var_v.sqrt());
    let r2 = corr * corr;
    let label = if r2 < 0.02 {
        "NO_SIGNAL"
    } else if lambda.abs() < 1e-8 {
        "LOW_IMPACT"
    } else if r2 > 0.20 {
        "HIGH_IMPACT"
    } else if r2 > 0.05 {
        "MODERATE_IMPACT"
    } else {
        "LOW_IMPACT"
    };
    KylelamSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        kyle_lambda: lambda,
        mean_abs_dp: mean_dp,
        mean_volume: mean_v,
        correlation: corr,
        r_squared: r2,
        kylelam_label: label.into(),
        note: String::new(),
    }
}

/// PEAKOVER compute: Peaks-Over-Threshold (EVT/GPD foundation).
pub fn compute_peakover_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> PeakoverSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return PeakoverSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            peakover_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let mut abs_r: Vec<f64> = log_rets.iter().map(|r| r.abs()).collect();
    abs_r.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p95 = quantile_f64(&abs_r, 0.95);
    let p99 = quantile_f64(&abs_r, 0.99);
    if p95 < f64::EPSILON {
        return PeakoverSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            peakover_label: "INSUFFICIENT_DATA".into(),
            note: "zero P95".into(),
            ..Default::default()
        };
    }
    let mut count95 = 0usize;
    let mut count99 = 0usize;
    let mut sum95 = 0.0_f64;
    let mut sum99 = 0.0_f64;
    let mut max95 = 0.0_f64;
    let mut max99 = 0.0_f64;
    for &r in &abs_r {
        if r > p95 {
            count95 += 1;
            let ex = r - p95;
            sum95 += ex;
            if ex > max95 {
                max95 = ex;
            }
        }
        if r > p99 {
            count99 += 1;
            let ex = r - p99;
            sum99 += ex;
            if ex > max99 {
                max99 = ex;
            }
        }
    }
    let mean95 = if count95 > 0 {
        sum95 / count95 as f64
    } else {
        0.0
    };
    let mean99 = if count99 > 0 {
        sum99 / count99 as f64
    } else {
        0.0
    };
    // Label by mean-excess / threshold ratio at P95 (Pickands' GPD shape proxy).
    let ratio = if p95 > f64::EPSILON {
        mean95 / p95
    } else {
        0.0
    };
    let label = if ratio > 0.80 {
        "EXTREME_TAIL"
    } else if ratio > 0.40 {
        "HEAVY_TAIL"
    } else if ratio > 0.20 {
        "MODERATE_TAIL"
    } else {
        "LIGHT_TAIL"
    };
    PeakoverSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        threshold_p95: p95,
        threshold_p99: p99,
        count_p95: count95,
        count_p99: count99,
        mean_excess_p95: mean95,
        mean_excess_p99: mean99,
        max_excess_p95: max95,
        max_excess_p99: max99,
        peakover_label: label.into(),
        note: String::new(),
    }
}

// Higuchi, Pickands, kappa, Lyapunov, and rank-autocorrelation computes

/// HIGUCHI compute: Higuchi 1988 fractal dimension.
pub fn compute_higuchi_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> HiguchiSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 100 {
        return HiguchiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            higuchi_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥100 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    // cumulative sum so that "fluctuation" acts on a walk, per Higuchi convention
    let mut x = Vec::with_capacity(n);
    let mut s = 0.0f64;
    for &r in &log_rets {
        s += r;
        x.push(s);
    }
    let k_max = 10usize;
    let mut log_k: Vec<f64> = Vec::new();
    let mut log_l: Vec<f64> = Vec::new();
    for k in 1..=k_max {
        let mut lk_sum = 0.0f64;
        let mut count = 0usize;
        for m in 0..k {
            // indices m, m+k, m+2k, ...
            let max_i = (n - 1 - m) / k;
            if max_i < 1 {
                continue;
            }
            let mut l_m = 0.0f64;
            for i in 1..=max_i {
                l_m += (x[m + i * k] - x[m + (i - 1) * k]).abs();
            }
            let norm = ((n - 1) as f64) / ((max_i * k) as f64);
            l_m = l_m * norm / (k as f64);
            lk_sum += l_m;
            count += 1;
        }
        if count == 0 {
            continue;
        }
        let l_avg = lk_sum / count as f64;
        if l_avg > 0.0 {
            log_k.push((1.0 / k as f64).ln());
            log_l.push(l_avg.ln());
        }
    }
    if log_k.len() < 3 {
        return HiguchiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            higuchi_label: "INSUFFICIENT_DATA".into(),
            note: "insufficient log-k points".into(),
            ..Default::default()
        };
    }
    // Linear regression log_l = fd · log_k + c  (note: we want slope w.r.t. ln(1/k))
    let m = log_k.len() as f64;
    let mx: f64 = log_k.iter().sum::<f64>() / m;
    let my: f64 = log_l.iter().sum::<f64>() / m;
    let mut sxx = 0.0f64;
    let mut sxy = 0.0f64;
    let mut syy = 0.0f64;
    for i in 0..log_k.len() {
        let dx = log_k[i] - mx;
        let dy = log_l[i] - my;
        sxx += dx * dx;
        sxy += dx * dy;
        syy += dy * dy;
    }
    if sxx < f64::EPSILON {
        return HiguchiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            higuchi_label: "INSUFFICIENT_DATA".into(),
            note: "no variation in log k".into(),
            ..Default::default()
        };
    }
    let fd = sxy / sxx; // slope
    let r2 = if syy > f64::EPSILON {
        (sxy * sxy) / (sxx * syy)
    } else {
        0.0
    };
    let label = if fd < 1.1 {
        "SMOOTH"
    } else if fd < 1.4 {
        "PERSISTENT"
    } else if fd < 1.6 {
        "RANDOM"
    } else {
        "ROUGH"
    };
    HiguchiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        k_max,
        fractal_dim: fd,
        r_squared: r2,
        log_k_count: log_k.len(),
        higuchi_label: label.into(),
        note: String::new(),
    }
}

/// PICKANDS compute: Pickands 1975 tail-index estimator.
pub fn compute_pickands_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> PickandsSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 80 {
        return PickandsSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            pickands_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥80 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let mut abs_r: Vec<f64> = log_rets.iter().map(|r| r.abs()).collect();
    // sort descending so index i=0 is the largest
    abs_r.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    // Pickands requires at least 4k+1 samples. Use k = n/16 ⇒ 4k < n.
    let k = (n / 16).max(5);
    if 4 * k >= n {
        return PickandsSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            pickands_label: "INSUFFICIENT_DATA".into(),
            note: format!("4k={} ≥ n={}", 4 * k, n),
            ..Default::default()
        };
    }
    let x_k = abs_r[k - 1];
    let x_2k = abs_r[2 * k - 1];
    let x_4k = abs_r[4 * k - 1];
    let num = x_k - x_2k;
    let den = x_2k - x_4k;
    if den.abs() < f64::EPSILON || num.abs() < f64::EPSILON {
        return PickandsSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            pickands_label: "INSUFFICIENT_DATA".into(),
            note: "degenerate order-stat differences".into(),
            ..Default::default()
        };
    }
    let ratio = num / den;
    if ratio <= 0.0 {
        return PickandsSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            pickands_label: "INSUFFICIENT_DATA".into(),
            note: format!("ratio {} ≤ 0", ratio),
            ..Default::default()
        };
    }
    let gamma_hat = ratio.ln() / std::f64::consts::LN_2;
    let tail_index = if gamma_hat.abs() < f64::EPSILON {
        f64::INFINITY
    } else {
        1.0 / gamma_hat
    };
    let label = if gamma_hat > 0.5 {
        "FRECHET_HEAVY"
    } else if gamma_hat > 0.1 {
        "FRECHET_MODERATE"
    } else if gamma_hat > -0.1 {
        "GUMBEL_EXPONENTIAL"
    } else {
        "WEIBULL_BOUNDED"
    };
    PickandsSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        k_index: k,
        gamma_hat,
        tail_index,
        x_k,
        x_2k,
        x_4k,
        pickands_label: label.into(),
        note: String::new(),
    }
}

/// KAPPA3 compute: Kaplan-Knowles 2004 Kappa-3 ratio.
pub fn compute_kappa3_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> Kappa3Snapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return Kappa3Snapshot {
            symbol: sym,
            as_of: as_of.into(),
            kappa3_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let nf = n as f64;
    let mar = 0.0_f64;
    let mean = log_rets.iter().sum::<f64>() / nf;
    // Annualise with ×252 for excess-mean and ×√252 for lpm roots
    let excess_mean_ann = (mean - mar) * 252.0;
    let mut lpm2 = 0.0f64;
    let mut lpm3 = 0.0f64;
    for &r in &log_rets {
        let d = (mar - r).max(0.0);
        lpm2 += d * d;
        lpm3 += d * d * d;
    }
    lpm2 /= nf;
    lpm3 /= nf;
    if lpm2 < f64::EPSILON || lpm3 < f64::EPSILON {
        return Kappa3Snapshot {
            symbol: sym,
            as_of: as_of.into(),
            kappa3_label: "INSUFFICIENT_DATA".into(),
            note: "zero lower partial moment".into(),
            ..Default::default()
        };
    }
    let lpm3_root = lpm3.powf(1.0 / 3.0);
    // Annualise the downside risk: ×252^(1/3) for cube-root LPM, ×√252 for squared LPM
    let lpm3_root_ann = lpm3_root * (252.0_f64).powf(1.0 / 3.0);
    let lpm2_root_ann = lpm2.sqrt() * (252.0_f64).sqrt();
    let kappa3 = excess_mean_ann / lpm3_root_ann;
    let sortino = excess_mean_ann / lpm2_root_ann;
    let label = if kappa3 > 1.0 {
        "STRONG"
    } else if kappa3 > 0.0 {
        "POSITIVE"
    } else if kappa3 > -0.5 {
        "NEUTRAL"
    } else {
        "NEGATIVE"
    };
    Kappa3Snapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        mar,
        excess_mean: excess_mean_ann,
        lpm3,
        lpm3_root: lpm3_root_ann,
        kappa3,
        sortino_compare: sortino,
        kappa3_label: label.into(),
        note: String::new(),
    }
}

/// LYAPUNOV compute: Rosenstein et al. 1993 largest Lyapunov exponent.
pub fn compute_lyapunov_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> LyapunovSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 100 {
        return LyapunovSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            lyapunov_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥100 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let m = 3usize;
    let tau = 1usize;
    let n_vec = n - (m - 1) * tau;
    if n_vec < 30 {
        return LyapunovSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            lyapunov_label: "INSUFFICIENT_DATA".into(),
            note: "too few embedding vectors".into(),
            ..Default::default()
        };
    }
    // Build embedded vectors
    let mut vecs: Vec<[f64; 3]> = Vec::with_capacity(n_vec);
    for i in 0..n_vec {
        vecs.push([log_rets[i], log_rets[i + tau], log_rets[i + 2 * tau]]);
    }
    // For each reference point, find nearest neighbour (excluding Theiler window)
    let theiler = 10usize;
    let max_steps = 20usize;
    let mut log_d_sum = vec![0.0f64; max_steps];
    let mut log_d_cnt = vec![0usize; max_steps];
    for i in 0..vecs.len() {
        let mut best_j: Option<usize> = None;
        let mut best_d = f64::INFINITY;
        for j in 0..vecs.len() {
            if (j as i64 - i as i64).unsigned_abs() as usize <= theiler {
                continue;
            }
            let dx = vecs[i][0] - vecs[j][0];
            let dy = vecs[i][1] - vecs[j][1];
            let dz = vecs[i][2] - vecs[j][2];
            let d2 = dx * dx + dy * dy + dz * dz;
            if d2 < best_d {
                best_d = d2;
                best_j = Some(j);
            }
        }
        if let Some(j) = best_j {
            if best_d <= f64::EPSILON {
                continue;
            }
            for step in 0..max_steps {
                let ii = i + step;
                let jj = j + step;
                if ii >= vecs.len() || jj >= vecs.len() {
                    break;
                }
                let dx = vecs[ii][0] - vecs[jj][0];
                let dy = vecs[ii][1] - vecs[jj][1];
                let dz = vecs[ii][2] - vecs[jj][2];
                let d = (dx * dx + dy * dy + dz * dz).sqrt();
                if d > f64::EPSILON {
                    log_d_sum[step] += d.ln();
                    log_d_cnt[step] += 1;
                }
            }
        }
    }
    let mut xs: Vec<f64> = Vec::new();
    let mut ys: Vec<f64> = Vec::new();
    for step in 0..max_steps {
        if log_d_cnt[step] > 5 {
            xs.push(step as f64);
            ys.push(log_d_sum[step] / log_d_cnt[step] as f64);
        }
    }
    if xs.len() < 5 {
        return LyapunovSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            lyapunov_label: "INSUFFICIENT_DATA".into(),
            note: "too few regression points".into(),
            ..Default::default()
        };
    }
    let mlen = xs.len() as f64;
    let mx: f64 = xs.iter().sum::<f64>() / mlen;
    let my: f64 = ys.iter().sum::<f64>() / mlen;
    let mut sxx = 0.0f64;
    let mut sxy = 0.0f64;
    let mut syy = 0.0f64;
    for i in 0..xs.len() {
        let dx = xs[i] - mx;
        let dy = ys[i] - my;
        sxx += dx * dx;
        sxy += dx * dy;
        syy += dy * dy;
    }
    if sxx < f64::EPSILON {
        return LyapunovSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            lyapunov_label: "INSUFFICIENT_DATA".into(),
            note: "degenerate regression".into(),
            ..Default::default()
        };
    }
    let lambda = sxy / sxx;
    let r2 = if syy > f64::EPSILON {
        (sxy * sxy) / (sxx * syy)
    } else {
        0.0
    };
    let label = if lambda > 0.10 {
        "CHAOTIC"
    } else if lambda > 0.02 {
        "WEAKLY_CHAOTIC"
    } else if lambda > -0.02 {
        "PERIODIC"
    } else {
        "STABLE"
    };
    LyapunovSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        embed_dim: m,
        time_delay: tau,
        lambda_max: lambda,
        r_squared: r2,
        steps_used: xs.len(),
        lyapunov_label: label.into(),
        note: String::new(),
    }
}

/// RANKAC compute: Spearman rank autocorrelation at lags 1, 5, 10.
pub fn compute_rankac_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> RankacSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return RankacSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            rankac_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    // Compute ranks (average rank for ties, Spearman-style)
    let mut indexed: Vec<(usize, f64)> =
        log_rets.iter().enumerate().map(|(i, &v)| (i, v)).collect();
    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut ranks = vec![0.0f64; n];
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j + 1 < n && (indexed[j + 1].1 - indexed[i].1).abs() < f64::EPSILON {
            j += 1;
        }
        let avg_rank = ((i + j) as f64) / 2.0 + 1.0;
        for k in i..=j {
            ranks[indexed[k].0] = avg_rank;
        }
        i = j + 1;
    }
    let compute_rho = |lag: usize| -> f64 {
        if lag >= n {
            return 0.0;
        }
        let m = n - lag;
        let mf = m as f64;
        let mut mx = 0.0f64;
        let mut my = 0.0f64;
        for i in 0..m {
            mx += ranks[i];
            my += ranks[i + lag];
        }
        mx /= mf;
        my /= mf;
        let mut sxx = 0.0f64;
        let mut syy = 0.0f64;
        let mut sxy = 0.0f64;
        for i in 0..m {
            let dx = ranks[i] - mx;
            let dy = ranks[i + lag] - my;
            sxx += dx * dx;
            syy += dy * dy;
            sxy += dx * dy;
        }
        if sxx < f64::EPSILON || syy < f64::EPSILON {
            0.0
        } else {
            sxy / (sxx.sqrt() * syy.sqrt())
        }
    };
    let r1 = compute_rho(1);
    let r5 = compute_rho(5);
    let r10 = compute_rho(10);
    let mean_abs = (r1.abs() + r5.abs() + r10.abs()) / 3.0;
    let max_abs = r1.abs().max(r5.abs()).max(r10.abs());
    let label = if max_abs > 0.30 {
        "STRONG_DEPENDENCE"
    } else if max_abs > 0.15 {
        "MODERATE_DEPENDENCE"
    } else if max_abs > 0.05 {
        "WEAK_DEPENDENCE"
    } else {
        "INDEPENDENT"
    };
    RankacSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        rho_lag1: r1,
        rho_lag5: r5,
        rho_lag10: r10,
        mean_abs_rho: mean_abs,
        max_abs_rho: max_abs,
        rankac_label: label.into(),
        note: String::new(),
    }
}

// Jump-test, Phillips-Perron, MF-DFA, Hill-KS, and trend-strength computes

/// BNSJUMP compute: Barndorff-Nielsen & Shephard 2006 jump-test Z-statistic.
pub fn compute_bnsjump_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> BnsjumpSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return BnsjumpSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bnsjump_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    // Realized variance RV = Σ r_i²
    let rv: f64 = log_rets.iter().map(|r| r * r).sum();
    // Bipower variation BV = (π/2) · Σ |r_i|·|r_{i-1}|
    let mu1_sq_inv = std::f64::consts::FRAC_PI_2; // 1/μ₁² for normal μ₁=√(2/π)
    let mut bv_sum = 0.0f64;
    for i in 1..n {
        bv_sum += log_rets[i - 1].abs() * log_rets[i].abs();
    }
    let bv = mu1_sq_inv * bv_sum;
    if rv < f64::EPSILON {
        return BnsjumpSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bnsjump_label: "INSUFFICIENT_DATA".into(),
            note: "zero realised variance".into(),
            ..Default::default()
        };
    }
    let jump_ratio = ((rv - bv) / rv).max(0.0);
    // Quarticity proxy for standardisation: (π²/4 + π − 5) · Σ r_i⁴
    let theta = (std::f64::consts::PI * std::f64::consts::PI) / 4.0 + std::f64::consts::PI - 5.0;
    let qv: f64 = log_rets.iter().map(|r| r.powi(4)).sum();
    let var_term = theta * qv;
    let z_stat = if var_term > f64::EPSILON {
        (rv - bv) / var_term.sqrt()
    } else {
        0.0
    };
    // Approx p-value using a rough normal CDF (Abramowitz-Stegun 26.2.17)
    fn norm_cdf(x: f64) -> f64 {
        let t = 1.0 / (1.0 + 0.2316419 * x.abs());
        let d = (-x * x / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt();
        let poly = (((1.330274429 * t - 1.821255978) * t + 1.781477937) * t - 0.356563782) * t
            + 0.319381530;
        let rhs = d * poly * t;
        if x >= 0.0 { 1.0 - rhs } else { rhs }
    }
    let p_value = (1.0 - norm_cdf(z_stat.abs())).max(0.0).min(1.0);
    let label = if z_stat > 3.09 {
        "STRONG_JUMP"
    } else if z_stat > 2.33 {
        "MODERATE_JUMP"
    } else if z_stat > 1.65 {
        "WEAK_JUMP"
    } else {
        "NO_JUMP"
    };
    BnsjumpSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        realized_variance: rv,
        bipower_variance: bv,
        jump_ratio,
        jump_z_stat: z_stat,
        p_value,
        bnsjump_label: label.into(),
        note: String::new(),
    }
}

/// PPROOT compute: Phillips-Perron 1988 nonparametric unit-root test.
pub fn compute_pproot_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> PprootSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 30 {
        return PprootSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            pproot_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 bars, got {}", bars.len()),
            ..Default::default()
        };
    }
    // Use log-price series (the level process that might contain a unit root).
    let prices: Vec<f64> = bars
        .iter()
        .filter_map(|b| {
            if b.close > 0.0 {
                Some(b.close.ln())
            } else {
                None
            }
        })
        .collect();
    if prices.len() < 30 {
        return PprootSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            pproot_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 positive closes, got {}", prices.len()),
            ..Default::default()
        };
    }
    let n = prices.len();
    // OLS: Δy_t = (ρ − 1)·y_{t-1} + u_t  — estimate ρ directly from y_t on y_{t-1}
    let mut sum_xy = 0.0f64;
    let mut sum_xx = 0.0f64;
    for t in 1..n {
        let yl = prices[t - 1];
        let yc = prices[t];
        sum_xy += yl * yc;
        sum_xx += yl * yl;
    }
    if sum_xx < f64::EPSILON {
        return PprootSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            pproot_label: "INSUFFICIENT_DATA".into(),
            note: "degenerate regressor".into(),
            ..Default::default()
        };
    }
    let rho_hat = sum_xy / sum_xx;
    let m = (n - 1) as f64;
    // Residuals û_t
    let mut resid: Vec<f64> = Vec::with_capacity(n - 1);
    let mut rss = 0.0f64;
    for t in 1..n {
        let u = prices[t] - rho_hat * prices[t - 1];
        rss += u * u;
        resid.push(u);
    }
    let sigma2 = rss / m;
    let se_rho = (sigma2 / sum_xx).sqrt().max(f64::EPSILON);
    let t_rho = (rho_hat - 1.0) / se_rho;
    // Long-run variance via Newey-West / Bartlett kernel, lag truncation q = floor(4·(n/100)^0.25)
    let q = ((4.0 * (m / 100.0).powf(0.25)).floor() as usize).max(1);
    let gamma0 = sigma2;
    let mut sigma2_lr = gamma0;
    for j in 1..=q {
        if j >= resid.len() {
            break;
        }
        let mut gamma_j = 0.0f64;
        for t in j..resid.len() {
            gamma_j += resid[t] * resid[t - j];
        }
        gamma_j /= m;
        let w = 1.0 - (j as f64) / ((q + 1) as f64);
        sigma2_lr += 2.0 * w * gamma_j;
    }
    let sigma2_lr = sigma2_lr.max(f64::EPSILON);
    // PP Z(ρ) and Z(t) corrections
    let z_rho = m * (rho_hat - 1.0) - 0.5 * m * m * (sigma2_lr - gamma0) / sum_xx;
    let z_t = (gamma0 / sigma2_lr).sqrt() * t_rho
        - 0.5 * (sigma2_lr - gamma0) * (m * se_rho / sigma2_lr.sqrt()) / sigma2_lr.sqrt();
    // Dickey-Fuller critical values for Z(t), no-trend case
    let label = if z_t < -3.43 {
        "STATIONARY_STRONG"
    } else if z_t < -2.86 {
        "STATIONARY_WEAK"
    } else if z_t < -2.57 {
        "BORDERLINE"
    } else {
        "UNIT_ROOT"
    };
    PprootSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        rho_hat,
        t_rho,
        z_rho,
        z_t,
        lag_truncation: q,
        pproot_label: label.into(),
        note: String::new(),
    }
}

/// MFDFA compute: Multifractal DFA at q ∈ {−2, 0, +2}.
pub fn compute_mfdfa_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MfdfaSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 120 {
        return MfdfaSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            mfdfa_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥120 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    // Cumulative-sum walk Y_k = Σ_{i≤k} (r_i − r̄)
    let rbar: f64 = log_rets.iter().sum::<f64>() / (n as f64);
    let mut y = Vec::with_capacity(n);
    let mut cum = 0.0f64;
    for &r in &log_rets {
        cum += r - rbar;
        y.push(cum);
    }
    // Scales: s = 8, 12, 16, 24, 32, 48, 64 (bounded by n/4)
    let scales: Vec<usize> = [8usize, 12, 16, 24, 32, 48, 64]
        .iter()
        .copied()
        .filter(|&s| s * 4 <= n)
        .collect();
    if scales.len() < 3 {
        return MfdfaSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            mfdfa_label: "INSUFFICIENT_DATA".into(),
            note: "too few viable scales".into(),
            ..Default::default()
        };
    }
    // For each scale, split walk into non-overlapping windows, fit linear detrend, compute F²(s,v).
    // Then aggregate: F_q(s) = { (1/N_s) Σ [F²(s,v)]^(q/2) }^(1/q)  (q ≠ 0)
    //                 F_0(s) = exp{ (1/(2 N_s)) Σ ln F²(s,v) }     (q = 0)
    // Fit h(q) as slope of ln F_q(s) vs ln s.
    let compute_hq = |q: f64| -> Option<f64> {
        let mut log_s = Vec::new();
        let mut log_f = Vec::new();
        for &s in &scales {
            let ns = n / s;
            if ns < 4 {
                continue;
            }
            let mut f2_vals = Vec::with_capacity(ns * 2);
            for direction in 0..2usize {
                for v in 0..ns {
                    let offset = if direction == 0 {
                        v * s
                    } else {
                        n - (v + 1) * s
                    };
                    // Linear detrend over y[offset..offset+s]
                    let sf = s as f64;
                    let mut sx = 0.0f64;
                    let mut sy = 0.0f64;
                    for k in 0..s {
                        sx += k as f64;
                        sy += y[offset + k];
                    }
                    let mx = sx / sf;
                    let my = sy / sf;
                    let mut sxx = 0.0f64;
                    let mut sxy = 0.0f64;
                    for k in 0..s {
                        let dx = (k as f64) - mx;
                        let dy = y[offset + k] - my;
                        sxx += dx * dx;
                        sxy += dx * dy;
                    }
                    let slope = if sxx > f64::EPSILON { sxy / sxx } else { 0.0 };
                    let intercept = my - slope * mx;
                    let mut ss = 0.0f64;
                    for k in 0..s {
                        let fitted = intercept + slope * (k as f64);
                        let d = y[offset + k] - fitted;
                        ss += d * d;
                    }
                    f2_vals.push((ss / sf).max(f64::EPSILON));
                }
            }
            if f2_vals.is_empty() {
                continue;
            }
            let nv = f2_vals.len() as f64;
            let fq = if q.abs() < f64::EPSILON {
                (f2_vals.iter().map(|v| v.ln()).sum::<f64>() / (2.0 * nv)).exp()
            } else {
                let m: f64 = f2_vals.iter().map(|v| v.powf(q / 2.0)).sum::<f64>() / nv;
                m.powf(1.0 / q)
            };
            if fq.is_finite() && fq > 0.0 {
                log_s.push((s as f64).ln());
                log_f.push(fq.ln());
            }
        }
        if log_s.len() < 3 {
            return None;
        }
        let ln = log_s.len() as f64;
        let mx = log_s.iter().sum::<f64>() / ln;
        let my = log_f.iter().sum::<f64>() / ln;
        let mut sxx = 0.0f64;
        let mut sxy = 0.0f64;
        for i in 0..log_s.len() {
            let dx = log_s[i] - mx;
            let dy = log_f[i] - my;
            sxx += dx * dx;
            sxy += dx * dy;
        }
        if sxx < f64::EPSILON {
            None
        } else {
            Some(sxy / sxx)
        }
    };
    let h_neg2 = compute_hq(-2.0);
    let h_zero = compute_hq(0.0);
    let h_pos2 = compute_hq(2.0);
    let (h_n, h_0, h_p) = match (h_neg2, h_zero, h_pos2) {
        (Some(a), Some(b), Some(c)) => (a, b, c),
        _ => {
            return MfdfaSnapshot {
                symbol: sym,
                as_of: as_of.into(),
                mfdfa_label: "INSUFFICIENT_DATA".into(),
                note: "h(q) regression failed".into(),
                ..Default::default()
            };
        }
    };
    let delta_h = h_n - h_p;
    let label = if delta_h > 0.30 {
        "STRONG_MULTIFRACTAL"
    } else if delta_h > 0.15 {
        "MODERATE_MULTIFRACTAL"
    } else if delta_h > 0.05 {
        "WEAK_MULTIFRACTAL"
    } else {
        "MONOFRACTAL"
    };
    MfdfaSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        h_q_neg2: h_n,
        h_q_zero: h_0,
        h_q_pos2: h_p,
        delta_h,
        scales_used: scales.len(),
        mfdfa_label: label.into(),
        note: String::new(),
    }
}

/// HILLKS compute: KS goodness-of-fit for Hill-tail Pareto.
pub fn compute_hillks_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> HillksSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 50 {
        return HillksSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            hillks_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥50 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    // Use absolute log-returns as tail sample (two-sided symmetric tail model).
    let mut abs_r: Vec<f64> = log_rets
        .iter()
        .map(|r| r.abs())
        .filter(|v| *v > f64::EPSILON)
        .collect();
    abs_r.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal)); // descending
    let k = (n as f64 * 0.10).floor() as usize;
    if k < 10 || k >= abs_r.len() {
        return HillksSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            hillks_label: "INSUFFICIENT_DATA".into(),
            note: "tail sample too small".into(),
            ..Default::default()
        };
    }
    // Hill estimator of α: 1/α̂ = (1/k) Σ_{i=1..k} ln(x_i / x_{k+1})
    let threshold = abs_r[k];
    if threshold < f64::EPSILON {
        return HillksSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            hillks_label: "INSUFFICIENT_DATA".into(),
            note: "zero threshold".into(),
            ..Default::default()
        };
    }
    let mut inv_alpha = 0.0f64;
    for i in 0..k {
        inv_alpha += (abs_r[i] / threshold).ln();
    }
    inv_alpha /= k as f64;
    if inv_alpha < f64::EPSILON {
        return HillksSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            hillks_label: "INSUFFICIENT_DATA".into(),
            note: "degenerate tail".into(),
            ..Default::default()
        };
    }
    let alpha = 1.0 / inv_alpha;
    // KS statistic between empirical CDF of (x_i / threshold) for i=1..k and Pareto(α) CDF F(y) = 1 − y^{−α}.
    // Sort tail sample x_1..x_k in ascending order.
    let mut tail: Vec<f64> = abs_r[..k].to_vec();
    tail.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut d_max = 0.0f64;
    for (i, &x) in tail.iter().enumerate() {
        let y = x / threshold;
        if y < 1.0 - f64::EPSILON {
            continue;
        }
        let f_model = 1.0 - y.powf(-alpha);
        let f_emp_lo = i as f64 / k as f64;
        let f_emp_hi = (i + 1) as f64 / k as f64;
        d_max = d_max
            .max((f_emp_lo - f_model).abs())
            .max((f_emp_hi - f_model).abs());
    }
    let ks_crit = 1.36 / (k as f64).sqrt();
    let label = if d_max < ks_crit * 0.50 {
        "GOOD_FIT"
    } else if d_max < ks_crit * 0.90 {
        "ACCEPTABLE_FIT"
    } else if d_max < ks_crit * 1.30 {
        "POOR_FIT"
    } else {
        "REJECT"
    };
    HillksSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        k_order: k,
        alpha_hat: alpha,
        ks_statistic: d_max,
        ks_critical_5pct: ks_crit,
        hillks_label: label.into(),
        note: String::new(),
    }
}

/// TSI compute: Blau 1991 True Strength Index.
pub fn compute_tsi_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> TsiSnapshot {
    let sym = symbol.to_uppercase();
    let closes: Vec<f64> = bars
        .iter()
        .filter_map(|b| if b.close > 0.0 { Some(b.close) } else { None })
        .collect();
    if closes.len() < 60 {
        return TsiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            tsi_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥60 closes, got {}", closes.len()),
            ..Default::default()
        };
    }
    let n = closes.len();
    let long_p = 25usize;
    let short_p = 13usize;
    let diffs: Vec<f64> = (1..n).map(|i| closes[i] - closes[i - 1]).collect();
    let abs_diffs: Vec<f64> = diffs.iter().map(|d| d.abs()).collect();
    // EMA helper: EMA(x, p) where α = 2/(p+1); seed with first value.
    fn ema_series(x: &[f64], p: usize) -> Vec<f64> {
        if x.is_empty() {
            return Vec::new();
        }
        let alpha = 2.0 / ((p + 1) as f64);
        let mut out = Vec::with_capacity(x.len());
        out.push(x[0]);
        for i in 1..x.len() {
            out.push(alpha * x[i] + (1.0 - alpha) * out[i - 1]);
        }
        out
    }
    let long_smooth_num = ema_series(&diffs, long_p);
    let double_num = ema_series(&long_smooth_num, short_p);
    let long_smooth_den = ema_series(&abs_diffs, long_p);
    let double_den = ema_series(&long_smooth_den, short_p);
    let last = diffs.len() - 1;
    let den = double_den[last];
    if den.abs() < f64::EPSILON {
        return TsiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            tsi_label: "INSUFFICIENT_DATA".into(),
            note: "flat tape".into(),
            ..Default::default()
        };
    }
    let tsi_series: Vec<f64> = double_num
        .iter()
        .zip(double_den.iter())
        .map(|(n, d)| {
            if d.abs() < f64::EPSILON {
                0.0
            } else {
                100.0 * n / d
            }
        })
        .collect();
    let signal_series = ema_series(&tsi_series, short_p);
    let tsi = tsi_series[last];
    let signal = signal_series[last];
    let diff = tsi - signal;
    let label = if tsi > 25.0 {
        "STRONG_BULL"
    } else if tsi > 0.0 {
        "BULL"
    } else if tsi > -25.0 {
        if tsi > -5.0 && tsi < 5.0 {
            "NEUTRAL"
        } else {
            "BEAR"
        }
    } else {
        "STRONG_BEAR"
    };
    TsiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        ema_long: long_p,
        ema_short: short_p,
        tsi_value: tsi,
        signal_value: signal,
        tsi_minus_signal: diff,
        tsi_label: label.into(),
        note: String::new(),
    }
}

// GARCH, SADF, correlation-dimension, spectral-skew, and automutual-information computes

/// GARCH11 compute: Bollerslev 1986 GARCH(1,1) fit via coordinate-descent MLE.
pub fn compute_garch11_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> Garch11Snapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 60 {
        return Garch11Snapshot {
            symbol: sym,
            as_of: as_of.into(),
            garch11_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥60 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let mean_r: f64 = log_rets.iter().sum::<f64>() / n as f64;
    let resid: Vec<f64> = log_rets.iter().map(|r| r - mean_r).collect();
    let sample_var: f64 = resid.iter().map(|r| r * r).sum::<f64>() / n as f64;
    if sample_var < f64::EPSILON {
        return Garch11Snapshot {
            symbol: sym,
            as_of: as_of.into(),
            garch11_label: "INSUFFICIENT_DATA".into(),
            note: "zero sample variance".into(),
            ..Default::default()
        };
    }
    // Evaluate the GARCH(1,1) Gaussian log-likelihood for a candidate (ω, α, β).
    let log_lik = |omega: f64, alpha: f64, beta: f64| -> f64 {
        if omega <= 0.0 || alpha < 0.0 || beta < 0.0 || alpha + beta >= 0.999 {
            return f64::NEG_INFINITY;
        }
        let mut sigma2 = sample_var;
        let mut ll = 0.0f64;
        for r in &resid {
            if sigma2 <= 0.0 {
                return f64::NEG_INFINITY;
            }
            ll += -0.5 * ((2.0 * std::f64::consts::PI * sigma2).ln() + (r * r) / sigma2);
            sigma2 = omega + alpha * r * r + beta * sigma2;
        }
        ll
    };
    // Coarse grid search over (α, β) with ω implied by the unconditional-variance constraint.
    let mut best = (0.05f64, 0.90f64, f64::NEG_INFINITY);
    let alphas: Vec<f64> = (1..=20).map(|i| i as f64 * 0.02).collect(); // 0.02..0.40
    let betas: Vec<f64> = (1..=95).map(|i| i as f64 * 0.01).collect(); // 0.01..0.95
    for &a in &alphas {
        for &b in &betas {
            if a + b >= 0.995 {
                continue;
            }
            let omega = sample_var * (1.0 - a - b);
            let ll = log_lik(omega, a, b);
            if ll.is_finite() && ll > best.2 {
                best = (a, b, ll);
            }
        }
    }
    if !best.2.is_finite() {
        return Garch11Snapshot {
            symbol: sym,
            as_of: as_of.into(),
            garch11_label: "INSUFFICIENT_DATA".into(),
            note: "grid search failed".into(),
            ..Default::default()
        };
    }
    let (alpha, beta, ll) = best;
    let omega = sample_var * (1.0 - alpha - beta);
    let persistence = alpha + beta;
    let unc_var = if persistence < 1.0 {
        omega / (1.0 - persistence)
    } else {
        f64::NAN
    };
    let half_life = if persistence < 1.0 && persistence > 0.0 {
        (0.5_f64.ln()) / persistence.ln()
    } else {
        f64::NAN
    };
    let label = if persistence > 0.98 {
        "NEAR_INTEGRATED"
    } else if persistence > 0.90 {
        "HIGH_PERSISTENCE"
    } else if persistence > 0.70 {
        "MODERATE_PERSISTENCE"
    } else {
        "LOW_PERSISTENCE"
    };
    Garch11Snapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        omega,
        alpha,
        beta,
        persistence,
        unconditional_var: unc_var,
        half_life_bars: half_life,
        log_likelihood: ll,
        garch11_label: label.into(),
        note: String::new(),
    }
}

/// SADF compute: Phillips-Wu-Yu 2011 Sup-ADF explosive-root test on log-prices.
pub fn compute_sadf_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> SadfSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 60 {
        return SadfSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            sadf_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥60 bars, got {}", bars.len()),
            ..Default::default()
        };
    }
    let prices: Vec<f64> = bars
        .iter()
        .filter_map(|b| {
            if b.close > 0.0 {
                Some(b.close.ln())
            } else {
                None
            }
        })
        .collect();
    let n = prices.len();
    if n < 60 {
        return SadfSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            sadf_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥60 positive closes, got {}", n),
            ..Default::default()
        };
    }
    // ADF-t on series prices[0..end]: regression Δy_t = β·y_{t-1} + c + ε, return t-stat for β.
    let adf_t = |series: &[f64]| -> Option<f64> {
        let m = series.len();
        if m < 10 {
            return None;
        }
        // Build Δy and regressors (y_{t-1}, constant)
        let mut sx1 = 0.0f64; // Σy_{t-1}
        let mut sx1x1 = 0.0f64;
        let mut sx1x2 = 0.0f64;
        let mut sx2x2 = 0.0f64;
        let mut sx1dy = 0.0f64;
        let mut sx2dy = 0.0f64;
        let mm = (m - 1) as f64;
        for i in 1..m {
            let x1 = series[i - 1];
            let x2 = 1.0;
            let dy = series[i] - series[i - 1];
            sx1 += x1;
            sx1x1 += x1 * x1;
            sx1x2 += x1 * x2;
            sx2x2 += x2 * x2;
            sx1dy += x1 * dy;
            sx2dy += x2 * dy;
        }
        let _ = sx1;
        // Normal equations for 2-var OLS (x1, x2)
        let det = sx1x1 * sx2x2 - sx1x2 * sx1x2;
        if det.abs() < 1e-12 {
            return None;
        }
        let beta = (sx2x2 * sx1dy - sx1x2 * sx2dy) / det;
        let cons = (-sx1x2 * sx1dy + sx1x1 * sx2dy) / det;
        // Residuals → σ² → SE(β)
        let mut rss = 0.0f64;
        for i in 1..m {
            let x1 = series[i - 1];
            let dy = series[i] - series[i - 1];
            let e = dy - beta * x1 - cons;
            rss += e * e;
        }
        let dfree = mm - 2.0;
        if dfree <= 0.0 {
            return None;
        }
        let sigma2 = rss / dfree;
        let var_beta = sigma2 * sx2x2 / det;
        if var_beta <= 0.0 {
            return None;
        }
        Some(beta / var_beta.sqrt())
    };
    let adf_full = adf_t(&prices).unwrap_or(0.0);
    // Sup-ADF: expand from r0 to n
    let r0 = ((0.01 + 1.8 / (n as f64).sqrt()) * n as f64).floor() as usize;
    let r0 = r0.max(20).min(n - 1);
    let mut sadf = f64::NEG_INFINITY;
    let mut sadf_end = r0;
    let mut end = r0;
    while end <= n {
        if let Some(t) = adf_t(&prices[..end]) {
            if t > sadf {
                sadf = t;
                sadf_end = end;
            }
        }
        end += 1;
    }
    if !sadf.is_finite() {
        return SadfSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            sadf_label: "INSUFFICIENT_DATA".into(),
            note: "SADF regression failed".into(),
            ..Default::default()
        };
    }
    // Conservative 95% critical via small lookup (interpolated). Values for the standard no-trend SADF from PWY 2011.
    let crit = match n {
        0..=100 => 1.35,
        101..=200 => 1.49,
        201..=400 => 1.57,
        _ => 1.63,
    };
    let reject = sadf > crit;
    let label = if sadf > crit + 0.5 {
        "EXPLOSIVE_CONFIRMED"
    } else if sadf > crit {
        "EXPLOSIVE_LIKELY"
    } else if sadf > crit - 0.3 {
        "BORDERLINE"
    } else {
        "STABLE"
    };
    SadfSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        min_window: r0,
        adf_full,
        sadf_stat: sadf,
        sadf_argmax_end: sadf_end,
        critical_95: crit,
        reject_null: reject,
        sadf_label: label.into(),
        note: String::new(),
    }
}

/// CORDIM compute: Grassberger-Procaccia 1983 correlation dimension D2.
pub fn compute_cordim_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CordimSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 60 {
        return CordimSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cordim_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥60 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let m = 3usize;
    let tau = 1usize;
    if n <= (m - 1) * tau + 2 {
        return CordimSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cordim_label: "INSUFFICIENT_DATA".into(),
            note: "too few embedded vectors".into(),
            ..Default::default()
        };
    }
    // Standardise so that radii are on a consistent scale
    let mean_r: f64 = log_rets.iter().sum::<f64>() / n as f64;
    let var_r: f64 = log_rets.iter().map(|r| (r - mean_r).powi(2)).sum::<f64>() / n as f64;
    let std_r = var_r.sqrt().max(f64::EPSILON);
    let z: Vec<f64> = log_rets.iter().map(|r| (r - mean_r) / std_r).collect();
    // Build embedded vectors
    let nv = n - (m - 1) * tau;
    let vecs: Vec<[f64; 3]> = (0..nv)
        .map(|i| [z[i], z[i + tau], z[i + 2 * tau]])
        .collect();
    // Choose log-spaced radii ε between 0.1 and 2.0 (standardised units), 10 points
    let log_radii: Vec<f64> = (0..10)
        .map(|i| -1.0 + (i as f64) * (0.3010 / 10.0 * 10.0))
        .collect(); // log10 spacing 0.0..0.3 would be too narrow
    // Simpler: ε in {0.10, 0.14, 0.20, 0.28, 0.40, 0.56, 0.79, 1.12, 1.58, 2.24} (geometric)
    let radii: Vec<f64> = (0..10)
        .map(|i| 0.10 * (10f64.powf(i as f64 / 10.0)))
        .collect(); // log10-spaced 0.1 → 10^(0.9) ≈ 0.79 ... actually goes to 1.0
    // Use that set.
    let _ = log_radii; // keep unused (prototype kept for docs)
    let nv_f = (nv * (nv - 1)) as f64; // denominator for C(ε): pairs (i<j)
    let mut log_eps: Vec<f64> = Vec::new();
    let mut log_c: Vec<f64> = Vec::new();
    for &eps in &radii {
        let eps2 = eps * eps;
        let mut count = 0usize;
        for i in 0..nv {
            for j in (i + 1)..nv {
                let a = &vecs[i];
                let b = &vecs[j];
                let dx = a[0] - b[0];
                let dy = a[1] - b[1];
                let dz = a[2] - b[2];
                if dx * dx + dy * dy + dz * dz <= eps2 {
                    count += 1;
                }
            }
        }
        if count == 0 {
            continue;
        }
        let c = 2.0 * count as f64 / nv_f;
        if c > 0.0 {
            log_eps.push(eps.ln());
            log_c.push(c.ln());
        }
    }
    if log_eps.len() < 3 {
        return CordimSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            embed_dim: m,
            cordim_label: "INSUFFICIENT_DATA".into(),
            note: "too few valid radii for fit".into(),
            ..Default::default()
        };
    }
    let ln = log_eps.len() as f64;
    let mx = log_eps.iter().sum::<f64>() / ln;
    let my = log_c.iter().sum::<f64>() / ln;
    let mut sxx = 0.0f64;
    let mut sxy = 0.0f64;
    let mut syy = 0.0f64;
    for i in 0..log_eps.len() {
        let dx = log_eps[i] - mx;
        let dy = log_c[i] - my;
        sxx += dx * dx;
        sxy += dx * dy;
        syy += dy * dy;
    }
    if sxx < f64::EPSILON {
        return CordimSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            embed_dim: m,
            cordim_label: "INSUFFICIENT_DATA".into(),
            note: "zero ε spread".into(),
            ..Default::default()
        };
    }
    let d2 = sxy / sxx;
    let r2 = if syy > f64::EPSILON {
        (sxy * sxy) / (sxx * syy)
    } else {
        0.0
    };
    let label = if d2 < 1.5 {
        "LOW_DIM"
    } else if d2 < 2.5 {
        "MODERATE_DIM"
    } else if d2 < 3.0 {
        "HIGH_DIM"
    } else {
        "STOCHASTIC"
    };
    CordimSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        embed_dim: m,
        radii_count: log_eps.len(),
        d2,
        r_squared: r2,
        cordim_label: label.into(),
        note: String::new(),
    }
}

/// SKSPEC compute: Rolling-window skewness spectrum / stability.
pub fn compute_skspec_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> SkspecSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 60 {
        return SkspecSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            skspec_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥60 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let w = 30usize;
    if n < w * 2 {
        return SkspecSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            window_size: w,
            skspec_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥2 windows of returns".into(),
            ..Default::default()
        };
    }
    // Rolling skew on [i..i+w]
    let mut skews: Vec<f64> = Vec::with_capacity(n - w);
    for i in 0..=(n - w) {
        let slice = &log_rets[i..i + w];
        let wf = w as f64;
        let mean = slice.iter().sum::<f64>() / wf;
        let var = slice.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / wf;
        if var < f64::EPSILON {
            continue;
        }
        let s = var.sqrt();
        let skew = slice.iter().map(|r| ((r - mean) / s).powi(3)).sum::<f64>() / wf;
        if skew.is_finite() {
            skews.push(skew);
        }
    }
    if skews.len() < 5 {
        return SkspecSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            window_size: w,
            skspec_label: "INSUFFICIENT_DATA".into(),
            note: "too few valid rolling skew windows".into(),
            ..Default::default()
        };
    }
    let sk_n = skews.len() as f64;
    let mean_sk = skews.iter().sum::<f64>() / sk_n;
    let var_sk = skews.iter().map(|v| (v - mean_sk).powi(2)).sum::<f64>() / sk_n;
    let std_sk = var_sk.sqrt();
    let min_sk = skews.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_sk = skews.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range_sk = max_sk - min_sk;
    let label = if std_sk > 1.0 {
        "UNSTABLE"
    } else if mean_sk.abs() < 0.2 && std_sk > 0.5 {
        "DRIFTING"
    } else if mean_sk > 0.2 {
        "STABLE_POSITIVE"
    } else if mean_sk < -0.2 {
        "STABLE_NEGATIVE"
    } else {
        "DRIFTING"
    };
    SkspecSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        window_size: w,
        mean_skew: mean_sk,
        std_skew: std_sk,
        min_skew: min_sk,
        max_skew: max_sk,
        range_skew: range_sk,
        skspec_label: label.into(),
        note: String::new(),
    }
}

/// AUTOMI compute: Lag-1/5/10 auto-mutual-information (histogram-based).
pub fn compute_automi_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> AutomiSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 50 {
        return AutomiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            automi_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥50 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let k = 8usize;
    // Equi-probable bin edges via sorted-order quantiles
    let mut sorted = log_rets.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut edges: Vec<f64> = Vec::with_capacity(k + 1);
    edges.push(f64::NEG_INFINITY);
    for b in 1..k {
        let idx = ((b as f64 / k as f64) * n as f64).floor() as usize;
        let idx = idx.min(n - 1);
        edges.push(sorted[idx]);
    }
    edges.push(f64::INFINITY);
    let bin_of = |x: f64| -> usize {
        for b in 0..k {
            if x >= edges[b] && x < edges[b + 1] {
                return b;
            }
        }
        k - 1
    };
    let bins: Vec<usize> = log_rets.iter().map(|&r| bin_of(r)).collect();
    // Marginal entropy H(X)
    let mut marg = vec![0usize; k];
    for &b in &bins {
        marg[b] += 1;
    }
    let nf = n as f64;
    let ln2 = std::f64::consts::LN_2;
    let h_x: f64 = marg
        .iter()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = c as f64 / nf;
            -p * p.ln() / ln2
        })
        .sum();
    let compute_mi = |lag: usize| -> f64 {
        if lag >= n {
            return 0.0;
        }
        let m = n - lag;
        let mut joint = vec![vec![0usize; k]; k];
        let mut mx = vec![0usize; k];
        let mut my = vec![0usize; k];
        for i in 0..m {
            let a = bins[i];
            let b = bins[i + lag];
            joint[a][b] += 1;
            mx[a] += 1;
            my[b] += 1;
        }
        let mf = m as f64;
        let mut mi = 0.0f64;
        for a in 0..k {
            for b in 0..k {
                let pjoint = joint[a][b] as f64 / mf;
                if pjoint <= 0.0 {
                    continue;
                }
                let pa = mx[a] as f64 / mf;
                let pb = my[b] as f64 / mf;
                if pa <= 0.0 || pb <= 0.0 {
                    continue;
                }
                mi += pjoint * (pjoint / (pa * pb)).ln() / ln2;
            }
        }
        mi.max(0.0)
    };
    let mi1 = compute_mi(1);
    let mi5 = compute_mi(5);
    let mi10 = compute_mi(10);
    let norm_mi1 = if h_x > f64::EPSILON { mi1 / h_x } else { 0.0 };
    let label = if norm_mi1 > 0.25 {
        "STRONG"
    } else if norm_mi1 > 0.12 {
        "MODERATE"
    } else if norm_mi1 > 0.05 {
        "WEAK"
    } else {
        "INDEPENDENT"
    };
    AutomiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        num_bins: k,
        mi_lag1: mi1,
        mi_lag5: mi5,
        mi_lag10: mi10,
        h_marginal: h_x,
        normalized_mi1: norm_mi1,
        automi_label: label.into(),
        note: String::new(),
    }
}
