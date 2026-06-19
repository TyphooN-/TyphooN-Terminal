use super::*;

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
