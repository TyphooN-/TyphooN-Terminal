use super::*;

mod adaptive_average_transforms;
pub use adaptive_average_transforms::*;
mod price_average_variance;
pub use price_average_variance::*;
mod directional_movement;
pub use directional_movement::*;
mod rate_correlation;
pub use rate_correlation::*;

// ── Round 61 compute fns ──────────────────────────────────────────

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

/// Least-squares slope + intercept of `y[0..N]` vs `x=0..N`.
/// Returns (slope, intercept).
fn least_squares_slope_intercept(y: &[f64]) -> (f64, f64) {
    let n = y.len() as f64;
    if n < 2.0 {
        return (0.0, if y.is_empty() { 0.0 } else { y[0] });
    }
    let mx = (n - 1.0) / 2.0;
    let my: f64 = y.iter().sum::<f64>() / n;
    let mut num = 0.0;
    let mut den = 0.0;
    for (i, &v) in y.iter().enumerate() {
        let dx = i as f64 - mx;
        num += dx * (v - my);
        den += dx * dx;
    }
    let slope = if den.abs() > 1e-12 { num / den } else { 0.0 };
    let intercept = my - slope * mx;
    (slope, intercept)
}

pub fn compute_linearreg_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> LinearregSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length: usize = 14;
    if n < length + 1 {
        return LinearregSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            linearreg_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length + 1, n),
            ..Default::default()
        };
    }
    let closes_now: Vec<f64> = sorted[n - length..].iter().map(|r| r.close).collect();
    let closes_prev: Vec<f64> = sorted[n - length - 1..n - 1]
        .iter()
        .map(|r| r.close)
        .collect();
    let (slope, intercept) = least_squares_slope_intercept(&closes_now);
    let (slope_p, intercept_p) = least_squares_slope_intercept(&closes_prev);
    let fitted = slope * (length as f64 - 1.0) + intercept;
    let fitted_prev = slope_p * (length as f64 - 1.0) + intercept_p;
    let last_close = sorted[n - 1].close;
    let residual = last_close - fitted;
    let residual_pct = if last_close.abs() > 1e-12 {
        100.0 * residual / last_close
    } else {
        0.0
    };
    let label = if residual_pct > 2.0 {
        "ABOVE_TREND"
    } else if residual_pct < -2.0 {
        "BELOW_TREND"
    } else {
        "NEAR_TREND"
    };
    LinearregSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        fitted,
        fitted_prev,
        residual,
        residual_pct,
        last_close,
        linearreg_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_linearreg_angle_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> LinearregAngleSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length: usize = 14;
    if n < length + 1 {
        return LinearregAngleSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            angle_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length + 1, n),
            ..Default::default()
        };
    }
    let closes_now: Vec<f64> = sorted[n - length..].iter().map(|r| r.close).collect();
    let closes_prev: Vec<f64> = sorted[n - length - 1..n - 1]
        .iter()
        .map(|r| r.close)
        .collect();
    let (slope, _) = least_squares_slope_intercept(&closes_now);
    let (slope_p, _) = least_squares_slope_intercept(&closes_prev);
    let angle_deg = slope.atan() * 180.0 / std::f64::consts::PI;
    let angle_deg_prev = slope_p.atan() * 180.0 / std::f64::consts::PI;
    let label = if angle_deg > 30.0 {
        "STRONG_UP"
    } else if angle_deg > 5.0 {
        "UP"
    } else if angle_deg < -30.0 {
        "STRONG_DOWN"
    } else if angle_deg < -5.0 {
        "DOWN"
    } else {
        "FLAT"
    };
    LinearregAngleSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        slope,
        angle_deg,
        angle_deg_prev,
        last_close: sorted[n - 1].close,
        angle_label: label.into(),
        note: String::new(),
    }
}

