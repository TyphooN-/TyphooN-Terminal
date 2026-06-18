use super::*;

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
