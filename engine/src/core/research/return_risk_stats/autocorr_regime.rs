use super::*;

// Autocorrelation, Hurst, hit-rate, gap asymmetry, and volatility-ratio computes

/// Helper: autocorrelation of a return series at a given lag, computed
/// via the standard estimator `sum((r_t - mean)(r_{t-k} - mean)) /
/// sum((r_t - mean)^2)`. Returns 0.0 when the series is too short
/// (<= lag) or the denominator is 0.
pub(super) fn acf_at_lag(rets: &[f64], lag: usize) -> f64 {
    if lag == 0 || rets.len() <= lag {
        return 0.0;
    }
    let n = rets.len() as f64;
    let mean: f64 = rets.iter().sum::<f64>() / n;
    let denom: f64 = rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>();
    if denom <= f64::EPSILON {
        return 0.0;
    }
    let num: f64 = (lag..rets.len())
        .map(|t| (rets[t] - mean) * (rets[t - lag] - mean))
        .sum();
    num / denom
}

/// AUTOCOR compute: autocorrelation of log returns at lags 1/5/10/20.
/// Labels from lag-1 ACF: strong mean-reversion, mean-reversion,
/// neutral, momentum, strong momentum.
pub fn compute_autocor_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> AutocorrelationSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return AutocorrelationSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            regime_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    let mean: f64 = log_rets.iter().sum::<f64>() / log_rets.len() as f64;
    let lag1 = acf_at_lag(&log_rets, 1);
    let lag5 = acf_at_lag(&log_rets, 5);
    let lag10 = acf_at_lag(&log_rets, 10);
    let lag20 = acf_at_lag(&log_rets, 20);
    let regime_label = if lag1 <= -0.15 {
        "STRONG_MEAN_REVERT"
    } else if lag1 <= -0.05 {
        "MEAN_REVERT"
    } else if lag1 < 0.05 {
        "NEUTRAL"
    } else if lag1 < 0.15 {
        "MOMENTUM"
    } else {
        "STRONG_MOMENTUM"
    };
    AutocorrelationSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        lag1_acf: lag1,
        lag5_acf: lag5,
        lag10_acf: lag10,
        lag20_acf: lag20,
        mean_log_return: mean,
        regime_label: regime_label.into(),
        note: String::new(),
    }
}

/// HURST compute: Hurst exponent via rescaled-range analysis.
/// Partitions the log return series into non-overlapping chunks of
/// size `scale`, computes R/S (range of cumulative deviations divided
/// by stdev) per chunk, averages across chunks, and regresses
/// `log(R/S_avg)` against `log(scale)`. The slope is H.
pub fn compute_hurst_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> HurstSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 40 {
        return HurstSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            memory_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    // Build candidate scales: powers-of-two-ish, bounded so we always get
    // at least 2 chunks per scale.
    let n = log_rets.len();
    let candidate_scales: Vec<usize> = [8, 12, 16, 24, 32, 48, 64, 96, 128]
        .into_iter()
        .filter(|&s| s <= n / 2)
        .collect();
    if candidate_scales.len() < 2 {
        return HurstSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            memory_label: "INSUFFICIENT_DATA".into(),
            note: "too few R/S scales".into(),
            ..Default::default()
        };
    }

    let mut xs: Vec<f64> = Vec::new();
    let mut ys: Vec<f64> = Vec::new();
    for &scale in &candidate_scales {
        let num_chunks = n / scale;
        if num_chunks == 0 {
            continue;
        }
        let mut rs_vals: Vec<f64> = Vec::with_capacity(num_chunks);
        for c in 0..num_chunks {
            let start = c * scale;
            let end = start + scale;
            let slice = &log_rets[start..end];
            let mean: f64 = slice.iter().sum::<f64>() / scale as f64;
            // Cumulative deviations from the chunk mean.
            let mut cum: Vec<f64> = Vec::with_capacity(scale);
            let mut running = 0.0;
            for r in slice {
                running += r - mean;
                cum.push(running);
            }
            let max_c = cum.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let min_c = cum.iter().cloned().fold(f64::INFINITY, f64::min);
            let range = max_c - min_c;
            let var: f64 = slice.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / scale as f64;
            let sd = var.sqrt();
            if sd > f64::EPSILON && range > 0.0 {
                rs_vals.push(range / sd);
            }
        }
        if rs_vals.is_empty() {
            continue;
        }
        let avg_rs: f64 = rs_vals.iter().sum::<f64>() / rs_vals.len() as f64;
        if avg_rs > 0.0 {
            xs.push((scale as f64).ln());
            ys.push(avg_rs.ln());
        }
    }
    if xs.len() < 2 {
        return HurstSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            memory_label: "INSUFFICIENT_DATA".into(),
            note: "R/S regression had < 2 points".into(),
            ..Default::default()
        };
    }
    // OLS slope.
    let np = xs.len() as f64;
    let mean_x: f64 = xs.iter().sum::<f64>() / np;
    let mean_y: f64 = ys.iter().sum::<f64>() / np;
    let mut num = 0.0;
    let mut den = 0.0;
    for i in 0..xs.len() {
        let dx = xs[i] - mean_x;
        num += dx * (ys[i] - mean_y);
        den += dx * dx;
    }
    let h = if den > f64::EPSILON { num / den } else { 0.5 };
    let label = if h < 0.35 {
        "STRONG_MEAN_REVERT"
    } else if h < 0.45 {
        "MEAN_REVERT"
    } else if h < 0.55 {
        "RANDOM_WALK"
    } else if h < 0.65 {
        "PERSISTENT"
    } else {
        "STRONG_PERSISTENT"
    };
    HurstSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        hurst_exponent: h,
        scales_used: xs.len(),
        min_scale: *candidate_scales.iter().min().unwrap_or(&0),
        max_scale: *candidate_scales.iter().max().unwrap_or(&0),
        memory_label: label.into(),
        note: String::new(),
    }
}