/// Shared Ehlers Hilbert homodyne pipeline — returns (phase_deg[],
/// period[], i_comp[], q_comp[], smooth_price[]) aligned to bars.
/// Minimum warmup is 64 bars (first 5 are burn-in for smoothing).
fn ehlers_hilbert_pipeline(closes: &[f64]) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let n = closes.len();
    if n < 7 {
        return (vec![0.0; n], vec![0.0; n], vec![0.0; n], vec![0.0; n]);
    }
    let mut smooth = vec![0.0f64; n];
    for t in 3..n {
        smooth[t] =
            (4.0 * closes[t] + 3.0 * closes[t - 1] + 2.0 * closes[t - 2] + closes[t - 3]) / 10.0;
    }
    let mut detrender = vec![0.0f64; n];
    let mut i1 = vec![0.0f64; n];
    let mut q1 = vec![0.0f64; n];
    let mut i2 = vec![0.0f64; n];
    let mut q2 = vec![0.0f64; n];
    let mut period = vec![0.0f64; n];
    let mut phase = vec![0.0f64; n];
    for t in 6..n {
        let p = period[t - 1];
        let ad = 0.0962 * smooth[t] + 0.5769 * smooth[t - 2]
            - 0.5769 * smooth[t - 4]
            - 0.0962 * smooth[t - 6];
        let adj = 0.075 * p + 0.54;
        detrender[t] = ad * adj;
        q1[t] = (0.0962 * detrender[t] + 0.5769 * detrender[t - 2]
            - 0.5769 * detrender[t - 4]
            - 0.0962 * detrender[t - 6])
            * adj;
        i1[t] = detrender[t - 3];
        let ji =
            (0.0962 * i1[t] + 0.5769 * i1[t - 2] - 0.5769 * i1[t - 4] - 0.0962 * i1[t - 6]) * adj;
        let jq =
            (0.0962 * q1[t] + 0.5769 * q1[t - 2] - 0.5769 * q1[t - 4] - 0.0962 * q1[t - 6]) * adj;
        let i2_raw = i1[t] - jq;
        let q2_raw = q1[t] + ji;
        i2[t] = 0.2 * i2_raw + 0.8 * i2[t - 1];
        q2[t] = 0.2 * q2_raw + 0.8 * q2[t - 1];
        let re = i2[t] * i2[t - 1] + q2[t] * q2[t - 1];
        let im = i2[t] * q2[t - 1] - q2[t] * i2[t - 1];
        let re_s = 0.2 * re + 0.8 * if t > 0 { re } else { 0.0 };
        let im_s = 0.2 * im + 0.8 * if t > 0 { im } else { 0.0 };
        let mut per = if im_s.abs() > 1e-12 && re_s.abs() > 1e-12 {
            360.0 / (im_s / re_s).atan().to_degrees().max(0.0001)
        } else {
            p
        };
        if per > 1.5 * p && p > 0.0 {
            per = 1.5 * p;
        }
        if per < 0.67 * p && p > 0.0 {
            per = 0.67 * p;
        }
        if per < 6.0 {
            per = 6.0;
        }
        if per > 50.0 {
            per = 50.0;
        }
        period[t] = 0.2 * per + 0.8 * p;
        let ph_rad = if i1[t].abs() > 1e-12 {
            (q1[t] / i1[t]).atan()
        } else {
            0.0
        };
        let mut ph_deg = ph_rad.to_degrees();
        if i1[t] < 0.0 {
            ph_deg += 180.0;
        }
        if ph_deg < 0.0 {
            ph_deg += 360.0;
        }
        if ph_deg >= 360.0 {
            ph_deg -= 360.0;
        }
        phase[t] = ph_deg;
    }
    (phase, period, i1, q1)
}

