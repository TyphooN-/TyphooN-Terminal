use super::*;

// Fisher, directional trend, money-flow, stop-and-reverse, vortex, and choppiness models

/// FISHER — Ehlers' Fisher Transform over a 10-bar normalised-price window.
pub fn compute_fisher_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> FisherSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 10usize;
    if n < period + 12 {
        return FisherSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            fisher_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", period + 12, n),
            ..Default::default()
        };
    }
    // Typical price midline for Fisher input.
    let hl2: Vec<f64> = sorted.iter().map(|b| (b.high + b.low) / 2.0).collect();
    let mut value = vec![0.0_f64; n];
    let mut fisher = vec![0.0_f64; n];
    for i in period..n {
        let slice = &hl2[(i + 1 - period)..=i];
        let hi = slice.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let lo = slice.iter().cloned().fold(f64::INFINITY, f64::min);
        let rng = (hi - lo).max(1e-12);
        // Normalise current to [-1, 1] with 0.33·2 smoothing (Ehlers' formulation).
        let raw = 0.66 * ((hl2[i] - lo) / rng - 0.5) + 0.67 * value[i - 1];
        value[i] = raw.clamp(-0.999, 0.999);
        fisher[i] = 0.5 * ((1.0 + value[i]) / (1.0 - value[i])).ln() + 0.5 * fisher[i - 1];
    }
    let fisher_now = fisher[n - 1];
    let fisher_prev = fisher[n - 2];
    // Peak abs over last 10 bars
    let tail_start = n.saturating_sub(10);
    let peak_abs = fisher[tail_start..n]
        .iter()
        .map(|x| x.abs())
        .fold(0.0_f64, f64::max);
    // ±2 cross in last 3 bars
    let mut crossed = false;
    for i in (n - 3)..n {
        if i > 0
            && ((fisher[i - 1] >= 2.0) != (fisher[i] >= 2.0)
                || (fisher[i - 1] <= -2.0) != (fisher[i] <= -2.0))
        {
            crossed = true;
            break;
        }
    }
    let label = if fisher_now > 2.0 {
        "STRONG_POS"
    } else if fisher_now > 0.5 {
        "POS"
    } else if fisher_now < -2.0 {
        "STRONG_NEG"
    } else if fisher_now < -0.5 {
        "NEG"
    } else {
        "NEUTRAL"
    };
    FisherSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        fisher_value: fisher_now,
        fisher_signal: fisher_prev,
        extreme_2_cross: crossed,
        peak_abs_10: peak_abs,
        last_close: sorted[n - 1].close,
        fisher_label: label.into(),
        note: String::new(),
    }
}

/// AROON — Aroon Up / Down / Oscillator over 25 bars.
pub fn compute_aroon_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> AroonSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 25usize;
    if n < period + 1 {
        return AroonSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            aroon_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", period + 1, n),
            ..Default::default()
        };
    }
    // Window of the last (period+1) bars, inclusive.
    let window = &sorted[(n - period - 1)..n];
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
    let last_idx = window.len() - 1; // == period
    let bars_since_high = last_idx - hi_idx;
    let bars_since_low = last_idx - lo_idx;
    let up = 100.0 * (period as f64 - bars_since_high as f64) / period as f64;
    let down = 100.0 * (period as f64 - bars_since_low as f64) / period as f64;
    let osc = up - down;
    let label = if osc > 50.0 {
        "STRONG_UP"
    } else if osc > 15.0 {
        "WEAK_UP"
    } else if osc < -50.0 {
        "STRONG_DOWN"
    } else if osc < -15.0 {
        "WEAK_DOWN"
    } else {
        "CONSOLIDATION"
    };
    AroonSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        aroon_up: up,
        aroon_down: down,
        aroon_oscillator: osc,
        bars_since_high,
        bars_since_low,
        last_close: sorted[n - 1].close,
        aroon_label: label.into(),
        note: String::new(),
    }
}

