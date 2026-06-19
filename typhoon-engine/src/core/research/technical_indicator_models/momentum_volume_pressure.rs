use super::*;

// Stochastic RSI, Williams momentum, Elder force, volume-index, candle-pressure, and disparity models

/// STOCHRSI — Chande Stochastic RSI (RSI=14, Stoch=14, %K smoothing=3, %D=3).
pub fn compute_stochrsi_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> StochRsiSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let rsi_p = 14usize;
    let stoch_p = 14usize;
    let k_p = 3usize;
    let d_p = 3usize;
    let min_bars = rsi_p + stoch_p + k_p + d_p + 2;
    if n < min_bars {
        return StochRsiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            rsi_period: rsi_p,
            stoch_period: stoch_p,
            k_period: k_p,
            d_period: d_p,
            stochrsi_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let mut gain = vec![0.0_f64; n];
    let mut loss = vec![0.0_f64; n];
    for i in 1..n {
        let d = closes[i] - closes[i - 1];
        if d > 0.0 {
            gain[i] = d;
        } else {
            loss[i] = -d;
        }
    }
    let mut avg_g = vec![0.0_f64; n];
    let mut avg_l = vec![0.0_f64; n];
    let seed_end = rsi_p;
    if seed_end < n {
        avg_g[seed_end] = gain[1..=rsi_p].iter().sum::<f64>() / rsi_p as f64;
        avg_l[seed_end] = loss[1..=rsi_p].iter().sum::<f64>() / rsi_p as f64;
        for i in (seed_end + 1)..n {
            avg_g[i] = (avg_g[i - 1] * (rsi_p - 1) as f64 + gain[i]) / rsi_p as f64;
            avg_l[i] = (avg_l[i - 1] * (rsi_p - 1) as f64 + loss[i]) / rsi_p as f64;
        }
    }
    let mut rsi = vec![0.0_f64; n];
    for i in seed_end..n {
        let rs = if avg_l[i] > f64::EPSILON {
            avg_g[i] / avg_l[i]
        } else {
            100.0
        };
        rsi[i] = 100.0 - 100.0 / (1.0 + rs);
    }
    let mut raw = vec![0.0_f64; n];
    for i in (seed_end + stoch_p - 1)..n {
        let window = &rsi[(i + 1 - stoch_p)..=i];
        let mn = window.iter().cloned().fold(f64::INFINITY, f64::min);
        let mx = window.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        raw[i] = if (mx - mn).abs() > f64::EPSILON {
            (rsi[i] - mn) / (mx - mn) * 100.0
        } else {
            50.0
        };
    }
    let sma = |src: &[f64], p: usize, start: usize| -> Vec<f64> {
        let mut out = vec![0.0_f64; src.len()];
        for i in (start + p - 1)..src.len() {
            out[i] = src[(i + 1 - p)..=i].iter().sum::<f64>() / p as f64;
        }
        out
    };
    let raw_start = seed_end + stoch_p - 1;
    let k_series = sma(&raw, k_p, raw_start);
    let d_series = sma(&k_series, d_p, raw_start + k_p - 1);
    let rsi_now = rsi[n - 1];
    let rsi_window = &rsi[(n - stoch_p)..n];
    let rsi_min = rsi_window.iter().cloned().fold(f64::INFINITY, f64::min);
    let rsi_max = rsi_window.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let raw_now = raw[n - 1];
    let k_now = k_series[n - 1];
    let d_now = d_series[n - 1];
    let label = if k_now > 80.0 {
        "OVERBOUGHT"
    } else if k_now > 50.0 {
        "BULL"
    } else if k_now < 20.0 {
        "OVERSOLD"
    } else if k_now < 50.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    StochRsiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        rsi_period: rsi_p,
        stoch_period: stoch_p,
        k_period: k_p,
        d_period: d_p,
        rsi_value: rsi_now,
        rsi_min,
        rsi_max,
        stoch_rsi_raw: raw_now,
        k_value: k_now,
        d_value: d_now,
        last_close: closes[n - 1],
        stochrsi_label: label.into(),
        note: String::new(),
    }
}

