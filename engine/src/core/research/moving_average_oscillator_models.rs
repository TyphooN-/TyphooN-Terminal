use super::*;

mod regression_pivot_candles;
pub use regression_pivot_candles::*;
mod adaptive_forecast_vigor;
pub use adaptive_forecast_vigor::*;

// Shared moving-average helpers used by oscillator model families.

pub(super) fn ema_series(values: &[f64], length: usize) -> Vec<f64> {
    let n = values.len();
    if n == 0 || length == 0 {
        return Vec::new();
    }
    let alpha = 2.0 / (length as f64 + 1.0);
    let mut out = Vec::with_capacity(n);
    out.push(values[0]);
    for i in 1..n {
        out.push(alpha * values[i] + (1.0 - alpha) * out[i - 1]);
    }
    out
}

// ── Round 53: TRIMA / T3 / VIDYA / SMI / PVT ───────────────────────

pub(super) fn sma_series(values: &[f64], length: usize) -> Vec<f64> {
    let n = values.len();
    let mut out = vec![0.0; n];
    if n == 0 || length == 0 {
        return out;
    }
    let mut acc = 0.0;
    for i in 0..n {
        acc += values[i];
        if i >= length {
            acc -= values[i - length];
        }
        out[i] = if i + 1 >= length {
            acc / length as f64
        } else {
            0.0
        };
    }
    out
}

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

// ── Round 54 compute functions ───────────────────────────────────

/// Bill Williams's Accelerator Oscillator (AC).
///
/// Awesome Oscillator = `SMA₅(medprice) − SMA₃₄(medprice)`.
/// AC = `AO − SMA₅(AO)`.
/// AC is the "acceleration" of price momentum — positive + rising means
/// momentum is accelerating to the upside.
pub fn compute_ac_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> AcSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let min_bars = 34 + 5 + 2;
    if n < min_bars {
        return AcSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            ac_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let medprice: Vec<f64> = sorted.iter().map(|b| 0.5 * (b.high + b.low)).collect();
    let sma5 = sma_series(&medprice, 5);
    let sma34 = sma_series(&medprice, 34);
    let mut ao = vec![0.0; n];
    for i in 0..n {
        ao[i] = sma5[i] - sma34[i];
    }
    let ao_sma5 = sma_series(&ao, 5);
    let mut ac = vec![0.0; n];
    for i in 0..n {
        ac[i] = ao[i] - ao_sma5[i];
    }
    let ac_value = ac[n - 1];
    let ac_prev = ac[n - 2];
    let last_close = sorted[n - 1].close;
    let rising = ac_value > ac_prev;
    let label = if ac_value > 0.0 && rising {
        "STRONG_BULL"
    } else if ac_value > 0.0 && !rising {
        "BULL"
    } else if ac_value < 0.0 && !rising {
        "STRONG_BEAR"
    } else if ac_value < 0.0 && rising {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    AcSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        ac_value,
        ac_prev,
        ao_value: ao[n - 1],
        ao_sma5: ao_sma5[n - 1],
        last_close,
        ac_label: label.into(),
        note: String::new(),
    }
}

/// Marc Chaikin's Volatility. `CHV = 100·(EMA₁₀(H−L) − EMA₁₀(H−L)[−10])/
/// EMA₁₀(H−L)[−10]`. Positive = range expansion; negative = contraction.
pub fn compute_chvol_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> ChvolSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let ema_length = 10usize;
    let roc_length = 10usize;
    let min_bars = ema_length + roc_length + 2;
    if n < min_bars {
        return ChvolSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            ema_length,
            roc_length,
            chvol_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let range: Vec<f64> = sorted.iter().map(|b| (b.high - b.low).max(0.0)).collect();
    let ema = ema_series(&range, ema_length);
    let mut chv = vec![0.0; n];
    for i in roc_length..n {
        let base = ema[i - roc_length].abs().max(1e-12);
        chv[i] = 100.0 * (ema[i] - ema[i - roc_length]) / base;
    }
    let chvol_value = chv[n - 1];
    let chvol_prev = chv[n - 2];
    let label = if chvol_value > 5.0 {
        "EXPANDING"
    } else if chvol_value < -5.0 {
        "CONTRACTING"
    } else {
        "NEUTRAL"
    };
    ChvolSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        ema_length,
        roc_length,
        chvol_value,
        chvol_prev,
        ema_range: ema[n - 1],
        last_close: sorted[n - 1].close,
        chvol_label: label.into(),
        note: String::new(),
    }
}

/// John Bollinger's Bandwidth. `BBW = (upper − lower)/middle` with
/// middle = SMA₂₀(close) and ±2σ bands. The 125-bar percentile flags
/// how extreme the current squeeze is.
pub fn compute_bbwidth_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> BbwidthSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 20usize;
    let num_stdev = 2.0f64;
    let lookback = 125usize;
    let min_bars = length + 2;
    if n < min_bars {
        return BbwidthSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            num_stdev,
            bbw_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let mut bbw = vec![0.0; n];
    let mut last_middle = 0.0;
    let mut last_upper = 0.0;
    let mut last_lower = 0.0;
    for i in (length - 1)..n {
        let window = &closes[(i + 1 - length)..=i];
        let mean: f64 = window.iter().sum::<f64>() / length as f64;
        let var: f64 = window.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / length as f64;
        let sd = var.sqrt();
        let upper = mean + num_stdev * sd;
        let lower = mean - num_stdev * sd;
        bbw[i] = if mean.abs() > 1e-12 {
            (upper - lower) / mean
        } else {
            0.0
        };
        if i == n - 1 {
            last_middle = mean;
            last_upper = upper;
            last_lower = lower;
        }
    }
    let bbw_value = bbw[n - 1];
    let bbw_prev = bbw[n - 2];
    let history_start = n.saturating_sub(lookback);
    let history: Vec<f64> = bbw[history_start..n]
        .iter()
        .cloned()
        .filter(|x| x.abs() > 1e-12)
        .collect();
    let pct = if history.is_empty() {
        0.0
    } else {
        let below = history.iter().filter(|&&x| x < bbw_value).count() as f64;
        100.0 * below / history.len() as f64
    };
    let label = if pct <= 5.0 {
        "SQUEEZE"
    } else if pct <= 25.0 {
        "LOW"
    } else if pct >= 95.0 {
        "EXPANDED"
    } else {
        "NORMAL"
    };
    BbwidthSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        num_stdev,
        bbw_value,
        bbw_prev,
        bbw_percentile: pct,
        middle: last_middle,
        upper: last_upper,
        lower: last_lower,
        last_close: sorted[n - 1].close,
        bbw_label: label.into(),
        note: String::new(),
    }
}

/// Dr. Alexander Elder's Impulse System.
///
/// Colour = GREEN when 13-EMA rises AND MACD histogram rises.
/// RED when 13-EMA falls AND MACD histogram falls.
/// BLUE (mixed/transition) otherwise.
/// MACD uses standard 12/26/9 parameters.
pub fn compute_elder_impulse_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> ElderImpulseSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let ema_length = 13usize;
    let min_bars = 26 + 9 + 2;
    if n < min_bars {
        return ElderImpulseSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            ema_length,
            impulse_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let ema13 = ema_series(&closes, ema_length);
    let ema12 = ema_series(&closes, 12);
    let ema26 = ema_series(&closes, 26);
    let mut macd_line = vec![0.0; n];
    for i in 0..n {
        macd_line[i] = ema12[i] - ema26[i];
    }
    let signal = ema_series(&macd_line, 9);
    let mut hist = vec![0.0; n];
    for i in 0..n {
        hist[i] = macd_line[i] - signal[i];
    }
    let ema_value = ema13[n - 1];
    let ema_prev = ema13[n - 2];
    let ema_slope = ema_value - ema_prev;
    let macd_hist = hist[n - 1];
    let macd_hist_prev = hist[n - 2];
    let macd_hist_slope = macd_hist - macd_hist_prev;
    let label = if ema_slope > 0.0 && macd_hist_slope > 0.0 {
        "GREEN"
    } else if ema_slope < 0.0 && macd_hist_slope < 0.0 {
        "RED"
    } else {
        "BLUE"
    };
    ElderImpulseSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        ema_length,
        ema_value,
        ema_slope,
        macd_hist,
        macd_hist_prev,
        macd_hist_slope,
        last_close: sorted[n - 1].close,
        impulse_label: label.into(),
        note: String::new(),
    }
}

