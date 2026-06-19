use super::*;

// Calmar, ulcer, variance-ratio, Amihud, and normality-test computes

pub fn compute_calmar_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CalmarRatioSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 30 {
        return CalmarRatioSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            calmar_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 bars, got {}", bars.len()),
            ..Default::default()
        };
    }
    let window = if bars.len() > 253 {
        &bars[bars.len() - 253..]
    } else {
        bars
    };
    let first = window[0].close;
    let last = window[window.len() - 1].close;
    if first <= 0.0 || last <= 0.0 {
        return CalmarRatioSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            calmar_label: "INSUFFICIENT_DATA".into(),
            note: "non-positive close".into(),
            ..Default::default()
        };
    }
    let total_ret = (last / first - 1.0) * 100.0;
    let ann_ret = total_ret * (252.0 / window.len() as f64);
    let mut peak = window[0].close;
    let mut max_dd: f64 = 0.0;
    for b in window.iter() {
        if b.close > peak {
            peak = b.close;
        }
        let dd = (peak - b.close) / peak * 100.0;
        if dd > max_dd {
            max_dd = dd;
        }
    }
    let calmar = if max_dd < f64::EPSILON {
        0.0
    } else {
        ann_ret / max_dd
    };
    let label = if max_dd < f64::EPSILON && ann_ret <= 0.0 {
        "INSUFFICIENT_DATA"
    } else if max_dd < f64::EPSILON {
        "EXCELLENT"
    } else if calmar < 0.5 {
        "VERY_POOR"
    } else if calmar < 1.0 {
        "POOR"
    } else if calmar < 2.0 {
        "NEUTRAL"
    } else if calmar < 3.0 {
        "GOOD"
    } else {
        "EXCELLENT"
    };
    CalmarRatioSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: window.len(),
        total_return_pct: total_ret,
        annualized_return_pct: ann_ret,
        max_drawdown_pct: max_dd,
        calmar_ratio: calmar,
        calmar_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_ulcer_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> UlcerIndexSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 30 {
        return UlcerIndexSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            ulcer_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 bars, got {}", bars.len()),
            ..Default::default()
        };
    }
    let window = if bars.len() > 253 {
        &bars[bars.len() - 253..]
    } else {
        bars
    };
    let first = window[0].close;
    let last = window[window.len() - 1].close;
    if first <= 0.0 || last <= 0.0 {
        return UlcerIndexSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            ulcer_label: "INSUFFICIENT_DATA".into(),
            note: "non-positive close".into(),
            ..Default::default()
        };
    }
    let total_ret = (last / first - 1.0) * 100.0;
    let ann_ret = total_ret * (252.0 / window.len() as f64);
    let mut peak = window[0].close;
    let mut dd_sum = 0.0_f64;
    let mut dd_sq_sum = 0.0_f64;
    let mut max_dd = 0.0_f64;
    let mut in_dd_count = 0_usize;
    for b in window.iter() {
        if b.close > peak {
            peak = b.close;
        }
        let dd_pct = (b.close - peak) / peak * 100.0; // ≤ 0
        dd_sum += dd_pct;
        dd_sq_sum += dd_pct * dd_pct;
        if dd_pct < max_dd {
            max_dd = dd_pct;
        }
        if dd_pct < -f64::EPSILON {
            in_dd_count += 1;
        }
    }
    let n = window.len() as f64;
    let ulcer = (dd_sq_sum / n).sqrt();
    let mean_dd = dd_sum / n;
    let pct_in_dd = in_dd_count as f64 / n * 100.0;
    let martin = if ulcer < f64::EPSILON {
        0.0
    } else {
        ann_ret / ulcer
    };
    let label = if ulcer < f64::EPSILON && ann_ret <= 0.0 {
        "INSUFFICIENT_DATA"
    } else if ulcer < 2.0 {
        "LOW_PAIN"
    } else if ulcer < 5.0 {
        "MILD"
    } else if ulcer < 10.0 {
        "MODERATE"
    } else if ulcer < 20.0 {
        "HIGH"
    } else {
        "SEVERE"
    };
    UlcerIndexSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: window.len(),
        ulcer_index: ulcer,
        mean_drawdown_pct: mean_dd,
        max_drawdown_pct: max_dd,
        pct_in_drawdown: pct_in_dd,
        annualized_return_pct: ann_ret,
        martin_ratio: martin,
        ulcer_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_varratio_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> VarianceRatioSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 40 {
        return VarianceRatioSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            rw_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥40 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let mean = log_rets.iter().sum::<f64>() / n as f64;
    let demeaned: Vec<f64> = log_rets.iter().map(|r| r - mean).collect();
    let var1 = demeaned.iter().map(|d| d * d).sum::<f64>() / (n - 1) as f64;
    if var1 < f64::EPSILON {
        return VarianceRatioSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            rw_label: "INSUFFICIENT_DATA".into(),
            note: "zero variance".into(),
            ..Default::default()
        };
    }
    let compute_vr = |q: usize| -> f64 {
        if q > n {
            return 1.0;
        }
        let mut agg = Vec::with_capacity(n / q + 1);
        let mut i = 0;
        while i + q <= n {
            let s: f64 = demeaned[i..i + q].iter().sum();
            agg.push(s);
            i += 1;
        }
        if agg.is_empty() {
            return 1.0;
        }
        let var_q = agg.iter().map(|s| s * s).sum::<f64>() / (agg.len() - 1).max(1) as f64;
        var_q / (q as f64 * var1)
    };
    let vr2 = compute_vr(2);
    let vr5 = compute_vr(5);
    let vr10 = compute_vr(10);
    let vr20 = compute_vr(20);
    let z_stat = |vr: f64, q: usize| -> f64 {
        let nf = n as f64;
        let se = (2.0 * (q as f64 - 1.0) / (3.0 * q as f64 * nf)).sqrt();
        if se < f64::EPSILON {
            0.0
        } else {
            (vr - 1.0) / se
        }
    };
    let z2 = z_stat(vr2, 2);
    let z5 = z_stat(vr5, 5);
    let label = if vr5 < 0.7 {
        "STRONG_REVERT"
    } else if vr5 < 0.9 {
        "MEAN_REVERT"
    } else if vr5 <= 1.1 {
        "RANDOM_WALK"
    } else if vr5 < 1.3 {
        "TRENDING"
    } else {
        "STRONG_TREND"
    };
    VarianceRatioSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        vr_2: vr2,
        vr_5: vr5,
        vr_10: vr10,
        vr_20: vr20,
        z_stat_2: z2,
        z_stat_5: z5,
        rw_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_amihud_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> AmihudIlliqSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 30 {
        return AmihudIlliqSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            illiq_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 bars, got {}", bars.len()),
            ..Default::default()
        };
    }
    let window = if bars.len() > 253 {
        &bars[bars.len() - 253..]
    } else {
        bars
    };
    let mut daily_illiq: Vec<f64> = Vec::with_capacity(window.len());
    let mut dvol_sum = 0.0_f64;
    let mut dvol_count = 0_usize;
    for pair in window.windows(2) {
        let prev_close = pair[0].close;
        let cur = &pair[1];
        if prev_close <= 0.0 || cur.close <= 0.0 {
            continue;
        }
        let dollar_vol = cur.close * cur.volume;
        if dollar_vol < f64::EPSILON {
            continue;
        }
        let abs_ret = (cur.close / prev_close).ln().abs();
        daily_illiq.push(abs_ret / dollar_vol * 1e6);
        dvol_sum += dollar_vol;
        dvol_count += 1;
    }
    if daily_illiq.len() < 20 {
        return AmihudIlliqSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            illiq_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥20 valid bars, got {}", daily_illiq.len()),
            ..Default::default()
        };
    }
    let n = daily_illiq.len();
    let mean_illiq = daily_illiq.iter().sum::<f64>() / n as f64;
    daily_illiq.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_illiq = daily_illiq[n / 2];
    let p90_idx = (n as f64 * 0.9).ceil() as usize;
    let illiq_90th = daily_illiq[p90_idx.min(n - 1)];
    let avg_dvol = if dvol_count > 0 {
        dvol_sum / dvol_count as f64
    } else {
        0.0
    };
    let label = if mean_illiq < 0.01 {
        "VERY_LIQUID"
    } else if mean_illiq < 0.1 {
        "LIQUID"
    } else if mean_illiq < 1.0 {
        "MODERATE"
    } else if mean_illiq < 10.0 {
        "ILLIQUID"
    } else {
        "VERY_ILLIQUID"
    };
    AmihudIlliqSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        mean_illiq,
        median_illiq,
        illiq_90th,
        avg_dollar_volume: avg_dvol,
        illiq_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_jbnorm_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> JarqueBeraSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return JarqueBeraSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            normal_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len() as f64;
    let mean = log_rets.iter().sum::<f64>() / n;
    let m2 = log_rets
        .iter()
        .map(|r| {
            let d = r - mean;
            d * d
        })
        .sum::<f64>()
        / n;
    if m2 < f64::EPSILON {
        return JarqueBeraSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            normal_label: "INSUFFICIENT_DATA".into(),
            note: "zero variance".into(),
            ..Default::default()
        };
    }
    let m3 = log_rets
        .iter()
        .map(|r| {
            let d = r - mean;
            d * d * d
        })
        .sum::<f64>()
        / n;
    let m4 = log_rets
        .iter()
        .map(|r| {
            let d = r - mean;
            d * d * d * d
        })
        .sum::<f64>()
        / n;
    let skew = m3 / m2.powf(1.5);
    let kurt = m4 / (m2 * m2) - 3.0; // excess kurtosis
    let jb = (n / 6.0) * (skew * skew + kurt * kurt / 4.0);
    let pvalue = (-jb / 2.0).exp(); // exact for chi²(2)
    let label = if pvalue > 0.10 {
        "NORMAL"
    } else if pvalue > 0.05 {
        "MILD_DEPARTURE"
    } else if pvalue > 0.01 {
        "MODERATE_DEPARTURE"
    } else if pvalue > 0.001 {
        "NON_NORMAL"
    } else {
        "STRONGLY_NON_NORMAL"
    };
    JarqueBeraSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: log_rets.len(),
        skewness: skew,
        excess_kurtosis: kurt,
        jb_statistic: jb,
        jb_pvalue: pvalue,
        normal_label: label.into(),
        note: String::new(),
    }
}