/// AWESOME — Bill Williams Awesome Oscillator: SMA5(hl2) − SMA34(hl2).
pub fn compute_awesome_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> AwesomeSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let fast_p = 5usize;
    let slow_p = 34usize;
    let min_bars = slow_p + 2;
    if n < min_bars {
        return AwesomeSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            fast_period: fast_p,
            slow_period: slow_p,
            awesome_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let hl2: Vec<f64> = sorted.iter().map(|b| (b.high + b.low) * 0.5).collect();
    let sma = |src: &[f64], p: usize, idx: usize| -> f64 {
        src[(idx + 1 - p)..=idx].iter().sum::<f64>() / p as f64
    };
    let t = n - 1;
    let s_fast_t = sma(&hl2, fast_p, t);
    let s_slow_t = sma(&hl2, slow_p, t);
    let ao_t = s_fast_t - s_slow_t;
    let s_fast_p = sma(&hl2, fast_p, t - 1);
    let s_slow_p = sma(&hl2, slow_p, t - 1);
    let ao_prev = s_fast_p - s_slow_p;
    let color_up = ao_t > ao_prev;
    let abs_scale = sorted[t].close.abs().max(1.0);
    let norm_pct = ao_t / abs_scale * 100.0;
    let label = if ao_t > 0.0 && color_up && norm_pct.abs() > 0.5 {
        "STRONG_BULL"
    } else if ao_t > 0.0 {
        "BULL"
    } else if ao_t < 0.0 && !color_up && norm_pct.abs() > 0.5 {
        "STRONG_BEAR"
    } else if ao_t < 0.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    AwesomeSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        fast_period: fast_p,
        slow_period: slow_p,
        sma_fast: s_fast_t,
        sma_slow: s_slow_t,
        ao_value: ao_t,
        ao_prev,
        ao_color_up: color_up,
        last_close: sorted[t].close,
        awesome_label: label.into(),
        note: String::new(),
    }
}

/// EFI — Elder Force Index: EMA13 of `volume × (close − prev_close)`.
/// Positive + rising = bull pressure; negative + falling = bear pressure;
/// near-zero cross = momentum exhaustion.
pub fn compute_efi_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> EfiSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let ema_p = 13usize;
    let min_bars = ema_p + 4;
    if n < min_bars {
        return EfiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            ema_period: ema_p,
            efi_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    // Raw Force Index per bar from i=1..n-1
    let mut raw: Vec<f64> = Vec::with_capacity(n - 1);
    for i in 1..n {
        raw.push(sorted[i].volume * (sorted[i].close - sorted[i - 1].close));
    }
    if raw.len() < ema_p + 1 {
        return EfiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            ema_period: ema_p,
            efi_label: "INSUFFICIENT_DATA".into(),
            note: format!("raw series {} < EMA+1 {}", raw.len(), ema_p + 1),
            ..Default::default()
        };
    }
    let alpha = 2.0 / (ema_p as f64 + 1.0);
    let mut ema: Vec<f64> = Vec::with_capacity(raw.len());
    let seed: f64 = raw[..ema_p].iter().sum::<f64>() / ema_p as f64;
    ema.push(seed);
    for i in ema_p..raw.len() {
        let prev = *ema.last().unwrap();
        ema.push(alpha * raw[i] + (1.0 - alpha) * prev);
    }
    let efi_value = *ema.last().unwrap();
    let efi_prev = ema
        .get(ema.len().saturating_sub(2))
        .copied()
        .unwrap_or(efi_value);
    let raw_efi = *raw.last().unwrap();
    let last_close = sorted[n - 1].close;
    let abs_scale = (last_close.abs() * sorted[n - 1].volume.max(1.0)).max(1.0);
    let norm = efi_value / abs_scale * 100.0;
    let rising = efi_value > efi_prev;
    let label = if efi_value > 0.0 && rising && norm.abs() > 0.05 {
        "STRONG_BULL"
    } else if efi_value > 0.0 {
        "BULL"
    } else if efi_value < 0.0 && !rising && norm.abs() > 0.05 {
        "STRONG_BEAR"
    } else if efi_value < 0.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    EfiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        ema_period: ema_p,
        raw_efi,
        efi_value,
        efi_prev,
        last_close,
        efi_label: label.into(),
        note: String::new(),
    }
}

