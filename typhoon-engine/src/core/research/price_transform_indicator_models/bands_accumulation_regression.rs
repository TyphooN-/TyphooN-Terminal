use super::*;

// Bands, accumulation/distribution, rolling sum, and regression-intercept transforms

/// Compute SMA + sample stddev over a window ending at end_idx.
fn sma_stddev(sorted: &[&HistoricalPriceRow], end_idx: usize, period: usize) -> (f64, f64) {
    let start = end_idx + 1 - period;
    let mut sum = 0.0f64;
    for i in start..=end_idx {
        sum += sorted[i].close;
    }
    let mean = sum / period as f64;
    let mut ss = 0.0f64;
    for i in start..=end_idx {
        let d = sorted[i].close - mean;
        ss += d * d;
    }
    let var = ss / period as f64; // TA-Lib uses population variance for BBANDS
    (mean, var.max(0.0).sqrt())
}

/// Cumulative Chaikin A/D line across all bars (same ordering as input).
fn ad_line(sorted: &[&HistoricalPriceRow]) -> Vec<f64> {
    let mut out = Vec::with_capacity(sorted.len());
    let mut ad = 0.0f64;
    for b in sorted.iter() {
        let hl = b.high - b.low;
        let mf = if hl > 0.0 {
            ((b.close - b.low) - (b.high - b.close)) / hl
        } else {
            0.0
        };
        ad += mf * b.volume as f64;
        out.push(ad);
    }
    out
}

/// Least-squares slope of y over x = [0..n) for the last `period` samples.
fn last_window_slope(values: &[f64], period: usize) -> f64 {
    let n = values.len();
    if n < period || period < 2 {
        return 0.0;
    }
    let start = n - period;
    let pf = period as f64;
    let mean_x = (period as f64 - 1.0) / 2.0;
    let mut mean_y = 0.0f64;
    for i in start..n {
        mean_y += values[i];
    }
    mean_y /= pf;
    let mut num = 0.0f64;
    let mut den = 0.0f64;
    for i in 0..period {
        let dx = i as f64 - mean_x;
        let dy = values[start + i] - mean_y;
        num += dx * dy;
        den += dx * dx;
    }
    if den == 0.0 { 0.0 } else { num / den }
}

