use super::*;

/// RETSKEW compute: skewness of daily log returns over the trailing 253
/// sessions. Uses Fisher-Pearson (sample) skew with N denominator to match
/// RVCONE's stdev convention.
pub fn compute_retskew_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> ReturnSkewnessSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 20 {
        return ReturnSkewnessSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            skew_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len() as f64;
    let mean: f64 = log_rets.iter().sum::<f64>() / n;
    let var: f64 = log_rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n;
    let stdev = var.sqrt();
    let skew = if stdev > 0.0 {
        let m3: f64 = log_rets.iter().map(|r| (r - mean).powi(3)).sum::<f64>() / n;
        m3 / stdev.powi(3)
    } else {
        0.0
    };
    let positive = log_rets.iter().filter(|&&r| r > 0.0).count() as f64;
    let positive_pct = (positive / n) * 100.0;
    let largest_up = log_rets.iter().cloned().fold(f64::NEG_INFINITY, f64::max) * 100.0;
    let largest_down = log_rets.iter().cloned().fold(f64::INFINITY, f64::min) * 100.0;
    let skew_label = if skew <= -1.0 {
        "STRONG_LEFT"
    } else if skew <= -0.3 {
        "LEFT"
    } else if skew < 0.3 {
        "SYMMETRIC"
    } else if skew < 1.0 {
        "RIGHT"
    } else {
        "STRONG_RIGHT"
    };
    ReturnSkewnessSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        mean_log_return: mean,
        stdev_log_return: stdev,
        skewness: skew,
        positive_return_pct: positive_pct,
        largest_up_pct: largest_up,
        largest_down_pct: largest_down,
        skew_label: skew_label.into(),
        note: String::new(),
    }
}

/// RETKURT compute: excess kurtosis of daily log returns over trailing 253
/// sessions. Counts 2-sigma and 3-sigma outliers for a non-parametric fat-
/// tail check alongside the moment-based number.
pub fn compute_retkurt_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> ReturnKurtosisSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 20 {
        return ReturnKurtosisSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            kurt_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len() as f64;
    let mean: f64 = log_rets.iter().sum::<f64>() / n;
    let var: f64 = log_rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n;
    let stdev = var.sqrt();
    let excess = if stdev > 0.0 {
        let m4: f64 = log_rets.iter().map(|r| (r - mean).powi(4)).sum::<f64>() / n;
        (m4 / stdev.powi(4)) - 3.0
    } else {
        0.0
    };
    let (out2, out3) = if stdev > 0.0 {
        let mut c2 = 0usize;
        let mut c3 = 0usize;
        for r in &log_rets {
            let z = (r - mean).abs() / stdev;
            if z > 2.0 {
                c2 += 1;
            }
            if z > 3.0 {
                c3 += 1;
            }
        }
        (c2, c3)
    } else {
        (0, 0)
    };
    let out2_pct = (out2 as f64 / n) * 100.0;
    let kurt_label = if excess <= -0.5 {
        "PLATYKURTIC"
    } else if excess < 1.0 {
        "NORMAL"
    } else if excess < 3.0 {
        "MILD_FAT"
    } else if excess < 6.0 {
        "FAT"
    } else {
        "EXTREME_FAT"
    };
    ReturnKurtosisSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        mean_log_return: mean,
        stdev_log_return: stdev,
        excess_kurtosis: excess,
        outlier_2sigma_count: out2,
        outlier_3sigma_count: out3,
        outlier_2sigma_pct: out2_pct,
        kurt_label: kurt_label.into(),
        note: String::new(),
    }
}

/// TAILR compute: 95/5 and 99/1 tail ratios over trailing 253 sessions.
/// Non-parametric counterpart to RETSKEW.
pub fn compute_tailr_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> TailRatioSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 20 {
        return TailRatioSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            bias_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    let pct_returns: Vec<f64> = log_rets.iter().map(|r| r * 100.0).collect();
    let mut sorted = pct_returns.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p95 = quantile_f64(&sorted, 0.95);
    let p05 = quantile_f64(&sorted, 0.05);
    let p99 = quantile_f64(&sorted, 0.99);
    let p01 = quantile_f64(&sorted, 0.01);
    let tail_ratio = if p05.abs() > f64::EPSILON {
        p95 / p05.abs()
    } else {
        0.0
    };
    let tail_ratio_99_01 = if p01.abs() > f64::EPSILON {
        p99 / p01.abs()
    } else {
        0.0
    };
    let bias_label = if tail_ratio <= 0.6 {
        "DOWNSIDE_HEAVY"
    } else if tail_ratio <= 0.85 {
        "SLIGHT_DOWNSIDE"
    } else if tail_ratio < 1.15 {
        "BALANCED"
    } else if tail_ratio < 1.4 {
        "SLIGHT_UPSIDE"
    } else {
        "UPSIDE_HEAVY"
    };
    TailRatioSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        pct_95_return: p95,
        pct_05_return: p05,
        pct_99_return: p99,
        pct_01_return: p01,
        tail_ratio,
        tail_ratio_99_01,
        bias_label: bias_label.into(),
        note: String::new(),
    }
}