/// HITRATE compute: share of positive-return bars over 5/20/60/252
/// trailing windows. Label combines the 20d and 60d hit rates: both
/// above 55% → BULLISH, both below 45% → BEARISH, otherwise NEUTRAL /
/// WEAK_BULLISH / WEAK_BEARISH based on the 20d alone.
pub fn compute_hitrate_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> HitRateSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 20 {
        return HitRateSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            hit_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    fn hit_over(rets: &[f64], take: usize) -> f64 {
        let start = rets.len().saturating_sub(take);
        let slice = &rets[start..];
        if slice.is_empty() {
            return 0.0;
        }
        let up = slice.iter().filter(|&&r| r > 0.0).count() as f64;
        up / slice.len() as f64
    }
    let h5 = hit_over(&log_rets, 5) * 100.0;
    let h20 = hit_over(&log_rets, 20) * 100.0;
    let h60 = hit_over(&log_rets, 60) * 100.0;
    let h252 = hit_over(&log_rets, 252) * 100.0;
    let up = log_rets.iter().filter(|&&r| r > 0.0).count();
    let down = log_rets.iter().filter(|&&r| r < 0.0).count();
    let flat = log_rets.len() - up - down;

    let label = if h20 >= 60.0 && h60 >= 55.0 {
        "BULLISH"
    } else if h20 >= 55.0 {
        "WEAK_BULLISH"
    } else if h20 <= 40.0 && h60 <= 45.0 {
        "BEARISH"
    } else if h20 <= 45.0 {
        "WEAK_BEARISH"
    } else {
        "NEUTRAL"
    };
    HitRateSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        hitrate_5d: h5,
        hitrate_20d: h20,
        hitrate_60d: h60,
        hitrate_252d: h252,
        up_days: up,
        down_days: down,
        flat_days: flat,
        hit_label: label.into(),
        note: String::new(),
    }
}

