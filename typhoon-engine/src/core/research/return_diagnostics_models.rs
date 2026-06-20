use super::*;

// ── compute functions ──

/// DURBINWATSON compute: Durbin-Watson d statistic on log-returns.
pub fn compute_durbinwatson_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DurbinWatsonSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return DurbinWatsonSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            dw_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let mean: f64 = log_rets.iter().sum::<f64>() / n as f64;
    let mut sum_sq = 0.0f64;
    let mut sum_diff_sq = 0.0f64;
    for i in 0..n {
        let e = log_rets[i] - mean;
        sum_sq += e * e;
        if i > 0 {
            let prev = log_rets[i - 1] - mean;
            let d = e - prev;
            sum_diff_sq += d * d;
        }
    }
    if sum_sq <= f64::EPSILON {
        return DurbinWatsonSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            dw_label: "INSUFFICIENT_DATA".into(),
            note: "zero residual variance".into(),
            ..Default::default()
        };
    }
    let d = sum_diff_sq / sum_sq;
    let rho = 1.0 - d / 2.0;
    let label = if d < 1.0 {
        "STRONG_POS"
    } else if d < 1.5 {
        "WEAK_POS"
    } else if d <= 2.5 {
        "NO_AUTOCORR"
    } else if d <= 3.0 {
        "WEAK_NEG"
    } else {
        "STRONG_NEG"
    };
    DurbinWatsonSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        dw_stat: d,
        rho_estimate: rho,
        dw_label: label.into(),
        note: String::new(),
    }
}

/// BDSTEST compute: Brock-Dechert-Scheinkman iid test at embedding dim m=2.
/// Uses ε = epsilon_mult × sample_std; reports standardized statistic.
pub fn compute_bdstest_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> BdsTestSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    let n = log_rets.len();
    if n < 100 {
        return BdsTestSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bds_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥100 returns, got {}", n),
            ..Default::default()
        };
    }
    let m = 2usize;
    let eps_mult = 0.7f64;
    let mean: f64 = log_rets.iter().sum::<f64>() / n as f64;
    let var: f64 = log_rets.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / n as f64;
    let sigma = var.sqrt();
    if sigma <= f64::EPSILON {
        return BdsTestSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            bds_label: "INSUFFICIENT_DATA".into(),
            note: "zero sample std".into(),
            ..Default::default()
        };
    }
    let eps = eps_mult * sigma;
    // Correlation integrals C_1(ε) and C_m(ε) via O(n²) pair enumeration.
    let mut c1_pairs: usize = 0;
    let mut cm_pairs: usize = 0;
    let mut total_pairs: usize = 0;
    let upper = n.saturating_sub(m - 1);
    for i in 0..upper {
        for j in (i + 1)..upper {
            total_pairs += 1;
            let d1 = (log_rets[i] - log_rets[j]).abs();
            if d1 < eps {
                c1_pairs += 1;
                let d2 = (log_rets[i + 1] - log_rets[j + 1]).abs();
                if d2 < eps {
                    cm_pairs += 1;
                }
            }
        }
    }
    if total_pairs == 0 {
        return BdsTestSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            bds_label: "INSUFFICIENT_DATA".into(),
            note: "no pairs".into(),
            ..Default::default()
        };
    }
    let c1 = c1_pairs as f64 / total_pairs as f64;
    let cm = cm_pairs as f64 / total_pairs as f64;
    // Asymptotic variance approximation (Brock et al. 1996): V_m ≈ 4 × (c1^m(1−c1^m) + 2 Σ_k c1^{m−k}(K − c1²)) → use simplified:
    // Under H0, BDS statistic ≈ sqrt(n) × (C_m − C_1^m) / σ_m.
    // Use a practical approximation: σ_m² ≈ 4 × c1^{2m} × (1 − c1^{2m}) × m, which is a rough upper bound but keeps the
    // statistic interpretable. Clamp to avoid division by zero.
    let c1_2m = c1.powi((2 * m) as i32);
    let sigma_m = (4.0 * c1_2m * (1.0 - c1_2m).max(1e-9) * m as f64)
        .sqrt()
        .max(1e-9);
    let bds = (n as f64).sqrt() * (cm - c1.powi(m as i32)) / sigma_m;
    let p = 2.0 * (1.0 - std_normal_cdf(bds.abs()));
    let reject = p < 0.05;
    let label = if !reject {
        "IID_CONFIRMED"
    } else if bds.abs() < 4.0 {
        "WEAK_DEPENDENCE"
    } else {
        "STRONG_DEPENDENCE"
    };
    BdsTestSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        embed_dim: m,
        epsilon_mult: eps_mult,
        bds_stat: bds,
        p_value_two_sided: p,
        reject_null: reject,
        bds_label: label.into(),
        note: String::new(),
    }
}

