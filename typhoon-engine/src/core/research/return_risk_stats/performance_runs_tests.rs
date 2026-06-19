use super::*;

// Sterling, Kelly, Ljung-Box, runs-test, and zero-return computes

/// Standard normal CDF via Abramowitz & Stegun 7.1.26 rational erf approximation.
/// Max error ~1.5e-7 — plenty for label-granularity p-values.
pub(crate) fn std_normal_cdf(z: f64) -> f64 {
    let a1 = 0.254829592_f64;
    let a2 = -0.284496736_f64;
    let a3 = 1.421413741_f64;
    let a4 = -1.453152027_f64;
    let a5 = 1.061405429_f64;
    let p_c = 0.3275911_f64;
    let sign = if z < 0.0 { -1.0 } else { 1.0 };
    let x = z.abs() / (2.0_f64).sqrt();
    let t = 1.0 / (1.0 + p_c * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();
    0.5 * (1.0 + sign * y)
}

/// Chi-squared upper-tail P(χ²(df) ≥ q) via Wilson-Hilferty cube-root normal approximation.
/// Accurate to ~1% for df ≥ 3 — more than sufficient for label thresholds.
pub(crate) fn chi2_upper_tail(q: f64, df: usize) -> f64 {
    if df == 0 || q <= 0.0 {
        return 1.0;
    }
    let k = df as f64;
    let cube = (q / k).cbrt();
    let mean_term = 1.0 - 2.0 / (9.0 * k);
    let var_term = (2.0 / (9.0 * k)).sqrt();
    let z = (cube - mean_term) / var_term;
    1.0 - std_normal_cdf(z)
}

fn drawdown_events_from_window(window: &[&HistoricalPriceRow]) -> Vec<f64> {
    let first = window.first().map(|b| b.close).unwrap_or(0.0);
    let mut peak = first;
    let mut in_dd = false;
    let mut worst_in_ep = 0.0_f64;
    let mut events: Vec<f64> = Vec::new();
    for b in window.iter().skip(1) {
        let p = b.close;
        if p >= peak {
            if in_dd {
                if worst_in_ep > 0.0 {
                    events.push(worst_in_ep);
                }
                in_dd = false;
                worst_in_ep = 0.0;
            }
            peak = p;
        } else {
            in_dd = true;
            if peak > f64::EPSILON {
                let dd = (peak - p) / peak * 100.0;
                if dd > worst_in_ep {
                    worst_in_ep = dd;
                }
            }
        }
    }
    if in_dd && worst_in_ep > 0.0 {
        events.push(worst_in_ep);
    }
    events
}

pub fn compute_sterling_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> SterlingRatioSnapshot {
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
        return SterlingRatioSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            sterling_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 bars, got {}", window.len()),
            ..Default::default()
        };
    }
    let first = window.first().map(|b| b.close).unwrap_or(0.0);
    let last = window.last().map(|b| b.close).unwrap_or(0.0);
    if first < f64::EPSILON {
        return SterlingRatioSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            sterling_label: "INSUFFICIENT_DATA".into(),
            note: "zero starting price".into(),
            ..Default::default()
        };
    }
    let total_ret = (last / first - 1.0) * 100.0;
    let ann_ret = total_ret * (252.0 / window.len() as f64);
    let mut events = drawdown_events_from_window(&window);
    if events.is_empty() {
        let label = if ann_ret > 0.0 {
            "EXCELLENT"
        } else {
            "NEUTRAL"
        };
        return SterlingRatioSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: window.len(),
            annualized_return_pct: ann_ret,
            worst_n: 0,
            dd_event_count: 0,
            mean_worst_dd_pct: 0.0,
            sterling_ratio: 0.0,
            sterling_label: label.into(),
            note: "no drawdown events in window".into(),
        };
    }
    // Sort descending by magnitude (all positive %).
    events.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let worst_n = 5usize.min(events.len());
    let mean_worst: f64 = events.iter().take(worst_n).sum::<f64>() / worst_n as f64;
    let ratio = if mean_worst < f64::EPSILON {
        0.0
    } else {
        ann_ret / mean_worst
    };
    let label = if ratio < -0.5 {
        "VERY_POOR"
    } else if ratio < 0.0 {
        "POOR"
    } else if ratio < 0.5 {
        "NEUTRAL"
    } else if ratio < 1.5 {
        "GOOD"
    } else {
        "EXCELLENT"
    };
    SterlingRatioSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: window.len(),
        annualized_return_pct: ann_ret,
        worst_n,
        dd_event_count: events.len(),
        mean_worst_dd_pct: mean_worst,
        sterling_ratio: ratio,
        sterling_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_kellyf_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> KellyFractionSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return KellyFractionSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            kelly_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    // Convert log returns back to simple returns in % for Kelly inputs.
    let simple_pct: Vec<f64> = log_rets.iter().map(|r| (r.exp() - 1.0) * 100.0).collect();
    let mut wins: Vec<f64> = Vec::new();
    let mut losses: Vec<f64> = Vec::new();
    for r in &simple_pct {
        if *r > 0.0 {
            wins.push(*r);
        } else if *r < 0.0 {
            losses.push(-*r);
        }
    }
    if wins.is_empty() || losses.is_empty() {
        return KellyFractionSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            kelly_label: "INSUFFICIENT_DATA".into(),
            note: "need both win and loss days".into(),
            ..Default::default()
        };
    }
    let avg_win = wins.iter().sum::<f64>() / wins.len() as f64;
    let avg_loss = losses.iter().sum::<f64>() / losses.len() as f64;
    let n_dir = (wins.len() + losses.len()) as f64;
    let p = wins.len() as f64 / n_dir;
    let q = 1.0 - p;
    let b = if avg_loss < f64::EPSILON {
        0.0
    } else {
        avg_win / avg_loss
    };
    let kelly = if b < f64::EPSILON {
        0.0
    } else {
        (b * p - q) / b
    };
    let half = kelly / 2.0;
    let label = if kelly <= 0.0 {
        "SKIP"
    } else if kelly < 0.10 {
        "MARGINAL"
    } else if kelly < 0.25 {
        "MODERATE"
    } else if kelly < 0.50 {
        "AGGRESSIVE"
    } else {
        "ALL_IN"
    };
    KellyFractionSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: log_rets.len(),
        win_rate: p,
        loss_rate: q,
        avg_win_pct: avg_win,
        avg_loss_pct: avg_loss,
        win_loss_ratio: b,
        kelly_fraction: kelly,
        half_kelly: half,
        kelly_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_ljungb_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> LjungBoxSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    let h = 10usize;
    if log_rets.len() < 30 + h {
        return LjungBoxSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            lag_h: h,
            ljungb_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} returns, got {}", 30 + h, log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let nf = n as f64;
    let mean = log_rets.iter().sum::<f64>() / nf;
    let centered: Vec<f64> = log_rets.iter().map(|r| r - mean).collect();
    let var = centered.iter().map(|d| d * d).sum::<f64>() / nf;
    if var < f64::EPSILON {
        return LjungBoxSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            lag_h: h,
            ljungb_label: "INSUFFICIENT_DATA".into(),
            note: "zero variance".into(),
            ..Default::default()
        };
    }
    let mut q_stat = 0.0_f64;
    for k in 1..=h {
        let mut num = 0.0_f64;
        for i in k..n {
            num += centered[i] * centered[i - k];
        }
        let rho_k = num / (nf * var);
        let denom = nf - k as f64;
        if denom > 0.0 {
            q_stat += rho_k * rho_k / denom;
        }
    }
    q_stat *= nf * (nf + 2.0);
    let p_value = chi2_upper_tail(q_stat, h);
    let reject = p_value < 0.05;
    let label = if p_value >= 0.10 {
        "WHITE_NOISE"
    } else if p_value >= 0.05 {
        "WEAK_DEP"
    } else if p_value >= 0.01 {
        "MODERATE_DEP"
    } else {
        "STRONG_DEP"
    };
    LjungBoxSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        lag_h: h,
        q_statistic: q_stat,
        p_value,
        reject_white_noise: reject,
        ljungb_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_runstest_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> RunsTestSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return RunsTestSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            runs_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let signs: Vec<i8> = log_rets
        .iter()
        .filter_map(|r| {
            if *r > 0.0 {
                Some(1)
            } else if *r < 0.0 {
                Some(-1)
            } else {
                None
            }
        })
        .collect();
    let n = signs.len();
    if n < 20 {
        return RunsTestSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            runs_label: "INSUFFICIENT_DATA".into(),
            note: format!("fewer than 20 signed returns: {n}"),
            ..Default::default()
        };
    }
    let n1 = signs.iter().filter(|s| **s > 0).count();
    let n2 = n - n1;
    if n1 == 0 || n2 == 0 {
        return RunsTestSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            runs_label: "INSUFFICIENT_DATA".into(),
            note: "need both positive and negative signs".into(),
            ..Default::default()
        };
    }
    let mut runs = 1usize;
    for i in 1..n {
        if signs[i] != signs[i - 1] {
            runs += 1;
        }
    }
    let nf = n as f64;
    let n1f = n1 as f64;
    let n2f = n2 as f64;
    let expected = 2.0 * n1f * n2f / nf + 1.0;
    let variance = 2.0 * n1f * n2f * (2.0 * n1f * n2f - nf) / (nf * nf * (nf - 1.0));
    let std = variance.max(0.0).sqrt();
    let z = if std < f64::EPSILON {
        0.0
    } else {
        (runs as f64 - expected) / std
    };
    // Two-sided p-value
    let p_value = 2.0 * (1.0 - std_normal_cdf(z.abs()));
    let reject = p_value < 0.05;
    let label = if !reject {
        "RANDOM"
    } else if z > 0.0 {
        "ANTI_CLUST"
    } else if p_value >= 0.01 {
        "SLIGHT_CLUST"
    } else if p_value >= 0.001 {
        "MOD_CLUST"
    } else {
        "STRONG_CLUST"
    };
    RunsTestSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        positive_days: n1,
        negative_days: n2,
        runs_observed: runs,
        runs_expected: expected,
        runs_std: std,
        z_statistic: z,
        p_value,
        reject_randomness: reject,
        runs_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_zeroret_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> ZeroReturnSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return ZeroReturnSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            zero_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let epsilon = 1e-6_f64;
    let n = log_rets.len();
    let mut zero_count = 0usize;
    let mut longest = 0usize;
    let mut current = 0usize;
    for r in &log_rets {
        if r.abs() < epsilon {
            zero_count += 1;
            current += 1;
            if current > longest {
                longest = current;
            }
        } else {
            current = 0;
        }
    }
    let pct = zero_count as f64 / n as f64 * 100.0;
    let label = if pct < 1.0 {
        "HIGHLY_LIQUID"
    } else if pct < 5.0 {
        "LIQUID"
    } else if pct < 15.0 {
        "MODERATE"
    } else if pct < 30.0 {
        "ILLIQUID"
    } else {
        "VERY_ILLIQUID"
    };
    ZeroReturnSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        zero_day_count: zero_count,
        zero_day_pct: pct,
        longest_zero_streak: longest,
        epsilon,
        zero_label: label.into(),
        note: String::new(),
    }
}