/// ADX — Wilder's Average Directional Index (period 14) with +DI / −DI / DX / ADX.
pub fn compute_adx_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> AdxSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 14usize;
    // Need 2·period + 1 bars for DX smoothing to have a full period.
    let min_bars = 2 * period + 1;
    if n < min_bars {
        return AdxSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            adx_label: "INSUFFICIENT_DATA".into(),
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
    // Wilder smoothing of TR / +DM / −DM.
    let mut atr = vec![0.0_f64; n];
    let mut plus_di = vec![0.0_f64; n];
    let mut minus_di = vec![0.0_f64; n];
    let mut dx = vec![0.0_f64; n];
    let p_f = period as f64;
    let tr_init: f64 = tr[1..=period].iter().sum();
    let plus_init: f64 = plus_dm[1..=period].iter().sum();
    let minus_init: f64 = minus_dm[1..=period].iter().sum();
    atr[period] = tr_init / p_f;
    let mut tr_smooth = tr_init;
    let mut plus_smooth = plus_init;
    let mut minus_smooth = minus_init;
    if tr_smooth > 0.0 {
        plus_di[period] = 100.0 * plus_smooth / tr_smooth;
        minus_di[period] = 100.0 * minus_smooth / tr_smooth;
        let s = plus_di[period] + minus_di[period];
        if s > 0.0 {
            dx[period] = 100.0 * (plus_di[period] - minus_di[period]).abs() / s;
        }
    }
    for i in (period + 1)..n {
        tr_smooth = tr_smooth - tr_smooth / p_f + tr[i];
        plus_smooth = plus_smooth - plus_smooth / p_f + plus_dm[i];
        minus_smooth = minus_smooth - minus_smooth / p_f + minus_dm[i];
        atr[i] = tr_smooth / p_f;
        if tr_smooth > 0.0 {
            plus_di[i] = 100.0 * plus_smooth / tr_smooth;
            minus_di[i] = 100.0 * minus_smooth / tr_smooth;
            let s = plus_di[i] + minus_di[i];
            if s > 0.0 {
                dx[i] = 100.0 * (plus_di[i] - minus_di[i]).abs() / s;
            }
        }
    }
    // ADX = Wilder-smoothed DX (again over `period`), seeded at index 2·period.
    let adx_seed_idx = 2 * period;
    let mut adx_cur = dx[(period + 1)..=adx_seed_idx].iter().sum::<f64>() / p_f;
    for i in (adx_seed_idx + 1)..n {
        adx_cur = (adx_cur * (p_f - 1.0) + dx[i]) / p_f;
    }
    let plus_now = plus_di[n - 1];
    let minus_now = minus_di[n - 1];
    let dx_now = dx[n - 1];
    let atr_now = atr[n - 1];
    let label = if adx_cur >= 40.0 {
        "STRONG_TREND"
    } else if adx_cur >= 25.0 {
        "TREND"
    } else if adx_cur >= 15.0 {
        "WEAK_TREND"
    } else {
        "NO_TREND"
    };
    AdxSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        plus_di: plus_now,
        minus_di: minus_now,
        adx: adx_cur,
        dx: dx_now,
        atr: atr_now,
        last_close: sorted[n - 1].close,
        adx_label: label.into(),
        note: String::new(),
    }
}

/// CCI — Lambert's Commodity Channel Index (period 20).
pub fn compute_cci_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> CciSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 20usize;
    if n < period {
        return CciSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            cci_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", period, n),
            ..Default::default()
        };
    }
    let tp: Vec<f64> = sorted
        .iter()
        .map(|b| (b.high + b.low + b.close) / 3.0)
        .collect();
    let window = &tp[(n - period)..n];
    let sma: f64 = window.iter().sum::<f64>() / period as f64;
    let mad: f64 = window.iter().map(|x| (x - sma).abs()).sum::<f64>() / period as f64;
    let tp_now = tp[n - 1];
    let cci = if mad > 0.0 {
        (tp_now - sma) / (0.015 * mad)
    } else {
        0.0
    };
    let label = if cci > 100.0 {
        "OVERBOUGHT"
    } else if cci > 0.0 {
        "BULL"
    } else if cci < -100.0 {
        "OVERSOLD"
    } else if cci < 0.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    CciSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        typical_price: tp_now,
        tp_sma: sma,
        mean_abs_dev: mad,
        cci_value: cci,
        last_close: sorted[n - 1].close,
        cci_label: label.into(),
        note: String::new(),
    }
}