pub fn compute_ht_dcphase_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> HtDcphaseSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let need = 64;
    if n < need {
        return HtDcphaseSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            phase_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", need, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|r| r.close).collect();
    let (phase, period, _i1, _q1) = ehlers_hilbert_pipeline(&closes);
    let phase_deg = phase[n - 1];
    let phase_deg_prev = phase[n - 2];
    let mut phase_delta = phase_deg - phase_deg_prev;
    if phase_delta > 180.0 {
        phase_delta -= 360.0;
    }
    if phase_delta < -180.0 {
        phase_delta += 360.0;
    }
    let label = if phase_deg < 45.0 || phase_deg > 315.0 {
        "CYCLE_BOTTOM"
    } else if phase_deg < 135.0 {
        "RISING"
    } else if phase_deg < 225.0 {
        "CYCLE_TOP"
    } else {
        "FALLING"
    };
    HtDcphaseSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        phase_deg,
        phase_deg_prev,
        phase_delta,
        period: period[n - 1],
        last_close: sorted[n - 1].close,
        phase_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_ht_sine_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> HtSineSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let need = 64;
    if n < need {
        return HtSineSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            sine_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", need, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|r| r.close).collect();
    let (phase, period, _i1, _q1) = ehlers_hilbert_pipeline(&closes);
    let sine = (phase[n - 1].to_radians()).sin();
    let sine_prev = (phase[n - 2].to_radians()).sin();
    let leadsine = ((phase[n - 1] + 45.0).to_radians()).sin();
    let leadsine_prev = ((phase[n - 2] + 45.0).to_radians()).sin();
    let crossover = if leadsine > sine && leadsine_prev <= sine_prev {
        1
    } else if leadsine < sine && leadsine_prev >= sine_prev {
        -1
    } else {
        0
    };
    let label = if crossover == 1 {
        "CYCLE_TURN_UP"
    } else if crossover == -1 {
        "CYCLE_TURN_DOWN"
    } else if sine > 0.3 {
        "BULL"
    } else if sine < -0.3 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    HtSineSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        sine,
        sine_prev,
        leadsine,
        leadsine_prev,
        crossover,
        period: period[n - 1],
        last_close: sorted[n - 1].close,
        sine_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_ht_phasor_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> HtPhasorSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let need = 64;
    if n < need {
        return HtPhasorSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            phasor_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", need, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|r| r.close).collect();
    let (_phase, _period, i1, q1) = ehlers_hilbert_pipeline(&closes);
    let i_comp = i1[n - 1];
    let q_comp = q1[n - 1];
    let i_prev = i1[n - 2];
    let q_prev = q1[n - 2];
    let magnitude = (i_comp * i_comp + q_comp * q_comp).sqrt();
    let phase_deg = q_comp.atan2(i_comp).to_degrees();
    let last_close = sorted[n - 1].close;
    let rel_mag = if last_close.abs() > 1e-12 {
        magnitude / last_close
    } else {
        0.0
    };
    let label = if rel_mag > 0.02 {
        "STRONG_CYCLE"
    } else if rel_mag > 0.005 {
        "CYCLE"
    } else {
        "WEAK_CYCLE"
    };
    HtPhasorSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        i_comp,
        q_comp,
        i_prev,
        q_prev,
        magnitude,
        phase_deg,
        last_close,
        phasor_label: label.into(),
        note: String::new(),
    }
}

/// Compute TA-Lib MIDPRICE over a 14-bar default window.
pub fn compute_midprice_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MidpriceSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 14usize;
    if n < length + 1 {
        return MidpriceSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            midprice_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length + 1, n),
            ..Default::default()
        };
    }
    let window = |end_idx: usize| -> (f64, f64, f64) {
        let start = end_idx + 1 - length;
        let mut hhv = f64::NEG_INFINITY;
        let mut llv = f64::INFINITY;
        for r in &sorted[start..=end_idx] {
            if r.high > hhv {
                hhv = r.high;
            }
            if r.low < llv {
                llv = r.low;
            }
        }
        (hhv, llv, 0.5 * (hhv + llv))
    };
    let (hhv_now, llv_now, mid_now) = window(n - 1);
    let (_, _, mid_prev) = window(n - 2);
    let last_close = sorted[n - 1].close;
    let range = hhv_now - llv_now;
    let position = if range.abs() > 1e-12 {
        (last_close - llv_now) / range
    } else {
        0.5
    };
    let label = if position > 0.85 {
        "NEAR_HIGH"
    } else if position > 0.55 {
        "ABOVE_MID"
    } else if position < 0.15 {
        "NEAR_LOW"
    } else if position < 0.45 {
        "BELOW_MID"
    } else {
        "AT_MID"
    };
    MidpriceSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        midprice: mid_now,
        midprice_prev: mid_prev,
        hhv: hhv_now,
        llv: llv_now,
        last_close,
        position,
        midprice_label: label.into(),
        note: String::new(),
    }
}

/// Compute TA-Lib APO (Absolute Price Oscillator) with fast=12, slow=26.
pub fn compute_apo_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> ApoSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let fast_period = 12usize;
    let slow_period = 26usize;
    if n < slow_period + 1 {
        return ApoSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            fast_period,
            slow_period,
            apo_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", slow_period + 1, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|r| r.close).collect();
    let ema = |period: usize| -> Vec<f64> {
        let alpha = 2.0 / (period as f64 + 1.0);
        let mut out = vec![0.0_f64; n];
        let seed: f64 = closes[0..period].iter().sum::<f64>() / period as f64;
        out[period - 1] = seed;
        for i in period..n {
            out[i] = closes[i] * alpha + out[i - 1] * (1.0 - alpha);
        }
        out
    };
    let fast = ema(fast_period);
    let slow = ema(slow_period);
    let apo_now = fast[n - 1] - slow[n - 1];
    let apo_prev = fast[n - 2] - slow[n - 2];
    let last_close = sorted[n - 1].close;
    let apo_pct = if last_close.abs() > 1e-12 {
        apo_now / last_close * 100.0
    } else {
        0.0
    };
    let label = if apo_pct > 1.5 {
        "STRONG_BULL"
    } else if apo_pct > 0.3 {
        "BULL"
    } else if apo_pct < -1.5 {
        "STRONG_BEAR"
    } else if apo_pct < -0.3 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    ApoSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        fast_period,
        slow_period,
        apo: apo_now,
        apo_prev,
        fast_ema: fast[n - 1],
        slow_ema: slow[n - 1],
        last_close,
        apo_label: label.into(),
        note: String::new(),
    }
}

