use super::*;

mod regression_pivot_candles;
pub use regression_pivot_candles::*;
mod adaptive_forecast_vigor;
pub use adaptive_forecast_vigor::*;
mod adaptive_volume_momentum;
pub use adaptive_volume_momentum::*;
mod acceleration_range_impulse;
pub use acceleration_range_impulse::*;
mod momentum_envelope_volume;
pub use momentum_envelope_volume::*;

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

// Shared simple moving-average helper used by oscillator model families.

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
