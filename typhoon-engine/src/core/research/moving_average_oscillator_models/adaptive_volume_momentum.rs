use super::*;

// Triangular, T3, VIDYA, stochastic-momentum, and price-volume-trend models

pub fn compute_trima_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> TrimaSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 20usize;
    let inner = length / 2 + 1; // SMA of SMA with (N/2 + 1) windows
    let min_bars = length + inner;
    if n < min_bars {
        return TrimaSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            trima_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let s1 = sma_series(&closes, inner);
    let s2 = sma_series(&s1, inner);
    let trima_value = s2[n - 1];
    let trima_prev = s2[n - 2];
    let last_close = closes[n - 1];
    let dev = if trima_value.abs() > 1e-12 {
        (last_close - trima_value) / trima_value * 100.0
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
    TrimaSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        trima_value,
        trima_prev,
        deviation_pct: dev,
        last_close,
        trima_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_t3_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> T3Snapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 20usize;
    let v = 0.7f64;
    let min_bars = length * 6 / 5; // rough warmup floor; EMA stabilises
    if n < min_bars.max(length + 2) {
        return T3Snapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            v_factor: v,
            t3_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars.max(length + 2), n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let e1 = ema_series(&closes, length);
    let e2 = ema_series(&e1, length);
    let e3 = ema_series(&e2, length);
    let e4 = ema_series(&e3, length);
    let e5 = ema_series(&e4, length);
    let e6 = ema_series(&e5, length);
    let c1 = -v.powi(3);
    let c2 = 3.0 * v * v + 3.0 * v.powi(3);
    let c3 = -6.0 * v * v - 3.0 * v - 3.0 * v.powi(3);
    let c4 = 1.0 + 3.0 * v + v.powi(3) + 3.0 * v * v;
    let t3_at = |i: usize| c1 * e6[i] + c2 * e5[i] + c3 * e4[i] + c4 * e3[i];
    let t3_value = t3_at(n - 1);
    let t3_prev = t3_at(n - 2);
    let last_close = closes[n - 1];
    let dev = if t3_value.abs() > 1e-12 {
        (last_close - t3_value) / t3_value * 100.0
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
    T3Snapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        v_factor: v,
        t3_value,
        t3_prev,
        deviation_pct: dev,
        last_close,
        t3_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_vidya_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> VidyaSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 20usize;
    let cmo_length = 9usize;
    let min_bars = length + cmo_length + 2;
    if n < min_bars {
        return VidyaSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            cmo_length,
            vidya_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    // |CMO(cmo_length)| at each bar i: 100 · |sum_up − sum_down| / (sum_up + sum_down)
    let cmo_abs = |end: usize| -> f64 {
        let start = end + 1 - cmo_length;
        let mut up = 0.0;
        let mut dn = 0.0;
        for k in start..=end {
            if k == 0 {
                continue;
            }
            let d = closes[k] - closes[k - 1];
            if d > 0.0 {
                up += d;
            } else {
                dn += -d;
            }
        }
        let s = up + dn;
        if s > 1e-12 {
            100.0 * (up - dn).abs() / s
        } else {
            0.0
        }
    };
    let base_alpha = 2.0 / (length as f64 + 1.0);
    let warmup = cmo_length.max(length);
    let mut vidya = closes[warmup];
    let mut prev_vidya = vidya;
    let mut current_alpha = 0.0;
    let mut cmo_now = 0.0;
    for i in (warmup + 1)..n {
        prev_vidya = vidya;
        let cmo_i = cmo_abs(i);
        let alpha = base_alpha * cmo_i / 100.0;
        vidya = alpha * closes[i] + (1.0 - alpha) * vidya;
        current_alpha = alpha;
        cmo_now = cmo_i;
    }
    let vidya_value = vidya;
    let last_close = closes[n - 1];
    let dev = if vidya_value.abs() > 1e-12 {
        (last_close - vidya_value) / vidya_value * 100.0
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
    VidyaSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        cmo_length,
        vidya_value,
        vidya_prev: prev_vidya,
        current_alpha,
        cmo_magnitude: cmo_now,
        deviation_pct: dev,
        last_close,
        vidya_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_smi_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> SmiSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 10usize;
    let smooth = 3usize;
    let signal = 3usize;
    let min_bars = length + 2 * smooth + signal + 2;
    if n < min_bars {
        return SmiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            smooth_length: smooth,
            signal_length: signal,
            smi_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    // Per-bar numerator = close − mid where mid = (H_max + L_min)/2 over lookback.
    // Per-bar denominator = (H_max − L_min)/2.
    let mut num_series = vec![0.0; n];
    let mut den_series = vec![0.0; n];
    for i in (length - 1)..n {
        let start = i + 1 - length;
        let mut hmax = f64::MIN;
        let mut lmin = f64::MAX;
        for k in start..=i {
            if sorted[k].high > hmax {
                hmax = sorted[k].high;
            }
            if sorted[k].low < lmin {
                lmin = sorted[k].low;
            }
        }
        let mid = (hmax + lmin) * 0.5;
        num_series[i] = sorted[i].close - mid;
        den_series[i] = (hmax - lmin) * 0.5;
    }
    let num_1 = ema_series(&num_series, smooth);
    let num_2 = ema_series(&num_1, smooth);
    let den_1 = ema_series(&den_series, smooth);
    let den_2 = ema_series(&den_1, smooth);
    let smi_series: Vec<f64> = (0..n)
        .map(|i| {
            if den_2[i].abs() > 1e-12 {
                100.0 * num_2[i] / den_2[i]
            } else {
                0.0
            }
        })
        .collect();
    let sig_series = ema_series(&smi_series, signal);
    let smi_value = smi_series[n - 1];
    let smi_prev = smi_series[n - 2];
    let signal_value = sig_series[n - 1];
    let signal_prev = sig_series[n - 2];
    let last_close = sorted[n - 1].close;
    let cross_up = smi_prev <= signal_prev && smi_value > signal_value;
    let cross_down = smi_prev >= signal_prev && smi_value < signal_value;
    let label = if smi_value > 40.0 {
        "OVERBOUGHT"
    } else if cross_up {
        "BULL_CROSS"
    } else if cross_down {
        "BEAR_CROSS"
    } else if smi_value < -40.0 {
        "OVERSOLD"
    } else if smi_value > signal_value {
        "BULL"
    } else if smi_value < signal_value {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    SmiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        smooth_length: smooth,
        signal_length: signal,
        smi_value,
        smi_prev,
        signal_value,
        signal_prev,
        last_close,
        smi_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_pvt_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> PvtSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let ema_length = 20usize;
    let slope_lookback = 20usize;
    let min_bars = ema_length + slope_lookback + 2;
    if n < min_bars {
        return PvtSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            pvt_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let mut pvt_series = vec![0.0; n];
    for i in 1..n {
        let prev_close = sorted[i - 1].close;
        let pct = if prev_close.abs() > 1e-12 {
            (sorted[i].close - prev_close) / prev_close
        } else {
            0.0
        };
        pvt_series[i] = pvt_series[i - 1] + sorted[i].volume * pct;
    }
    let pvt_ema = ema_series(&pvt_series, ema_length);
    let pvt_value = pvt_series[n - 1];
    let pvt_prev = pvt_series[n - 2];
    let pvt_ema_last = pvt_ema[n - 1];
    let slope_base = pvt_series[n - 1 - slope_lookback];
    let pvt_slope = pvt_value - slope_base;
    let last_close = sorted[n - 1].close;
    let abs_pvt = pvt_value.abs().max(1.0);
    let slope_ratio = pvt_slope / abs_pvt;
    let above_ema = pvt_value > pvt_ema_last;
    let below_ema = pvt_value < pvt_ema_last;
    let label = if slope_ratio > 0.05 && above_ema {
        "STRONG_BULL"
    } else if slope_ratio > 0.0 && above_ema {
        "BULL"
    } else if slope_ratio < -0.05 && below_ema {
        "STRONG_BEAR"
    } else if slope_ratio < 0.0 && below_ema {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    PvtSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pvt_value,
        pvt_prev,
        pvt_ema: pvt_ema_last,
        pvt_slope,
        last_close,
        pvt_label: label.into(),
        note: String::new(),
    }
}
