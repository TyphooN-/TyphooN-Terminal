use super::*;

// ── (Quant Stats) compute fns ──

/// MODSHARPE compute: Pezier-White Adjusted Sharpe Ratio.
/// ASR = SR · [1 + (S/6)·SR − ((K−3)/24)·SR²] where S, K are the skewness
/// and kurtosis of bar-level log-returns. Annualised with √252.
pub fn compute_modsharpe_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> ModSharpeSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    let n = log_rets.len();
    if n < 30 {
        return ModSharpeSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            modsharpe_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", n),
            ..Default::default()
        };
    }
    let nf = n as f64;
    let mean: f64 = log_rets.iter().sum::<f64>() / nf;
    let var: f64 = log_rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / nf;
    let sd = var.sqrt();
    if sd <= f64::EPSILON {
        return ModSharpeSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            modsharpe_label: "INSUFFICIENT_DATA".into(),
            note: "zero return stdev".into(),
            ..Default::default()
        };
    }
    let m3: f64 = log_rets
        .iter()
        .map(|r| ((r - mean) / sd).powi(3))
        .sum::<f64>()
        / nf;
    let m4: f64 = log_rets
        .iter()
        .map(|r| ((r - mean) / sd).powi(4))
        .sum::<f64>()
        / nf;
    let skew = m3;
    let ek = m4 - 3.0;
    let ann = 252.0_f64;
    let sr = ann.sqrt() * mean / sd;
    let adj = 1.0 + (skew / 6.0) * sr - (ek / 24.0) * sr * sr;
    let asr = sr * adj;
    let factor = if sr.abs() > f64::EPSILON {
        asr / sr
    } else {
        0.0
    };
    let label = if asr > 1.0 {
        "STRONG_POS"
    } else if asr > 0.3 {
        "MODERATE_POS"
    } else if asr > -0.3 {
        "WEAK"
    } else if asr > -1.0 {
        "MODERATE_NEG"
    } else {
        "STRONG_NEG"
    };
    ModSharpeSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        annualization_factor: ann,
        mean_return_bar: mean,
        stdev_return_bar: sd,
        skewness: skew,
        excess_kurtosis: ek,
        sharpe_ratio: sr,
        adjusted_sharpe: asr,
        adjustment_factor: factor,
        modsharpe_label: label.into(),
        note: String::new(),
    }
}

