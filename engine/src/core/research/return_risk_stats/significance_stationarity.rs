use super::*;

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