pub fn compute_bbands_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> BbandsSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 20usize;
    let num_std = 2.0f64;
    let min_bars = period + 1;
    if n < min_bars {
        return BbandsSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            num_std,
            bbands_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let (mid_now, sd_now) = sma_stddev(&sorted, n - 1, period);
    let (mid_prev, sd_prev) = sma_stddev(&sorted, n - 2, period);
    let upper = mid_now + num_std * sd_now;
    let lower = mid_now - num_std * sd_now;
    let upper_p = mid_prev + num_std * sd_prev;
    let lower_p = mid_prev - num_std * sd_prev;
    let close = sorted[n - 1].close;
    let width = upper - lower;
    let pct_b = if width > 0.0 {
        100.0 * (close - lower) / width
    } else {
        50.0
    };
    let bandwidth = if mid_now > 0.0 {
        100.0 * width / mid_now
    } else {
        0.0
    };
    let label = if close > upper {
        "ABOVE_UPPER"
    } else if close >= mid_now {
        "UPPER_HALF"
    } else if close >= lower {
        "LOWER_HALF"
    } else {
        "BELOW_LOWER"
    };
    BbandsSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        num_std,
        upper,
        middle: mid_now,
        lower,
        upper_prev: upper_p,
        middle_prev: mid_prev,
        lower_prev: lower_p,
        last_close: close,
        pct_b,
        bandwidth,
        bbands_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_ad_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> AdSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let slope_window = 10usize;
    let min_bars = slope_window + 2;
    if n < min_bars {
        return AdSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            ad_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let ad = ad_line(&sorted);
    let ad_now = ad[n - 1];
    let ad_prev = ad[n - 2];
    let delta = ad_now - ad_prev;
    let slope = last_window_slope(&ad, slope_window);
    let abs_slope = slope.abs();
    let mean_ad_abs = ad.iter().map(|v| v.abs()).sum::<f64>() / n as f64;
    let rel = if mean_ad_abs > 0.0 {
        abs_slope / mean_ad_abs
    } else {
        0.0
    };
    let label = if slope > 0.0 && rel >= 0.05 {
        "STRONG_ACCUM"
    } else if slope > 0.0 && rel >= 0.01 {
        "ACCUM"
    } else if slope < 0.0 && rel >= 0.05 {
        "STRONG_DIST"
    } else if slope < 0.0 && rel >= 0.01 {
        "DIST"
    } else {
        "FLAT"
    };
    AdSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        ad: ad_now,
        ad_prev,
        ad_delta: delta,
        ad_slope: slope,
        last_close: sorted[n - 1].close,
        ad_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_adosc_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> AdoscSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let fast = 3usize;
    let slow = 10usize;
    let min_bars = slow + 2;
    if n < min_bars {
        return AdoscSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            fast_period: fast,
            slow_period: slow,
            adosc_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let ad = ad_line(&sorted);
    let fast_ema = ema_series(&ad, fast);
    let slow_ema = ema_series(&ad, slow);
    let adosc_now = fast_ema[n - 1] - slow_ema[n - 1];
    let adosc_prev = fast_ema[n - 2] - slow_ema[n - 2];
    let mean_ad_abs = ad.iter().map(|v| v.abs()).sum::<f64>() / n as f64;
    let rel = if mean_ad_abs > 0.0 {
        adosc_now.abs() / mean_ad_abs
    } else {
        0.0
    };
    let label = if adosc_now > 0.0 && rel >= 0.1 {
        "STRONG_BULL"
    } else if adosc_now > 0.0 && rel >= 0.02 {
        "BULL"
    } else if adosc_now < 0.0 && rel >= 0.1 {
        "STRONG_BEAR"
    } else if adosc_now < 0.0 && rel >= 0.02 {
        "BEAR"
    } else {
        "FLAT"
    };
    AdoscSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        fast_period: fast,
        slow_period: slow,
        adosc: adosc_now,
        adosc_prev,
        last_close: sorted[n - 1].close,
        ad_ref: ad[n - 1],
        adosc_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_sum_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> SumSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 30usize;
    let min_bars = period + 1;
    if n < min_bars {
        return SumSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            sum_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let mut sum_now = 0.0f64;
    for i in (n - period)..n {
        sum_now += sorted[i].close;
    }
    let mut sum_prev = 0.0f64;
    for i in (n - 1 - period)..(n - 1) {
        sum_prev += sorted[i].close;
    }
    let delta = sum_now - sum_prev;
    let pct = if sum_prev != 0.0 {
        100.0 * delta / sum_prev
    } else {
        0.0
    };
    let label = if pct >= 1.0 {
        "STRONG_UP"
    } else if pct >= 0.2 {
        "UP"
    } else if pct <= -1.0 {
        "STRONG_DOWN"
    } else if pct <= -0.2 {
        "DOWN"
    } else {
        "FLAT"
    };
    SumSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        sum: sum_now,
        sum_prev,
        sum_delta: delta,
        sum_pct_change: pct,
        last_close: sorted[n - 1].close,
        sum_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_linearreg_intercept_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> LinearRegInterceptSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 14usize;
    let min_bars = period + 1;
    if n < min_bars {
        return LinearRegInterceptSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            linreg_intercept_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    // Compute slope + intercept on the last `period` closes against x = [0..period).
    let compute = |end_idx: usize| -> (f64, f64) {
        let start = end_idx + 1 - period;
        let pf = period as f64;
        let mean_x = (pf - 1.0) / 2.0;
        let mut mean_y = 0.0f64;
        for i in start..=end_idx {
            mean_y += sorted[i].close;
        }
        mean_y /= pf;
        let mut num = 0.0f64;
        let mut den = 0.0f64;
        for j in 0..period {
            let dx = j as f64 - mean_x;
            let dy = sorted[start + j].close - mean_y;
            num += dx * dy;
            den += dx * dx;
        }
        let m = if den == 0.0 { 0.0 } else { num / den };
        let b = mean_y - m * mean_x;
        (m, b)
    };
    let (slope_now, intercept_now) = compute(n - 1);
    let (_, intercept_prev) = compute(n - 2);
    let close = sorted[n - 1].close;
    let drift = close - intercept_now;
    let drift_pct = if intercept_now != 0.0 {
        100.0 * drift / intercept_now
    } else {
        0.0
    };
    let label = if drift_pct >= 5.0 {
        "STRONG_ADVANCE"
    } else if drift_pct >= 1.0 {
        "ADVANCE"
    } else if drift_pct <= -5.0 {
        "STRONG_DECLINE"
    } else if drift_pct <= -1.0 {
        "DECLINE"
    } else {
        "FLAT"
    };
    LinearRegInterceptSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        intercept: intercept_now,
        intercept_prev,
        slope: slope_now,
        last_close: close,
        drift,
        drift_pct,
        linreg_intercept_label: label.into(),
        note: String::new(),
    }
}