/// Roger Altman's Relative Momentum Index.
///
/// Like RSI but applied to the N-bar momentum `close − close[−M]` rather
/// than the 1-bar change. Gain = max(mom, 0); Loss = max(-mom, 0);
/// Wilder-smoothed with length L; `RMI = 100 − 100/(1 + avg_gain/avg_loss)`.
pub fn compute_rmi_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> RmiSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 14usize;
    let momentum_length = 5usize;
    let min_bars = length + momentum_length + 2;
    if n < min_bars {
        return RmiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            momentum_length,
            rmi_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let mut gains = vec![0.0; n];
    let mut losses = vec![0.0; n];
    for i in momentum_length..n {
        let m = closes[i] - closes[i - momentum_length];
        if m > 0.0 {
            gains[i] = m;
        } else {
            losses[i] = -m;
        }
    }
    // Wilder smoothing with seed = SMA over first `length` valid obs
    let mut avg_gain = vec![0.0; n];
    let mut avg_loss = vec![0.0; n];
    let seed_end = momentum_length + length - 1;
    if seed_end >= n {
        return RmiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            momentum_length,
            rmi_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", seed_end + 2, n),
            ..Default::default()
        };
    }
    let seed_g: f64 = gains[momentum_length..=seed_end].iter().sum::<f64>() / length as f64;
    let seed_l: f64 = losses[momentum_length..=seed_end].iter().sum::<f64>() / length as f64;
    avg_gain[seed_end] = seed_g;
    avg_loss[seed_end] = seed_l;
    for i in (seed_end + 1)..n {
        avg_gain[i] = (avg_gain[i - 1] * (length as f64 - 1.0) + gains[i]) / length as f64;
        avg_loss[i] = (avg_loss[i - 1] * (length as f64 - 1.0) + losses[i]) / length as f64;
    }
    let rmi_of = |i: usize| -> f64 {
        let g = avg_gain[i];
        let l = avg_loss[i];
        if l.abs() < 1e-12 {
            100.0
        } else {
            let rs = g / l;
            100.0 - 100.0 / (1.0 + rs)
        }
    };
    let rmi_value = rmi_of(n - 1);
    let rmi_prev = rmi_of(n - 2);
    let label = if rmi_value >= 70.0 {
        "OVERBOUGHT"
    } else if rmi_value <= 30.0 {
        "OVERSOLD"
    } else if rmi_value > 50.0 {
        "BULL"
    } else if rmi_value < 50.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    RmiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        momentum_length,
        rmi_value,
        rmi_prev,
        last_close: sorted[n - 1].close,
        rmi_label: label.into(),
        note: String::new(),
    }
}

/// Wilder's Smoothed Moving Average (SMMA / RMA) — recursive MA with
/// `SMMA_t = (SMMA_{t-1}·(N-1) + price_t) / N`. Seed with SMA over first
/// N closes. Labels by close-vs-SMMA deviation percentage:
/// STRONG_BULL ≥ +2%, BULL > 0%, NEUTRAL = 0, BEAR < 0%, STRONG_BEAR ≤ -2%.
pub fn compute_smma_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> SmmaSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 14usize;
    let min_bars = length + 2;
    if n < min_bars {
        return SmmaSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            smma_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let mut smma = vec![0.0; n];
    let seed: f64 = closes[0..length].iter().sum::<f64>() / length as f64;
    smma[length - 1] = seed;
    for i in length..n {
        smma[i] = (smma[i - 1] * (length as f64 - 1.0) + closes[i]) / length as f64;
    }
    let smma_value = smma[n - 1];
    let smma_prev = smma[n - 2];
    let last_close = closes[n - 1];
    let deviation_pct = if smma_value.abs() > 1e-12 {
        (last_close - smma_value) / smma_value * 100.0
    } else {
        0.0
    };
    let label = if deviation_pct >= 2.0 {
        "STRONG_BULL"
    } else if deviation_pct <= -2.0 {
        "STRONG_BEAR"
    } else if deviation_pct > 0.0 {
        "BULL"
    } else if deviation_pct < 0.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    SmmaSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        smma_value,
        smma_prev,
        deviation_pct,
        last_close,
        smma_label: label.into(),
        note: String::new(),
    }
}

/// Bill Williams's Alligator — three displaced SMMAs of the median price:
/// jaw = SMMA₁₃(medprice) evaluated 8 bars ago, teeth = SMMA₈ evaluated
/// 5 bars ago, lips = SMMA₅ evaluated 3 bars ago. Labelling inspects the
/// ordering and total spread: SLEEPING when spread is near zero,
/// EATING_UP when lips > teeth > jaw, EATING_DOWN when reversed,
/// AWAKENING otherwise (crossing).
pub fn compute_alligator_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> AlligatorSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    // jaw needs SMMA₁₃ at index n-1-8 and the prior bar at n-2-8.
    let min_bars = 13 + 8 + 2;
    if n < min_bars {
        return AlligatorSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            alligator_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let med: Vec<f64> = sorted.iter().map(|b| (b.high + b.low) / 2.0).collect();
    let smma_of = |len: usize| -> Vec<f64> {
        let mut out = vec![0.0; n];
        if n < len {
            return out;
        }
        let seed: f64 = med[0..len].iter().sum::<f64>() / len as f64;
        out[len - 1] = seed;
        for i in len..n {
            out[i] = (out[i - 1] * (len as f64 - 1.0) + med[i]) / len as f64;
        }
        out
    };
    let smma13 = smma_of(13);
    let smma8 = smma_of(8);
    let smma5 = smma_of(5);
    let jaw_t = n - 1 - 8;
    let teeth_t = n - 1 - 5;
    let lips_t = n - 1 - 3;
    let jaw = smma13[jaw_t];
    let jaw_prev = smma13[jaw_t - 1];
    let teeth = smma8[teeth_t];
    let teeth_prev = smma8[teeth_t - 1];
    let lips = smma5[lips_t];
    let lips_prev = smma5[lips_t - 1];
    let last_close = sorted[n - 1].close;
    let mn = jaw.min(teeth).min(lips);
    let mx = jaw.max(teeth).max(lips);
    let spread_pct = if last_close.abs() > 1e-12 {
        (mx - mn) / last_close * 100.0
    } else {
        0.0
    };
    let asleep_thresh = 0.15_f64; // percent spread below which state = SLEEPING
    let label = if spread_pct < asleep_thresh {
        "SLEEPING"
    } else if lips > teeth && teeth > jaw {
        "EATING_UP"
    } else if lips < teeth && teeth < jaw {
        "EATING_DOWN"
    } else {
        "AWAKENING"
    };
    AlligatorSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        jaw,
        teeth,
        lips,
        jaw_prev,
        teeth_prev,
        lips_prev,
        spread_pct,
        last_close,
        alligator_label: label.into(),
        note: String::new(),
    }
}