/// BREUSCHPAGAN compute: Breusch-Pagan heteroskedasticity test with bar index as sole regressor.
pub fn compute_breuschpagan_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> BreuschPaganSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    let n = log_rets.len();
    if n < 30 {
        return BreuschPaganSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bp_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", n),
            ..Default::default()
        };
    }
    let mean_r: f64 = log_rets.iter().sum::<f64>() / n as f64;
    let sq_res: Vec<f64> = log_rets.iter().map(|&r| (r - mean_r).powi(2)).collect();
    let mean_sq: f64 = sq_res.iter().sum::<f64>() / n as f64;
    if mean_sq <= f64::EPSILON {
        return BreuschPaganSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            bp_label: "INSUFFICIENT_DATA".into(),
            note: "zero residual variance".into(),
            ..Default::default()
        };
    }
    // Regress sq_res on x = bar index. Auxiliary OLS: y_i = a + b·x_i + u_i
    let x_bar = (n as f64 - 1.0) / 2.0;
    let y_bar = mean_sq;
    let mut sxx = 0.0f64;
    let mut sxy = 0.0f64;
    let mut syy = 0.0f64;
    for i in 0..n {
        let xi = i as f64 - x_bar;
        let yi = sq_res[i] - y_bar;
        sxx += xi * xi;
        sxy += xi * yi;
        syy += yi * yi;
    }
    let r_sq = if sxx > f64::EPSILON && syy > f64::EPSILON {
        (sxy * sxy) / (sxx * syy)
    } else {
        0.0
    };
    let lm = n as f64 * r_sq;
    let df = 1usize;
    let critical_95 = 3.841; // χ²(1) 95%
    let reject = lm > critical_95;
    let label = if !reject {
        "HOMOSKEDASTIC"
    } else if lm < 10.83 {
        "MILD_HETERO"
    }
    // χ²(1) 99.9% ≈ 10.83
    else {
        "STRONG_HETERO"
    };
    BreuschPaganSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        lm_stat: lm,
        r_squared: r_sq,
        df,
        critical_95,
        reject_null: reject,
        bp_label: label.into(),
        note: String::new(),
    }
}

/// TURNPTS compute: Bartels turning-points test on log-returns.
pub fn compute_turnpts_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> TurnPtsSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    let n = log_rets.len();
    if n < 10 {
        return TurnPtsSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            turnpts_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥10 returns, got {}", n),
            ..Default::default()
        };
    }
    let mut observed = 0usize;
    for i in 1..(n - 1) {
        let a = log_rets[i - 1];
        let b = log_rets[i];
        let c = log_rets[i + 1];
        // strict local extremum — ties count as no turn
        if (b > a && b > c) || (b < a && b < c) {
            observed += 1;
        }
    }
    let nf = n as f64;
    let expected = 2.0 * (nf - 2.0) / 3.0;
    let variance = (16.0 * nf - 29.0) / 90.0;
    let z = if variance > f64::EPSILON {
        (observed as f64 - expected) / variance.sqrt()
    } else {
        0.0
    };
    let p = 2.0 * (1.0 - std_normal_cdf(z.abs()));
    let reject = p < 0.05;
    let label = if !reject {
        "RANDOM_IID"
    } else if z > 0.0 {
        "OVER_TURNING"
    } else {
        "UNDER_TURNING"
    };
    TurnPtsSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        observed_turnpts: observed,
        expected_turnpts: expected,
        variance_turnpts: variance,
        z_stat: z,
        p_value_two_sided: p,
        reject_null: reject,
        turnpts_label: label.into(),
        note: String::new(),
    }
}

