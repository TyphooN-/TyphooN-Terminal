use super::*;

// Jump-test, Phillips-Perron, MF-DFA, Hill-KS, and trend-strength computes

/// BNSJUMP compute: Barndorff-Nielsen & Shephard 2006 jump-test Z-statistic.
pub fn compute_bnsjump_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> BnsjumpSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return BnsjumpSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bnsjump_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    // Realized variance RV = Σ r_i²
    let rv: f64 = log_rets.iter().map(|r| r * r).sum();
    // Bipower variation BV = (π/2) · Σ |r_i|·|r_{i-1}|
    let mu1_sq_inv = std::f64::consts::FRAC_PI_2; // 1/μ₁² for normal μ₁=√(2/π)
    let mut bv_sum = 0.0f64;
    for i in 1..n {
        bv_sum += log_rets[i - 1].abs() * log_rets[i].abs();
    }
    let bv = mu1_sq_inv * bv_sum;
    if rv < f64::EPSILON {
        return BnsjumpSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bnsjump_label: "INSUFFICIENT_DATA".into(),
            note: "zero realised variance".into(),
            ..Default::default()
        };
    }
    let jump_ratio = ((rv - bv) / rv).max(0.0);
    // Quarticity proxy for standardisation: (π²/4 + π − 5) · Σ r_i⁴
    let theta = (std::f64::consts::PI * std::f64::consts::PI) / 4.0 + std::f64::consts::PI - 5.0;
    let qv: f64 = log_rets.iter().map(|r| r.powi(4)).sum();
    let var_term = theta * qv;
    let z_stat = if var_term > f64::EPSILON {
        (rv - bv) / var_term.sqrt()
    } else {
        0.0
    };
    // Approx p-value using a rough normal CDF (Abramowitz-Stegun 26.2.17)
    fn norm_cdf(x: f64) -> f64 {
        let t = 1.0 / (1.0 + 0.2316419 * x.abs());
        let d = (-x * x / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt();
        let poly = (((1.330274429 * t - 1.821255978) * t + 1.781477937) * t - 0.356563782) * t
            + 0.319381530;
        let rhs = d * poly * t;
        if x >= 0.0 { 1.0 - rhs } else { rhs }
    }
    let p_value = (1.0 - norm_cdf(z_stat.abs())).max(0.0).min(1.0);
    let label = if z_stat > 3.09 {
        "STRONG_JUMP"
    } else if z_stat > 2.33 {
        "MODERATE_JUMP"
    } else if z_stat > 1.65 {
        "WEAK_JUMP"
    } else {
        "NO_JUMP"
    };
    BnsjumpSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        realized_variance: rv,
        bipower_variance: bv,
        jump_ratio,
        jump_z_stat: z_stat,
        p_value,
        bnsjump_label: label.into(),
        note: String::new(),
    }
}

