use super::*;

// Upside-potential, leverage-effect, drawdown-at-risk, VaR-half-life, and Gini computes

/// UPR compute: Upside Potential Ratio = E[max(r,0)] / √E[min(r,0)²] (MAR=0).
pub fn compute_upr_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> UprSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return UprSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            upr_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let nf = n as f64;
    let upm1 = log_rets.iter().map(|&r| r.max(0.0)).sum::<f64>() / nf;
    let lpm2 = log_rets.iter().map(|&r| r.min(0.0).powi(2)).sum::<f64>() / nf;
    let dd = lpm2.sqrt();
    let upr = if dd > f64::EPSILON { upm1 / dd } else { 0.0 };
    let label = if upr < 0.5 {
        "LOW_UPSIDE"
    } else if upr < 1.0 {
        "MODERATE_UPSIDE"
    } else if upr < 1.5 {
        "BALANCED"
    } else if upr < 2.5 {
        "HIGH_UPSIDE"
    } else {
        "VERY_HIGH_UPSIDE"
    };
    UprSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        upm1,
        lpm2,
        downside_dev: dd,
        upr,
        upr_label: label.into(),
        note: String::new(),
    }
}

/// LEVEREFF compute: leverage effect corr(rₜ, rₜ₊₁²) + asymmetric vol ratio.
pub fn compute_levereff_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> LeverEffSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return LeverEffSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            lever_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let pairs: Vec<(f64, f64)> = (0..n - 1)
        .map(|i| (log_rets[i], log_rets[i + 1] * log_rets[i + 1]))
        .collect();
    let np = pairs.len() as f64;
    let mean_r = pairs.iter().map(|(r, _)| r).sum::<f64>() / np;
    let mean_s = pairs.iter().map(|(_, s)| s).sum::<f64>() / np;
    let cov = pairs
        .iter()
        .map(|(r, s)| (r - mean_r) * (s - mean_s))
        .sum::<f64>()
        / np;
    let var_r = pairs.iter().map(|(r, _)| (r - mean_r).powi(2)).sum::<f64>() / np;
    let var_s = pairs.iter().map(|(_, s)| (s - mean_s).powi(2)).sum::<f64>() / np;
    let corr = if var_r > f64::EPSILON && var_s > f64::EPSILON {
        cov / (var_r.sqrt() * var_s.sqrt())
    } else {
        0.0
    };
    let mut sum_vol_neg = 0.0_f64;
    let mut cnt_neg = 0usize;
    let mut sum_vol_pos = 0.0_f64;
    let mut cnt_pos = 0usize;
    for i in 0..n - 1 {
        let next_abs = log_rets[i + 1].abs();
        if log_rets[i] < 0.0 {
            sum_vol_neg += next_abs;
            cnt_neg += 1;
        } else if log_rets[i] > 0.0 {
            sum_vol_pos += next_abs;
            cnt_pos += 1;
        }
    }
    let mvn = if cnt_neg > 0 {
        sum_vol_neg / cnt_neg as f64 * 100.0
    } else {
        0.0
    };
    let mvp = if cnt_pos > 0 {
        sum_vol_pos / cnt_pos as f64 * 100.0
    } else {
        0.0
    };
    let asym = if mvp > f64::EPSILON { mvn / mvp } else { 0.0 };
    let label = if corr < -0.15 {
        "STRONG_LEVERAGE"
    } else if corr < -0.05 {
        "MILD_LEVERAGE"
    } else if corr <= 0.05 {
        "SYMMETRIC"
    } else {
        "REVERSE_LEVERAGE"
    };
    LeverEffSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        corr_r_nextsq: corr,
        mean_vol_after_neg: mvn,
        mean_vol_after_pos: mvp,
        asym_ratio: asym,
        lever_label: label.into(),
        note: String::new(),
    }
}

/// DRAWDAR compute: Drawdown-at-Risk + Conditional DaR at 5% and 1%.
pub fn compute_drawdar_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DrawDaRSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 30 {
        return DrawDaRSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            drawdar_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 bars, got {}", bars.len()),
            ..Default::default()
        };
    }
    let n = bars.len();
    let mut peak = bars[0].close;
    let mut dds: Vec<f64> = Vec::with_capacity(n);
    for b in bars {
        if b.close > peak {
            peak = b.close;
        }
        let dd = if peak > f64::EPSILON {
            (peak - b.close) / peak * 100.0
        } else {
            0.0
        };
        dds.push(dd);
    }
    let mut sorted = dds.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let quantile = |q: f64| -> f64 {
        let idx = ((n as f64 * q).ceil() as usize).min(n) - 1;
        sorted[idx]
    };
    let dar5 = quantile(0.95);
    let dar1 = quantile(0.99);
    let cdar5 = {
        let tail: Vec<f64> = sorted.iter().filter(|&&d| d >= dar5).cloned().collect();
        if tail.is_empty() {
            dar5
        } else {
            tail.iter().sum::<f64>() / tail.len() as f64
        }
    };
    let cdar1 = {
        let tail: Vec<f64> = sorted.iter().filter(|&&d| d >= dar1).cloned().collect();
        if tail.is_empty() {
            dar1
        } else {
            tail.iter().sum::<f64>() / tail.len() as f64
        }
    };
    let max_dd = sorted.last().cloned().unwrap_or(0.0);
    let nonzero: Vec<f64> = dds.iter().filter(|&&d| d > f64::EPSILON).cloned().collect();
    let mean_dd = if nonzero.is_empty() {
        0.0
    } else {
        nonzero.iter().sum::<f64>() / nonzero.len() as f64
    };
    let label = if dar5 < 3.0 {
        "LOW_DD_RISK"
    } else if dar5 < 7.0 {
        "MODERATE_DD_RISK"
    } else if dar5 < 15.0 {
        "HIGH_DD_RISK"
    } else {
        "SEVERE_DD_RISK"
    };
    DrawDaRSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        dar_5pct: dar5,
        cdar_5pct: cdar5,
        dar_1pct: dar1,
        cdar_1pct: cdar1,
        max_dd_pct: max_dd,
        mean_dd_pct: mean_dd,
        drawdar_label: label.into(),
        note: String::new(),
    }
}