/// HSIEHTEST compute: Hsieh (1989) third-moment nonlinearity test.
/// Fits AR(1) to log-returns, standardises the residuals, then probes
/// T(i,j) = E[e_{t-i} e_{t-j} e_t] at lag pairs (1,1) and (2,2).
/// Under H0 of linearity, m·T(i,j) is asymptotically N(0, 6) (for i=j
/// under approximate normality of e). Returns z = T · √(m/6).
pub fn compute_hsieh_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> HsiehTestSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    let n = log_rets.len();
    if n < 50 {
        return HsiehTestSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            hsieh_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥50 returns, got {}", n),
            ..Default::default()
        };
    }
    // AR(1) fit: r_t = α + φ·r_{t-1} + ε_t
    let m = n - 1;
    let mut sx = 0.0f64;
    let mut sy = 0.0f64;
    for i in 0..m {
        sx += log_rets[i];
        sy += log_rets[i + 1];
    }
    let mx = sx / m as f64;
    let my = sy / m as f64;
    let mut sxx = 0.0f64;
    let mut sxy = 0.0f64;
    for i in 0..m {
        let dx = log_rets[i] - mx;
        sxx += dx * dx;
        sxy += dx * (log_rets[i + 1] - my);
    }
    let phi = if sxx > f64::EPSILON { sxy / sxx } else { 0.0 };
    let alpha = my - phi * mx;
    let mut resid: Vec<f64> = Vec::with_capacity(m);
    for i in 0..m {
        resid.push(log_rets[i + 1] - (alpha + phi * log_rets[i]));
    }
    let mr = resid.len();
    let rmean: f64 = resid.iter().sum::<f64>() / mr as f64;
    let rvar: f64 = resid.iter().map(|e| (e - rmean).powi(2)).sum::<f64>() / mr as f64;
    let rsd = rvar.sqrt();
    if rsd <= f64::EPSILON {
        return HsiehTestSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            ar_order: 1,
            hsieh_label: "INSUFFICIENT_DATA".into(),
            note: "zero residual variance".into(),
            ..Default::default()
        };
    }
    let e: Vec<f64> = resid.iter().map(|v| (v - rmean) / rsd).collect();
    let compute_t = |i: usize, j: usize| -> (f64, usize) {
        let start = i.max(j);
        if start >= e.len() {
            return (0.0, 0);
        }
        let mut acc = 0.0f64;
        let mut cnt = 0usize;
        for t in start..e.len() {
            acc += e[t - i] * e[t - j] * e[t];
            cnt += 1;
        }
        if cnt == 0 {
            (0.0, 0)
        } else {
            (acc / cnt as f64, cnt)
        }
    };
    let (t11, n11) = compute_t(1, 1);
    let (t22, n22) = compute_t(2, 2);
    let z_for = |t: f64, c: usize| -> f64 {
        if c < 8 {
            return 0.0;
        }
        t * ((c as f64) / 6.0).sqrt()
    };
    let z11 = z_for(t11, n11);
    let z22 = z_for(t22, n22);
    let max_abs = z11.abs().max(z22.abs());
    let crit = 1.96_f64;
    let reject = max_abs > crit;
    let label = if !reject {
        "LINEAR"
    } else if max_abs > 2.0 * crit {
        "STRONG_NONLIN"
    } else {
        "MILD_NONLIN"
    };
    HsiehTestSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        ar_order: 1,
        t_11: t11,
        t_22: t22,
        z_11: z11,
        z_22: z22,
        max_abs_z: max_abs,
        critical_95: crit,
        reject_null: reject,
        hsieh_label: label.into(),
        note: String::new(),
    }
}

/// CHOWBREAK compute: mean-shift Chow F-test at n/2.
/// F = [(RSS_p − RSS_u) / k] / [RSS_u / (n − 2k)] with k = 1 (constant).
/// Compares pooled-mean RSS against the sum of within-half RSS values.
pub fn compute_chowbreak_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> ChowBreakSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    let n = log_rets.len();
    if n < 40 {
        return ChowBreakSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            chowbreak_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥40 returns, got {}", n),
            ..Default::default()
        };
    }
    let bp = n / 2;
    let pre = &log_rets[..bp];
    let post = &log_rets[bp..];
    let n1 = pre.len();
    let n2 = post.len();
    let mean_p: f64 = log_rets.iter().sum::<f64>() / n as f64;
    let mean_pre: f64 = pre.iter().sum::<f64>() / n1 as f64;
    let mean_post: f64 = post.iter().sum::<f64>() / n2 as f64;
    let rss_p: f64 = log_rets.iter().map(|r| (r - mean_p).powi(2)).sum();
    let rss_1: f64 = pre.iter().map(|r| (r - mean_pre).powi(2)).sum();
    let rss_2: f64 = post.iter().map(|r| (r - mean_post).powi(2)).sum();
    let rss_u = rss_1 + rss_2;
    let k = 1usize;
    let df1 = k;
    let df2 = n.saturating_sub(2 * k);
    if rss_u <= f64::EPSILON || df2 == 0 {
        return ChowBreakSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            break_point_idx: bp,
            k_regressors: k,
            chowbreak_label: "INSUFFICIENT_DATA".into(),
            note: "degenerate RSS".into(),
            ..Default::default()
        };
    }
    let f_stat = ((rss_p - rss_u) / df1 as f64) / (rss_u / df2 as f64);
    let f_stat = f_stat.max(0.0);
    let critical_95 = 3.84_f64; // χ²(1)/1 ≈ F(1, ∞) at α=5%
    let reject = f_stat > critical_95;
    let label = if !reject {
        "NO_BREAK"
    } else if f_stat > 3.0 * critical_95 {
        "STRONG_BREAK"
    } else {
        "MILD_BREAK"
    };
    ChowBreakSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        break_point_idx: bp,
        rss_pooled: rss_p,
        rss_unrestricted: rss_u,
        mean_pre,
        mean_post,
        k_regressors: k,
        f_stat,
        df_num: df1,
        df_den: df2,
        critical_95,
        reject_null: reject,
        chowbreak_label: label.into(),
        note: String::new(),
    }
}