/// PERIODOGRAM compute: direct-DFT peak cycle detection on mean-centered log-returns.
pub fn compute_periodogram_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> PeriodogramSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    let n = log_rets.len();
    if n < 64 {
        return PeriodogramSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            periodogram_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥64 returns, got {}", n),
            ..Default::default()
        };
    }
    let mean: f64 = log_rets.iter().sum::<f64>() / n as f64;
    let x: Vec<f64> = log_rets.iter().map(|&r| r - mean).collect();
    // Positive Fourier frequencies k = 1..n/2 (skip DC); direct DFT O(n × n/2).
    // Cap k-range to keep cost bounded for long histories.
    let max_k = (n / 2).min(256);
    let mut powers: Vec<(f64, f64)> = Vec::with_capacity(max_k); // (freq, power)
    let tau = 2.0 * std::f64::consts::PI;
    let mut total_power = 0.0f64;
    for k in 1..=max_k {
        let omega = tau * k as f64 / n as f64;
        let mut re = 0.0f64;
        let mut im = 0.0f64;
        for t in 0..n {
            let theta = omega * t as f64;
            re += x[t] * theta.cos();
            im += x[t] * theta.sin();
        }
        let p = (re * re + im * im) / n as f64;
        let f = k as f64 / n as f64;
        powers.push((f, p));
        total_power += p;
    }
    if total_power <= f64::EPSILON || powers.is_empty() {
        return PeriodogramSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            n_freqs: powers.len(),
            periodogram_label: "INSUFFICIENT_DATA".into(),
            note: "zero total power".into(),
            ..Default::default()
        };
    }
    let (peak_f, peak_p) =
        powers.iter().cloned().fold(
            (0.0_f64, 0.0_f64),
            |acc, (f, p)| if p > acc.1 { (f, p) } else { acc },
        );
    let ratio = peak_p / total_power;
    let period = if peak_f > 0.0 { 1.0 / peak_f } else { 0.0 };
    let label = if ratio > 0.25 {
        "STRONG_CYCLE"
    } else if ratio > 0.12 {
        "MODERATE_CYCLE"
    } else if ratio > 0.05 {
        "WEAK_CYCLE"
    } else {
        "NO_CYCLE"
    };
    PeriodogramSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        n_freqs: powers.len(),
        dominant_freq: peak_f,
        dominant_period_bars: period,
        dominant_power: peak_p,
        total_power,
        dominant_power_ratio: ratio,
        periodogram_label: label.into(),
        note: String::new(),
    }
}

// ── computes ──

/// MCLEODLI compute: McLeod-Li portmanteau on squared log-returns.
/// Q = n(n+2) Σ_k=1..h ρ̂²(k) / (n-k), compared against χ²(h).
pub fn compute_mcleodli_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> McLeodLiSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    let n = log_rets.len();
    if n < 30 {
        return McLeodLiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            mcleodli_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", n),
            ..Default::default()
        };
    }
    let mean_r: f64 = log_rets.iter().sum::<f64>() / n as f64;
    let sq: Vec<f64> = log_rets.iter().map(|&r| (r - mean_r).powi(2)).collect();
    let mean_sq: f64 = sq.iter().sum::<f64>() / n as f64;
    let var_sq: f64 = sq.iter().map(|&s| (s - mean_sq).powi(2)).sum::<f64>() / n as f64;
    if var_sq <= f64::EPSILON {
        return McLeodLiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            mcleodli_label: "INSUFFICIENT_DATA".into(),
            note: "zero variance of squared returns".into(),
            ..Default::default()
        };
    }
    let h = (10.min(n / 5)).max(5);
    let mut q = 0.0f64;
    for k in 1..=h {
        let mut num = 0.0f64;
        for t in k..n {
            num += (sq[t] - mean_sq) * (sq[t - k] - mean_sq);
        }
        let denom = n as f64 * var_sq;
        let rho_k = num / denom;
        q += rho_k * rho_k / (n as f64 - k as f64);
    }
    q *= n as f64 * (n as f64 + 2.0);
    let critical_95 = match h {
        1 => 3.841,
        2 => 5.991,
        3 => 7.815,
        4 => 9.488,
        5 => 11.07,
        6 => 12.592,
        7 => 14.067,
        8 => 15.507,
        9 => 16.919,
        10 => 18.307,
        _ => 18.307 + (h as f64 - 10.0) * 1.4,
    };
    let p = chi2_upper_tail(q, h);
    let reject = q > critical_95;
    let label = if !reject {
        "NO_ARCH"
    } else if q < 2.0 * critical_95 {
        "MILD_ARCH"
    } else {
        "STRONG_ARCH"
    };
    McLeodLiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        lag_h: h,
        q_stat: q,
        df: h,
        critical_95,
        p_value: p,
        reject_null: reject,
        mcleodli_label: label.into(),
        note: String::new(),
    }
}