/// Larry Connors's Connors RSI — composite of three components:
/// `CRSI = (RSI₃(close) + RSI₂(streak) + percent_rank(ROC₁, 100)) / 3`.
/// Streak is the signed run-length counter (+k on k up-days, -k on
/// k down-days, 0 on flat). Canonical extremes: > 90 (short), < 10 (long).
pub fn compute_crsi_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CrsiSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let rsi_length = 3usize;
    let streak_length = 2usize;
    let rank_lookback = 100usize;
    let min_bars = rank_lookback + rsi_length + 5;
    if n < min_bars {
        return CrsiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            rsi_length,
            streak_length,
            rank_lookback,
            crsi_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let rsi_series = |series: &[f64], length: usize| -> Vec<f64> {
        let m = series.len();
        let mut out = vec![50.0; m];
        if m < length + 1 {
            return out;
        }
        let mut gains = vec![0.0; m];
        let mut losses = vec![0.0; m];
        for i in 1..m {
            let d = series[i] - series[i - 1];
            if d > 0.0 {
                gains[i] = d;
            } else {
                losses[i] = -d;
            }
        }
        let mut avg_g: f64 = gains[1..=length].iter().sum::<f64>() / length as f64;
        let mut avg_l: f64 = losses[1..=length].iter().sum::<f64>() / length as f64;
        out[length] = if avg_l.abs() < 1e-12 {
            100.0
        } else {
            let rs = avg_g / avg_l;
            100.0 - 100.0 / (1.0 + rs)
        };
        for i in (length + 1)..m {
            avg_g = (avg_g * (length as f64 - 1.0) + gains[i]) / length as f64;
            avg_l = (avg_l * (length as f64 - 1.0) + losses[i]) / length as f64;
            out[i] = if avg_l.abs() < 1e-12 {
                100.0
            } else {
                let rs = avg_g / avg_l;
                100.0 - 100.0 / (1.0 + rs)
            };
        }
        out
    };
    let rsi_close_series = rsi_series(&closes, rsi_length);
    let rsi_close = rsi_close_series[n - 1];
    let mut streak = vec![0.0; n];
    for i in 1..n {
        let d = closes[i] - closes[i - 1];
        streak[i] = if d > 0.0 {
            if streak[i - 1] > 0.0 {
                streak[i - 1] + 1.0
            } else {
                1.0
            }
        } else if d < 0.0 {
            if streak[i - 1] < 0.0 {
                streak[i - 1] - 1.0
            } else {
                -1.0
            }
        } else {
            0.0
        };
    }
    let rsi_streak_series = rsi_series(&streak, streak_length);
    let rsi_streak = rsi_streak_series[n - 1];
    let mut roc = vec![0.0; n];
    for i in 1..n {
        roc[i] = if closes[i - 1].abs() > 1e-12 {
            (closes[i] - closes[i - 1]) / closes[i - 1] * 100.0
        } else {
            0.0
        };
    }
    let today_roc = roc[n - 1];
    let window_start = n.saturating_sub(rank_lookback);
    let window = &roc[window_start..(n - 1)];
    let below = window.iter().filter(|&&x| x < today_roc).count() as f64;
    let percent_rank = if window.is_empty() {
        0.0
    } else {
        100.0 * below / window.len() as f64
    };
    let crsi_value = (rsi_close + rsi_streak + percent_rank) / 3.0;
    let rsi_close_prev = rsi_close_series[n - 2];
    let rsi_streak_prev = rsi_streak_series[n - 2];
    let prev_roc = roc[n - 2];
    let prev_window_start = (n - 1).saturating_sub(rank_lookback);
    let prev_window = &roc[prev_window_start..(n - 2)];
    let prev_below = prev_window.iter().filter(|&&x| x < prev_roc).count() as f64;
    let prev_pr = if prev_window.is_empty() {
        0.0
    } else {
        100.0 * prev_below / prev_window.len() as f64
    };
    let crsi_prev = (rsi_close_prev + rsi_streak_prev + prev_pr) / 3.0;
    let label = if crsi_value >= 75.0 {
        "OVERBOUGHT"
    } else if crsi_value <= 25.0 {
        "OVERSOLD"
    } else if crsi_value >= 60.0 {
        "BULLISH"
    } else if crsi_value <= 40.0 {
        "BEARISH"
    } else {
        "NEUTRAL"
    };
    CrsiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        rsi_length,
        streak_length,
        rank_lookback,
        rsi_close,
        rsi_streak,
        percent_rank,
        crsi_value,
        crsi_prev,
        last_close: closes[n - 1],
        crsi_label: label.into(),
        note: String::new(),
    }
}

/// Standard Error Bands — linear-regression endpoint fit ± k·SE channels.
/// Center = regression value at `t = N − 1`; SE = residual standard error
/// with (N − 2) degrees of freedom. Labels by close position vs bands.
pub fn compute_seb_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> SebSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 20usize;
    let num_se = 2.0f64;
    let min_bars = length + 2;
    if n < min_bars {
        return SebSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            num_se,
            seb_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let start = n - length;
    let xs: Vec<f64> = (0..length).map(|i| i as f64).collect();
    let ys: &[f64] = &closes[start..n];
    let x_mean: f64 = xs.iter().sum::<f64>() / length as f64;
    let y_mean: f64 = ys.iter().sum::<f64>() / length as f64;
    let mut sxy = 0.0;
    let mut sxx = 0.0;
    for i in 0..length {
        sxy += (xs[i] - x_mean) * (ys[i] - y_mean);
        sxx += (xs[i] - x_mean).powi(2);
    }
    let slope = if sxx.abs() < 1e-12 { 0.0 } else { sxy / sxx };
    let intercept = y_mean - slope * x_mean;
    let mut ss_res = 0.0;
    for i in 0..length {
        let yhat = slope * xs[i] + intercept;
        ss_res += (ys[i] - yhat).powi(2);
    }
    let dof = (length as f64 - 2.0).max(1.0);
    let se = (ss_res / dof).sqrt();
    let middle = slope * (length as f64 - 1.0) + intercept;
    let upper = middle + num_se * se;
    let lower = middle - num_se * se;
    let bandwidth = if middle.abs() > 1e-12 {
        (upper - lower) / middle
    } else {
        0.0
    };
    let last_close = closes[n - 1];
    let range = upper - lower;
    let position_pct = if range.abs() > 1e-12 {
        (last_close - lower) / range * 100.0
    } else {
        50.0
    };
    let label = if last_close > upper {
        "ABOVE_BAND"
    } else if last_close < lower {
        "BELOW_BAND"
    } else if position_pct >= 66.6667 {
        "UPPER_HALF"
    } else if position_pct <= 33.3333 {
        "LOWER_HALF"
    } else {
        "NEUTRAL"
    };
    SebSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        num_se,
        upper,
        middle,
        lower,
        bandwidth,
        position_pct,
        last_close,
        seb_label: label.into(),
        note: String::new(),
    }
}

/// Tushar Chande's Intraday Momentum Index — RSI-style ratio built from
/// per-bar `close − open` buying/selling pressure rather than close-to-
/// close momentum. `IMI = 100 · ΣUp / (ΣUp + ΣDown)` over N bars, where
/// Up = max(close − open, 0), Down = max(open − close, 0).
pub fn compute_imi_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> ImiSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 14usize;
    let min_bars = length + 2;
    if n < min_bars {
        return ImiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            imi_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let mut gains = vec![0.0; n];
    let mut losses = vec![0.0; n];
    for i in 0..n {
        let d = sorted[i].close - sorted[i].open;
        if d > 0.0 {
            gains[i] = d;
        } else {
            losses[i] = -d;
        }
    }
    let imi_at = |end: usize| -> (f64, f64, f64) {
        let start = end + 1 - length;
        let sg: f64 = gains[start..=end].iter().sum();
        let sl: f64 = losses[start..=end].iter().sum();
        let tot = sg + sl;
        let v = if tot > 1e-12 { 100.0 * sg / tot } else { 50.0 };
        (sg, sl, v)
    };
    let (sum_gains, sum_losses, imi_value) = imi_at(n - 1);
    let (_, _, imi_prev) = imi_at(n - 2);
    let label = if imi_value >= 70.0 {
        "OVERBOUGHT"
    } else if imi_value <= 30.0 {
        "OVERSOLD"
    } else if imi_value >= 60.0 {
        "BULL"
    } else if imi_value <= 40.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    ImiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        sum_gains,
        sum_losses,
        imi_value,
        imi_prev,
        last_close: sorted[n - 1].close,
        imi_label: label.into(),
        note: String::new(),
    }
}

/// Guppy Multiple Moving Average — fan of 6 short + 6 long EMAs.
/// Reports group averages, spread, and trend label (STRONG_UPTREND when
/// short-avg > long-avg and both groups fanned; COMPRESSION when short
/// group width < 0.25 · long group width).
pub fn compute_gmma_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> GmmaSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let short_lengths: [usize; 6] = [3, 5, 8, 10, 12, 15];
    let long_lengths: [usize; 6] = [30, 35, 40, 45, 50, 60];
    let min_bars = 60 + 2;
    if n < min_bars {
        return GmmaSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            gmma_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|r| r.close).collect();
    let ema = |period: usize| -> f64 {
        let alpha = 2.0 / (period as f64 + 1.0);
        let mut e = closes[0];
        for &c in &closes[1..] {
            e = alpha * c + (1.0 - alpha) * e;
        }
        e
    };
    let shorts: Vec<f64> = short_lengths.iter().map(|&p| ema(p)).collect();
    let longs: Vec<f64> = long_lengths.iter().map(|&p| ema(p)).collect();
    let short_ema_avg = shorts.iter().sum::<f64>() / shorts.len() as f64;
    let long_ema_avg = longs.iter().sum::<f64>() / longs.len() as f64;
    let short_min = shorts.iter().cloned().fold(f64::INFINITY, f64::min);
    let short_max = shorts.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let long_min = longs.iter().cloned().fold(f64::INFINITY, f64::min);
    let long_max = longs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let last_close = closes[n - 1];
    let short_compression_pct = if last_close > 0.0 {
        (short_max - short_min) / last_close * 100.0
    } else {
        0.0
    };
    let long_compression_pct = if last_close > 0.0 {
        (long_max - long_min) / last_close * 100.0
    } else {
        0.0
    };
    let group_gap_pct = if last_close > 0.0 {
        (short_ema_avg - long_ema_avg) / last_close * 100.0
    } else {
        0.0
    };
    let fanned_up = short_min > long_max;
    let fanned_down = short_max < long_min;
    let compressed = short_compression_pct < 0.25 * long_compression_pct.max(1e-6);
    let label = if fanned_up && group_gap_pct > 1.0 {
        "STRONG_UPTREND"
    } else if short_ema_avg > long_ema_avg {
        "UPTREND"
    } else if fanned_down && group_gap_pct < -1.0 {
        "STRONG_DOWNTREND"
    } else if short_ema_avg < long_ema_avg {
        "DOWNTREND"
    } else if compressed {
        "COMPRESSION"
    } else {
        "NEUTRAL"
    };
    GmmaSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        short_ema_avg,
        long_ema_avg,
        short_min,
        short_max,
        long_min,
        long_max,
        short_compression_pct,
        long_compression_pct,
        group_gap_pct,
        last_close,
        gmma_label: label.into(),
        note: String::new(),
    }
}

