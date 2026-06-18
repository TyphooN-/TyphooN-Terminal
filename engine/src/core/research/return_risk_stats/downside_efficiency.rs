use super::*;

// Downside-volatility, Sharpe, efficiency, wick-bias, and volatility-of-volatility computes

/// DOWNVOL compute: semi-deviation + Sortino ratio.
pub fn compute_downvol_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DownsideVolSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return DownsideVolSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            sortino_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} returns", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let nf = n as f64;
    let mean: f64 = log_rets.iter().sum::<f64>() / nf;
    let mut down_sq = 0.0f64;
    let mut up_sq = 0.0f64;
    let mut total_sq = 0.0f64;
    for &r in &log_rets {
        let c = r - mean;
        total_sq += c * c;
        if r < 0.0 {
            down_sq += r * r;
        }
        if r > 0.0 {
            up_sq += r * r;
        }
    }
    let total_var = total_sq / nf;
    let down_dev = (down_sq / nf).sqrt();
    let up_dev = (up_sq / nf).sqrt();
    let sortino = if down_dev > f64::EPSILON {
        mean / down_dev
    } else {
        0.0
    };
    let sqrt_252 = (252.0f64).sqrt();
    let down_dev_ann = down_dev * sqrt_252;
    let sortino_ann = if down_dev_ann > f64::EPSILON {
        (mean * 252.0) / down_dev_ann
    } else {
        0.0
    };
    let downside_pct = if total_var > f64::EPSILON {
        (down_sq / nf) / total_var * 100.0
    } else {
        0.0
    };
    let label = if down_dev < f64::EPSILON && mean <= 0.0 {
        "INSUFFICIENT_DATA"
    } else if sortino_ann < -1.0 {
        "VERY_POOR"
    } else if sortino_ann < 0.0 {
        "POOR"
    } else if sortino_ann < 1.0 {
        "NEUTRAL"
    } else if sortino_ann < 2.0 {
        "GOOD"
    } else {
        "EXCELLENT"
    };
    DownsideVolSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: n,
        mean_log_return: mean,
        downside_dev: down_dev,
        downside_dev_ann: down_dev_ann,
        upside_dev: up_dev,
        sortino_ratio: sortino,
        sortino_ratio_ann: sortino_ann,
        downside_pct_of_total: downside_pct,
        sortino_label: label.into(),
        note: String::new(),
    }
}

/// SHARPR compute: Sharpe ratio over trailing window (rf = 0).
pub fn compute_sharpr_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> SharpeRatioSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return SharpeRatioSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            sharpe_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} returns", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let nf = n as f64;
    let mean: f64 = log_rets.iter().sum::<f64>() / nf;
    let var: f64 = log_rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / nf;
    let stdev = var.sqrt();
    if stdev < f64::EPSILON {
        return SharpeRatioSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: n,
            mean_log_return: mean,
            stdev_log_return: stdev,
            sharpe_label: "INSUFFICIENT_DATA".into(),
            note: "zero variance".into(),
            ..Default::default()
        };
    }
    let sharpe = mean / stdev;
    let sqrt_252 = (252.0f64).sqrt();
    let sharpe_ann = sharpe * sqrt_252;
    let mean_ann = mean * 252.0;
    let stdev_ann = stdev * sqrt_252;
    let label = if sharpe_ann < -0.5 {
        "POOR"
    } else if sharpe_ann < 0.5 {
        "BELOW_AVG"
    } else if sharpe_ann < 1.0 {
        "NEUTRAL"
    } else if sharpe_ann < 2.0 {
        "GOOD"
    } else {
        "EXCELLENT"
    };
    SharpeRatioSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: n,
        mean_log_return: mean,
        stdev_log_return: stdev,
        sharpe_ratio: sharpe,
        sharpe_ratio_ann: sharpe_ann,
        mean_return_ann: mean_ann,
        stdev_return_ann: stdev_ann,
        sharpe_label: label.into(),
        note: String::new(),
    }
}

/// EFFRATIO compute: Kaufman's efficiency ratio on closes.
pub fn compute_effratio_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> EfficiencyRatioSnapshot {
    let sym = symbol.to_uppercase();
    if bars.is_empty() {
        return EfficiencyRatioSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            efficiency_label: "INSUFFICIENT_DATA".into(),
            note: "no bars".into(),
            ..Default::default()
        };
    }
    let n = bars.len().min(253);
    let window = &bars[bars.len().saturating_sub(n)..];
    if window.len() < 30 {
        return EfficiencyRatioSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            efficiency_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} bars", window.len()),
            ..Default::default()
        };
    }
    let start_close = window.first().map(|b| b.close).unwrap_or(0.0);
    let end_close = window.last().map(|b| b.close).unwrap_or(0.0);
    if start_close <= 0.0 {
        return EfficiencyRatioSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            efficiency_label: "INSUFFICIENT_DATA".into(),
            note: "start close ≤ 0".into(),
            ..Default::default()
        };
    }
    let net = end_close - start_close;
    let net_pct = (end_close / start_close - 1.0) * 100.0;
    let sum_abs: f64 = window
        .windows(2)
        .map(|pair| (pair[1].close - pair[0].close).abs())
        .sum();
    if sum_abs < f64::EPSILON {
        return EfficiencyRatioSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            start_close,
            end_close,
            net_change: net,
            net_change_pct: net_pct,
            sum_abs_changes: sum_abs,
            efficiency_label: "INSUFFICIENT_DATA".into(),
            note: "flat window".into(),
            ..Default::default()
        };
    }
    let er = net.abs() / sum_abs;
    let signed_er = er * net.signum();
    let label = if er < 0.10 {
        "CHOP"
    } else if er < 0.25 {
        "NOISY"
    } else if er < 0.40 {
        "MIXED"
    } else if er < 0.60 {
        "TRENDING"
    } else {
        "STRONG_TREND"
    };
    EfficiencyRatioSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        start_close,
        end_close,
        net_change: net,
        net_change_pct: net_pct,
        sum_abs_changes: sum_abs,
        efficiency_ratio: er,
        signed_efficiency: signed_er,
        efficiency_label: label.into(),
        note: String::new(),
    }
}