/// OUFIT compute: Ornstein-Uhlenbeck fit via AR(1) on log-price.
/// x_{t+1} = a + b · x_t + ε  ⇒  θ = −ln(b), μ = a/(1−b), half-life = ln(2)/θ.
pub fn compute_oufit_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> OuFitSnapshot {
    let sym = symbol.to_uppercase();
    let (window, _) = trailing_log_returns(bars);
    let n = window.len();
    if n < 30 {
        return OuFitSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            oufit_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 bars, got {}", n),
            ..Default::default()
        };
    }
    let x: Vec<f64> = window
        .iter()
        .filter_map(|r| {
            if r.close > 0.0 {
                Some(r.close.ln())
            } else {
                None
            }
        })
        .collect();
    let m = x.len();
    if m < 30 {
        return OuFitSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            oufit_label: "INSUFFICIENT_DATA".into(),
            note: "need positive closes".into(),
            ..Default::default()
        };
    }
    // AR(1): y = x[1..], x_lag = x[0..m-1]
    let nn = m - 1;
    let mut sx = 0.0f64;
    let mut sy = 0.0f64;
    for i in 0..nn {
        sx += x[i];
        sy += x[i + 1];
    }
    let mx = sx / nn as f64;
    let my = sy / nn as f64;
    let mut sxx = 0.0f64;
    let mut sxy = 0.0f64;
    let mut syy = 0.0f64;
    for i in 0..nn {
        let dx = x[i] - mx;
        let dy = x[i + 1] - my;
        sxx += dx * dx;
        sxy += dx * dy;
        syy += dy * dy;
    }
    if sxx <= f64::EPSILON {
        return OuFitSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: m,
            oufit_label: "INSUFFICIENT_DATA".into(),
            note: "constant log-price".into(),
            ..Default::default()
        };
    }
    let b = sxy / sxx;
    let a = my - b * mx;
    let r_sq = if syy > f64::EPSILON {
        (sxy * sxy) / (sxx * syy)
    } else {
        0.0
    };
    let mut ss_res = 0.0f64;
    for i in 0..nn {
        let resid = x[i + 1] - (a + b * x[i]);
        ss_res += resid * resid;
    }
    let residual_sd = (ss_res / nn as f64).sqrt();
    // θ from discrete b: assumes unit time step
    let (theta, half_life) = if b > 0.0 && b < 1.0 {
        let th = -b.ln();
        (th, std::f64::consts::LN_2 / th)
    } else {
        (0.0, f64::INFINITY)
    };
    let mu = if (1.0 - b).abs() > f64::EPSILON {
        a / (1.0 - b)
    } else {
        mx
    };
    let sigma = residual_sd; // diffusion scale (per unit time)
    let label = if theta <= 0.0 {
        "TRENDING"
    } else {
        let nf = m as f64;
        if half_life > nf / 3.0 {
            "SLOW_REVERT"
        } else if half_life > nf / 10.0 {
            "MODERATE_REVERT"
        } else {
            "FAST_REVERT"
        }
    };
    OuFitSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: m,
        theta,
        mu,
        sigma,
        half_life_bars: half_life,
        residual_sd,
        r_squared: r_sq,
        oufit_label: label.into(),
        note: String::new(),
    }
}