/// Moving Average Envelope — SMA(N) ± k%. Labels the close's position
/// relative to the envelope.
pub fn compute_maenv_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MaenvSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 20usize;
    let pct_band = 2.5_f64;
    if n < length + 1 {
        return MaenvSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            pct_band,
            maenv_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length + 1, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|r| r.close).collect();
    let start = n - length;
    let sma: f64 = closes[start..].iter().sum::<f64>() / length as f64;
    let middle = sma;
    let factor = pct_band / 100.0;
    let upper = middle * (1.0 + factor);
    let lower = middle * (1.0 - factor);
    let last_close = closes[n - 1];
    let bandwidth_pct = 2.0 * pct_band;
    let position_pct = if upper > lower {
        (last_close - lower) / (upper - lower) * 100.0
    } else {
        50.0
    };
    let label = if last_close > upper {
        "ABOVE_BAND"
    } else if last_close < lower {
        "BELOW_BAND"
    } else if position_pct >= 75.0 {
        "UPPER_HALF"
    } else if position_pct <= 25.0 {
        "LOWER_HALF"
    } else {
        "NEUTRAL"
    };
    MaenvSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        pct_band,
        upper,
        middle,
        lower,
        bandwidth_pct,
        position_pct,
        last_close,
        maenv_label: label.into(),
        note: String::new(),
    }
}

/// Chaikin Accumulation/Distribution Line — cumulative ∑(MFM · volume).
/// Reports ADL, 20-bar SMA, OLS slope of last 20 ADL points, and
/// accumulation/distribution label.
pub fn compute_adl_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> AdlSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let adl_sma_length = 20usize;
    if n < adl_sma_length + 2 {
        return AdlSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            adl_sma_length,
            adl_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", adl_sma_length + 2, n),
            ..Default::default()
        };
    }
    let mut adl = vec![0.0; n];
    let mut running = 0.0_f64;
    for i in 0..n {
        let r = sorted[i];
        let range = r.high - r.low;
        let mfm = if range > 1e-12 {
            ((r.close - r.low) - (r.high - r.close)) / range
        } else {
            0.0
        };
        running += mfm * r.volume;
        adl[i] = running;
    }
    let adl_value = adl[n - 1];
    let adl_prev = adl[n - 2];
    let sma_start = n - adl_sma_length;
    let adl_sma: f64 = adl[sma_start..].iter().sum::<f64>() / adl_sma_length as f64;
    // OLS slope of last 20 points
    let nf = adl_sma_length as f64;
    let xs: Vec<f64> = (0..adl_sma_length).map(|i| i as f64).collect();
    let ys: &[f64] = &adl[sma_start..];
    let mx: f64 = xs.iter().sum::<f64>() / nf;
    let my: f64 = ys.iter().sum::<f64>() / nf;
    let mut num = 0.0;
    let mut den = 0.0;
    for i in 0..adl_sma_length {
        let dx = xs[i] - mx;
        num += dx * (ys[i] - my);
        den += dx * dx;
    }
    let slope_per_bar = if den > 1e-12 { num / den } else { 0.0 };
    let last_close = sorted[n - 1].close;
    let price_past = sorted[sma_start].close;
    let price_delta_pct = if price_past > 0.0 {
        (last_close - price_past) / price_past * 100.0
    } else {
        0.0
    };
    let norm_slope = if last_close > 0.0 {
        slope_per_bar / last_close
    } else {
        0.0
    };
    let label = if norm_slope > 1_000_000.0 {
        "STRONG_ACCUMULATION"
    } else if norm_slope > 100_000.0 {
        "ACCUMULATION"
    } else if norm_slope < -1_000_000.0 {
        "STRONG_DISTRIBUTION"
    } else if norm_slope < -100_000.0 {
        "DISTRIBUTION"
    } else {
        "NEUTRAL"
    };
    AdlSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        adl_value,
        adl_prev,
        adl_sma_length,
        adl_sma,
        slope_per_bar,
        last_close,
        price_delta_pct,
        adl_label: label.into(),
        note: String::new(),
    }
}

/// Vertical Horizontal Filter — (HHV − LLV) / Σ|Δclose| over N=28.
/// High = trending, low = ranging.
pub fn compute_vhf_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> VhfSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 28usize;
    if n < length + 2 {
        return VhfSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            vhf_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length + 2, n),
            ..Default::default()
        };
    }
    let vhf_at = |end: usize| -> (f64, f64, f64, f64) {
        let start = end + 1 - length;
        let mut hh = f64::NEG_INFINITY;
        let mut ll = f64::INFINITY;
        for i in start..=end {
            if sorted[i].high > hh {
                hh = sorted[i].high;
            }
            if sorted[i].low < ll {
                ll = sorted[i].low;
            }
        }
        let mut sum_abs = 0.0;
        for i in start..=end {
            sum_abs += (sorted[i].close - sorted[i - 1].close).abs();
        }
        let v = if sum_abs > 1e-12 {
            (hh - ll) / sum_abs
        } else {
            0.0
        };
        (hh, ll, sum_abs, v)
    };
    let (highest_high, lowest_low, sum_abs_delta, vhf_value) = vhf_at(n - 1);
    let (_, _, _, vhf_prev) = vhf_at(n - 2);
    let label = if vhf_value >= 0.6 {
        "STRONG_TREND"
    } else if vhf_value >= 0.4 {
        "TREND"
    } else if vhf_value <= 0.2 {
        "STRONG_RANGING"
    } else if vhf_value <= 0.3 {
        "RANGING"
    } else {
        "NEUTRAL"
    };
    VhfSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        highest_high,
        lowest_low,
        sum_abs_delta,
        vhf_value,
        vhf_prev,
        last_close: sorted[n - 1].close,
        vhf_label: label.into(),
        note: String::new(),
    }
}

/// Volume Rate of Change — 14-bar ROC of volume.
pub fn compute_vroc_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> VrocSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 14usize;
    if n < length + 2 {
        return VrocSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            vroc_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length + 2, n),
            ..Default::default()
        };
    }
    let vroc_at = |end: usize| -> (f64, f64, f64) {
        let then = sorted[end - length].volume;
        let now = sorted[end].volume;
        let v = if then > 1e-12 {
            (now - then) / then * 100.0
        } else {
            0.0
        };
        (now, then, v)
    };
    let (volume_now, volume_then, vroc_value) = vroc_at(n - 1);
    let (_, _, vroc_prev) = vroc_at(n - 2);
    let label = if vroc_value >= 100.0 {
        "SURGE"
    } else if vroc_value >= 30.0 {
        "ELEVATED"
    } else if vroc_value <= -50.0 {
        "COLLAPSE"
    } else if vroc_value <= -20.0 {
        "QUIET"
    } else {
        "NEUTRAL"
    };
    VrocSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        volume_now,
        volume_then,
        vroc_value,
        vroc_prev,
        last_close: sorted[n - 1].close,
        vroc_label: label.into(),
        note: String::new(),
    }
}

