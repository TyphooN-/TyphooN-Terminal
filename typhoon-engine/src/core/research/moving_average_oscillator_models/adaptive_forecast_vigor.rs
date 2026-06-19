use super::*;

// Arnaud Legoux, zero-lag EMA, Elder Ray, time-series forecast, and relative-vigor models

pub fn compute_alma_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> AlmaSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 20usize;
    let offset = 0.85f64;
    let sigma = 6.0f64;
    if n < length + 1 {
        return AlmaSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            offset,
            sigma,
            alma_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length + 1, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let m = offset * (length as f64 - 1.0);
    let s = length as f64 / sigma;
    let weights: Vec<f64> = (0..length)
        .map(|i| {
            let z = (i as f64 - m) / s;
            (-0.5 * z * z).exp()
        })
        .collect();
    let w_sum: f64 = weights.iter().sum();
    let compute_at = |end: usize| -> f64 {
        let start = end + 1 - length;
        let mut acc = 0.0;
        for i in 0..length {
            acc += weights[i] * closes[start + i];
        }
        acc / w_sum
    };
    let alma_value = compute_at(n - 1);
    let alma_prev = compute_at(n - 2);
    let last_close = closes[n - 1];
    let dev = if alma_value.abs() > 1e-12 {
        (last_close - alma_value) / alma_value * 100.0
    } else {
        0.0
    };
    let label = if dev > 2.0 {
        "STRONG_BULL"
    } else if dev > 0.0 {
        "BULL"
    } else if dev < -2.0 {
        "STRONG_BEAR"
    } else if dev < 0.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    AlmaSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        offset,
        sigma,
        alma_value,
        alma_prev,
        deviation_pct: dev,
        last_close,
        alma_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_zlema_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> ZlemaSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 20usize;
    let lag = (length - 1) / 2; // 9
    let min_bars = length + lag + 2;
    if n < min_bars {
        return ZlemaSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            lag_shift: lag,
            zlema_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    // De-lagged series: 2*price[i] - price[i-lag] for i >= lag; else price[i]
    let delagged: Vec<f64> = (0..n)
        .map(|i| {
            if i >= lag {
                2.0 * closes[i] - closes[i - lag]
            } else {
                closes[i]
            }
        })
        .collect();
    let zl = ema_series(&delagged, length);
    let zlema_value = zl[n - 1];
    let zlema_prev = zl[n - 2];
    let last_close = closes[n - 1];
    let dev = if zlema_value.abs() > 1e-12 {
        (last_close - zlema_value) / zlema_value * 100.0
    } else {
        0.0
    };
    let label = if dev > 2.0 {
        "STRONG_BULL"
    } else if dev > 0.0 {
        "BULL"
    } else if dev < -2.0 {
        "STRONG_BEAR"
    } else if dev < 0.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    ZlemaSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        lag_shift: lag,
        zlema_value,
        zlema_prev,
        deviation_pct: dev,
        last_close,
        zlema_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_elderray_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> ElderRaySnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let ema_length = 13usize;
    let min_bars = ema_length + 2;
    if n < min_bars {
        return ElderRaySnapshot {
            symbol: sym,
            as_of: as_of.into(),
            ema_length,
            elder_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let ema = ema_series(&closes, ema_length);
    let ema13 = ema[n - 1];
    let ema13_prev = ema[n - 2];
    let bull_power = sorted[n - 1].high - ema13;
    let bull_power_prev = sorted[n - 2].high - ema13_prev;
    let bear_power = sorted[n - 1].low - ema13;
    let bear_power_prev = sorted[n - 2].low - ema13_prev;
    let last_close = closes[n - 1];
    let ema_rising = ema13 > ema13_prev;
    let ema_falling = ema13 < ema13_prev;
    let label = if bull_power > 0.0 && bear_power > 0.0 && ema_rising {
        "STRONG_BULL"
    } else if bull_power > 0.0 && ema_rising {
        "BULL"
    } else if bull_power < 0.0 && bear_power < 0.0 && ema_falling {
        "STRONG_BEAR"
    } else if bear_power < 0.0 && ema_falling {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    ElderRaySnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        ema_length,
        ema13,
        ema13_prev,
        bull_power,
        bull_power_prev,
        bear_power,
        bear_power_prev,
        last_close,
        elder_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_tsf_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> TsfSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 20usize;
    if n < length {
        return TsfSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            tsf_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    // OLS on last `length` bars with t = 0..length-1
    let nf = length as f64;
    let window = &closes[n - length..];
    let mut sum_t = 0.0;
    let mut sum_y = 0.0;
    let mut sum_tt = 0.0;
    let mut sum_ty = 0.0;
    for (i, &y) in window.iter().enumerate() {
        let t = i as f64;
        sum_t += t;
        sum_y += y;
        sum_tt += t * t;
        sum_ty += t * y;
    }
    let mean_t = sum_t / nf;
    let mean_y = sum_y / nf;
    let sxx = sum_tt - nf * mean_t * mean_t;
    let sxy = sum_ty - nf * mean_t * mean_y;
    let slope = if sxx.abs() > 1e-12 { sxy / sxx } else { 0.0 };
    let intercept = mean_y - slope * mean_t;
    // Forecast one bar forward: t = length (next bar after window)
    let forecast_value = slope * nf + intercept;
    let last_close = closes[n - 1];
    // R² from residuals
    let mut ss_res = 0.0;
    let mut ss_tot = 0.0;
    for (i, &y) in window.iter().enumerate() {
        let y_hat = slope * (i as f64) + intercept;
        ss_res += (y - y_hat).powi(2);
        ss_tot += (y - mean_y).powi(2);
    }
    let r_squared = if ss_tot > 1e-12 {
        1.0 - ss_res / ss_tot
    } else {
        0.0
    };
    let dev_pct = if last_close.abs() > 1e-12 {
        (forecast_value - last_close) / last_close * 100.0
    } else {
        0.0
    };
    let label = if dev_pct.abs() < 0.1 {
        "FLAT"
    } else if forecast_value > last_close && slope > 0.0 {
        "LEADING_UP"
    } else if forecast_value > last_close {
        "LAGGING_UP"
    } else if forecast_value < last_close && slope < 0.0 {
        "LEADING_DOWN"
    } else {
        "LAGGING_DOWN"
    };
    TsfSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        slope,
        intercept,
        forecast_value,
        last_close,
        forecast_deviation_pct: dev_pct,
        r_squared,
        tsf_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_rvi_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> RviSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 10usize;
    // Need: 3 bars for the triangular weighting lookback + `length` bars for the SMA + 3 signal bars + 1 prev
    let min_bars = length + 3 + 4;
    if n < min_bars {
        return RviSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            rvi_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    // Per-bar raw numerator/denominator with triangular weighting back 0..3
    let tri_num = |i: usize| -> f64 {
        let co = |k: usize| sorted[k].close - sorted[k].open;
        co(i) + 2.0 * co(i - 1) + 2.0 * co(i - 2) + co(i - 3)
    };
    let tri_den = |i: usize| -> f64 {
        let hl = |k: usize| sorted[k].high - sorted[k].low;
        hl(i) + 2.0 * hl(i - 1) + 2.0 * hl(i - 2) + hl(i - 3)
    };
    // SMA(length) over tri values
    let rvi_at = |end: usize| -> f64 {
        let mut num_sum = 0.0;
        let mut den_sum = 0.0;
        for k in (end + 1 - length)..=end {
            num_sum += tri_num(k);
            den_sum += tri_den(k);
        }
        if den_sum.abs() > 1e-12 {
            num_sum / den_sum
        } else {
            0.0
        }
    };
    // Need RVI at the last 4 bars (for signal[N] and signal[N-1])
    let end = n - 1;
    let rvi_0 = rvi_at(end);
    let rvi_1 = rvi_at(end - 1);
    let rvi_2 = rvi_at(end - 2);
    let rvi_3 = rvi_at(end - 3);
    let rvi_4 = rvi_at(end - 4);
    let signal_0 = (rvi_0 + 2.0 * rvi_1 + 2.0 * rvi_2 + rvi_3) / 6.0;
    let signal_1 = (rvi_1 + 2.0 * rvi_2 + 2.0 * rvi_3 + rvi_4) / 6.0;
    let last_close = sorted[end].close;
    let cross_up = rvi_1 <= signal_1 && rvi_0 > signal_0;
    let cross_down = rvi_1 >= signal_1 && rvi_0 < signal_0;
    let label = if cross_up {
        "BULL_CROSS"
    } else if cross_down {
        "BEAR_CROSS"
    } else if rvi_0 > signal_0 {
        "BULL"
    } else if rvi_0 < signal_0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    RviSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        rvi_value: rvi_0,
        rvi_prev: rvi_1,
        signal_value: signal_0,
        signal_prev: signal_1,
        last_close,
        rvi_label: label.into(),
        note: String::new(),
    }
}