/// DRIFTBURST compute: Christensen-Oomen-Renò (2018) drift-burst statistic.
/// For each candidate time t in [bw, n), compute Gaussian-kernel-weighted
/// mean μ̂(t) and sd σ̂(t) of the log-return series, then form the
/// standardised statistic T(t) = √h · μ̂(t)/σ̂(t) where h is the sum of
/// weights. Report max_t |T(t)|, its signed value, its offset from the
/// series end, and the count of excursions with |T| > 3.
pub fn compute_driftburst_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DriftBurstSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    let n = log_rets.len();
    if n < 50 {
        return DriftBurstSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            driftburst_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥50 returns, got {}", n),
            ..Default::default()
        };
    }
    let bw = 10.0_f64;
    let bw_int = bw as usize;
    let two_bw2 = 2.0 * bw * bw;
    let mut max_abs = 0.0f64;
    let mut max_signed = 0.0f64;
    let mut max_at: usize = 0;
    let mut excursions: usize = 0;
    for t in bw_int..n {
        let mut sw = 0.0f64;
        let mut swr = 0.0f64;
        let lo = t.saturating_sub(4 * bw_int);
        for s in lo..=t {
            let ds = (s as f64) - (t as f64);
            let w = (-(ds * ds) / two_bw2).exp();
            sw += w;
            swr += w * log_rets[s];
        }
        if sw <= f64::EPSILON {
            continue;
        }
        let mu = swr / sw;
        let mut sws = 0.0f64;
        for s in lo..=t {
            let ds = (s as f64) - (t as f64);
            let w = (-(ds * ds) / two_bw2).exp();
            sws += w * (log_rets[s] - mu).powi(2);
        }
        let var = sws / sw;
        let sigma = var.sqrt();
        if sigma <= f64::EPSILON {
            continue;
        }
        let stat = sw.sqrt() * mu / sigma;
        if stat.abs() > 3.0 {
            excursions += 1;
        }
        if stat.abs() > max_abs {
            max_abs = stat.abs();
            max_signed = stat;
            max_at = n - 1 - t;
        }
    }
    let crit = 3.0_f64;
    let label = if max_abs < crit {
        "NO_BURST"
    } else if max_abs < 5.0 {
        "MILD_BURST"
    } else {
        "STRONG_BURST"
    };
    DriftBurstSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        kernel_bandwidth_bars: bw,
        max_abs_statistic: max_abs,
        max_stat_signed: max_signed,
        max_at_offset: max_at,
        excursions_gt_3: excursions,
        critical_99_approx: crit,
        driftburst_label: label.into(),
        note: String::new(),
    }
}