/// WICKBIAS compute: upper vs lower wick asymmetry (requires open column).
pub fn compute_wickbias_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> WickBiasSnapshot {
    let sym = symbol.to_uppercase();
    if bars.is_empty() {
        return WickBiasSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bias_label: "INSUFFICIENT_DATA".into(),
            note: "no bars".into(),
            ..Default::default()
        };
    }
    let n = bars.len().min(253);
    let window = &bars[bars.len().saturating_sub(n)..];
    let mut uppers: Vec<f64> = Vec::with_capacity(window.len());
    let mut lowers: Vec<f64> = Vec::with_capacity(window.len());
    let mut bodies: Vec<f64> = Vec::with_capacity(window.len());
    for b in window {
        let range = b.high - b.low;
        if range <= f64::EPSILON {
            continue;
        }
        let body_top = b.open.max(b.close);
        let body_bot = b.open.min(b.close);
        let upper = (b.high - body_top) / range;
        let lower = (body_bot - b.low) / range;
        let body = (body_top - body_bot) / range;
        uppers.push(upper);
        lowers.push(lower);
        bodies.push(body);
    }
    if uppers.len() < 20 {
        return WickBiasSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: uppers.len(),
            bias_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} non-flat bars", uppers.len()),
            ..Default::default()
        };
    }
    let nf = uppers.len() as f64;
    let avg_upper: f64 = uppers.iter().sum::<f64>() / nf;
    let avg_lower: f64 = lowers.iter().sum::<f64>() / nf;
    let avg_body: f64 = bodies.iter().sum::<f64>() / nf;
    let median = |v: &mut Vec<f64>| -> f64 {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        v[v.len() / 2]
    };
    let mut up_copy = uppers.clone();
    let mut lo_copy = lowers.clone();
    let med_upper = median(&mut up_copy);
    let med_lower = median(&mut lo_copy);
    let bias = avg_lower - avg_upper;
    let label = if bias < -0.05 {
        "SELLER_REJECT"
    } else if bias < -0.02 {
        "SELLER_LEAN"
    } else if bias <= 0.02 {
        "NEUTRAL"
    } else if bias <= 0.05 {
        "BUYER_LEAN"
    } else {
        "BUYER_DEFEND"
    };
    WickBiasSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: uppers.len(),
        avg_upper_wick: avg_upper,
        avg_lower_wick: avg_lower,
        median_upper_wick: med_upper,
        median_lower_wick: med_lower,
        avg_body_share: avg_body,
        wick_bias_score: bias,
        bias_label: label.into(),
        note: String::new(),
    }
}

/// VOLOFVOL compute: stdev of rolling 20-day realized vol.
pub fn compute_volofvol_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> VolOfVolSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    const RV_WINDOW: usize = 20;
    if log_rets.len() < RV_WINDOW + 30 {
        return VolOfVolSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            cv_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} returns", log_rets.len()),
            ..Default::default()
        };
    }
    let mut rv: Vec<f64> = Vec::with_capacity(log_rets.len().saturating_sub(RV_WINDOW - 1));
    for i in (RV_WINDOW - 1)..log_rets.len() {
        let slice = &log_rets[i + 1 - RV_WINDOW..=i];
        let m: f64 = slice.iter().sum::<f64>() / (RV_WINDOW as f64);
        let v: f64 = slice.iter().map(|r| (r - m).powi(2)).sum::<f64>() / (RV_WINDOW as f64);
        rv.push(v.sqrt());
    }
    if rv.len() < 30 {
        return VolOfVolSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: rv.len(),
            cv_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} rv points", rv.len()),
            ..Default::default()
        };
    }
    let nf = rv.len() as f64;
    let mean_rv: f64 = rv.iter().sum::<f64>() / nf;
    let var_rv: f64 = rv.iter().map(|x| (x - mean_rv).powi(2)).sum::<f64>() / nf;
    let stdev_rv = var_rv.sqrt();
    let mut min_rv = f64::INFINITY;
    let mut max_rv = f64::NEG_INFINITY;
    for &x in &rv {
        if x < min_rv {
            min_rv = x;
        }
        if x > max_rv {
            max_rv = x;
        }
    }
    let latest_rv = *rv.last().unwrap_or(&0.0);
    let cv = if mean_rv > f64::EPSILON {
        stdev_rv / mean_rv
    } else {
        0.0
    };
    let label = if mean_rv < f64::EPSILON {
        "INSUFFICIENT_DATA"
    } else if cv < 0.15 {
        "STABLE"
    } else if cv < 0.25 {
        "MILD"
    } else if cv < 0.40 {
        "MODERATE"
    } else if cv < 0.60 {
        "UNSTABLE"
    } else {
        "CHAOTIC"
    };
    VolOfVolSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: rv.len(),
        mean_rv20: mean_rv,
        stdev_rv20: stdev_rv,
        min_rv20: min_rv,
        max_rv20: max_rv,
        latest_rv20: latest_rv,
        cv_rv20: cv,
        cv_label: label.into(),
        note: String::new(),
    }
}