/// EMV — Ease of Movement: `(midpoint_change) / (box_ratio)` smoothed by SMA14.
/// `midpoint_change = (H+L)/2 − (H_prev+L_prev)/2`;
/// `box_ratio = (volume / scale) / (H − L)`.
/// High positive = easy upward movement on low volume; low-effort rally.
pub fn compute_emv_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> EmvSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let sma_p = 14usize;
    let vol_scale = 100_000_000.0f64;
    let min_bars = sma_p + 4;
    if n < min_bars {
        return EmvSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            sma_period: sma_p,
            volume_scale: vol_scale,
            emv_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let mut raw: Vec<f64> = Vec::with_capacity(n - 1);
    for i in 1..n {
        let mid_now = (sorted[i].high + sorted[i].low) * 0.5;
        let mid_prev = (sorted[i - 1].high + sorted[i - 1].low) * 0.5;
        let range = (sorted[i].high - sorted[i].low).max(1e-9);
        let box_ratio = (sorted[i].volume / vol_scale) / range;
        let bx = box_ratio.max(1e-9);
        raw.push((mid_now - mid_prev) / bx);
    }
    if raw.len() < sma_p {
        return EmvSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            sma_period: sma_p,
            volume_scale: vol_scale,
            emv_label: "INSUFFICIENT_DATA".into(),
            note: format!("raw series {} < SMA {}", raw.len(), sma_p),
            ..Default::default()
        };
    }
    let t = raw.len() - 1;
    let sma: f64 = raw[(t + 1 - sma_p)..=t].iter().sum::<f64>() / sma_p as f64;
    let raw_t = raw[t];
    let last_close = sorted[n - 1].close;
    let abs_scale = last_close.abs().max(1.0);
    let norm = sma / abs_scale * 100.0;
    let label = if sma > 0.0 && norm.abs() > 1.0 {
        "STRONG_BULL"
    } else if sma > 0.0 {
        "BULL"
    } else if sma < 0.0 && norm.abs() > 1.0 {
        "STRONG_BEAR"
    } else if sma < 0.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    EmvSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        sma_period: sma_p,
        volume_scale: vol_scale,
        raw_emv: raw_t,
        emv_value: sma,
        last_close,
        emv_label: label.into(),
        note: String::new(),
    }
}

/// NVI — Negative Volume Index: accumulates pct-change only when today's volume
/// is LOWER than yesterday's. Fosback: NVI > 1-year-EMA signals "smart money"
/// accumulation (historically 95% odds of bull market).
pub fn compute_nvi_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> NviSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let signal_p = 255usize;
    let min_bars = 30usize; // enough to have a meaningful series even without full signal EMA
    if n < min_bars {
        return NviSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            signal_period: signal_p,
            nvi_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let mut nvi: Vec<f64> = Vec::with_capacity(n);
    nvi.push(1000.0);
    for i in 1..n {
        let prev_nvi = *nvi.last().unwrap();
        if sorted[i].volume < sorted[i - 1].volume && sorted[i - 1].close > 0.0 {
            let pct = (sorted[i].close - sorted[i - 1].close) / sorted[i - 1].close;
            nvi.push(prev_nvi * (1.0 + pct));
        } else {
            nvi.push(prev_nvi);
        }
    }
    // Signal EMA — use whatever period we can fit (min(signal_p, nvi_len/2)).
    let eff_p = signal_p.min(nvi.len().saturating_sub(2).max(3));
    let alpha = 2.0 / (eff_p as f64 + 1.0);
    let seed: f64 = nvi[..eff_p.min(nvi.len())].iter().sum::<f64>() / (eff_p.min(nvi.len())) as f64;
    let mut ema = seed;
    for i in eff_p.min(nvi.len())..nvi.len() {
        ema = alpha * nvi[i] + (1.0 - alpha) * ema;
    }
    let nvi_value = *nvi.last().unwrap();
    let signal_value = ema;
    let last_close = sorted[n - 1].close;
    let spread = (nvi_value - signal_value) / signal_value.abs().max(1.0) * 100.0;
    let label = if nvi_value > signal_value && spread > 0.25 {
        "BULL"
    } else if nvi_value < signal_value && spread < -0.25 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    NviSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        signal_period: eff_p,
        nvi_value,
        signal_value,
        last_close,
        nvi_label: label.into(),
        note: String::new(),
    }
}