/// HLVCLUST compute: Parkinson high-low volatility clustering Ljung-Box.
/// Forms the log-range series lr_t = ln(H_t/L_t), applies Ljung-Box at
/// h=10, and reports lag-1 / lag-5 autocorrelations plus Q statistic.
pub fn compute_hlvclust_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> HlvClustSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let mut lr: Vec<f64> = Vec::with_capacity(sorted.len());
    for bar in sorted.iter() {
        if bar.high > 0.0 && bar.low > 0.0 && bar.high > bar.low {
            let v = (bar.high / bar.low).ln();
            if v.is_finite() && v > 0.0 {
                lr.push(v);
            }
        }
    }
    let n = lr.len();
    if n < 30 {
        return HlvClustSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            hlvclust_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 valid H/L bars, got {}", n),
            ..Default::default()
        };
    }
    let pk: Vec<f64> = lr
        .iter()
        .map(|v| v / (4.0_f64 * std::f64::consts::LN_2).sqrt())
        .collect();
    let pk_mean: f64 = pk.iter().sum::<f64>() / n as f64;
    let pk_ann = pk_mean * (252.0_f64).sqrt();
    // Autocorrelation computed on the centred log-range series
    let mean: f64 = lr.iter().sum::<f64>() / n as f64;
    let var: f64 = lr.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
    if var <= f64::EPSILON {
        return HlvClustSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            hlvclust_label: "INSUFFICIENT_DATA".into(),
            note: "zero log-range variance".into(),
            ..Default::default()
        };
    }
    let rho_k = |k: usize| -> f64 {
        let mut num = 0.0f64;
        for t in k..n {
            num += (lr[t] - mean) * (lr[t - k] - mean);
        }
        num / (n as f64 * var)
    };
    let h = 10usize;
    let mut q = 0.0f64;
    for k in 1..=h {
        let r = rho_k(k);
        q += r * r / (n as f64 - k as f64);
    }
    q *= n as f64 * (n as f64 + 2.0);
    let ac1 = rho_k(1);
    let ac5 = rho_k(5);
    let crit = 18.307_f64; // χ²(10) at 95%
    let p = chi2_upper_tail(q, h);
    let reject = q > crit;
    let label = if !reject {
        "NO_CLUST"
    } else if q < 2.0 * crit {
        "MILD_CLUST"
    } else {
        "STRONG_CLUST"
    };
    HlvClustSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        lag_h: h,
        parkinson_vol_bar: pk_mean,
        parkinson_vol_annualised: pk_ann,
        ac_lag1: ac1,
        ac_lag5: ac5,
        lb_q_stat: q,
        critical_95: crit,
        p_value: p,
        reject_null: reject,
        hlvclust_label: label.into(),
        note: String::new(),
    }
}

// ── YANGZHANG / KUIPER / DAGOSTINO / BAIPERRON / KUPIECPOF ──