/// Chaikin Oscillator — EMA(ADL,3) − EMA(ADL,10).
pub fn compute_kdj_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> KdjSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let stoch_length = 9usize;
    let k_smooth = 3usize;
    let min_bars = stoch_length + k_smooth + 2;
    if n < min_bars {
        return KdjSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            stoch_length,
            k_smooth,
            kdj_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let mut rsv_series = vec![0.0_f64; n];
    for i in (stoch_length - 1)..n {
        let mut hi = f64::NEG_INFINITY;
        let mut lo = f64::INFINITY;
        for j in (i + 1 - stoch_length)..=i {
            if sorted[j].high > hi {
                hi = sorted[j].high;
            }
            if sorted[j].low < lo {
                lo = sorted[j].low;
            }
        }
        let range = hi - lo;
        rsv_series[i] = if range > 1e-12 {
            100.0 * (sorted[i].close - lo) / range
        } else {
            50.0
        };
    }
    let alpha = 1.0_f64 / k_smooth as f64;
    let mut k_series = vec![50.0_f64; n];
    let mut d_series = vec![50.0_f64; n];
    let mut j_series = vec![50.0_f64; n];
    let start = stoch_length - 1;
    k_series[start] = 50.0;
    d_series[start] = 50.0;
    j_series[start] = 3.0 * k_series[start] - 2.0 * d_series[start];
    for i in (start + 1)..n {
        k_series[i] = alpha * rsv_series[i] + (1.0 - alpha) * k_series[i - 1];
        d_series[i] = alpha * k_series[i] + (1.0 - alpha) * d_series[i - 1];
        j_series[i] = 3.0 * k_series[i] - 2.0 * d_series[i];
    }
    let rsv = rsv_series[n - 1];
    let k_value = k_series[n - 1];
    let d_value = d_series[n - 1];
    let j_value = j_series[n - 1];
    let j_prev = j_series[n - 2];
    let last_close = sorted[n - 1].close;
    let label = if k_value > 80.0 && j_value > 90.0 {
        "OVERBOUGHT"
    } else if k_value < 20.0 && j_value < 10.0 {
        "OVERSOLD"
    } else if k_value > d_value && j_value > 50.0 {
        "BULL"
    } else if k_value < d_value && j_value < 50.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    KdjSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        stoch_length,
        k_smooth,
        rsv,
        k_value,
        d_value,
        j_value,
        j_prev,
        last_close,
        kdj_label: label.into(),
        note: String::new(),
    }
}

/// QQE — smoothed RSI with adaptive bands.
pub fn compute_qqe_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> QqeSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let rsi_length = 14usize;
    let smooth_length = 5usize;
    let qqe_factor = 4.236_f64;
    let wilder_length = rsi_length * 2 + 1;
    let min_bars = rsi_length + smooth_length + wilder_length + 2;
    if n < min_bars {
        return QqeSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            rsi_length,
            smooth_length,
            qqe_factor,
            qqe_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|r| r.close).collect();
    // Build RSI series
    let mut gains = vec![0.0; n];
    let mut losses = vec![0.0; n];
    for i in 1..n {
        let d = closes[i] - closes[i - 1];
        if d > 0.0 {
            gains[i] = d;
        } else {
            losses[i] = -d;
        }
    }
    let mut avg_gain = gains[1..=rsi_length].iter().sum::<f64>() / rsi_length as f64;
    let mut avg_loss = losses[1..=rsi_length].iter().sum::<f64>() / rsi_length as f64;
    let mut rsi = vec![f64::NAN; n];
    rsi[rsi_length] = if avg_loss > 1e-12 {
        100.0 - 100.0 / (1.0 + avg_gain / avg_loss)
    } else {
        100.0
    };
    for i in (rsi_length + 1)..n {
        avg_gain = (avg_gain * (rsi_length as f64 - 1.0) + gains[i]) / rsi_length as f64;
        avg_loss = (avg_loss * (rsi_length as f64 - 1.0) + losses[i]) / rsi_length as f64;
        rsi[i] = if avg_loss > 1e-12 {
            100.0 - 100.0 / (1.0 + avg_gain / avg_loss)
        } else {
            100.0
        };
    }
    // EMA smoothing on RSI
    let alpha_s = 2.0 / (smooth_length as f64 + 1.0);
    let mut rsi_s = vec![f64::NAN; n];
    rsi_s[rsi_length] = rsi[rsi_length];
    for i in (rsi_length + 1)..n {
        rsi_s[i] = alpha_s * rsi[i] + (1.0 - alpha_s) * rsi_s[i - 1];
    }
    // |ΔRSI_smoothed| and Wilder smoothing (period = rsi_length*2+1)
    let mut delta_abs = vec![0.0; n];
    for i in (rsi_length + 2)..n {
        delta_abs[i] = (rsi_s[i] - rsi_s[i - 1]).abs();
    }
    let wl = wilder_length as f64;
    let mut atr_rsi = vec![0.0; n];
    let start = rsi_length + wilder_length + 2;
    if n < start + 1 {
        return QqeSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            rsi_length,
            smooth_length,
            qqe_factor,
            qqe_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", start + 1, n),
            ..Default::default()
        };
    }
    let init: f64 = delta_abs[(rsi_length + 2)..(rsi_length + 2 + wilder_length)]
        .iter()
        .sum::<f64>()
        / wl;
    atr_rsi[rsi_length + wilder_length + 1] = init;
    for i in (rsi_length + wilder_length + 2)..n {
        atr_rsi[i] = (atr_rsi[i - 1] * (wl - 1.0) + delta_abs[i]) / wl;
    }
    // Second smoothing (Wilder on atr_rsi)
    let mut fast_avg = vec![0.0; n];
    let s2_start = rsi_length + wilder_length * 2 + 1;
    if n < s2_start + 1 {
        return QqeSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            rsi_length,
            smooth_length,
            qqe_factor,
            qqe_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", s2_start + 1, n),
            ..Default::default()
        };
    }
    let init2: f64 = atr_rsi
        [(rsi_length + wilder_length + 1)..(rsi_length + wilder_length + 1 + wilder_length)]
        .iter()
        .sum::<f64>()
        / wl;
    fast_avg[rsi_length + wilder_length * 2] = init2;
    for i in (rsi_length + wilder_length * 2 + 1)..n {
        fast_avg[i] = (fast_avg[i - 1] * (wl - 1.0) + atr_rsi[i]) / wl;
    }
    let rsi_value = rsi[n - 1];
    let rsi_smoothed = rsi_s[n - 1];
    let fast_atr_rsi_avg = fast_avg[n - 1];
    let upper_band = rsi_smoothed + qqe_factor * fast_atr_rsi_avg;
    let lower_band = rsi_smoothed - qqe_factor * fast_atr_rsi_avg;
    let qqe_prev = rsi_s[n - 2];
    let label = if rsi_smoothed >= 70.0 {
        "STRONG_BULL"
    } else if rsi_smoothed >= 55.0 {
        "BULL"
    } else if rsi_smoothed <= 30.0 {
        "STRONG_BEAR"
    } else if rsi_smoothed <= 45.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    QqeSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        rsi_length,
        smooth_length,
        qqe_factor,
        rsi_value,
        rsi_smoothed,
        fast_atr_rsi_avg,
        upper_band,
        lower_band,
        qqe_prev,
        last_close: sorted[n - 1].close,
        qqe_label: label.into(),
        note: String::new(),
    }
}

/// Pring PMO — double-smoothed ROC with 10-bar signal.
pub fn compute_pmo_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> PmoSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let s1 = 35usize;
    let s2 = 20usize;
    let sig = 10usize;
    let min_bars = s1 + s2 + sig + 4;
    if n < min_bars {
        return PmoSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            smooth1_length: s1,
            smooth2_length: s2,
            signal_length: sig,
            pmo_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|r| r.close).collect();
    let mut roc = vec![0.0; n];
    for i in 1..n {
        if closes[i - 1] > 1e-12 {
            roc[i] = (closes[i] / closes[i - 1] - 1.0) * 1000.0;
        }
    }
    let ema = |src: &[f64], period: usize| -> Vec<f64> {
        let alpha = 2.0 / (period as f64 + 1.0);
        let mut out = vec![0.0; src.len()];
        out[0] = src[0];
        for i in 1..src.len() {
            out[i] = alpha * src[i] + (1.0 - alpha) * out[i - 1];
        }
        out
    };
    let e1 = ema(&roc, s1);
    let e2 = ema(&e1, s2);
    let signal = ema(&e2, sig);
    let pmo_value = e2[n - 1];
    let pmo_prev = e2[n - 2];
    let pmo_signal = signal[n - 1];
    let histogram = pmo_value - pmo_signal;
    let label = if pmo_value > pmo_signal && histogram > 0.5 {
        "STRONG_BULL"
    } else if pmo_value > pmo_signal {
        "BULL"
    } else if pmo_value < pmo_signal && histogram < -0.5 {
        "STRONG_BEAR"
    } else if pmo_value < pmo_signal {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    PmoSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        smooth1_length: s1,
        smooth2_length: s2,
        signal_length: sig,
        pmo_value,
        pmo_signal,
        pmo_prev,
        histogram,
        last_close: sorted[n - 1].close,
        pmo_label: label.into(),
        note: String::new(),
    }
}

