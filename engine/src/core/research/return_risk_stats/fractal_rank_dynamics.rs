use super::*;

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
