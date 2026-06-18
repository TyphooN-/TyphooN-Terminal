use super::*;

// Regression, Hilbert-transform, midpoint, oscillator, SAR, and ADX-rating transforms

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