/// PVI — Positive Volume Index: mirror of NVI, updating only on UP-volume days.
/// Fosback: crowd-following indicator; PVI < 1-yr-EMA signals "smart money"
/// selling while crowd bought.
pub fn compute_pvi_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> PviSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let signal_p = 255usize;
    let min_bars = 30usize;
    if n < min_bars {
        return PviSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            signal_period: signal_p,
            pvi_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let mut pvi: Vec<f64> = Vec::with_capacity(n);
    pvi.push(1000.0);
    for i in 1..n {
        let prev_pvi = *pvi.last().unwrap();
        if sorted[i].volume > sorted[i - 1].volume && sorted[i - 1].close > 0.0 {
            let pct = (sorted[i].close - sorted[i - 1].close) / sorted[i - 1].close;
            pvi.push(prev_pvi * (1.0 + pct));
        } else {
            pvi.push(prev_pvi);
        }
    }
    let eff_p = signal_p.min(pvi.len().saturating_sub(2).max(3));
    let alpha = 2.0 / (eff_p as f64 + 1.0);
    let seed: f64 = pvi[..eff_p.min(pvi.len())].iter().sum::<f64>() / (eff_p.min(pvi.len())) as f64;
    let mut ema = seed;
    for i in eff_p.min(pvi.len())..pvi.len() {
        ema = alpha * pvi[i] + (1.0 - alpha) * ema;
    }
    let pvi_value = *pvi.last().unwrap();
    let signal_value = ema;
    let last_close = sorted[n - 1].close;
    let spread = (pvi_value - signal_value) / signal_value.abs().max(1.0) * 100.0;
    let label = if pvi_value > signal_value && spread > 0.25 {
        "BULL"
    } else if pvi_value < signal_value && spread < -0.25 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    PviSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        signal_period: eff_p,
        pvi_value,
        signal_value,
        last_close,
        pvi_label: label.into(),
        note: String::new(),
    }
}

/// COPPOCK — Coppock Curve: 10-bar WMA of (14-bar ROC + 11-bar ROC).
/// Originally designed for monthly bars on equity indices; zero-line cross
/// from below = BUY_CROSS (Coppock's "guide" signal).
pub fn compute_coppock_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CoppockSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let roc_fast = 11usize;
    let roc_slow = 14usize;
    let wma_p = 10usize;
    let min_bars = roc_slow + wma_p + 2;
    if n < min_bars {
        return CoppockSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            roc_fast,
            roc_slow,
            wma_period: wma_p,
            coppock_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    // ROC series aligned: index i in ROC corresponds to bar i + roc_slow.
    let mut roc_sum: Vec<f64> = Vec::with_capacity(n - roc_slow);
    for i in roc_slow..n {
        let prev_f = sorted[i - roc_fast].close;
        let prev_s = sorted[i - roc_slow].close;
        let roc_f = if prev_f > 0.0 {
            (sorted[i].close - prev_f) / prev_f * 100.0
        } else {
            0.0
        };
        let roc_s = if prev_s > 0.0 {
            (sorted[i].close - prev_s) / prev_s * 100.0
        } else {
            0.0
        };
        roc_sum.push(roc_f + roc_s);
    }
    if roc_sum.len() < wma_p + 1 {
        return CoppockSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            roc_fast,
            roc_slow,
            wma_period: wma_p,
            coppock_label: "INSUFFICIENT_DATA".into(),
            note: format!("roc len {} < wma+1 {}", roc_sum.len(), wma_p + 1),
            ..Default::default()
        };
    }
    let wma = |src: &[f64], p: usize, idx: usize| -> f64 {
        let mut num = 0.0f64;
        let mut den = 0.0f64;
        for k in 0..p {
            let w = (k as f64) + 1.0; // linear weights: oldest=1, newest=p
            num += w * src[idx + 1 - p + k];
            den += w;
        }
        num / den.max(1e-9)
    };
    let t = roc_sum.len() - 1;
    let coppock_value = wma(&roc_sum, wma_p, t);
    let coppock_prev = wma(&roc_sum, wma_p, t - 1);
    let last_close = sorted[n - 1].close;
    let label = if coppock_prev <= 0.0 && coppock_value > 0.0 {
        "BUY_CROSS"
    } else if coppock_prev >= 0.0 && coppock_value < 0.0 {
        "SELL_CROSS"
    } else if coppock_value > 0.0 {
        "BULL"
    } else if coppock_value < 0.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    CoppockSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        roc_fast,
        roc_slow,
        wma_period: wma_p,
        coppock_value,
        coppock_prev,
        last_close,
        coppock_label: label.into(),
        note: String::new(),
    }
}