/// Chande Forecast Oscillator — 100·(close − forecast)/close.
pub fn compute_cfo_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> CfoSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 14usize;
    if n < length + 2 {
        return CfoSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            cfo_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length + 2, n),
            ..Default::default()
        };
    }
    let cfo_at = |end: usize| -> (f64, f64, f64, f64) {
        let start = end + 1 - length;
        let xs: Vec<f64> = (0..length).map(|i| i as f64).collect();
        let ys: Vec<f64> = sorted[start..=end].iter().map(|r| r.close).collect();
        let nf = length as f64;
        let mx: f64 = xs.iter().sum::<f64>() / nf;
        let my: f64 = ys.iter().sum::<f64>() / nf;
        let mut num = 0.0;
        let mut den = 0.0;
        for i in 0..length {
            let dx = xs[i] - mx;
            num += dx * (ys[i] - my);
            den += dx * dx;
        }
        let slope = if den > 1e-12 { num / den } else { 0.0 };
        let intercept = my - slope * mx;
        let forecast = slope * (length as f64) + intercept;
        let close = ys[length - 1];
        let v = if close > 1e-12 {
            100.0 * (close - forecast) / close
        } else {
            0.0
        };
        (slope, intercept, forecast, v)
    };
    let (slope, intercept, forecast, cfo_value) = cfo_at(n - 1);
    let (_, _, _, cfo_prev) = cfo_at(n - 2);
    let label = if cfo_value >= 2.0 {
        "STRONG_ABOVE_TREND"
    } else if cfo_value >= 0.5 {
        "ABOVE_TREND"
    } else if cfo_value <= -2.0 {
        "STRONG_BELOW_TREND"
    } else if cfo_value <= -0.5 {
        "BELOW_TREND"
    } else {
        "NEUTRAL"
    };
    CfoSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        slope,
        intercept,
        forecast,
        cfo_value,
        cfo_prev,
        last_close: sorted[n - 1].close,
        cfo_label: label.into(),
        note: String::new(),
    }
}