/// VARHALF compute: volatility half-life via AR(1) on rolling 20d realized vol.
pub fn compute_varhalf_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> VarHalfSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 50 {
        return VarHalfSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            varhalf_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥50 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let window = 20usize;
    let mut vols: Vec<f64> = Vec::new();
    for i in window..=n {
        let slice = &log_rets[i - window..i];
        let m = slice.iter().sum::<f64>() / window as f64;
        let v = slice.iter().map(|r| (r - m).powi(2)).sum::<f64>() / window as f64;
        vols.push(v.sqrt());
    }
    if vols.len() < 10 {
        return VarHalfSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            varhalf_label: "INSUFFICIENT_DATA".into(),
            note: "too few vol observations".into(),
            ..Default::default()
        };
    }
    let nv = vols.len();
    let pairs: Vec<(f64, f64)> = (0..nv - 1).map(|i| (vols[i], vols[i + 1])).collect();
    let np = pairs.len() as f64;
    let mx = pairs.iter().map(|(x, _)| x).sum::<f64>() / np;
    let my = pairs.iter().map(|(_, y)| y).sum::<f64>() / np;
    let sxy = pairs.iter().map(|(x, y)| (x - mx) * (y - my)).sum::<f64>();
    let sxx = pairs.iter().map(|(x, _)| (x - mx).powi(2)).sum::<f64>();
    let beta = if sxx > f64::EPSILON { sxy / sxx } else { 0.0 };
    let alpha = my - beta * mx;
    let ss_res = pairs
        .iter()
        .map(|(x, y)| (y - alpha - beta * x).powi(2))
        .sum::<f64>();
    let ss_tot = pairs.iter().map(|(_, y)| (y - my).powi(2)).sum::<f64>();
    let r2 = if ss_tot > f64::EPSILON {
        1.0 - ss_res / ss_tot
    } else {
        0.0
    };
    let hl = if beta > f64::EPSILON && beta < 1.0 {
        -(2.0_f64.ln()) / beta.ln()
    } else if beta >= 1.0 {
        f64::INFINITY
    } else {
        0.0
    };
    let label = if hl.is_infinite() || hl > 60.0 {
        "VERY_PERSISTENT"
    } else if hl > 30.0 {
        "SLOW_PERSIST"
    } else if hl > 10.0 {
        "MODERATE_PERSIST"
    } else {
        "FAST_REVERT"
    };
    VarHalfSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        vol_obs: nv,
        ar1_beta: beta,
        ar1_alpha: alpha,
        ar1_r2: r2,
        half_life_days: hl,
        varhalf_label: label.into(),
        note: String::new(),
    }
}

/// GINI compute: Gini coefficient of |returns|.
pub fn compute_gini_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> GiniSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return GiniSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            gini_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let nf = n as f64;
    let mut abs_rets: Vec<f64> = log_rets.iter().map(|r| r.abs()).collect();
    abs_rets.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let total = abs_rets.iter().sum::<f64>();
    if total < f64::EPSILON {
        return GiniSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            gini_label: "INSUFFICIENT_DATA".into(),
            note: "zero total |returns|".into(),
            ..Default::default()
        };
    }
    let weighted_sum: f64 = abs_rets
        .iter()
        .enumerate()
        .map(|(i, &v)| (i as f64 + 1.0) * v)
        .sum();
    let gini = (2.0 * weighted_sum) / (nf * total) - (nf + 1.0) / nf;
    let mean_abs = total / nf * 100.0;
    let median_abs = abs_rets[n / 2] * 100.0;
    let label = if gini < 0.30 {
        "LOW_CONCENTRATION"
    } else if gini < 0.45 {
        "MODERATE_CONCENTRATION"
    } else if gini < 0.60 {
        "HIGH_CONCENTRATION"
    } else {
        "VERY_HIGH_CONCENTRATION"
    };
    GiniSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        gini_coeff: gini,
        mean_abs_return_pct: mean_abs,
        median_abs_return_pct: median_abs,
        gini_label: label.into(),
        note: String::new(),
    }
}