pub fn compute_yangzhang_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> YangZhangVolSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    // Filter to bars with valid OHLC
    let valid: Vec<&HistoricalPriceRow> = sorted
        .iter()
        .copied()
        .filter(|b| {
            b.open > 0.0
                && b.high > 0.0
                && b.low > 0.0
                && b.close > 0.0
                && b.high >= b.low
                && b.high >= b.open.max(b.close)
                && b.low <= b.open.min(b.close)
        })
        .collect();
    let n = valid.len();
    if n < 30 {
        return YangZhangVolSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            yangzhang_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 valid OHLC bars, got {}", n),
            ..Default::default()
        };
    }
    // Overnight: o_t = ln(O_t / C_{t-1}) for t=1..n-1; n_o = n-1
    let mut o: Vec<f64> = Vec::with_capacity(n - 1);
    for t in 1..n {
        let v = (valid[t].open / valid[t - 1].close).ln();
        if v.is_finite() {
            o.push(v);
        }
    }
    // Open-to-close: c_t = ln(C_t / O_t) for t=0..n-1
    let mut c: Vec<f64> = Vec::with_capacity(n);
    for t in 0..n {
        let v = (valid[t].close / valid[t].open).ln();
        if v.is_finite() {
            c.push(v);
        }
    }
    // Rogers-Satchell: rs_t = ln(H/C)·ln(H/O) + ln(L/C)·ln(L/O)
    let mut rs: Vec<f64> = Vec::with_capacity(n);
    for b in valid.iter() {
        let lh_c = (b.high / b.close).ln();
        let lh_o = (b.high / b.open).ln();
        let ll_c = (b.low / b.close).ln();
        let ll_o = (b.low / b.open).ln();
        let v = lh_c * lh_o + ll_c * ll_o;
        if v.is_finite() {
            rs.push(v);
        }
    }
    if o.len() < 2 || c.len() < 2 || rs.is_empty() {
        return YangZhangVolSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            yangzhang_label: "INSUFFICIENT_DATA".into(),
            note: "empty O/C/RS after filter".into(),
            ..Default::default()
        };
    }
    let mean_o = o.iter().sum::<f64>() / o.len() as f64;
    let mean_c = c.iter().sum::<f64>() / c.len() as f64;
    let var_o = o.iter().map(|v| (v - mean_o).powi(2)).sum::<f64>() / (o.len() - 1) as f64;
    let var_c = c.iter().map(|v| (v - mean_c).powi(2)).sum::<f64>() / (c.len() - 1) as f64;
    let mean_rs = rs.iter().sum::<f64>() / rs.len() as f64;
    // Yang-Zhang weight (requires n≥2)
    let n_f = n as f64;
    let k = 0.34 / (1.34 + (n_f + 1.0) / (n_f - 1.0));
    let var_yz = var_o + k * var_c + (1.0 - k) * mean_rs;
    if !var_yz.is_finite() || var_yz < 0.0 {
        return YangZhangVolSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            yangzhang_label: "INSUFFICIENT_DATA".into(),
            note: format!("non-finite or negative σ²_YZ: {:.3e}", var_yz),
            ..Default::default()
        };
    }
    let yz_bar = var_yz.sqrt();
    let yz_ann = yz_bar * (252.0_f64).sqrt();
    let yz_ann_pct = yz_ann * 100.0;
    // Close-to-close reference σ
    let mut r: Vec<f64> = Vec::with_capacity(n - 1);
    for t in 1..n {
        let v = (valid[t].close / valid[t - 1].close).ln();
        if v.is_finite() {
            r.push(v);
        }
    }
    let mean_r = r.iter().sum::<f64>() / r.len() as f64;
    let var_r = r.iter().map(|v| (v - mean_r).powi(2)).sum::<f64>() / (r.len() - 1).max(1) as f64;
    let cc_ann_pct = var_r.sqrt() * (252.0_f64).sqrt() * 100.0;
    let eff = if yz_ann_pct > 0.0 {
        cc_ann_pct / yz_ann_pct
    } else {
        0.0
    };
    let label = match yz_ann_pct {
        x if x < 10.0 => "VERY_LOW",
        x if x < 20.0 => "LOW",
        x if x < 35.0 => "MODERATE",
        x if x < 60.0 => "HIGH",
        _ => "VERY_HIGH",
    };
    YangZhangVolSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        overnight_var: var_o,
        open_to_close_var: var_c,
        rs_component: mean_rs,
        k_weight: k,
        yz_vol_bar: yz_bar,
        yz_vol_annualised_pct: yz_ann_pct,
        cc_vol_annualised_pct: cc_ann_pct,
        efficiency_vs_close: eff,
        yangzhang_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_kuiper_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> KuiperSnapshot {
    let sym = symbol.to_uppercase();
    let (_, rets) = trailing_log_returns(bars);
    let n = rets.len();
    if n < 30 {
        return KuiperSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            kuiper_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", n),
            ..Default::default()
        };
    }
    let mean = rets.iter().sum::<f64>() / n as f64;
    let var = rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
    if var <= f64::EPSILON {
        return KuiperSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            kuiper_label: "INSUFFICIENT_DATA".into(),
            note: "zero return variance".into(),
            ..Default::default()
        };
    }
    let sd = var.sqrt();
    let mut z: Vec<f64> = rets.iter().map(|r| (r - mean) / sd).collect();
    z.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let nf = n as f64;
    let mut d_plus = 0.0f64;
    let mut d_minus = 0.0f64;
    for (i, zi) in z.iter().enumerate() {
        let f_theoretical = std_normal_cdf(*zi);
        let f_above = (i + 1) as f64 / nf;
        let f_below = i as f64 / nf;
        let dp = f_above - f_theoretical;
        let dm = f_theoretical - f_below;
        if dp > d_plus {
            d_plus = dp;
        }
        if dm > d_minus {
            d_minus = dm;
        }
    }
    let v = d_plus + d_minus;
    let sqrt_n = nf.sqrt();
    let v_adj = v * (sqrt_n + 0.155 + 0.24 / sqrt_n);
    let crit = 1.747_f64;
    // Stephens (1970) asymptotic p-value: p ≈ Σ_{m=1}^∞ (8m²V*² − 2) e^{-2m²V*²} · 2
    let mut p = 0.0f64;
    for m in 1..=6 {
        let mf = m as f64;
        let arg = 2.0 * mf * mf * v_adj * v_adj;
        p += (4.0 * mf * mf * v_adj * v_adj - 1.0) * 2.0 * (-arg).exp();
    }
    let p = p.clamp(0.0, 1.0);
    let reject = v_adj > crit;
    let label = if !reject {
        "NORMAL"
    } else if v_adj < 2.0 * crit {
        "MILD_DEPART"
    } else {
        "STRONG_DEPART"
    };
    KuiperSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        mean,
        stdev: sd,
        d_plus,
        d_minus,
        v_stat: v,
        v_stat_adj: v_adj,
        critical_95: crit,
        p_value_approx: p,
        reject_null: reject,
        kuiper_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_dagostino_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DagostinoSnapshot {
    let sym = symbol.to_uppercase();
    let (_, rets) = trailing_log_returns(bars);
    let n = rets.len();
    if n < 30 {
        return DagostinoSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            dagostino_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", n),
            ..Default::default()
        };
    }
    let nf = n as f64;
    let mean = rets.iter().sum::<f64>() / nf;
    let m2 = rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / nf;
    let m3 = rets.iter().map(|r| (r - mean).powi(3)).sum::<f64>() / nf;
    let m4 = rets.iter().map(|r| (r - mean).powi(4)).sum::<f64>() / nf;
    if m2 <= f64::EPSILON {
        return DagostinoSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            dagostino_label: "INSUFFICIENT_DATA".into(),
            note: "zero return variance".into(),
            ..Default::default()
        };
    }
    let b1 = m3 / m2.powf(1.5);
    let b2 = m4 / (m2 * m2); // raw kurtosis
    let ex_kurt = b2 - 3.0;
    // D'Agostino (1970) transformed skewness z
    let y = b1 * ((nf + 1.0) * (nf + 3.0) / (6.0 * (nf - 2.0))).sqrt();
    let beta2 = 3.0 * (nf * nf + 27.0 * nf - 70.0) * (nf + 1.0) * (nf + 3.0)
        / ((nf - 2.0) * (nf + 5.0) * (nf + 7.0) * (nf + 9.0));
    let wsq = -1.0 + (2.0 * (beta2 - 1.0)).sqrt();
    let z_skew = if wsq > 1.0 {
        let w = wsq.sqrt();
        let delta = 1.0 / (w.ln()).sqrt();
        let alpha = (2.0 / (wsq - 1.0)).sqrt();
        let ya = y / alpha;
        delta * (ya + (ya * ya + 1.0).sqrt()).ln()
    } else {
        // fall back to raw z
        b1 * (nf / 6.0).sqrt()
    };
    // Anscombe-Glynn (1983) transformed kurtosis z
    let mu_k = 3.0 * (nf - 1.0) / (nf + 1.0);
    let sigma2_k =
        24.0 * nf * (nf - 2.0) * (nf - 3.0) / ((nf + 1.0).powi(2) * (nf + 3.0) * (nf + 5.0));
    let x_k = (b2 - mu_k) / sigma2_k.sqrt();
    let beta_skew_k = 6.0 * (nf * nf - 5.0 * nf + 2.0) / ((nf + 7.0) * (nf + 9.0))
        * (6.0 * (nf + 3.0) * (nf + 5.0) / (nf * (nf - 2.0) * (nf - 3.0))).sqrt();
    let z_kurt = if beta_skew_k > 0.0 {
        let a_k = 6.0
            + (8.0 / beta_skew_k)
                * (2.0 / beta_skew_k + (1.0 + 4.0 / (beta_skew_k * beta_skew_k)).sqrt());
        let denom = 1.0 + x_k * (2.0 / (a_k - 4.0)).sqrt();
        if denom > 0.0 && a_k > 4.0 {
            let term = (1.0 - 2.0 / a_k) / denom;
            let inner = term.cbrt(); // cube root handles negative reliably
            (1.0 - 2.0 / (9.0 * a_k) - inner) / (2.0 / (9.0 * a_k)).sqrt()
        } else {
            (b2 - 3.0) * (nf / 24.0).sqrt()
        }
    } else {
        (b2 - 3.0) * (nf / 24.0).sqrt()
    };
    let k2 = z_skew * z_skew + z_kurt * z_kurt;
    let crit = 5.991_f64; // χ²_95(2)
    let p = chi2_upper_tail(k2, 2);
    let reject = k2 > crit;
    let az_s = z_skew.abs();
    let az_k = z_kurt.abs();
    let label = if !reject {
        "NORMAL"
    } else if az_s > 1.96 && az_k > 1.96 {
        "BOTH_DEPART"
    } else if az_s >= az_k {
        "SKEW_DOMINANT"
    } else {
        "KURT_DOMINANT"
    };
    DagostinoSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        skewness: b1,
        excess_kurtosis: ex_kurt,
        z_skew,
        z_kurt,
        k2_stat: k2,
        critical_95: crit,
        p_value: p,
        reject_null: reject,
        dagostino_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_baiperron_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> BaiPerronSnapshot {
    let sym = symbol.to_uppercase();
    let (_, rets) = trailing_log_returns(bars);
    let n = rets.len();
    let trim = 0.15_f64;
    if n < 40 {
        return BaiPerronSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            trim_fraction: trim,
            baiperron_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥40 returns, got {}", n),
            ..Default::default()
        };
    }
    let nf = n as f64;
    let mean_full = rets.iter().sum::<f64>() / nf;
    let rss_full = rets.iter().map(|r| (r - mean_full).powi(2)).sum::<f64>();
    if rss_full <= f64::EPSILON {
        return BaiPerronSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            trim_fraction: trim,
            baiperron_label: "INSUFFICIENT_DATA".into(),
            note: "zero total RSS".into(),
            ..Default::default()
        };
    }
    let lo = (trim * nf).ceil() as usize;
    let hi = ((1.0 - trim) * nf).floor() as usize;
    let mut best_tau = lo;
    let mut best_f = 0.0f64;
    let mut best_mean_pre = 0.0f64;
    let mut best_mean_post = 0.0f64;
    let mut best_rss_u = rss_full;
    for tau in lo..hi {
        let pre = &rets[..tau];
        let post = &rets[tau..];
        let mean_pre = pre.iter().sum::<f64>() / pre.len() as f64;
        let mean_post = post.iter().sum::<f64>() / post.len() as f64;
        let rss_pre: f64 = pre.iter().map(|r| (r - mean_pre).powi(2)).sum();
        let rss_post: f64 = post.iter().map(|r| (r - mean_post).powi(2)).sum();
        let rss_u = rss_pre + rss_post;
        if rss_u <= f64::EPSILON {
            continue;
        }
        let k = 1.0;
        let f_stat = ((rss_full - rss_u) / k) / (rss_u / (nf - 2.0 * k));
        if f_stat > best_f {
            best_f = f_stat;
            best_tau = tau;
            best_mean_pre = mean_pre;
            best_mean_post = mean_post;
            best_rss_u = rss_u;
        }
    }
    // Andrews (1993) critical value for sup-F with π0=0.15, k=1, 95%: ≈ 8.58
    let crit = 8.58_f64;
    // Hansen (1997) conservative p-value upper-bound using χ²(1) (slightly optimistic vs true sup-F)
    let p = chi2_upper_tail(best_f, 1);
    let reject = best_f > crit;
    let label = if !reject {
        "NO_BREAK"
    } else if best_f < 2.0 * crit {
        "MILD_BREAK"
    } else {
        "STRONG_BREAK"
    };
    BaiPerronSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        trim_fraction: trim,
        search_lo: lo,
        search_hi: hi,
        best_break_idx: best_tau,
        sup_f_stat: best_f,
        mean_pre: best_mean_pre,
        mean_post: best_mean_post,
        rss_no_break: rss_full,
        rss_at_best: best_rss_u,
        critical_95: crit,
        p_value_approx: p,
        reject_null: reject,
        baiperron_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_kupiecpof_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> KupiecPofSnapshot {
    let sym = symbol.to_uppercase();
    let (_, rets) = trailing_log_returns(bars);
    let n = rets.len();
    let conf = 0.95_f64;
    let alpha = 1.0 - conf; // 0.05
    let rolling = 60usize;
    if n < rolling + 30 {
        return KupiecPofSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            confidence_level: conf,
            nominal_exceedance_rate: alpha,
            rolling_window: rolling,
            kupiec_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} returns for VaR backtest, got {}", rolling + 30, n),
            ..Default::default()
        };
    }
    let mut n_exc = 0usize;
    let mut last_var = 0.0f64;
    for t in rolling..n {
        let mut win: Vec<f64> = rets[t - rolling..t].to_vec();
        win.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        // Historical VaR at 95% confidence: 5th percentile of returns, reported as positive loss
        let q5 = quantile_f64(&win, alpha);
        let var_t = -q5;
        if rets[t] < q5 {
            n_exc += 1;
        }
        if t == n - 1 {
            last_var = var_t;
        }
    }
    let test_window = n - rolling;
    let expected = test_window as f64 * alpha;
    let p_hat = n_exc as f64 / test_window as f64;
    let t_ok = test_window - n_exc;
    let lr = {
        // LR_POF = 2 · [ T_f·ln(p̂/α) + T_o·ln((1-p̂)/(1-α)) ]; handle p̂=0 or 1
        let term_f = if n_exc > 0 {
            n_exc as f64 * (p_hat / alpha).ln()
        } else {
            0.0
        };
        let term_o = if t_ok > 0 {
            t_ok as f64 * ((1.0 - p_hat) / (1.0 - alpha)).ln()
        } else {
            0.0
        };
        2.0 * (term_f + term_o)
    };
    let lr = if lr.is_finite() && lr >= 0.0 { lr } else { 0.0 };
    let crit = 3.841_f64; // χ²_95(1)
    let p = chi2_upper_tail(lr, 1);
    let reject = lr > crit;
    let label = if !reject {
        "GOOD_FIT"
    } else if p_hat < alpha {
        "OVER_ESTIMATED"
    } else {
        "UNDER_ESTIMATED"
    };
    KupiecPofSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        confidence_level: conf,
        nominal_exceedance_rate: alpha,
        rolling_window: rolling,
        test_window,
        var_latest_bar: last_var,
        n_exceedances: n_exc,
        expected_exceedances: expected,
        realised_exceedance_rate: p_hat,
        lr_pof_stat: lr,
        critical_95: crit,
        p_value: p,
        reject_null: reject,
        kupiec_label: label.into(),
        note: String::new(),
    }
}