/// PPROOT compute: Phillips-Perron 1988 nonparametric unit-root test.
pub fn compute_pproot_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> PprootSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 30 {
        return PprootSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            pproot_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 bars, got {}", bars.len()),
            ..Default::default()
        };
    }
    // Use log-price series (the level process that might contain a unit root).
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
    if prices.len() < 30 {
        return PprootSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            pproot_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 positive closes, got {}", prices.len()),
            ..Default::default()
        };
    }
    let n = prices.len();
    // OLS: Δy_t = (ρ − 1)·y_{t-1} + u_t  — estimate ρ directly from y_t on y_{t-1}
    let mut sum_xy = 0.0f64;
    let mut sum_xx = 0.0f64;
    for t in 1..n {
        let yl = prices[t - 1];
        let yc = prices[t];
        sum_xy += yl * yc;
        sum_xx += yl * yl;
    }
    if sum_xx < f64::EPSILON {
        return PprootSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            pproot_label: "INSUFFICIENT_DATA".into(),
            note: "degenerate regressor".into(),
            ..Default::default()
        };
    }
    let rho_hat = sum_xy / sum_xx;
    let m = (n - 1) as f64;
    // Residuals û_t
    let mut resid: Vec<f64> = Vec::with_capacity(n - 1);
    let mut rss = 0.0f64;
    for t in 1..n {
        let u = prices[t] - rho_hat * prices[t - 1];
        rss += u * u;
        resid.push(u);
    }
    let sigma2 = rss / m;
    let se_rho = (sigma2 / sum_xx).sqrt().max(f64::EPSILON);
    let t_rho = (rho_hat - 1.0) / se_rho;
    // Long-run variance via Newey-West / Bartlett kernel, lag truncation q = floor(4·(n/100)^0.25)
    let q = ((4.0 * (m / 100.0).powf(0.25)).floor() as usize).max(1);
    let gamma0 = sigma2;
    let mut sigma2_lr = gamma0;
    for j in 1..=q {
        if j >= resid.len() {
            break;
        }
        let mut gamma_j = 0.0f64;
        for t in j..resid.len() {
            gamma_j += resid[t] * resid[t - j];
        }
        gamma_j /= m;
        let w = 1.0 - (j as f64) / ((q + 1) as f64);
        sigma2_lr += 2.0 * w * gamma_j;
    }
    let sigma2_lr = sigma2_lr.max(f64::EPSILON);
    // PP Z(ρ) and Z(t) corrections
    let z_rho = m * (rho_hat - 1.0) - 0.5 * m * m * (sigma2_lr - gamma0) / sum_xx;
    let z_t = (gamma0 / sigma2_lr).sqrt() * t_rho
        - 0.5 * (sigma2_lr - gamma0) * (m * se_rho / sigma2_lr.sqrt()) / sigma2_lr.sqrt();
    // Dickey-Fuller critical values for Z(t), no-trend case
    let label = if z_t < -3.43 {
        "STATIONARY_STRONG"
    } else if z_t < -2.86 {
        "STATIONARY_WEAK"
    } else if z_t < -2.57 {
        "BORDERLINE"
    } else {
        "UNIT_ROOT"
    };
    PprootSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        rho_hat,
        t_rho,
        z_rho,
        z_t,
        lag_truncation: q,
        pproot_label: label.into(),
        note: String::new(),
    }
}

