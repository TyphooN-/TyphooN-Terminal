use super::*;

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