/// GPH compute: Geweke-Porter-Hudak log-periodogram long-memory d estimator.
/// m = floor(n^0.5); regress ln I(λ_j) on −2 ln(2 sin(λ_j/2)) to get d.
pub fn compute_gph_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> GphSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    let n = log_rets.len();
    if n < 64 {
        return GphSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            gph_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥64 returns, got {}", n),
            ..Default::default()
        };
    }
    let mean: f64 = log_rets.iter().sum::<f64>() / n as f64;
    let x: Vec<f64> = log_rets.iter().map(|&r| r - mean).collect();
    let m = (n as f64).sqrt().floor() as usize;
    let m = m.max(4).min(n / 2);
    let tau = 2.0 * std::f64::consts::PI;
    // Compute periodogram at Fourier frequencies j = 1..=m
    let mut xs = Vec::with_capacity(m); // log |2 sin(λ/2)|
    let mut ys = Vec::with_capacity(m); // ln periodogram
    for j in 1..=m {
        let lam = tau * j as f64 / n as f64;
        let mut re = 0.0f64;
        let mut im = 0.0f64;
        for t in 0..n {
            let theta = lam * t as f64;
            re += x[t] * theta.cos();
            im += x[t] * theta.sin();
        }
        let p_j = (re * re + im * im) / (tau * n as f64);
        if p_j <= 0.0 {
            continue;
        }
        let denom = 2.0 * (lam / 2.0).sin();
        if denom.abs() <= f64::EPSILON {
            continue;
        }
        let reg = (denom.abs()).ln();
        xs.push(reg);
        ys.push(p_j.ln());
    }
    let k = xs.len();
    if k < 4 {
        return GphSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            m_freqs: k,
            gph_label: "INSUFFICIENT_DATA".into(),
            note: "too few usable frequencies".into(),
            ..Default::default()
        };
    }
    let mx: f64 = xs.iter().sum::<f64>() / k as f64;
    let my: f64 = ys.iter().sum::<f64>() / k as f64;
    let mut sxx = 0.0f64;
    let mut sxy = 0.0f64;
    for i in 0..k {
        let dx = xs[i] - mx;
        sxx += dx * dx;
        sxy += dx * (ys[i] - my);
    }
    if sxx <= f64::EPSILON {
        return GphSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            m_freqs: k,
            gph_label: "INSUFFICIENT_DATA".into(),
            note: "singular regression".into(),
            ..Default::default()
        };
    }
    let slope = sxy / sxx;
    // d = −slope / 2 in the log-periodogram GPH regression
    let d = -slope * 0.5;
    let stderr = (std::f64::consts::PI * std::f64::consts::PI / (24.0 * k as f64)).sqrt();
    let t = if stderr > f64::EPSILON {
        d / stderr
    } else {
        0.0
    };
    let p = 2.0 * (1.0 - std_normal_cdf(t.abs()));
    let label = if d >= 0.5 {
        "NONSTATIONARY"
    } else if d > 0.1 {
        "LONG_MEMORY"
    } else if d >= -0.1 {
        "SHORT_MEMORY"
    } else {
        "ANTIPERSISTENT"
    };
    GphSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        m_freqs: k,
        d_estimate: d,
        d_stderr: stderr,
        t_stat: t,
        p_value_two_sided: p,
        gph_label: label.into(),
        note: String::new(),
    }
}