/// MFDFA compute: Multifractal DFA at q ∈ {−2, 0, +2}.
pub fn compute_mfdfa_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MfdfaSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 120 {
        return MfdfaSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            mfdfa_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥120 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    // Cumulative-sum walk Y_k = Σ_{i≤k} (r_i − r̄)
    let rbar: f64 = log_rets.iter().sum::<f64>() / (n as f64);
    let mut y = Vec::with_capacity(n);
    let mut cum = 0.0f64;
    for &r in &log_rets {
        cum += r - rbar;
        y.push(cum);
    }
    // Scales: s = 8, 12, 16, 24, 32, 48, 64 (bounded by n/4)
    let scales: Vec<usize> = [8usize, 12, 16, 24, 32, 48, 64]
        .iter()
        .copied()
        .filter(|&s| s * 4 <= n)
        .collect();
    if scales.len() < 3 {
        return MfdfaSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            mfdfa_label: "INSUFFICIENT_DATA".into(),
            note: "too few viable scales".into(),
            ..Default::default()
        };
    }
    // For each scale, split walk into non-overlapping windows, fit linear detrend, compute F²(s,v).
    // Then aggregate: F_q(s) = { (1/N_s) Σ [F²(s,v)]^(q/2) }^(1/q)  (q ≠ 0)
    //                 F_0(s) = exp{ (1/(2 N_s)) Σ ln F²(s,v) }     (q = 0)
    // Fit h(q) as slope of ln F_q(s) vs ln s.
    let compute_hq = |q: f64| -> Option<f64> {
        let mut log_s = Vec::new();
        let mut log_f = Vec::new();
        for &s in &scales {
            let ns = n / s;
            if ns < 4 {
                continue;
            }
            let mut f2_vals = Vec::with_capacity(ns * 2);
            for direction in 0..2usize {
                for v in 0..ns {
                    let offset = if direction == 0 {
                        v * s
                    } else {
                        n - (v + 1) * s
                    };
                    // Linear detrend over y[offset..offset+s]
                    let sf = s as f64;
                    let mut sx = 0.0f64;
                    let mut sy = 0.0f64;
                    for k in 0..s {
                        sx += k as f64;
                        sy += y[offset + k];
                    }
                    let mx = sx / sf;
                    let my = sy / sf;
                    let mut sxx = 0.0f64;
                    let mut sxy = 0.0f64;
                    for k in 0..s {
                        let dx = (k as f64) - mx;
                        let dy = y[offset + k] - my;
                        sxx += dx * dx;
                        sxy += dx * dy;
                    }
                    let slope = if sxx > f64::EPSILON { sxy / sxx } else { 0.0 };
                    let intercept = my - slope * mx;
                    let mut ss = 0.0f64;
                    for k in 0..s {
                        let fitted = intercept + slope * (k as f64);
                        let d = y[offset + k] - fitted;
                        ss += d * d;
                    }
                    f2_vals.push((ss / sf).max(f64::EPSILON));
                }
            }
            if f2_vals.is_empty() {
                continue;
            }
            let nv = f2_vals.len() as f64;
            let fq = if q.abs() < f64::EPSILON {
                (f2_vals.iter().map(|v| v.ln()).sum::<f64>() / (2.0 * nv)).exp()
            } else {
                let m: f64 = f2_vals.iter().map(|v| v.powf(q / 2.0)).sum::<f64>() / nv;
                m.powf(1.0 / q)
            };
            if fq.is_finite() && fq > 0.0 {
                log_s.push((s as f64).ln());
                log_f.push(fq.ln());
            }
        }
        if log_s.len() < 3 {
            return None;
        }
        let ln = log_s.len() as f64;
        let mx = log_s.iter().sum::<f64>() / ln;
        let my = log_f.iter().sum::<f64>() / ln;
        let mut sxx = 0.0f64;
        let mut sxy = 0.0f64;
        for i in 0..log_s.len() {
            let dx = log_s[i] - mx;
            let dy = log_f[i] - my;
            sxx += dx * dx;
            sxy += dx * dy;
        }
        if sxx < f64::EPSILON {
            None
        } else {
            Some(sxy / sxx)
        }
    };
    let h_neg2 = compute_hq(-2.0);
    let h_zero = compute_hq(0.0);
    let h_pos2 = compute_hq(2.0);
    let (h_n, h_0, h_p) = match (h_neg2, h_zero, h_pos2) {
        (Some(a), Some(b), Some(c)) => (a, b, c),
        _ => {
            return MfdfaSnapshot {
                symbol: sym,
                as_of: as_of.into(),
                mfdfa_label: "INSUFFICIENT_DATA".into(),
                note: "h(q) regression failed".into(),
                ..Default::default()
            };
        }
    };
    let delta_h = h_n - h_p;
    let label = if delta_h > 0.30 {
        "STRONG_MULTIFRACTAL"
    } else if delta_h > 0.15 {
        "MODERATE_MULTIFRACTAL"
    } else if delta_h > 0.05 {
        "WEAK_MULTIFRACTAL"
    } else {
        "MONOFRACTAL"
    };
    MfdfaSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        h_q_neg2: h_n,
        h_q_zero: h_0,
        h_q_pos2: h_p,
        delta_h,
        scales_used: scales.len(),
        mfdfa_label: label.into(),
        note: String::new(),
    }
}