/// Twiggs Money Flow — EMA-smoothed money-flow ratio with true-range MFM.
pub fn compute_tmf_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> TmfSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 21usize;
    if n < length + 2 {
        return TmfSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            tmf_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length + 2, n),
            ..Default::default()
        };
    }
    let mut money_flow_volume = vec![0.0; n];
    for i in 1..n {
        let r = sorted[i];
        let prev_close = sorted[i - 1].close;
        let tr_high = r.high.max(prev_close);
        let tr_low = r.low.min(prev_close);
        let trange = tr_high - tr_low;
        let mfm = if trange > 1e-12 {
            ((r.close - tr_low) - (tr_high - r.close)) / trange
        } else {
            0.0
        };
        money_flow_volume[i] = mfm * r.volume;
    }
    let ema = |src: &[f64], period: usize| -> Vec<f64> {
        let alpha = 2.0 / (period as f64 + 1.0);
        let mut out = vec![0.0; src.len()];
        out[0] = src[0];
        for i in 1..src.len() {
            out[i] = alpha * src[i] + (1.0 - alpha) * out[i - 1];
        }
        out
    };
    let volumes: Vec<f64> = sorted.iter().map(|r| r.volume).collect();
    let emf = ema(&money_flow_volume, length);
    let ev = ema(&volumes, length);
    let tmf_value = if ev[n - 1].abs() > 1e-12 {
        emf[n - 1] / ev[n - 1]
    } else {
        0.0
    };
    let tmf_prev = if ev[n - 2].abs() > 1e-12 {
        emf[n - 2] / ev[n - 2]
    } else {
        0.0
    };
    let label = if tmf_value >= 0.25 {
        "STRONG_INFLOW"
    } else if tmf_value >= 0.05 {
        "INFLOW"
    } else if tmf_value <= -0.25 {
        "STRONG_OUTFLOW"
    } else if tmf_value <= -0.05 {
        "OUTFLOW"
    } else {
        "NEUTRAL"
    };
    TmfSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        ema_money_flow: emf[n - 1],
        ema_volume: ev[n - 1],
        tmf_value,
        tmf_prev,
        last_close: sorted[n - 1].close,
        tmf_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_fractals_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> FractalsSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let window = 5usize;
    if n < window {
        return FractalsSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            window,
            fractals_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", window, n),
            ..Default::default()
        };
    }
    let mut last_up_idx: Option<usize> = None;
    let mut last_down_idx: Option<usize> = None;
    let mut up_count = 0usize;
    let mut down_count = 0usize;
    for i in 2..n.saturating_sub(2) {
        let h = sorted[i].high;
        let l = sorted[i].low;
        if h > sorted[i - 2].high
            && h > sorted[i - 1].high
            && h > sorted[i + 1].high
            && h > sorted[i + 2].high
        {
            up_count += 1;
            last_up_idx = Some(i);
        }
        if l < sorted[i - 2].low
            && l < sorted[i - 1].low
            && l < sorted[i + 1].low
            && l < sorted[i + 2].low
        {
            down_count += 1;
            last_down_idx = Some(i);
        }
    }
    let last_idx = n - 1;
    let (last_up_high, last_up_bars_ago) = match last_up_idx {
        Some(i) => (sorted[i].high, last_idx - i),
        None => (0.0, 0),
    };
    let (last_down_low, last_down_bars_ago) = match last_down_idx {
        Some(i) => (sorted[i].low, last_idx - i),
        None => (0.0, 0),
    };
    let up_recent = last_up_idx.is_some() && last_up_bars_ago <= 10;
    let down_recent = last_down_idx.is_some() && last_down_bars_ago <= 10;
    let label = match (up_recent, down_recent) {
        (true, true) => "BOTH_RECENT",
        (true, false) => "UP_RECENT",
        (false, true) => "DOWN_RECENT",
        (false, false) => "NONE_RECENT",
    };
    FractalsSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        window,
        last_up_high,
        last_up_bars_ago,
        last_down_low,
        last_down_bars_ago,
        up_fractal_count: up_count,
        down_fractal_count: down_count,
        last_close: sorted[last_idx].close,
        fractals_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_ift_rsi_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> IftRsiSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let rsi_length = 14usize;
    let wma_length = 9usize;
    let need = rsi_length + wma_length + 2;
    if n < need {
        return IftRsiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            rsi_length,
            wma_length,
            ift_rsi_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{need} bars, got {n}"),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|r| r.close).collect();
    let mut gains = vec![0.0; n];
    let mut losses = vec![0.0; n];
    for i in 1..n {
        let d = closes[i] - closes[i - 1];
        if d > 0.0 {
            gains[i] = d;
        } else {
            losses[i] = -d;
        }
    }
    let mut avg_gain = 0.0;
    let mut avg_loss = 0.0;
    for i in 1..=rsi_length {
        avg_gain += gains[i];
        avg_loss += losses[i];
    }
    avg_gain /= rsi_length as f64;
    avg_loss /= rsi_length as f64;
    let mut rsi_series = vec![0.0; n];
    for i in rsi_length..n {
        if i > rsi_length {
            avg_gain = (avg_gain * (rsi_length as f64 - 1.0) + gains[i]) / rsi_length as f64;
            avg_loss = (avg_loss * (rsi_length as f64 - 1.0) + losses[i]) / rsi_length as f64;
        }
        let rs = if avg_loss > 1e-12 {
            avg_gain / avg_loss
        } else {
            100.0
        };
        rsi_series[i] = 100.0 - 100.0 / (1.0 + rs);
    }
    let v_raw: Vec<f64> = rsi_series.iter().map(|r| 0.1 * (r - 50.0)).collect();
    let wma = |src: &[f64], period: usize, i: usize| -> f64 {
        let mut num = 0.0;
        let mut den = 0.0;
        for k in 0..period {
            let w = (period - k) as f64;
            num += w * src[i - k];
            den += w;
        }
        num / den
    };
    let idx = n - 1;
    let v_value = wma(&v_raw, wma_length, idx);
    let v_prev = wma(&v_raw, wma_length, idx - 1);
    let ift_value = ((2.0 * v_value).exp() - 1.0) / ((2.0 * v_value).exp() + 1.0);
    let ift_prev = ((2.0 * v_prev).exp() - 1.0) / ((2.0 * v_prev).exp() + 1.0);
    let label = if ift_value >= 0.5 {
        "STRONG_BULL"
    } else if ift_value >= 0.1 {
        "BULL"
    } else if ift_value <= -0.5 {
        "STRONG_BEAR"
    } else if ift_value <= -0.1 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    IftRsiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        rsi_length,
        wma_length,
        rsi_value: rsi_series[idx],
        v_value,
        ift_value,
        ift_prev,
        last_close: sorted[idx].close,
        ift_rsi_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_mama_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MamaSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let fast_limit = 0.5;
    let slow_limit = 0.05;
    if n < 32 {
        return MamaSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            fast_limit,
            slow_limit,
            mama_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥32 bars, got {n}"),
            ..Default::default()
        };
    }
    // Ehlers MAMA implementation — simplified Hilbert transform discriminator.
    let price: Vec<f64> = sorted.iter().map(|r| (r.high + r.low) / 2.0).collect();
    let mut smooth = vec![0.0; n];
    let mut detrender = vec![0.0; n];
    let mut q1 = vec![0.0; n];
    let mut i1 = vec![0.0; n];
    let mut j_i = vec![0.0; n];
    let mut j_q = vec![0.0; n];
    let mut i2 = vec![0.0; n];
    let mut q2 = vec![0.0; n];
    let mut re = vec![0.0; n];
    let mut im = vec![0.0; n];
    let mut period = vec![0.0; n];
    let mut smooth_period = vec![0.0; n];
    let mut phase = vec![0.0; n];
    let mut mama = vec![0.0; n];
    let mut fama = vec![0.0; n];
    for i in 6..n {
        smooth[i] =
            (4.0 * price[i] + 3.0 * price[i - 1] + 2.0 * price[i - 2] + price[i - 3]) / 10.0;
        let pf = 0.075 * period[i - 1] + 0.54;
        detrender[i] = (0.0962 * smooth[i] + 0.5769 * smooth[i - 2]
            - 0.5769 * smooth[i - 4]
            - 0.0962 * smooth[i - 6])
            * pf;
        q1[i] = (0.0962 * detrender[i] + 0.5769 * detrender[i - 2]
            - 0.5769 * detrender[i - 4]
            - 0.0962 * detrender[i - 6])
            * pf;
        i1[i] = detrender[i - 3];
        j_i[i] =
            (0.0962 * i1[i] + 0.5769 * i1[i - 2] - 0.5769 * i1[i - 4] - 0.0962 * i1[i - 6]) * pf;
        j_q[i] =
            (0.0962 * q1[i] + 0.5769 * q1[i - 2] - 0.5769 * q1[i - 4] - 0.0962 * q1[i - 6]) * pf;
        let i2_raw = i1[i] - j_q[i];
        let q2_raw = q1[i] + j_i[i];
        i2[i] = 0.2 * i2_raw + 0.8 * i2[i - 1];
        q2[i] = 0.2 * q2_raw + 0.8 * q2[i - 1];
        let re_raw = i2[i] * i2[i - 1] + q2[i] * q2[i - 1];
        let im_raw = i2[i] * q2[i - 1] - q2[i] * i2[i - 1];
        re[i] = 0.2 * re_raw + 0.8 * re[i - 1];
        im[i] = 0.2 * im_raw + 0.8 * im[i - 1];
        let mut per = if im[i].abs() > 1e-12 && re[i].abs() > 1e-12 {
            std::f64::consts::TAU / (im[i] / re[i]).atan()
        } else {
            period[i - 1]
        };
        if per > 1.5 * period[i - 1] && period[i - 1] > 0.0 {
            per = 1.5 * period[i - 1];
        }
        if per < 0.67 * period[i - 1] && period[i - 1] > 0.0 {
            per = 0.67 * period[i - 1];
        }
        per = per.clamp(6.0, 50.0);
        period[i] = 0.2 * per + 0.8 * period[i - 1];
        smooth_period[i] = 0.33 * period[i] + 0.67 * smooth_period[i - 1];
        let phase_new = if i1[i].abs() > 1e-12 {
            (q1[i] / i1[i]).atan().to_degrees()
        } else {
            phase[i - 1]
        };
        let delta_phase = (phase[i - 1] - phase_new).max(1.0);
        phase[i] = phase_new;
        let alpha = (fast_limit / delta_phase).max(slow_limit).min(fast_limit);
        mama[i] = alpha * price[i] + (1.0 - alpha) * mama[i - 1];
        fama[i] = 0.5 * alpha * mama[i] + (1.0 - 0.5 * alpha) * fama[i - 1];
    }
    let idx = n - 1;
    let mama_value = mama[idx];
    let fama_value = fama[idx];
    let mama_prev = mama[idx - 1];
    let fama_prev = fama[idx - 1];
    let last_close = sorted[idx].close;
    let diff_pct = if fama_value.abs() > 1e-12 {
        100.0 * (mama_value - fama_value) / fama_value
    } else {
        0.0
    };
    let label = if mama_value > fama_value && diff_pct > 2.0 {
        "STRONG_BULL"
    } else if mama_value > fama_value {
        "BULL"
    } else if mama_value < fama_value && diff_pct < -2.0 {
        "STRONG_BEAR"
    } else if mama_value < fama_value {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    let alpha_final = {
        let phase_diff = (phase[idx - 1] - phase[idx]).max(1.0);
        (fast_limit / phase_diff).max(slow_limit).min(fast_limit)
    };
    MamaSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        fast_limit,
        slow_limit,
        mama_value,
        fama_value,
        mama_prev,
        fama_prev,
        alpha: alpha_final,
        period: period[idx],
        last_close,
        mama_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cog_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> CogSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 10usize;
    if n < length + 4 {
        return CogSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            cog_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length + 4, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|r| r.close).collect();
    let cog_at = |end: usize| -> f64 {
        let mut num = 0.0;
        let mut den = 0.0;
        for k in 0..length {
            let c = closes[end - k];
            let w = (k + 1) as f64;
            num += w * c;
            den += c;
        }
        if den.abs() > 1e-12 { -num / den } else { 0.0 }
    };
    let idx = n - 1;
    let cog_value = cog_at(idx);
    let cog_prev = cog_at(idx - 1);
    let cog_signal = cog_at(idx - 3);
    let diff = cog_value - cog_signal;
    let label = if diff >= 0.5 {
        "STRONG_BULL"
    } else if diff >= 0.1 {
        "BULL"
    } else if diff <= -0.5 {
        "STRONG_BEAR"
    } else if diff <= -0.1 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    CogSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        cog_value,
        cog_signal,
        cog_prev,
        last_close: sorted[idx].close,
        cog_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_didi_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DidiSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let short_length = 3usize;
    let medium_length = 8usize;
    let long_length = 20usize;
    if n < long_length + 2 {
        return DidiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            short_length,
            medium_length,
            long_length,
            didi_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", long_length + 2, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|r| r.close).collect();
    let sma_at = |end: usize, period: usize| -> f64 {
        let mut s = 0.0;
        for k in 0..period {
            s += closes[end - k];
        }
        s / period as f64
    };
    let idx = n - 1;
    let compute_ratios = |end: usize| -> (f64, f64) {
        let s = sma_at(end, short_length);
        let m = sma_at(end, medium_length);
        let l = sma_at(end, long_length);
        let short_r = if m.abs() > 1e-12 { s / m - 1.0 } else { 0.0 };
        let long_r = if m.abs() > 1e-12 { l / m - 1.0 } else { 0.0 };
        (short_r, long_r)
    };
    let (short_ratio, long_ratio) = compute_ratios(idx);
    let (short_prev, long_prev) = compute_ratios(idx - 1);
    let needles_bull =
        short_prev <= 0.0 && short_ratio > 0.0 && long_prev >= 0.0 && long_ratio < 0.0;
    let needles_bear =
        short_prev >= 0.0 && short_ratio < 0.0 && long_prev <= 0.0 && long_ratio > 0.0;
    let label = if needles_bull {
        "BULL_NEEDLES"
    } else if needles_bear {
        "BEAR_NEEDLES"
    } else if short_ratio > 0.0 && long_ratio < 0.0 {
        "BULL"
    } else if short_ratio < 0.0 && long_ratio > 0.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    DidiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        short_length,
        medium_length,
        long_length,
        short_ratio,
        long_ratio,
        short_prev,
        long_prev,
        last_close: sorted[idx].close,
        didi_label: label.into(),
        note: String::new(),
    }
}

