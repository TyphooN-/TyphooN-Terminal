use super::*;

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