/// Compute TA-Lib MOM (raw momentum) with default period=10.
pub fn compute_mom_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> MomSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 10usize;
    if n < period + 2 {
        return MomSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            mom_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", period + 2, n),
            ..Default::default()
        };
    }
    let last_close = sorted[n - 1].close;
    let prev_close = sorted[n - 2].close;
    let ref_close = sorted[n - 1 - period].close;
    let ref_prev = sorted[n - 2 - period].close;
    let mom_now = last_close - ref_close;
    let mom_prev = prev_close - ref_prev;
    let mom_pct = if last_close.abs() > 1e-12 {
        mom_now / last_close * 100.0
    } else {
        0.0
    };
    let label = if mom_pct > 5.0 {
        "STRONG_UP"
    } else if mom_pct > 1.0 {
        "UP"
    } else if mom_pct < -5.0 {
        "STRONG_DOWN"
    } else if mom_pct < -1.0 {
        "DOWN"
    } else {
        "FLAT"
    };
    MomSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        mom: mom_now,
        mom_prev,
        mom_pct,
        last_close,
        mom_label: label.into(),
        note: String::new(),
    }
}

/// Compute TA-Lib SAREXT (Extended Parabolic SAR) with asymmetric
/// long/short acceleration factors.
#[allow(clippy::too_many_arguments)]
pub fn compute_sarext_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
    start_value: f64,
    af_init_long: f64,
    af_step_long: f64,
    af_max_long: f64,
    af_init_short: f64,
    af_step_short: f64,
    af_max_short: f64,
) -> SarextSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 4 {
        return SarextSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            start_value,
            af_init_long,
            af_step_long,
            af_max_long,
            af_init_short,
            af_step_short,
            af_max_short,
            sarext_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥4 bars, got {}", n),
            ..Default::default()
        };
    }
    let forced_up = start_value > 0.0;
    let forced_down = start_value < 0.0;
    let mut trend_up = if forced_up {
        true
    } else if forced_down {
        false
    } else {
        sorted[1].close >= sorted[0].close
    };
    let mut sar = if trend_up {
        sorted[0].low
    } else {
        sorted[0].high
    };
    let mut ep = if trend_up {
        sorted[0].high
    } else {
        sorted[0].low
    };
    let mut af = if trend_up {
        af_init_long
    } else {
        af_init_short
    };
    let mut bars_in_trend = 1usize;
    for i in 1..n {
        let hi = sorted[i].high;
        let lo = sorted[i].low;
        sar = sar + af * (ep - sar);
        if trend_up {
            let prev_lo = sorted[i - 1].low;
            let prev2_lo = if i >= 2 { sorted[i - 2].low } else { prev_lo };
            sar = sar.min(prev_lo).min(prev2_lo);
            if lo < sar {
                trend_up = false;
                sar = ep;
                ep = lo;
                af = af_init_short;
                bars_in_trend = 1;
            } else {
                if hi > ep {
                    ep = hi;
                    af = (af + af_step_long).min(af_max_long);
                }
                bars_in_trend += 1;
            }
        } else {
            let prev_hi = sorted[i - 1].high;
            let prev2_hi = if i >= 2 { sorted[i - 2].high } else { prev_hi };
            sar = sar.max(prev_hi).max(prev2_hi);
            if hi > sar {
                trend_up = true;
                sar = ep;
                ep = hi;
                af = af_init_long;
                bars_in_trend = 1;
            } else {
                if lo < ep {
                    ep = lo;
                    af = (af + af_step_short).min(af_max_short);
                }
                bars_in_trend += 1;
            }
        }
    }
    let last_close = sorted[n - 1].close;
    let dist_pct = if sar.abs() > f64::EPSILON {
        (last_close - sar) / sar * 100.0
    } else {
        0.0
    };
    let label = if trend_up && dist_pct > 3.0 {
        "STRONG_UP"
    } else if trend_up {
        "UP"
    } else if !trend_up && dist_pct < -3.0 {
        "STRONG_DOWN"
    } else {
        "DOWN"
    };
    SarextSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        start_value,
        af_init_long,
        af_step_long,
        af_max_long,
        af_init_short,
        af_step_short,
        af_max_short,
        sar_value: sar,
        extreme_point: ep,
        acceleration_factor: af,
        trend_is_up: trend_up,
        bars_in_trend,
        distance_pct: dist_pct,
        last_close,
        sarext_label: label.into(),
        note: String::new(),
    }
}