/// CMO — Chande Momentum Oscillator: raw gain/loss spread on [-100, +100].
/// Similar to RSI but uses signed gain/loss spread instead of ratio, giving
/// a more linear response at extremes.
pub fn compute_cmo_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> CmoSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 9usize;
    let min_bars = period + 2;
    if n < min_bars {
        return CmoSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            cmo_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let mut sum_up = 0.0f64;
    let mut sum_dn = 0.0f64;
    let start = n - period;
    for i in start..n {
        let d = sorted[i].close - sorted[i - 1].close;
        if d > 0.0 {
            sum_up += d;
        } else if d < 0.0 {
            sum_dn += -d;
        }
    }
    let denom = sum_up + sum_dn;
    let cmo = if denom > 1e-12 {
        100.0 * (sum_up - sum_dn) / denom
    } else {
        0.0
    };
    let last_close = sorted[n - 1].close;
    let label = if cmo > 50.0 {
        "OVERBOUGHT"
    } else if cmo > 0.0 {
        "BULL"
    } else if cmo < -50.0 {
        "OVERSOLD"
    } else if cmo < 0.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    CmoSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        sum_up,
        sum_dn,
        cmo_value: cmo,
        last_close,
        cmo_label: label.into(),
        note: String::new(),
    }
}

/// QSTICK — Q-Stick: simple N-bar average of candle body (close − open).
/// Positive sustained value = consistent bullish candles; negative = bearish.
pub fn compute_qstick_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> QstickSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 14usize;
    let min_bars = period + 2;
    if n < min_bars {
        return QstickSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            qstick_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let bodies: Vec<f64> = sorted.iter().map(|b| b.close - b.open).collect();
    let sma = |src: &[f64], p: usize, idx: usize| -> f64 {
        src[(idx + 1 - p)..=idx].iter().sum::<f64>() / p as f64
    };
    let t = n - 1;
    let qv = sma(&bodies, period, t);
    let qp = sma(&bodies, period, t - 1);
    let last_close = sorted[t].close;
    let abs_scale = last_close.abs().max(1.0);
    let norm = qv / abs_scale * 100.0;
    let label = if qv > 0.0 && norm.abs() > 1.0 {
        "STRONG_BULL"
    } else if qv > 0.0 {
        "BULL"
    } else if qv < 0.0 && norm.abs() > 1.0 {
        "STRONG_BEAR"
    } else if qv < 0.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    QstickSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        qstick_value: qv,
        qstick_prev: qp,
        last_close,
        qstick_label: label.into(),
        note: String::new(),
    }
}

/// DISPARITY — Disparity Index: percentage deviation of close from its SMA.
/// `(close / SMA(close, n) − 1) · 100`. Positive = price above MA (bullish);
/// large magnitude suggests mean-reversion pressure.
pub fn compute_disparity_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DisparitySnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 14usize;
    let min_bars = period + 2;
    if n < min_bars {
        return DisparitySnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            disparity_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let t = n - 1;
    let sma: f64 = closes[(t + 1 - period)..=t].iter().sum::<f64>() / period as f64;
    let last_close = sorted[t].close;
    let disp = if sma.abs() > 1e-12 {
        (last_close / sma - 1.0) * 100.0
    } else {
        0.0
    };
    let label = if disp > 3.0 {
        "STRONG_BULL"
    } else if disp > 0.0 {
        "BULL"
    } else if disp < -3.0 {
        "STRONG_BEAR"
    } else if disp < 0.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    DisparitySnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        sma_value: sma,
        disparity_value: disp,
        last_close,
        disparity_label: label.into(),
        note: String::new(),
    }
}

/// BOP — Balance of Power: per-bar `(close − open) / (high − low)` smoothed
/// by SMA14. Bounded to [-1, +1] per bar; the smoothed line measures
/// sustained buyer vs seller dominance independent of volume.
pub fn compute_bop_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> BopSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 14usize;
    let min_bars = period + 2;
    if n < min_bars {
        return BopSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            bop_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let raw: Vec<f64> = sorted
        .iter()
        .map(|b| {
            let rng = (b.high - b.low).max(1e-9);
            (b.close - b.open) / rng
        })
        .collect();
    let t = n - 1;
    let bop: f64 = raw[(t + 1 - period)..=t].iter().sum::<f64>() / period as f64;
    let raw_t = raw[t];
    let last_close = sorted[t].close;
    let label = if bop > 0.5 {
        "STRONG_BULL"
    } else if bop > 0.0 {
        "BULL"
    } else if bop < -0.5 {
        "STRONG_BEAR"
    } else if bop < 0.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    BopSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        raw_bop: raw_t,
        bop_value: bop,
        last_close,
        bop_label: label.into(),
        note: String::new(),
    }
}
