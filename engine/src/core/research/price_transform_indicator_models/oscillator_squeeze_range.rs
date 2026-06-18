use super::*;

// Laguerre, zigzag, squeeze, range, force, trendline, and fast-stochastic transforms

pub fn compute_laguerre_rsi_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> LaguerreRsiSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 20 {
        return LaguerreRsiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            gamma: 0.5,
            lrsi_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥20 bars, got {}", n),
            ..Default::default()
        };
    }
    let gamma = 0.5_f64;
    let mut l0 = 0.0_f64;
    let mut l1 = 0.0_f64;
    let mut l2 = 0.0_f64;
    let mut l3 = 0.0_f64;
    let mut lrsi_prev = 0.5_f64;
    let mut lrsi = 0.5_f64;
    for i in 0..n {
        let price = sorted[i].close;
        let l0_new = (1.0 - gamma) * price + gamma * l0;
        let l1_new = -gamma * l0_new + l0 + gamma * l1;
        let l2_new = -gamma * l1_new + l1 + gamma * l2;
        let l3_new = -gamma * l2_new + l2 + gamma * l3;
        let mut cu = 0.0_f64;
        let mut cd = 0.0_f64;
        if l0_new >= l1_new {
            cu += l0_new - l1_new;
        } else {
            cd += l1_new - l0_new;
        }
        if l1_new >= l2_new {
            cu += l1_new - l2_new;
        } else {
            cd += l2_new - l1_new;
        }
        if l2_new >= l3_new {
            cu += l2_new - l3_new;
        } else {
            cd += l3_new - l2_new;
        }
        lrsi_prev = lrsi;
        lrsi = if cu + cd > 1e-12 { cu / (cu + cd) } else { 0.5 };
        l0 = l0_new;
        l1 = l1_new;
        l2 = l2_new;
        l3 = l3_new;
    }
    let label = if lrsi > 0.85 {
        "OVERBOUGHT"
    } else if lrsi > 0.60 {
        "BULL"
    } else if lrsi < 0.15 {
        "OVERSOLD"
    } else if lrsi < 0.40 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    LaguerreRsiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        gamma,
        l0,
        l1,
        l2,
        l3,
        laguerre_rsi: lrsi,
        laguerre_rsi_prev: lrsi_prev,
        last_close: sorted[n - 1].close,
        lrsi_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_zigzag_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> ZigzagSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 10 {
        return ZigzagSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            threshold_pct: 5.0,
            current_leg: "INSUFFICIENT_DATA".into(),
            zigzag_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥10 bars, got {}", n),
            ..Default::default()
        };
    }
    let threshold_pct = 5.0_f64;
    let threshold = threshold_pct / 100.0;
    let mut pivot_high_val = sorted[0].high;
    let mut pivot_high_idx: usize = 0;
    let mut pivot_low_val = sorted[0].low;
    let mut pivot_low_idx: usize = 0;
    let mut leg: &str = "UP";
    for i in 1..n {
        let bar = sorted[i];
        if leg == "UP" {
            if bar.high > pivot_high_val {
                pivot_high_val = bar.high;
                pivot_high_idx = i;
            }
            if (pivot_high_val - bar.low) / pivot_high_val >= threshold {
                leg = "DOWN";
                pivot_low_val = bar.low;
                pivot_low_idx = i;
            }
        } else {
            if bar.low < pivot_low_val {
                pivot_low_val = bar.low;
                pivot_low_idx = i;
            }
            if (bar.high - pivot_low_val) / pivot_low_val >= threshold {
                leg = "UP";
                pivot_high_val = bar.high;
                pivot_high_idx = i;
            }
        }
    }
    let last_high_bars_ago = n - 1 - pivot_high_idx;
    let last_low_bars_ago = n - 1 - pivot_low_idx;
    let reversal_level = if leg == "UP" {
        pivot_high_val * (1.0 - threshold)
    } else {
        pivot_low_val * (1.0 + threshold)
    };
    let last_close = sorted[n - 1].close;
    let near_rev = (last_close - reversal_level).abs() / reversal_level < 0.01;
    let label = if near_rev {
        "AT_REVERSAL"
    } else if leg == "UP" {
        "UP_LEG"
    } else {
        "DOWN_LEG"
    };
    ZigzagSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        threshold_pct,
        last_high_value: pivot_high_val,
        last_high_bars_ago,
        last_low_value: pivot_low_val,
        last_low_bars_ago,
        current_leg: leg.into(),
        reversal_level,
        last_close,
        zigzag_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_pgo_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> PgoSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 14usize;
    if n < length + 2 {
        return PgoSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            pgo_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length + 2, n),
            ..Default::default()
        };
    }
    let sma_at = |end_idx: usize| -> f64 {
        let mut s = 0.0_f64;
        for k in 0..length {
            s += sorted[end_idx - k].close;
        }
        s / length as f64
    };
    let tr = |i: usize| -> f64 {
        if i == 0 {
            sorted[0].high - sorted[0].low
        } else {
            let h = sorted[i].high;
            let l = sorted[i].low;
            let pc = sorted[i - 1].close;
            (h - l).max((h - pc).abs()).max((l - pc).abs())
        }
    };
    let alpha = 2.0 / (length as f64 + 1.0);
    let mut ema_tr = {
        let mut s = 0.0_f64;
        for k in 0..length {
            s += tr(k);
        }
        s / length as f64
    };
    for i in length..n {
        ema_tr = alpha * tr(i) + (1.0 - alpha) * ema_tr;
    }
    let sma_now = sma_at(n - 1);
    let sma_prev = sma_at(n - 2);
    let pgo = if ema_tr > 1e-12 {
        (sorted[n - 1].close - sma_now) / ema_tr
    } else {
        0.0
    };
    let mut ema_tr_prev = {
        let mut s = 0.0_f64;
        for k in 0..length {
            s += tr(k);
        }
        s / length as f64
    };
    for i in length..n - 1 {
        ema_tr_prev = alpha * tr(i) + (1.0 - alpha) * ema_tr_prev;
    }
    let pgo_prev = if ema_tr_prev > 1e-12 {
        (sorted[n - 2].close - sma_prev) / ema_tr_prev
    } else {
        0.0
    };
    let label = if pgo > 3.0 {
        "STRONG_BULL"
    } else if pgo > 1.0 {
        "BULL"
    } else if pgo < -3.0 {
        "STRONG_BEAR"
    } else if pgo < -1.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    PgoSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        sma_value: sma_now,
        atr_value: ema_tr,
        pgo_value: pgo,
        pgo_prev,
        last_close: sorted[n - 1].close,
        pgo_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_ht_trendline_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> HtTrendlineSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 64 {
        return HtTrendlineSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            ht_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥64 bars, got {}", n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let mut smooth = vec![0.0_f64; n];
    for i in 3..n {
        smooth[i] =
            (4.0 * closes[i] + 3.0 * closes[i - 1] + 2.0 * closes[i - 2] + closes[i - 3]) / 10.0;
    }
    let detrender = |i: usize, src: &[f64]| -> f64 {
        if i < 6 {
            return 0.0;
        }
        (0.0962 * src[i] + 0.5769 * src[i - 2] - 0.5769 * src[i - 4] - 0.0962 * src[i - 6]) * 0.85
    };
    let mut dt = vec![0.0_f64; n];
    for i in 6..n {
        dt[i] = detrender(i, &smooth);
    }
    let mut q = vec![0.0_f64; n];
    let mut i_sig = vec![0.0_f64; n];
    for i in 6..n {
        q[i] = detrender(i, &dt);
        i_sig[i] = if i >= 3 { dt[i - 3] } else { 0.0 };
    }
    let mut period = 20.0_f64;
    for i in 32..n {
        let re = i_sig[i] * i_sig[i - 1] + q[i] * q[i - 1];
        let im = i_sig[i] * q[i - 1] - q[i] * i_sig[i - 1];
        let p = if re.abs() > 1e-12 {
            2.0 * std::f64::consts::PI / (im / re).atan()
        } else {
            period
        };
        period = p.abs().clamp(6.0, 50.0);
    }
    let period_usize = period.round() as usize;
    let mut num = 0.0_f64;
    let mut den = 0.0_f64;
    for k in 0..period_usize.min(n) {
        let w = (period_usize - k) as f64;
        num += closes[n - 1 - k] * w;
        den += w;
    }
    let trendline = num / den;
    let mut num_p = 0.0_f64;
    let mut den_p = 0.0_f64;
    for k in 0..period_usize.min(n - 1) {
        let w = (period_usize - k) as f64;
        num_p += closes[n - 2 - k] * w;
        den_p += w;
    }
    let trendline_prev = num_p / den_p;
    let close = closes[n - 1];
    let spread = close - trendline;
    let spread_pct = if trendline.abs() > 1e-12 {
        spread / trendline
    } else {
        0.0
    };
    let label = if spread_pct > 0.02 {
        "BULL"
    } else if spread_pct > 0.005 {
        "WEAK_BULL"
    } else if spread_pct < -0.02 {
        "BEAR"
    } else if spread_pct < -0.005 {
        "WEAK_BEAR"
    } else {
        "NEUTRAL"
    };
    HtTrendlineSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        trendline_value: trendline,
        trendline_prev,
        spread,
        spread_pct,
        last_close: close,
        ht_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_midpoint_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MidpointSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 14usize;
    if n < length + 1 {
        return MidpointSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            midpoint_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length + 1, n),
            ..Default::default()
        };
    }
    let hhv_llv = |end_idx: usize| -> (f64, f64) {
        let mut hi = f64::NEG_INFINITY;
        let mut lo = f64::INFINITY;
        for k in 0..length {
            let b = sorted[end_idx - k];
            if b.high > hi {
                hi = b.high;
            }
            if b.low < lo {
                lo = b.low;
            }
        }
        (hi, lo)
    };
    let (hhv, llv) = hhv_llv(n - 1);
    let (hhv_p, llv_p) = hhv_llv(n - 2);
    let midpoint = (hhv + llv) / 2.0;
    let midpoint_prev = (hhv_p + llv_p) / 2.0;
    let rng = hhv - llv;
    let close = sorted[n - 1].close;
    let pos = if rng.abs() > 1e-12 {
        ((close - llv) / rng).clamp(0.0, 1.0)
    } else {
        0.5
    };
    let label = if pos > 0.85 {
        "UPPER"
    } else if pos > 0.60 {
        "NEAR_UPPER"
    } else if pos < 0.15 {
        "LOWER"
    } else if pos < 0.40 {
        "NEAR_LOWER"
    } else {
        "MIDRANGE"
    };
    MidpointSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        hhv,
        llv,
        midpoint,
        midpoint_prev,
        close_position: pos,
        last_close: close,
        midpoint_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_mass_index_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MassIndexSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let ema_len = 9usize;
    let sum_len = 25usize;
    let min_needed = ema_len * 2 + sum_len;
    if n < min_needed {
        return MassIndexSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            ema_len,
            sum_len,
            mass_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_needed, n),
            ..Default::default()
        };
    }
    let alpha = 2.0 / (ema_len as f64 + 1.0);
    let mut ema1 = vec![0.0_f64; n];
    let mut ema2 = vec![0.0_f64; n];
    ema1[0] = sorted[0].high - sorted[0].low;
    for i in 1..n {
        let range = sorted[i].high - sorted[i].low;
        ema1[i] = alpha * range + (1.0 - alpha) * ema1[i - 1];
    }
    ema2[0] = ema1[0];
    for i in 1..n {
        ema2[i] = alpha * ema1[i] + (1.0 - alpha) * ema2[i - 1];
    }
    let ratio_at = |i: usize| -> f64 {
        if ema2[i].abs() > 1e-12 {
            ema1[i] / ema2[i]
        } else {
            1.0
        }
    };
    let mut mass_index = 0.0_f64;
    for k in 0..sum_len {
        mass_index += ratio_at(n - 1 - k);
    }
    let mut mass_prev = 0.0_f64;
    for k in 0..sum_len {
        mass_prev += ratio_at(n - 2 - k);
    }
    let label = if mass_index > 27.0 {
        "REVERSAL_BULGE"
    } else if mass_index > 26.0 {
        "ELEVATED"
    } else if mass_index < 24.0 {
        "COMPRESSED"
    } else {
        "NEUTRAL"
    };
    MassIndexSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        ema_len,
        sum_len,
        ema_range: ema1[n - 1],
        ema_ema_range: ema2[n - 1],
        ratio: ratio_at(n - 1),
        mass_index,
        mass_index_prev: mass_prev,
        last_close: sorted[n - 1].close,
        mass_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_natr_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> NatrSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 14usize;
    if n < length + 2 {
        return NatrSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            natr_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length + 2, n),
            ..Default::default()
        };
    }
    let tr = |i: usize| -> f64 {
        if i == 0 {
            sorted[0].high - sorted[0].low
        } else {
            let h = sorted[i].high;
            let l = sorted[i].low;
            let pc = sorted[i - 1].close;
            (h - l).max((h - pc).abs()).max((l - pc).abs())
        }
    };
    let mut atr = {
        let mut s = 0.0_f64;
        for k in 0..length {
            s += tr(k);
        }
        s / length as f64
    };
    for i in length..n - 1 {
        atr = (atr * (length - 1) as f64 + tr(i)) / length as f64;
    }
    let atr_final = (atr * (length - 1) as f64 + tr(n - 1)) / length as f64;
    let close = sorted[n - 1].close;
    let close_prev = sorted[n - 2].close;
    let natr = if close.abs() > 1e-12 {
        100.0 * atr_final / close
    } else {
        0.0
    };
    let natr_prev = if close_prev.abs() > 1e-12 {
        100.0 * atr / close_prev
    } else {
        0.0
    };
    let label = if natr > 5.0 {
        "HIGH_VOL"
    } else if natr > 2.5 {
        "ELEVATED"
    } else if natr < 1.0 {
        "LOW_VOL"
    } else {
        "NORMAL"
    };
    NatrSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        atr_value: atr_final,
        natr_value: natr,
        natr_prev,
        last_close: close,
        natr_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_ttm_squeeze_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> TtmSqueezeSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 20usize;
    if n < length + 2 {
        return TtmSqueezeSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            squeeze_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length + 2, n),
            ..Default::default()
        };
    }
    let sma = |end_idx: usize| -> f64 {
        let mut s = 0.0_f64;
        for k in 0..length {
            s += sorted[end_idx - k].close;
        }
        s / length as f64
    };
    let stddev = |end_idx: usize, mean: f64| -> f64 {
        let mut ss = 0.0_f64;
        for k in 0..length {
            let d = sorted[end_idx - k].close - mean;
            ss += d * d;
        }
        (ss / length as f64).sqrt()
    };
    let tr = |i: usize| -> f64 {
        if i == 0 {
            sorted[0].high - sorted[0].low
        } else {
            let h = sorted[i].high;
            let l = sorted[i].low;
            let pc = sorted[i - 1].close;
            (h - l).max((h - pc).abs()).max((l - pc).abs())
        }
    };
    let atr = {
        let mut s = 0.0_f64;
        for k in 0..length {
            s += tr(n - 1 - k);
        }
        s / length as f64
    };
    let mid = sma(n - 1);
    let sd = stddev(n - 1, mid);
    let bb_upper = mid + 2.0 * sd;
    let bb_lower = mid - 2.0 * sd;
    let kc_upper = mid + 1.5 * atr;
    let kc_lower = mid - 1.5 * atr;
    let squeeze_on = bb_upper < kc_upper && bb_lower > kc_lower;
    let hhv = |end_idx: usize| -> f64 {
        let mut hi = f64::NEG_INFINITY;
        for k in 0..length {
            if sorted[end_idx - k].high > hi {
                hi = sorted[end_idx - k].high;
            }
        }
        hi
    };
    let llv = |end_idx: usize| -> f64 {
        let mut lo = f64::INFINITY;
        for k in 0..length {
            if sorted[end_idx - k].low < lo {
                lo = sorted[end_idx - k].low;
            }
        }
        lo
    };
    let momentum = sorted[n - 1].close - (hhv(n - 1) + llv(n - 1)) / 2.0;
    let momentum_prev = sorted[n - 2].close - (hhv(n - 2) + llv(n - 2)) / 2.0;
    let label = if squeeze_on {
        "SQUEEZE_ON"
    } else if momentum > 0.0 && momentum_prev <= 0.0 {
        "FIRE_UP"
    } else if momentum < 0.0 && momentum_prev >= 0.0 {
        "FIRE_DOWN"
    } else {
        "NEUTRAL"
    };
    TtmSqueezeSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        bb_upper,
        bb_lower,
        kc_upper,
        kc_lower,
        squeeze_on,
        momentum,
        momentum_prev,
        last_close: sorted[n - 1].close,
        squeeze_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_force_index_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> ForceIndexSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 13usize;
    if n < length + 2 {
        return ForceIndexSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            force_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length + 2, n),
            ..Default::default()
        };
    }
    let force_at = |i: usize| -> f64 {
        if i == 0 {
            0.0
        } else {
            (sorted[i].close - sorted[i - 1].close) * sorted[i].volume as f64
        }
    };
    let alpha = 2.0 / (length as f64 + 1.0);
    let mut ema = force_at(1);
    for i in 2..n - 1 {
        ema = alpha * force_at(i) + (1.0 - alpha) * ema;
    }
    let ema_final = alpha * force_at(n - 1) + (1.0 - alpha) * ema;
    let mut abs_sum = 0.0_f64;
    for i in n.saturating_sub(50).max(1)..n {
        abs_sum += force_at(i).abs();
    }
    let mean_abs = abs_sum / (n - n.saturating_sub(50).max(1)) as f64;
    let ratio = if mean_abs > 1e-12 {
        ema_final / mean_abs
    } else {
        0.0
    };
    let label = if ratio > 1.5 {
        "STRONG_BULL"
    } else if ratio > 0.25 {
        "BULL"
    } else if ratio < -1.5 {
        "STRONG_BEAR"
    } else if ratio < -0.25 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    ForceIndexSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        force_raw: force_at(n - 1),
        force_ema: ema_final,
        force_ema_prev: ema,
        last_close: sorted[n - 1].close,
        last_volume: sorted[n - 1].volume as f64,
        force_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_trange_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> TrangeSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 21 {
        return TrangeSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            trange_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥21 bars, got {}", n),
            ..Default::default()
        };
    }
    let tr = |i: usize| -> f64 {
        if i == 0 {
            sorted[0].high - sorted[0].low
        } else {
            let h = sorted[i].high;
            let l = sorted[i].low;
            let pc = sorted[i - 1].close;
            (h - l).max((h - pc).abs()).max((l - pc).abs())
        }
    };
    let trange_now = tr(n - 1);
    let trange_prev = tr(n - 2);
    let mut sum = 0.0_f64;
    for k in 0..20 {
        sum += tr(n - 1 - k);
    }
    let mean_20 = sum / 20.0;
    let ratio = if mean_20 > 1e-12 {
        trange_now / mean_20
    } else {
        1.0
    };
    let label = if ratio > 1.5 {
        "EXPANSION"
    } else if ratio < 0.5 {
        "CONTRACTION"
    } else {
        "NORMAL"
    };
    TrangeSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        trange_value: trange_now,
        trange_prev,
        mean_trange_20: mean_20,
        trange_ratio: ratio,
        last_high: sorted[n - 1].high,
        last_low: sorted[n - 1].low,
        last_close: sorted[n - 1].close,
        prev_close: sorted[n - 2].close,
        trange_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_linearreg_slope_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> LinearregSlopeSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length: usize = 14;
    if n < length + 1 {
        return LinearregSlopeSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            slope_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length + 1, n),
            ..Default::default()
        };
    }
    let slope_at = |end: usize| -> f64 {
        let start = end + 1 - length;
        let n_f = length as f64;
        let mut sx = 0.0_f64;
        let mut sy = 0.0_f64;
        let mut sxy = 0.0_f64;
        let mut sxx = 0.0_f64;
        for i in 0..length {
            let x = i as f64;
            let y = sorted[start + i].close;
            sx += x;
            sy += y;
            sxy += x * y;
            sxx += x * x;
        }
        let denom = n_f * sxx - sx * sx;
        if denom.abs() > 1e-12 {
            (n_f * sxy - sx * sy) / denom
        } else {
            0.0
        }
    };
    let slope = slope_at(n - 1);
    let slope_prev = slope_at(n - 2);
    let last_close = sorted[n - 1].close;
    let slope_pct = if last_close.abs() > 1e-12 {
        100.0 * slope / last_close
    } else {
        0.0
    };
    let label = if slope_pct > 0.5 {
        "STRONG_UP"
    } else if slope_pct > 0.1 {
        "UP"
    } else if slope_pct < -0.5 {
        "STRONG_DOWN"
    } else if slope_pct < -0.1 {
        "DOWN"
    } else {
        "FLAT"
    };
    LinearregSlopeSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        slope,
        slope_prev,
        slope_pct,
        last_close,
        slope_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_ht_dcperiod_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> HtDcperiodSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 64 {
        return HtDcperiodSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥64 bars, got {}", n),
            ..Default::default()
        };
    }
    // Ehlers Hilbert-transform homodyne discriminator — simplified.
    let mut smooth = vec![0.0_f64; n];
    for i in 3..n {
        smooth[i] = (4.0 * sorted[i].close
            + 3.0 * sorted[i - 1].close
            + 2.0 * sorted[i - 2].close
            + sorted[i - 3].close)
            / 10.0;
    }
    let mut detrender = vec![0.0_f64; n];
    let mut period = vec![0.0_f64; n];
    let mut q1 = vec![0.0_f64; n];
    let mut i1 = vec![0.0_f64; n];
    let mut ji = vec![0.0_f64; n];
    let mut jq = vec![0.0_f64; n];
    let mut i2 = vec![0.0_f64; n];
    let mut q2 = vec![0.0_f64; n];
    let mut re = vec![0.0_f64; n];
    let mut im = vec![0.0_f64; n];
    for i in 6..n {
        let prev_period = if i > 0 { period[i - 1] } else { 0.0 };
        let mult = 0.075 * prev_period + 0.54;
        detrender[i] = (0.0962 * smooth[i] + 0.5769 * smooth[i - 2]
            - 0.5769 * smooth[i - 4]
            - 0.0962 * smooth[i - 6])
            * mult;
        q1[i] = (0.0962 * detrender[i] + 0.5769 * detrender[i - 2]
            - 0.5769 * detrender[i - 4]
            - 0.0962 * detrender[i - 6])
            * mult;
        i1[i] = detrender[i - 3];
        ji[i] =
            (0.0962 * i1[i] + 0.5769 * i1[i - 2] - 0.5769 * i1[i - 4] - 0.0962 * i1[i - 6]) * mult;
        jq[i] =
            (0.0962 * q1[i] + 0.5769 * q1[i - 2] - 0.5769 * q1[i - 4] - 0.0962 * q1[i - 6]) * mult;
        i2[i] = i1[i] - jq[i];
        q2[i] = q1[i] + ji[i];
        i2[i] = 0.2 * i2[i] + 0.8 * i2[i - 1];
        q2[i] = 0.2 * q2[i] + 0.8 * q2[i - 1];
        re[i] = i2[i] * i2[i - 1] + q2[i] * q2[i - 1];
        im[i] = i2[i] * q2[i - 1] - q2[i] * i2[i - 1];
        re[i] = 0.2 * re[i] + 0.8 * re[i - 1];
        im[i] = 0.2 * im[i] + 0.8 * im[i - 1];
        let mut p = if im[i].abs() > 1e-12 && re[i].abs() > 1e-12 {
            360.0 / (im[i] / re[i]).atan().to_degrees()
        } else {
            prev_period
        };
        if p.is_nan() || p.is_infinite() {
            p = prev_period;
        }
        if p > 1.5 * prev_period && prev_period > 0.0 {
            p = 1.5 * prev_period;
        }
        if p < 0.67 * prev_period && prev_period > 0.0 {
            p = 0.67 * prev_period;
        }
        if p < 6.0 {
            p = 6.0;
        }
        if p > 50.0 {
            p = 50.0;
        }
        period[i] = 0.2 * p + 0.8 * prev_period;
    }
    let per = period[n - 1];
    let per_prev = period[n - 2];
    let start = n.saturating_sub(64);
    let p_slice = &period[start..n];
    let p_min = p_slice.iter().cloned().fold(f64::INFINITY, f64::min);
    let p_max = p_slice.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let label = if per > 40.0 {
        "VERY_LONG"
    } else if per > 25.0 {
        "LONG"
    } else if per > 15.0 {
        "MEDIUM"
    } else if per > 8.0 {
        "SHORT"
    } else {
        "VERY_SHORT"
    };
    HtDcperiodSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period: per,
        period_prev: per_prev,
        period_min_64: p_min,
        period_max_64: p_max,
        last_close: sorted[n - 1].close,
        period_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_ht_trendmode_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> HtTrendmodeSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 64 {
        return HtTrendmodeSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            mode_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥64 bars, got {}", n),
            ..Default::default()
        };
    }
    // Reuse the HT_DCPERIOD pipeline (simplified) to get a period
    // series, then classify trend vs cycle based on recent period stability.
    let mut smooth = vec![0.0_f64; n];
    for i in 3..n {
        smooth[i] = (4.0 * sorted[i].close
            + 3.0 * sorted[i - 1].close
            + 2.0 * sorted[i - 2].close
            + sorted[i - 3].close)
            / 10.0;
    }
    let mut period = vec![0.0_f64; n];
    let mut detrender = vec![0.0_f64; n];
    let mut q1 = vec![0.0_f64; n];
    let mut i1 = vec![0.0_f64; n];
    let mut ji = vec![0.0_f64; n];
    let mut jq = vec![0.0_f64; n];
    let mut i2 = vec![0.0_f64; n];
    let mut q2 = vec![0.0_f64; n];
    let mut re = vec![0.0_f64; n];
    let mut im = vec![0.0_f64; n];
    for i in 6..n {
        let prev_period = if i > 0 { period[i - 1] } else { 0.0 };
        let mult = 0.075 * prev_period + 0.54;
        detrender[i] = (0.0962 * smooth[i] + 0.5769 * smooth[i - 2]
            - 0.5769 * smooth[i - 4]
            - 0.0962 * smooth[i - 6])
            * mult;
        q1[i] = (0.0962 * detrender[i] + 0.5769 * detrender[i - 2]
            - 0.5769 * detrender[i - 4]
            - 0.0962 * detrender[i - 6])
            * mult;
        i1[i] = detrender[i - 3];
        ji[i] =
            (0.0962 * i1[i] + 0.5769 * i1[i - 2] - 0.5769 * i1[i - 4] - 0.0962 * i1[i - 6]) * mult;
        jq[i] =
            (0.0962 * q1[i] + 0.5769 * q1[i - 2] - 0.5769 * q1[i - 4] - 0.0962 * q1[i - 6]) * mult;
        i2[i] = 0.2 * (i1[i] - jq[i]) + 0.8 * i2[i - 1];
        q2[i] = 0.2 * (q1[i] + ji[i]) + 0.8 * q2[i - 1];
        re[i] = 0.2 * (i2[i] * i2[i - 1] + q2[i] * q2[i - 1]) + 0.8 * re[i - 1];
        im[i] = 0.2 * (i2[i] * q2[i - 1] - q2[i] * i2[i - 1]) + 0.8 * im[i - 1];
        let mut p = if im[i].abs() > 1e-12 && re[i].abs() > 1e-12 {
            360.0 / (im[i] / re[i]).atan().to_degrees()
        } else {
            prev_period
        };
        if p.is_nan() || p.is_infinite() {
            p = prev_period;
        }
        if p > 1.5 * prev_period && prev_period > 0.0 {
            p = 1.5 * prev_period;
        }
        if p < 0.67 * prev_period && prev_period > 0.0 {
            p = 0.67 * prev_period;
        }
        if p < 6.0 {
            p = 6.0;
        }
        if p > 50.0 {
            p = 50.0;
        }
        period[i] = 0.2 * p + 0.8 * prev_period;
    }
    // Trendmode classifier: compare recent period variance vs mean.
    // Stable period → cycle mode. Erratic or long/accelerating → trend mode.
    let mut trendmode = vec![0_i32; n];
    for i in 20..n {
        let slice = &period[i - 20..=i];
        let mean = slice.iter().sum::<f64>() / slice.len() as f64;
        let var = slice.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / slice.len() as f64;
        let cv = if mean > 1e-12 { var.sqrt() / mean } else { 0.0 };
        trendmode[i] = if cv > 0.15 || period[i] > 35.0 { 1 } else { 0 };
    }
    let mode_now = trendmode[n - 1];
    let mode_prev = trendmode[n - 2];
    let mut lock_in = 1_usize;
    for i in (0..n - 1).rev() {
        if trendmode[i] == mode_now {
            lock_in += 1;
        } else {
            break;
        }
    }
    let label = match mode_now {
        1 => "TREND",
        _ => "CYCLE",
    };
    HtTrendmodeSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        trendmode: mode_now,
        trendmode_prev: mode_prev,
        lock_in_bars: lock_in,
        period: period[n - 1],
        last_close: sorted[n - 1].close,
        mode_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_accbands_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> AccbandsSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length: usize = 20;
    if n < length {
        return AccbandsSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            accbands_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length, n),
            ..Default::default()
        };
    }
    let mut sum_up = 0.0_f64;
    let mut sum_mid = 0.0_f64;
    let mut sum_lo = 0.0_f64;
    for i in (n - length)..n {
        let h = sorted[i].high;
        let l = sorted[i].low;
        let span = if (h + l).abs() > 1e-12 {
            4.0 * (h - l) / (h + l)
        } else {
            0.0
        };
        sum_up += h * (1.0 + span);
        sum_mid += sorted[i].close;
        sum_lo += l * (1.0 - span);
    }
    let n_f = length as f64;
    let upper = sum_up / n_f;
    let middle = sum_mid / n_f;
    let lower = sum_lo / n_f;
    let width = if middle.abs() > 1e-12 {
        (upper - lower) / middle
    } else {
        0.0
    };
    let close = sorted[n - 1].close;
    let pos = if (upper - lower).abs() > 1e-12 {
        (close - lower) / (upper - lower)
    } else {
        0.5
    };
    let label = if close > upper {
        "BREAKOUT_UP"
    } else if close < lower {
        "BREAKOUT_DOWN"
    } else if pos > 0.66 {
        "UPPER"
    } else if pos < 0.34 {
        "LOWER"
    } else {
        "MID"
    };
    AccbandsSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        acc_upper: upper,
        acc_middle: middle,
        acc_lower: lower,
        width,
        position: pos,
        last_close: close,
        accbands_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_stochf_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> StochfSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length: usize = 14;
    let d_period: usize = 3;
    let need = length + d_period + 1;
    if n < need {
        return StochfSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            d_period,
            stochf_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", need, n),
            ..Default::default()
        };
    }
    let fastk_at = |end: usize| -> f64 {
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
        if (hh - ll).abs() > 1e-12 {
            100.0 * (sorted[end].close - ll) / (hh - ll)
        } else {
            50.0
        }
    };
    let fastk = fastk_at(n - 1);
    let fastk_prev = fastk_at(n - 2);
    let fastd: f64 = (0..d_period).map(|k| fastk_at(n - 1 - k)).sum::<f64>() / d_period as f64;
    let fastd_prev: f64 = (0..d_period).map(|k| fastk_at(n - 2 - k)).sum::<f64>() / d_period as f64;
    let label = if fastk > 80.0 {
        "OVERBOUGHT"
    } else if fastk > 60.0 {
        "BULL"
    } else if fastk < 20.0 {
        "OVERSOLD"
    } else if fastk < 40.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    StochfSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        d_period,
        fastk,
        fastk_prev,
        fastd,
        fastd_prev,
        last_close: sorted[n - 1].close,
        stochf_label: label.into(),
        note: String::new(),
    }
}