/// Compute TA-Lib ADXR (ADX Rating) over a 14-bar default period.
pub fn compute_adxr_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> AdxrSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 14usize;
    // ADXR needs 2·period (ADX seed) + period (lookback) + 1 bars.
    let min_bars = 3 * period + 1;
    if n < min_bars {
        return AdxrSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            adxr_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let mut tr = vec![0.0_f64; n];
    let mut plus_dm = vec![0.0_f64; n];
    let mut minus_dm = vec![0.0_f64; n];
    for i in 1..n {
        let hi = sorted[i].high;
        let lo = sorted[i].low;
        let pc = sorted[i - 1].close;
        tr[i] = (hi - lo).max((hi - pc).abs()).max((lo - pc).abs());
        let up_move = hi - sorted[i - 1].high;
        let dn_move = sorted[i - 1].low - lo;
        plus_dm[i] = if up_move > dn_move && up_move > 0.0 {
            up_move
        } else {
            0.0
        };
        minus_dm[i] = if dn_move > up_move && dn_move > 0.0 {
            dn_move
        } else {
            0.0
        };
    }
    let p_f = period as f64;
    let mut tr_smooth: f64 = tr[1..=period].iter().sum();
    let mut plus_smooth: f64 = plus_dm[1..=period].iter().sum();
    let mut minus_smooth: f64 = minus_dm[1..=period].iter().sum();
    let mut dx = vec![0.0_f64; n];
    {
        let (pdi, mdi) = if tr_smooth > 0.0 {
            (
                100.0 * plus_smooth / tr_smooth,
                100.0 * minus_smooth / tr_smooth,
            )
        } else {
            (0.0, 0.0)
        };
        let s = pdi + mdi;
        if s > 0.0 {
            dx[period] = 100.0 * (pdi - mdi).abs() / s;
        }
    }
    for i in (period + 1)..n {
        tr_smooth = tr_smooth - tr_smooth / p_f + tr[i];
        plus_smooth = plus_smooth - plus_smooth / p_f + plus_dm[i];
        minus_smooth = minus_smooth - minus_smooth / p_f + minus_dm[i];
        let (pdi, mdi) = if tr_smooth > 0.0 {
            (
                100.0 * plus_smooth / tr_smooth,
                100.0 * minus_smooth / tr_smooth,
            )
        } else {
            (0.0, 0.0)
        };
        let s = pdi + mdi;
        if s > 0.0 {
            dx[i] = 100.0 * (pdi - mdi).abs() / s;
        }
    }
    // Build ADX series from DX via Wilder smoothing, seeded at 2·period.
    let mut adx = vec![0.0_f64; n];
    let seed_idx = 2 * period;
    adx[seed_idx] = dx[(period + 1)..=seed_idx].iter().sum::<f64>() / p_f;
    for i in (seed_idx + 1)..n {
        adx[i] = (adx[i - 1] * (p_f - 1.0) + dx[i]) / p_f;
    }
    let adx_now = adx[n - 1];
    let adx_prior = adx[n - 1 - period];
    let adxr_now = 0.5 * (adx_now + adx_prior);
    let adx_prev_now = adx[n - 2];
    let adx_prev_prior = adx[n - 2 - period];
    let adxr_prev = 0.5 * (adx_prev_now + adx_prev_prior);
    let label = if adxr_now >= 40.0 {
        "STRONG_TREND"
    } else if adxr_now >= 25.0 {
        "TREND"
    } else if adxr_now >= 15.0 {
        "WEAK_TREND"
    } else {
        "NO_TREND"
    };
    AdxrSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        adx_now,
        adx_prior,
        adxr: adxr_now,
        adxr_prev,
        last_close: sorted[n - 1].close,
        adxr_label: label.into(),
        note: String::new(),
    }
}

// ── Round 69 compute fns ────────────────────────────────────────────

/// Walk the trailing `period`-bar window ending at `end_idx` and return
/// `(min_val, min_idx_in_series, max_val, max_idx_in_series)`. The two
/// indices are absolute positions in the sorted array (so `end_idx -
/// idx_in_series` gives a "bars ago" recency).
fn window_extrema(
    sorted: &[&HistoricalPriceRow],
    end_idx: usize,
    period: usize,
) -> (f64, usize, f64, usize) {
    let start = end_idx + 1 - period;
    let mut min_val = sorted[start].close;
    let mut max_val = sorted[start].close;
    let mut min_idx = start;
    let mut max_idx = start;
    for i in (start + 1)..=end_idx {
        let c = sorted[i].close;
        if c < min_val {
            min_val = c;
            min_idx = i;
        }
        if c > max_val {
            max_val = c;
            max_idx = i;
        }
    }
    (min_val, min_idx, max_val, max_idx)
}