/// BURGSPEC compute: Burg maximum-entropy AR spectral estimator.
/// AR coefficients via Burg recursion; AR spectrum evaluated on a freq grid.
pub fn compute_burgspec_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> BurgSpecSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    let n = log_rets.len();
    if n < 32 {
        return BurgSpecSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            burgspec_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥32 returns, got {}", n),
            ..Default::default()
        };
    }
    let mean: f64 = log_rets.iter().sum::<f64>() / n as f64;
    let x: Vec<f64> = log_rets.iter().map(|&r| r - mean).collect();
    let p = (n / 4).min(20).max(2);
    // Burg recursion (Marple 1987, §6.6).
    let mut f: Vec<f64> = x.clone();
    let mut b: Vec<f64> = x.clone();
    let mut a: Vec<f64> = vec![0.0; p + 1];
    a[0] = 1.0;
    let mut e: f64 = x.iter().map(|&v| v * v).sum::<f64>() / n as f64;
    if e <= f64::EPSILON {
        return BurgSpecSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            ar_order: p,
            burgspec_label: "INSUFFICIENT_DATA".into(),
            note: "zero variance".into(),
            ..Default::default()
        };
    }
    for m in 1..=p {
        let mut num = 0.0f64;
        let mut den = 0.0f64;
        for k in m..n {
            num += f[k] * b[k - 1];
            den += f[k] * f[k] + b[k - 1] * b[k - 1];
        }
        let k_m = if den.abs() > f64::EPSILON {
            -2.0 * num / den
        } else {
            0.0
        };
        // Update a[0..=m]
        let mut a_new = a.clone();
        for j in 1..m {
            a_new[j] = a[j] + k_m * a[m - j];
        }
        a_new[m] = k_m;
        a = a_new;
        // Update f, b
        let f_old = f.clone();
        for k in (m..n).rev() {
            f[k] = f_old[k] + k_m * b[k - 1];
            b[k] = b[k - 1] + k_m * f_old[k];
        }
        e *= 1.0 - k_m * k_m;
        if e <= f64::EPSILON {
            break;
        }
    }
    // Spectral grid
    let tau = 2.0 * std::f64::consts::PI;
    let grid_n = 256usize;
    let mut peak_f = 0.0f64;
    let mut peak_p = 0.0f64;
    let mut sum_p = 0.0f64;
    let mut count = 0usize;
    for g in 1..grid_n {
        let freq = g as f64 / (2.0 * grid_n as f64); // 0 .. 0.5 (Nyquist)
        let omega = tau * freq;
        let mut re = 1.0f64;
        let mut im = 0.0f64;
        for j in 1..=p {
            let theta = omega * j as f64;
            re += a[j] * theta.cos();
            im -= a[j] * theta.sin();
        }
        let denom = re * re + im * im;
        if denom <= f64::EPSILON {
            continue;
        }
        let psd = e / denom;
        if psd > peak_p {
            peak_p = psd;
            peak_f = freq;
        }
        sum_p += psd;
        count += 1;
    }
    if count == 0 || peak_p <= 0.0 {
        return BurgSpecSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            ar_order: p,
            burgspec_label: "INSUFFICIENT_DATA".into(),
            note: "degenerate AR spectrum".into(),
            ..Default::default()
        };
    }
    let mean_p = sum_p / count as f64;
    let ratio = if mean_p > f64::EPSILON {
        peak_p / mean_p
    } else {
        0.0
    };
    let period = if peak_f > 0.0 { 1.0 / peak_f } else { 0.0 };
    let label = if ratio > 8.0 {
        "STRONG_AR_CYCLE"
    } else if ratio > 4.0 {
        "MODERATE_AR_CYCLE"
    } else if ratio > 2.0 {
        "WEAK_AR_CYCLE"
    } else {
        "NO_AR_CYCLE"
    };
    BurgSpecSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        ar_order: p,
        dominant_freq: peak_f,
        dominant_period_bars: period,
        peak_power: peak_p,
        mean_power: mean_p,
        peak_to_mean_ratio: ratio,
        burgspec_label: label.into(),
        note: String::new(),
    }
}

/// KENDALLTAU compute: lag-1 Kendall's tau rank autocorrelation on log-returns.
/// Pairs (r_t, r_{t+1}); concordant if both coordinates move same direction.
pub fn compute_kendalltau_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> KendallTauSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    let n = log_rets.len();
    if n < 30 {
        return KendallTauSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            kendalltau_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", n),
            ..Default::default()
        };
    }
    // Build lag-1 pairs (a_i, b_i) = (r_i, r_{i+1}) for i=0..n-2.
    let m = n - 1;
    let a: Vec<f64> = log_rets[..m].to_vec();
    let b: Vec<f64> = log_rets[1..].to_vec();
    let mut c = 0usize;
    let mut d = 0usize;
    for i in 0..m {
        for j in (i + 1)..m {
            let da = a[j] - a[i];
            let db = b[j] - b[i];
            let s = da * db;
            if s > 0.0 {
                c += 1;
            } else if s < 0.0 {
                d += 1;
            }
        }
    }
    let total = m * (m - 1) / 2;
    if total == 0 {
        return KendallTauSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            kendalltau_label: "INSUFFICIENT_DATA".into(),
            note: "too few pairs".into(),
            ..Default::default()
        };
    }
    let tau = (c as f64 - d as f64) / total as f64;
    let mf = m as f64;
    let var_tau = 2.0 * (2.0 * mf + 5.0) / (9.0 * mf * (mf - 1.0));
    let z = if var_tau > f64::EPSILON {
        tau / var_tau.sqrt()
    } else {
        0.0
    };
    let p = 2.0 * (1.0 - std_normal_cdf(z.abs()));
    let label = if tau > 0.1 {
        "STRONG_POS"
    } else if tau > 0.03 {
        "WEAK_POS"
    } else if tau < -0.1 {
        "STRONG_NEG"
    } else if tau < -0.03 {
        "WEAK_NEG"
    } else {
        "NO_RANK_AUTO"
    };
    KendallTauSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pair_count: total,
        concordant: c,
        discordant: d,
        tau,
        z_stat: z,
        p_value_two_sided: p,
        kendalltau_label: label.into(),
        note: String::new(),
    }
}