/// RUNLEN compute: up/down day run length statistics over trailing 253
/// sessions. Uses sign of log return (0 → flat, included in neither run).
pub fn compute_runlen_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> RunLengthSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 20 {
        return RunLengthSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            trend_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    let mut up_runs: Vec<usize> = Vec::new();
    let mut down_runs: Vec<usize> = Vec::new();
    let mut longest_up = 0usize;
    let mut longest_down = 0usize;
    let mut cur_up = 0usize;
    let mut cur_down = 0usize;
    for r in &log_rets {
        if *r > 0.0 {
            if cur_down > 0 {
                down_runs.push(cur_down);
                if cur_down > longest_down {
                    longest_down = cur_down;
                }
                cur_down = 0;
            }
            cur_up += 1;
        } else if *r < 0.0 {
            if cur_up > 0 {
                up_runs.push(cur_up);
                if cur_up > longest_up {
                    longest_up = cur_up;
                }
                cur_up = 0;
            }
            cur_down += 1;
        } else {
            if cur_up > 0 {
                up_runs.push(cur_up);
                if cur_up > longest_up {
                    longest_up = cur_up;
                }
                cur_up = 0;
            }
            if cur_down > 0 {
                down_runs.push(cur_down);
                if cur_down > longest_down {
                    longest_down = cur_down;
                }
                cur_down = 0;
            }
        }
    }
    // Tail: whichever run is still in progress is the "current" run.
    let current_run: i32 = if cur_up > 0 {
        up_runs.push(cur_up);
        if cur_up > longest_up {
            longest_up = cur_up;
        }
        cur_up as i32
    } else if cur_down > 0 {
        down_runs.push(cur_down);
        if cur_down > longest_down {
            longest_down = cur_down;
        }
        -(cur_down as i32)
    } else {
        0
    };
    let avg_up = if up_runs.is_empty() {
        0.0
    } else {
        up_runs.iter().sum::<usize>() as f64 / up_runs.len() as f64
    };
    let avg_down = if down_runs.is_empty() {
        0.0
    } else {
        down_runs.iter().sum::<usize>() as f64 / down_runs.len() as f64
    };
    let avg_run = (avg_up + avg_down) / 2.0;
    let longest_any = longest_up.max(longest_down) as f64;
    // Label combines avg run length and longest run length.
    let trend_label = if avg_run < 1.4 && longest_any < 4.0 {
        "CHOPPY"
    } else if avg_run < 1.7 && longest_any < 6.0 {
        "MIXED"
    } else if avg_run < 2.2 || longest_any < 8.0 {
        "TRENDING"
    } else {
        "STRONG_TRENDING"
    };
    RunLengthSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        avg_up_run: avg_up,
        avg_down_run: avg_down,
        longest_up_run: longest_up,
        longest_down_run: longest_down,
        up_runs_count: up_runs.len(),
        down_runs_count: down_runs.len(),
        current_run_length: current_run,
        trend_label: trend_label.into(),
        note: String::new(),
    }
}

/// DAYRANGE compute: average (high-low)/close ratio over 60d vs 252d
/// baseline. Compression ratio < 1 → tight; > 1 → expanded.
pub fn compute_dayrange_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DailyRangeSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let window: Vec<&HistoricalPriceRow> = sorted.iter().rev().take(253).rev().copied().collect();
    if window.len() < 20 {
        return DailyRangeSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            range_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} bars", window.len()),
            ..Default::default()
        };
    }
    // Per-bar range ratio.
    let ratios: Vec<f64> = window
        .iter()
        .filter(|r| r.close > 0.0 && r.high >= r.low)
        .map(|r| ((r.high - r.low) / r.close) * 100.0)
        .collect();
    if ratios.len() < 20 {
        return DailyRangeSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            range_label: "INSUFFICIENT_DATA".into(),
            note: "insufficient valid bars".into(),
            ..Default::default()
        };
    }
    let avg_all: f64 = ratios.iter().sum::<f64>() / ratios.len() as f64;
    let take60 = ratios.len().min(60);
    let slice60 = &ratios[ratios.len() - take60..];
    let avg60: f64 = slice60.iter().sum::<f64>() / take60 as f64;
    let latest = *ratios.last().unwrap();
    let widest = ratios.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let narrowest = ratios.iter().cloned().fold(f64::INFINITY, f64::min);
    let compression = if avg_all > f64::EPSILON {
        avg60 / avg_all
    } else {
        1.0
    };
    let range_label = if compression <= 0.75 {
        "TIGHT"
    } else if compression <= 0.9 {
        "COMPRESSED"
    } else if compression < 1.1 {
        "NORMAL"
    } else if compression < 1.35 {
        "EXPANDED"
    } else {
        "VERY_EXPANDED"
    };
    DailyRangeSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        avg_range_60_pct: avg60,
        avg_range_252_pct: avg_all,
        latest_range_pct: latest,
        compression_ratio: compression,
        widest_range_pct: widest,
        narrowest_range_pct: narrowest,
        range_label: range_label.into(),
        note: String::new(),
    }
}