fn position_label(pct: f64, high_is_positive: bool) -> &'static str {
    // Three-band cutoff (25% / 75%) — labels depend on whether the
    // caller is framing MIN (near low = bad / bullish-setup) or MAX
    // (near high = good / breakout-setup). Same cutoffs either way,
    // naming reversed.
    if high_is_positive {
        if pct >= 75.0 {
            "NEAR_HIGH"
        } else if pct <= 25.0 {
            "NEAR_LOW"
        } else {
            "MID"
        }
    } else {
        if pct <= 25.0 {
            "NEAR_LOW"
        } else if pct >= 75.0 {
            "NEAR_HIGH"
        } else {
            "MID"
        }
    }
}

fn recency_label(bars_ago: usize, period: usize, is_high: bool) -> &'static str {
    let frac = bars_ago as f64 / period as f64;
    if is_high {
        if frac <= 0.1 {
            "FRESH_HIGH"
        } else if frac <= 0.33 {
            "RECENT_HIGH"
        } else if frac <= 0.66 {
            "OLD_HIGH"
        } else {
            "STALE_HIGH"
        }
    } else {
        if frac <= 0.1 {
            "FRESH_LOW"
        } else if frac <= 0.33 {
            "RECENT_LOW"
        } else if frac <= 0.66 {
            "OLD_LOW"
        } else {
            "STALE_LOW"
        }
    }
}

pub fn compute_min_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> MinSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 30usize;
    let min_bars = period + 1;
    if n < min_bars {
        return MinSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            min_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let (min_now, _, max_now, _) = window_extrema(&sorted, n - 1, period);
    let (min_prev, _, _, _) = window_extrema(&sorted, n - 2, period);
    let close = sorted[n - 1].close;
    let range = max_now - min_now;
    let pct = if range.abs() > 1e-12 {
        (close - min_now) / range * 100.0
    } else {
        50.0
    };
    MinSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        min_val: min_now,
        min_prev,
        max_ref: max_now,
        last_close: close,
        position_pct: pct,
        min_label: position_label(pct, true).into(),
        note: String::new(),
    }
}

pub fn compute_max_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> MaxSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 30usize;
    let min_bars = period + 1;
    if n < min_bars {
        return MaxSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            max_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let (min_now, _, max_now, _) = window_extrema(&sorted, n - 1, period);
    let (_, _, max_prev, _) = window_extrema(&sorted, n - 2, period);
    let close = sorted[n - 1].close;
    let range = max_now - min_now;
    let pct = if range.abs() > 1e-12 {
        (close - min_now) / range * 100.0
    } else {
        50.0
    };
    MaxSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        max_val: max_now,
        max_prev,
        min_ref: min_now,
        last_close: close,
        position_pct: pct,
        max_label: position_label(pct, true).into(),
        note: String::new(),
    }
}

pub fn compute_minmax_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MinMaxSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 30usize;
    let min_bars = period + 1;
    if n < min_bars {
        return MinMaxSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            minmax_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let (min_now, _, max_now, _) = window_extrema(&sorted, n - 1, period);
    let close = sorted[n - 1].close;
    let range = max_now - min_now;
    let range_pct = if close.abs() > 1e-12 {
        100.0 * range / close
    } else {
        0.0
    };
    let pos_pct = if range.abs() > 1e-12 {
        (close - min_now) / range * 100.0
    } else {
        50.0
    };
    let label = if range_pct >= 15.0 {
        "RANGE_WIDE"
    } else if range_pct >= 5.0 {
        "RANGE_NORMAL"
    } else {
        "RANGE_TIGHT"
    };
    MinMaxSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        min_val: min_now,
        max_val: max_now,
        range_width: range,
        range_pct,
        last_close: close,
        position_pct: pos_pct,
        minmax_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_minindex_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MinIndexSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 30usize;
    let min_bars = period + 1;
    if n < min_bars {
        return MinIndexSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            min_index_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let (min_now, min_idx_now, _, _) = window_extrema(&sorted, n - 1, period);
    let (_, min_idx_prev, _, _) = window_extrema(&sorted, n - 2, period);
    let bars_ago = (n - 1).saturating_sub(min_idx_now);
    let bars_ago_prev = (n - 2).saturating_sub(min_idx_prev);
    MinIndexSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        min_val: min_now,
        min_index_bars_ago: bars_ago,
        min_index_bars_ago_prev: bars_ago_prev,
        last_close: sorted[n - 1].close,
        min_index_label: recency_label(bars_ago, period, false).into(),
        note: String::new(),
    }
}