/// Tom DeMark's DeMarker over N=14. See DemarkerSnapshot.
pub fn compute_demarker_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DemarkerSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 14usize;
    if n < length + 2 {
        return DemarkerSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            demarker_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length + 2, n),
            ..Default::default()
        };
    }
    let demax_demin_at = |end: usize| -> (f64, f64) {
        let mut dmx = 0.0_f64;
        let mut dmn = 0.0_f64;
        for k in 0..length {
            let i = end - k;
            if i == 0 {
                continue;
            }
            let h = sorted[i].high;
            let hp = sorted[i - 1].high;
            let l = sorted[i].low;
            let lp = sorted[i - 1].low;
            dmx += (h - hp).max(0.0);
            dmn += (lp - l).max(0.0);
        }
        (dmx, dmn)
    };
    let idx = n - 1;
    let (demax_sum, demin_sum) = demax_demin_at(idx);
    let (dmx_prev, dmn_prev) = demax_demin_at(idx - 1);
    let denom = demax_sum + demin_sum;
    let dem = if denom > 1e-12 {
        demax_sum / denom
    } else {
        0.5
    };
    let denom_prev = dmx_prev + dmn_prev;
    let dem_prev = if denom_prev > 1e-12 {
        dmx_prev / denom_prev
    } else {
        0.5
    };
    let label = if dem >= 0.7 {
        "OVERBOUGHT"
    } else if dem <= 0.3 {
        "OVERSOLD"
    } else if dem > 0.5 && dem > dem_prev {
        "BULL"
    } else if dem < 0.5 && dem < dem_prev {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    DemarkerSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        demax_sum,
        demin_sum,
        demarker_value: dem,
        demarker_prev: dem_prev,
        last_close: sorted[idx].close,
        demarker_label: label.into(),
        note: String::new(),
    }
}

/// Bill Williams Gator Oscillator. See GatorSnapshot.
pub fn compute_gator_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> GatorSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let jaw_length = 13usize;
    let teeth_length = 8usize;
    let lips_length = 5usize;
    let min_bars = jaw_length + 8 + 2; // jaw shift 8 + prior-bar
    if n < min_bars {
        return GatorSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            jaw_length,
            teeth_length,
            lips_length,
            gator_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let med: Vec<f64> = sorted.iter().map(|b| (b.high + b.low) / 2.0).collect();
    let smma_of = |len: usize| -> Vec<f64> {
        let mut out = vec![0.0; n];
        if n < len {
            return out;
        }
        let seed: f64 = med[0..len].iter().sum::<f64>() / len as f64;
        out[len - 1] = seed;
        for i in len..n {
            out[i] = (out[i - 1] * (len as f64 - 1.0) + med[i]) / len as f64;
        }
        out
    };
    let smma13 = smma_of(jaw_length);
    let smma8 = smma_of(teeth_length);
    let smma5 = smma_of(lips_length);
    let jaw_t = n - 1 - 8;
    let teeth_t = n - 1 - 5;
    let lips_t = n - 1 - 3;
    let jaw = smma13[jaw_t];
    let jaw_prev = smma13[jaw_t - 1];
    let teeth = smma8[teeth_t];
    let teeth_prev = smma8[teeth_t - 1];
    let lips = smma5[lips_t];
    let lips_prev = smma5[lips_t - 1];
    let upper_bar = (jaw - teeth).abs();
    let lower_bar = -(teeth - lips).abs();
    let upper_prev = (jaw_prev - teeth_prev).abs();
    let lower_prev = -(teeth_prev - lips_prev).abs();
    let last_close = sorted[n - 1].close;
    let upper_growing = upper_bar > upper_prev;
    let lower_growing = lower_bar.abs() > lower_prev.abs();
    let tiny = last_close.abs() * 0.0005; // 0.05% of price
    let label = if upper_bar < tiny && lower_bar.abs() < tiny {
        "SLEEPING"
    } else if upper_growing && lower_growing {
        "EATING"
    } else if !upper_growing && !lower_growing {
        "SATED"
    } else {
        "AWAKENING"
    };
    GatorSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        jaw_length,
        teeth_length,
        lips_length,
        upper_bar,
        lower_bar,
        upper_prev,
        lower_prev,
        last_close,
        gator_label: label.into(),
        note: String::new(),
    }
}

/// Bill Williams Market Facilitation Index. See BwMfiSnapshot.
pub fn compute_bw_mfi_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> BwMfiSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 2 {
        return BwMfiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bwmfi_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥2 bars, got {}", n),
            ..Default::default()
        };
    }
    let mfi_of = |r: &HistoricalPriceRow| -> f64 {
        if r.volume > 1e-9 {
            (r.high - r.low) / r.volume * 1_000_000.0
        } else {
            0.0
        }
    };
    let idx = n - 1;
    let cur = sorted[idx];
    let prv = sorted[idx - 1];
    let mfi = mfi_of(cur);
    let mfi_prev = mfi_of(prv);
    let vol = cur.volume;
    let vol_prev = prv.volume;
    let mfi_up = mfi > mfi_prev;
    let vol_up = vol > vol_prev;
    let color = match (mfi_up, vol_up) {
        (true, true) => "GREEN",
        (false, false) => "FADE",
        (true, false) => "FAKE",
        (false, true) => "SQUAT",
    };
    BwMfiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        mfi_value: mfi,
        mfi_prev,
        volume: vol,
        volume_prev: vol_prev,
        last_close: cur.close,
        bwmfi_color: color.into(),
        bwmfi_label: color.into(),
        note: String::new(),
    }
}

/// Volume Weighted Moving Average over N=20. See VwmaSnapshot.
pub fn compute_vwma_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> VwmaSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 20usize;
    if n < length + 1 {
        return VwmaSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            vwma_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length + 1, n),
            ..Default::default()
        };
    }
    let vwma_at = |end: usize| -> (f64, f64) {
        let mut pv = 0.0_f64;
        let mut vs = 0.0_f64;
        let mut ps = 0.0_f64;
        for k in 0..length {
            let i = end - k;
            let c = sorted[i].close;
            let v = sorted[i].volume.max(0.0);
            pv += c * v;
            vs += v;
            ps += c;
        }
        let vw = if vs > 1e-9 {
            pv / vs
        } else {
            ps / length as f64
        };
        let sm = ps / length as f64;
        (vw, sm)
    };
    let idx = n - 1;
    let (vwma, sma) = vwma_at(idx);
    let (vwma_prev, _) = vwma_at(idx - 1);
    let spread = vwma - sma;
    let spread_ratio = if sma.abs() > 1e-12 { spread / sma } else { 0.0 };
    let last_close = sorted[idx].close;
    let label = if last_close > vwma && vwma > sma {
        "BULL"
    } else if last_close < vwma && vwma < sma {
        "BEAR"
    } else if last_close > vwma {
        "WEAK_BULL"
    } else if last_close < vwma {
        "WEAK_BEAR"
    } else {
        "NEUTRAL"
    };
    VwmaSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        vwma_value: vwma,
        sma_value: sma,
        vwma_prev,
        spread,
        spread_ratio,
        last_close,
        vwma_label: label.into(),
        note: String::new(),
    }
}

/// Rolling sample standard deviation over N=20 with 60-bar regime
/// classifier. See StddevSnapshot.
pub fn compute_stddev_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> StddevSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 20usize;
    let long_length = 60usize;
    if n < long_length {
        return StddevSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            long_length,
            regime_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", long_length, n),
            ..Default::default()
        };
    }
    let stddev_of = |window: usize| -> (f64, f64, f64) {
        let mut s = 0.0_f64;
        let end = n - 1;
        for k in 0..window {
            s += sorted[end - k].close;
        }
        let mean = s / window as f64;
        let mut ss = 0.0_f64;
        for k in 0..window {
            let d = sorted[end - k].close - mean;
            ss += d * d;
        }
        let denom = (window - 1).max(1) as f64;
        let var = ss / denom;
        (mean, var, var.max(0.0).sqrt())
    };
    let (mean, variance, stddev) = stddev_of(length);
    let (_, _, stddev_long) = stddev_of(long_length);
    let cv = if mean.abs() > 1e-12 {
        stddev / mean
    } else {
        0.0
    };
    let annualized = stddev * (252.0_f64).sqrt();
    let ratio = if stddev_long > 1e-12 {
        stddev / stddev_long
    } else {
        1.0
    };
    let label = if ratio > 1.5 {
        "HIGH_VOL"
    } else if ratio < 0.67 {
        "LOW_VOL"
    } else {
        "MID_VOL"
    };
    StddevSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        long_length,
        mean,
        variance,
        stddev,
        stddev_long,
        cv,
        annualized,
        last_close: sorted[n - 1].close,
        regime_label: label.into(),
        note: String::new(),
    }
}