/// GLASYM compute: average and median magnitude of up vs down days.
/// Magnitudes are expressed as percent log returns × 100.
pub fn compute_glasym_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> GainLossAsymmetrySnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 20 {
        return GainLossAsymmetrySnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            asymmetry_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    let mut ups: Vec<f64> = log_rets
        .iter()
        .filter(|&&r| r > 0.0)
        .map(|r| r * 100.0)
        .collect();
    let mut downs: Vec<f64> = log_rets
        .iter()
        .filter(|&&r| r < 0.0)
        .map(|r| -r * 100.0)
        .collect();
    if ups.is_empty() || downs.is_empty() {
        return GainLossAsymmetrySnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            asymmetry_label: "INSUFFICIENT_DATA".into(),
            note: "all-up or all-down window".into(),
            up_days: ups.len(),
            down_days: downs.len(),
            ..Default::default()
        };
    }
    let avg_up: f64 = ups.iter().sum::<f64>() / ups.len() as f64;
    let avg_down: f64 = downs.iter().sum::<f64>() / downs.len() as f64;
    ups.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    downs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_up = quantile_f64(&ups, 0.5);
    let median_down = quantile_f64(&downs, 0.5);
    let ratio = if avg_down > f64::EPSILON {
        avg_up / avg_down
    } else {
        0.0
    };
    let label = if ratio <= 0.75 {
        "DOWNSIDE_HEAVY"
    } else if ratio <= 0.9 {
        "SLIGHT_DOWNSIDE"
    } else if ratio < 1.1 {
        "BALANCED"
    } else if ratio < 1.3 {
        "SLIGHT_UPSIDE"
    } else {
        "UPSIDE_HEAVY"
    };
    GainLossAsymmetrySnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        avg_up_pct: avg_up,
        avg_down_pct: avg_down,
        median_up_pct: median_up,
        median_down_pct: median_down,
        magnitude_ratio: ratio,
        up_days: ups.len(),
        down_days: downs.len(),
        asymmetry_label: label.into(),
        note: String::new(),
    }
}

/// VOLRATIO compute: up-day vs down-day volume summary over the
/// trailing 253-session window. Emits INSUFFICIENT_DATA when the HP
/// cache was populated without volume (all zeros).
pub fn compute_volratio_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> VolumeRatioSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let window: Vec<&HistoricalPriceRow> = sorted.iter().rev().take(253).rev().copied().collect();
    if window.len() < 20 {
        return VolumeRatioSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            flow_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} bars", window.len()),
            ..Default::default()
        };
    }
    let mut up_vols: Vec<f64> = Vec::new();
    let mut down_vols: Vec<f64> = Vec::new();
    for w in window.windows(2) {
        let prev = w[0].close;
        let curr = w[1].close;
        let vol = w[1].volume;
        if prev > 0.0 && curr > 0.0 && vol > 0.0 {
            let r = (curr / prev).ln();
            if r > 0.0 {
                up_vols.push(vol);
            } else if r < 0.0 {
                down_vols.push(vol);
            }
        }
    }
    if up_vols.is_empty() || down_vols.is_empty() {
        return VolumeRatioSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            flow_label: "INSUFFICIENT_DATA".into(),
            note: "HP cache lacks volume or one side empty".into(),
            up_days: up_vols.len(),
            down_days: down_vols.len(),
            ..Default::default()
        };
    }
    let avg_up: f64 = up_vols.iter().sum::<f64>() / up_vols.len() as f64;
    let avg_down: f64 = down_vols.iter().sum::<f64>() / down_vols.len() as f64;
    let max_up = up_vols.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let max_down = down_vols.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let mut sorted_up = up_vols.clone();
    let mut sorted_down = down_vols.clone();
    sorted_up.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    sorted_down.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_up = quantile_f64(&sorted_up, 0.5);
    let median_down = quantile_f64(&sorted_down, 0.5);
    let ratio = if avg_down > f64::EPSILON {
        avg_up / avg_down
    } else {
        0.0
    };
    let label = if ratio <= 0.8 {
        "DISTRIBUTION"
    } else if ratio <= 0.95 {
        "SLIGHT_DISTRIBUTION"
    } else if ratio < 1.05 {
        "NEUTRAL"
    } else if ratio < 1.25 {
        "SLIGHT_ACCUMULATION"
    } else {
        "ACCUMULATION"
    };
    VolumeRatioSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        avg_up_volume: avg_up,
        avg_down_volume: avg_down,
        median_up_volume: median_up,
        median_down_volume: median_down,
        up_down_volume_ratio: ratio,
        max_up_volume: max_up,
        max_down_volume: max_down,
        up_days: up_vols.len(),
        down_days: down_vols.len(),
        flow_label: label.into(),
        note: String::new(),
    }
}