pub fn compute_maxindex_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MaxIndexSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 30usize;
    let min_bars = period + 1;
    if n < min_bars {
        return MaxIndexSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            max_index_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let (_, _, max_now, max_idx_now) = window_extrema(&sorted, n - 1, period);
    let (_, _, _, max_idx_prev) = window_extrema(&sorted, n - 2, period);
    let bars_ago = (n - 1).saturating_sub(max_idx_now);
    let bars_ago_prev = (n - 2).saturating_sub(max_idx_prev);
    MaxIndexSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        max_val: max_now,
        max_index_bars_ago: bars_ago,
        max_index_bars_ago_prev: bars_ago_prev,
        last_close: sorted[n - 1].close,
        max_index_label: recency_label(bars_ago, period, true).into(),
        note: String::new(),
    }
}

// ── Round 70 — BBANDS / AD / ADOSC / SUM / LINEARREG_INTERCEPT ──

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

// ── Round 71 — AROONOSC / MINMAXINDEX / MACDEXT / MACDFIX / MAVP ──

/// Shared AROON computation over the last (period+1) bars ending at end_idx.
/// Returns (aroon_up, aroon_down). Uses `high` for up, `low` for down — the
/// TA-Lib convention. Matches the existing compute_aroon_snapshot math but
/// takes any period (AROONOSC uses 14, not 25).
fn aroon_up_down(sorted: &[&HistoricalPriceRow], end_idx: usize, period: usize) -> (f64, f64) {
    let start = end_idx - period;
    let window = &sorted[start..=end_idx];
    let mut hi_idx = 0usize;
    let mut lo_idx = 0usize;
    for (i, b) in window.iter().enumerate() {
        if b.high > window[hi_idx].high {
            hi_idx = i;
        }
        if b.low < window[lo_idx].low {
            lo_idx = i;
        }
    }
    let last_idx = window.len() - 1;
    let bars_since_high = (last_idx - hi_idx) as f64;
    let bars_since_low = (last_idx - lo_idx) as f64;
    let pf = period as f64;
    (
        100.0 * (pf - bars_since_high) / pf,
        100.0 * (pf - bars_since_low) / pf,
    )
}

pub fn compute_aroonosc_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> AroonoscSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 14usize;
    let min_bars = period + 2;
    if n < min_bars {
        return AroonoscSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            aroonosc_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let (up_now, down_now) = aroon_up_down(&sorted, n - 1, period);
    let (up_prev, down_prev) = aroon_up_down(&sorted, n - 2, period);
    let osc_now = up_now - down_now;
    let osc_prev = up_prev - down_prev;
    let label = if osc_now >= 50.0 {
        "STRONG_BULL"
    } else if osc_now >= 15.0 {
        "BULL"
    } else if osc_now <= -50.0 {
        "STRONG_BEAR"
    } else if osc_now <= -15.0 {
        "BEAR"
    } else {
        "FLAT"
    };
    AroonoscSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        aroonosc: osc_now,
        aroonosc_prev: osc_prev,
        aroon_up: up_now,
        aroon_down: down_now,
        last_close: sorted[n - 1].close,
        aroonosc_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_minmaxindex_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MinMaxIndexSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 30usize;
    let min_bars = period + 1;
    if n < min_bars {
        return MinMaxIndexSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            minmaxindex_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let (_, min_idx, _, max_idx) = window_extrema(&sorted, n - 1, period);
    let min_ago = (n - 1) - min_idx;
    let max_ago = (n - 1) - max_idx;
    let age_diff = min_ago as i64 - max_ago as i64;
    let order = if min_idx > max_idx {
        "LOW_FIRST"
    } else if min_idx < max_idx {
        "HIGH_FIRST"
    } else {
        "SAME_BAR"
    };
    // Priority label: whichever extremum is fresher, if close to present.
    let fresh_cutoff = (period as f64 / 6.0) as usize;
    let stale_cutoff = (2 * period / 3) as usize;
    let label = if min_ago <= fresh_cutoff && max_ago > min_ago {
        "FRESH_LOW"
    } else if max_ago <= fresh_cutoff && min_ago > max_ago {
        "FRESH_HIGH"
    } else if min_ago >= stale_cutoff && max_ago >= stale_cutoff {
        "OLD_EXTREMA"
    } else {
        "MID"
    };
    MinMaxIndexSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        min_index_bars_ago: min_ago,
        max_index_bars_ago: max_ago,
        age_diff,
        extrema_order: order.into(),
        last_close: sorted[n - 1].close,
        minmaxindex_label: label.into(),
        note: String::new(),
    }
}

