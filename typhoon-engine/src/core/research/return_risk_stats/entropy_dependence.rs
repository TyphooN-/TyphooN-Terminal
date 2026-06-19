use super::*;

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