/// CMF — Chaikin Money Flow (period 20).
pub fn compute_cmf_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> CmfSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 20usize;
    if n < period {
        return CmfSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            cmf_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", period, n),
            ..Default::default()
        };
    }
    let window = &sorted[(n - period)..n];
    let mut mfv_sum = 0.0_f64;
    let mut vol_sum = 0.0_f64;
    for b in window {
        let rng = b.high - b.low;
        let mfm = if rng > 0.0 {
            ((b.close - b.low) - (b.high - b.close)) / rng
        } else {
            0.0
        };
        let mfv = mfm * b.volume;
        mfv_sum += mfv;
        vol_sum += b.volume;
    }
    let cmf = if vol_sum > 0.0 {
        mfv_sum / vol_sum
    } else {
        0.0
    };
    let label = if cmf > 0.25 {
        "STRONG_ACCUM"
    } else if cmf > 0.05 {
        "ACCUM"
    } else if cmf < -0.25 {
        "STRONG_DIST"
    } else if cmf < -0.05 {
        "DIST"
    } else {
        "NEUTRAL"
    };
    CmfSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        cmf_value: cmf,
        money_flow_volume_sum: mfv_sum,
        volume_sum: vol_sum,
        last_close: sorted[n - 1].close,
        cmf_label: label.into(),
        note: String::new(),
    }
}

/// MFI — Money Flow Index (period 14).
pub fn compute_mfi_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> MfiSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 14usize;
    if n < period + 1 {
        return MfiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            mfi_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", period + 1, n),
            ..Default::default()
        };
    }
    let tp: Vec<f64> = sorted
        .iter()
        .map(|b| (b.high + b.low + b.close) / 3.0)
        .collect();
    let mf: Vec<f64> = sorted
        .iter()
        .enumerate()
        .map(|(i, b)| tp[i] * b.volume)
        .collect();
    let mut pos_sum = 0.0_f64;
    let mut neg_sum = 0.0_f64;
    for i in (n - period)..n {
        if i == 0 {
            continue;
        }
        if tp[i] > tp[i - 1] {
            pos_sum += mf[i];
        } else if tp[i] < tp[i - 1] {
            neg_sum += mf[i];
        }
    }
    let ratio = if neg_sum > 0.0 {
        pos_sum / neg_sum
    } else {
        f64::INFINITY
    };
    let mfi = if ratio.is_finite() {
        100.0 - (100.0 / (1.0 + ratio))
    } else {
        100.0
    };
    let label = if mfi > 80.0 {
        "OVERBOUGHT"
    } else if mfi > 50.0 {
        "BULL"
    } else if mfi < 20.0 {
        "OVERSOLD"
    } else if mfi < 50.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    MfiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        mfi_value: mfi,
        positive_mf_sum: pos_sum,
        negative_mf_sum: neg_sum,
        money_flow_ratio: if ratio.is_finite() { ratio } else { 0.0 },
        last_close: sorted[n - 1].close,
        mfi_label: label.into(),
        note: String::new(),
    }
}