/// Shared MACD line + signal + histogram generator given a per-bar
/// MA fn (ema_series or sma_series). Returns (macd_now, macd_prev,
/// sig_now, sig_prev, hist_now, hist_prev).
fn macd_triplet<F>(
    closes: &[f64],
    fast: usize,
    slow: usize,
    signal: usize,
    ma: F,
) -> (f64, f64, f64, f64, f64, f64)
where
    F: Fn(&[f64], usize) -> Vec<f64>,
{
    let n = closes.len();
    let fast_ma = ma(closes, fast);
    let slow_ma = ma(closes, slow);
    let mut macd_line = Vec::with_capacity(n);
    for i in 0..n {
        macd_line.push(fast_ma[i] - slow_ma[i]);
    }
    let sig_line = ma(&macd_line, signal);
    let macd_now = macd_line[n - 1];
    let macd_prev = macd_line[n - 2];
    let sig_now = sig_line[n - 1];
    let sig_prev = sig_line[n - 2];
    (
        macd_now,
        macd_prev,
        sig_now,
        sig_prev,
        macd_now - sig_now,
        macd_prev - sig_prev,
    )
}

fn macd_label(hist: f64, hist_prev: f64) -> &'static str {
    let rising = hist > hist_prev;
    let falling = hist < hist_prev;
    if hist > 0.0 && rising {
        "STRONG_BULL"
    } else if hist > 0.0 {
        "BULL"
    } else if hist < 0.0 && falling {
        "STRONG_BEAR"
    } else if hist < 0.0 {
        "BEAR"
    } else {
        "FLAT"
    }
}

pub fn compute_macdext_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MacdextSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let fast = 12usize;
    let slow = 26usize;
    let signal = 9usize;
    let min_bars = slow + signal + 2;
    if n < min_bars {
        return MacdextSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            fast_period: fast,
            slow_period: slow,
            signal_period: signal,
            ma_type: "SMA".into(),
            macdext_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let (macd, macd_p, sig, sig_p, hist, hist_p) =
        macd_triplet(&closes, fast, slow, signal, sma_series);
    MacdextSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        fast_period: fast,
        slow_period: slow,
        signal_period: signal,
        ma_type: "SMA".into(),
        macd,
        macd_prev: macd_p,
        signal: sig,
        signal_prev: sig_p,
        hist,
        hist_prev: hist_p,
        last_close: sorted[n - 1].close,
        macdext_label: macd_label(hist, hist_p).into(),
        note: String::new(),
    }
}

pub fn compute_macdfix_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MacdfixSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let fast = 12usize; // hardcoded per TA-Lib
    let slow = 26usize; // hardcoded per TA-Lib
    let signal = 9usize;
    let min_bars = slow + signal + 2;
    if n < min_bars {
        return MacdfixSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            fast_period: fast,
            slow_period: slow,
            signal_period: signal,
            macdfix_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let (macd, macd_p, sig, sig_p, hist, hist_p) =
        macd_triplet(&closes, fast, slow, signal, ema_series);
    MacdfixSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        fast_period: fast,
        slow_period: slow,
        signal_period: signal,
        macd,
        macd_prev: macd_p,
        signal: sig,
        signal_prev: sig_p,
        hist,
        hist_prev: hist_p,
        last_close: sorted[n - 1].close,
        macdfix_label: macd_label(hist, hist_p).into(),
        note: String::new(),
    }
}

pub fn compute_mavp_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MavpSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let min_period = 5usize;
    let max_period = 30usize;
    let min_bars = max_period + 2;
    if n < min_bars {
        return MavpSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            min_period,
            max_period,
            last_bar_period: max_period,
            mavp_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    // Per-bar period = linear ramp from min_period (start) to max_period (end).
    let period_at = |i: usize| -> usize {
        if n <= 1 {
            return max_period;
        }
        let frac = i as f64 / (n - 1) as f64;
        let p = min_period as f64 + frac * (max_period as f64 - min_period as f64);
        (p.round() as usize).clamp(min_period, max_period)
    };
    let ma_at = |end_idx: usize| -> f64 {
        let p = period_at(end_idx);
        if end_idx + 1 < p {
            return 0.0;
        }
        let start = end_idx + 1 - p;
        let mut s = 0.0;
        for i in start..=end_idx {
            s += sorted[i].close;
        }
        s / p as f64
    };
    let mavp_now = ma_at(n - 1);
    let mavp_prev = ma_at(n - 2);
    let delta = mavp_now - mavp_prev;
    let pct = if mavp_prev.abs() > 1e-12 {
        100.0 * delta / mavp_prev
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
    MavpSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        min_period,
        max_period,
        last_bar_period: period_at(n - 1),
        mavp: mavp_now,
        mavp_prev,
        mavp_delta: delta,
        last_close: sorted[n - 1].close,
        mavp_label: label.into(),
        note: String::new(),
    }
}
