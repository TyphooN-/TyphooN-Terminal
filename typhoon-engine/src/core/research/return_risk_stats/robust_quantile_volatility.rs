use super::*;

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