/// PSAR — Wilder's Parabolic Stop-And-Reverse.
pub fn compute_psar_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> PsarSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let af_start = 0.02_f64;
    let af_step = 0.02_f64;
    let af_max = 0.20_f64;
    if n < 4 {
        return PsarSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            af_start,
            af_step,
            af_max,
            psar_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥4 bars, got {}", n),
            ..Default::default()
        };
    }
    // Seed: trend direction by comparing first two bars' closes.
    let mut trend_up = sorted[1].close >= sorted[0].close;
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
    let mut af = af_start;
    let mut bars_in_trend = 1usize;
    for i in 1..n {
        let hi = sorted[i].high;
        let lo = sorted[i].low;
        sar = sar + af * (ep - sar);
        if trend_up {
            // SAR must not exceed prior two lows.
            let prev_lo = sorted[i - 1].low;
            let prev2_lo = if i >= 2 { sorted[i - 2].low } else { prev_lo };
            sar = sar.min(prev_lo).min(prev2_lo);
            if lo < sar {
                // Flip to down
                trend_up = false;
                sar = ep;
                ep = lo;
                af = af_start;
                bars_in_trend = 1;
            } else {
                if hi > ep {
                    ep = hi;
                    af = (af + af_step).min(af_max);
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
                af = af_start;
                bars_in_trend = 1;
            } else {
                if lo < ep {
                    ep = lo;
                    af = (af + af_step).min(af_max);
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
    } else if !trend_up {
        "DOWN"
    } else {
        "FLAT"
    };
    PsarSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        af_start,
        af_step,
        af_max,
        sar_value: sar,
        extreme_point: ep,
        acceleration_factor: af,
        trend_is_up: trend_up,
        bars_in_trend,
        distance_pct: dist_pct,
        last_close,
        psar_label: label.into(),
        note: String::new(),
    }
}

/// VORTEX — Botes & Siepman 2009 directional-movement alternative (period 14).
pub fn compute_vortex_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> VortexSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 14usize;
    if n < period + 1 {
        return VortexSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            vortex_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", period + 1, n),
            ..Default::default()
        };
    }
    let start = n - period;
    let mut sum_tr = 0.0_f64;
    let mut sum_vmp = 0.0_f64;
    let mut sum_vmn = 0.0_f64;
    for i in start..n {
        let hi = sorted[i].high;
        let lo = sorted[i].low;
        let pc = sorted[i - 1].close;
        let ph = sorted[i - 1].high;
        let pl = sorted[i - 1].low;
        let tr = (hi - lo).max((hi - pc).abs()).max((lo - pc).abs());
        sum_tr += tr;
        sum_vmp += (hi - pl).abs();
        sum_vmn += (lo - ph).abs();
    }
    let vi_plus = if sum_tr > 0.0 { sum_vmp / sum_tr } else { 0.0 };
    let vi_minus = if sum_tr > 0.0 { sum_vmn / sum_tr } else { 0.0 };
    let diff = vi_plus - vi_minus;
    let label = if diff > 0.1 && vi_plus > 1.0 {
        "BULL_CROSS"
    } else if diff > 0.0 {
        "BULL"
    } else if diff < -0.1 && vi_minus > 1.0 {
        "BEAR_CROSS"
    } else if diff < 0.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    VortexSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        vi_plus,
        vi_minus,
        vi_diff: diff,
        sum_tr,
        sum_vm_plus: sum_vmp,
        sum_vm_minus: sum_vmn,
        last_close: sorted[n - 1].close,
        vortex_label: label.into(),
        note: String::new(),
    }
}

/// CHOP — Choppiness Index (period 14).
pub fn compute_chop_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> ChopSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 14usize;
    if n < period + 1 {
        return ChopSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            chop_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", period + 1, n),
            ..Default::default()
        };
    }
    let start = n - period;
    let mut sum_tr = 0.0_f64;
    let mut rh = f64::MIN;
    let mut rl = f64::MAX;
    for i in start..n {
        let hi = sorted[i].high;
        let lo = sorted[i].low;
        let pc = sorted[i - 1].close;
        let tr = (hi - lo).max((hi - pc).abs()).max((lo - pc).abs());
        sum_tr += tr;
        if hi > rh {
            rh = hi;
        }
        if lo < rl {
            rl = lo;
        }
    }
    let span = rh - rl;
    let chop = if span > 0.0 && sum_tr > 0.0 {
        100.0 * (sum_tr / span).log10() / (period as f64).log10()
    } else {
        0.0
    };
    let label = if chop > 61.8 {
        "CHOP"
    } else if chop > 50.0 {
        "RANGING"
    } else if chop < 38.2 {
        "TRENDING"
    } else if chop < 50.0 {
        "TRANSITIONAL"
    } else {
        "NEUTRAL"
    };
    ChopSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        chop_value: chop,
        sum_tr,
        range_high: rh,
        range_low: rl,
        range_span: span,
        last_close: sorted[n - 1].close,
        chop_label: label.into(),
        note: String::new(),
    }
}