/// HILLKS compute: KS goodness-of-fit for Hill-tail Pareto.
pub fn compute_hillks_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> HillksSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 50 {
        return HillksSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            hillks_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥50 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    // Use absolute log-returns as tail sample (two-sided symmetric tail model).
    let mut abs_r: Vec<f64> = log_rets
        .iter()
        .map(|r| r.abs())
        .filter(|v| *v > f64::EPSILON)
        .collect();
    abs_r.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal)); // descending
    let k = (n as f64 * 0.10).floor() as usize;
    if k < 10 || k >= abs_r.len() {
        return HillksSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            hillks_label: "INSUFFICIENT_DATA".into(),
            note: "tail sample too small".into(),
            ..Default::default()
        };
    }
    // Hill estimator of α: 1/α̂ = (1/k) Σ_{i=1..k} ln(x_i / x_{k+1})
    let threshold = abs_r[k];
    if threshold < f64::EPSILON {
        return HillksSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            hillks_label: "INSUFFICIENT_DATA".into(),
            note: "zero threshold".into(),
            ..Default::default()
        };
    }
    let mut inv_alpha = 0.0f64;
    for i in 0..k {
        inv_alpha += (abs_r[i] / threshold).ln();
    }
    inv_alpha /= k as f64;
    if inv_alpha < f64::EPSILON {
        return HillksSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            hillks_label: "INSUFFICIENT_DATA".into(),
            note: "degenerate tail".into(),
            ..Default::default()
        };
    }
    let alpha = 1.0 / inv_alpha;
    // KS statistic between empirical CDF of (x_i / threshold) for i=1..k and Pareto(α) CDF F(y) = 1 − y^{−α}.
    // Sort tail sample x_1..x_k in ascending order.
    let mut tail: Vec<f64> = abs_r[..k].to_vec();
    tail.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut d_max = 0.0f64;
    for (i, &x) in tail.iter().enumerate() {
        let y = x / threshold;
        if y < 1.0 - f64::EPSILON {
            continue;
        }
        let f_model = 1.0 - y.powf(-alpha);
        let f_emp_lo = i as f64 / k as f64;
        let f_emp_hi = (i + 1) as f64 / k as f64;
        d_max = d_max
            .max((f_emp_lo - f_model).abs())
            .max((f_emp_hi - f_model).abs());
    }
    let ks_crit = 1.36 / (k as f64).sqrt();
    let label = if d_max < ks_crit * 0.50 {
        "GOOD_FIT"
    } else if d_max < ks_crit * 0.90 {
        "ACCEPTABLE_FIT"
    } else if d_max < ks_crit * 1.30 {
        "POOR_FIT"
    } else {
        "REJECT"
    };
    HillksSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        k_order: k,
        alpha_hat: alpha,
        ks_statistic: d_max,
        ks_critical_5pct: ks_crit,
        hillks_label: label.into(),
        note: String::new(),
    }
}

/// TSI compute: Blau 1991 True Strength Index.
pub fn compute_tsi_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> TsiSnapshot {
    let sym = symbol.to_uppercase();
    let closes: Vec<f64> = bars
        .iter()
        .filter_map(|b| if b.close > 0.0 { Some(b.close) } else { None })
        .collect();
    if closes.len() < 60 {
        return TsiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            tsi_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥60 closes, got {}", closes.len()),
            ..Default::default()
        };
    }
    let n = closes.len();
    let long_p = 25usize;
    let short_p = 13usize;
    let diffs: Vec<f64> = (1..n).map(|i| closes[i] - closes[i - 1]).collect();
    let abs_diffs: Vec<f64> = diffs.iter().map(|d| d.abs()).collect();
    // EMA helper: EMA(x, p) where α = 2/(p+1); seed with first value.
    fn ema_series(x: &[f64], p: usize) -> Vec<f64> {
        if x.is_empty() {
            return Vec::new();
        }
        let alpha = 2.0 / ((p + 1) as f64);
        let mut out = Vec::with_capacity(x.len());
        out.push(x[0]);
        for i in 1..x.len() {
            out.push(alpha * x[i] + (1.0 - alpha) * out[i - 1]);
        }
        out
    }
    let long_smooth_num = ema_series(&diffs, long_p);
    let double_num = ema_series(&long_smooth_num, short_p);
    let long_smooth_den = ema_series(&abs_diffs, long_p);
    let double_den = ema_series(&long_smooth_den, short_p);
    let last = diffs.len() - 1;
    let den = double_den[last];
    if den.abs() < f64::EPSILON {
        return TsiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            tsi_label: "INSUFFICIENT_DATA".into(),
            note: "flat tape".into(),
            ..Default::default()
        };
    }
    let tsi_series: Vec<f64> = double_num
        .iter()
        .zip(double_den.iter())
        .map(|(n, d)| {
            if d.abs() < f64::EPSILON {
                0.0
            } else {
                100.0 * n / d
            }
        })
        .collect();
    let signal_series = ema_series(&tsi_series, short_p);
    let tsi = tsi_series[last];
    let signal = signal_series[last];
    let diff = tsi - signal;
    let label = if tsi > 25.0 {
        "STRONG_BULL"
    } else if tsi > 0.0 {
        "BULL"
    } else if tsi > -25.0 {
        if tsi > -5.0 && tsi < 5.0 {
            "NEUTRAL"
        } else {
            "BEAR"
        }
    } else {
        "STRONG_BEAR"
    };
    TsiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        ema_long: long_p,
        ema_short: short_p,
        tsi_value: tsi,
        signal_value: signal,
        tsi_minus_signal: diff,
        tsi_label: label.into(),
        note: String::new(),
    }
}
