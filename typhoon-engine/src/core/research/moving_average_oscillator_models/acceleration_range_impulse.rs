use super::*;

// Accelerator, Chaikin volatility, bandwidth, Elder impulse, RMI, SMMA, and Alligator models

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
