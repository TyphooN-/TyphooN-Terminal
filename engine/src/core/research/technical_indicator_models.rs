use super::*;

mod squeeze_trend_channels;
pub use squeeze_trend_channels::*;
mod directional_flow_trend;
pub use directional_flow_trend::*;

/// OBV — On-Balance Volume (cumulative) with 20-bar linear-regression slope.
pub fn compute_obv_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> ObvSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let slope_window = 20usize;
    if n < slope_window + 1 {
        return ObvSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            slope_window,
            obv_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", slope_window + 1, n),
            ..Default::default()
        };
    }
    let mut obv = vec![0.0_f64; n];
    for i in 1..n {
        let delta = sorted[i].close - sorted[i - 1].close;
        let step = if delta > 0.0 {
            sorted[i].volume
        } else if delta < 0.0 {
            -sorted[i].volume
        } else {
            0.0
        };
        obv[i] = obv[i - 1] + step;
    }
    let w_start = n - slope_window;
    let ys = &obv[w_start..n];
    let w = slope_window as f64;
    let sx: f64 = (0..slope_window).map(|i| i as f64).sum();
    let sy: f64 = ys.iter().sum();
    let sxx: f64 = (0..slope_window).map(|i| (i as f64).powi(2)).sum();
    let sxy: f64 = ys.iter().enumerate().map(|(i, y)| (i as f64) * y).sum();
    let denom = w * sxx - sx * sx;
    let slope = if denom.abs() > f64::EPSILON {
        (w * sxy - sx * sy) / denom
    } else {
        0.0
    };
    let start_v = ys[0];
    let end_v = ys[slope_window - 1];
    let change_pct = if start_v.abs() > f64::EPSILON {
        (end_v - start_v) / start_v.abs() * 100.0
    } else {
        0.0
    };
    let omin = ys.iter().cloned().fold(f64::INFINITY, f64::min);
    let omax = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = (omax - omin).abs().max(1.0);
    let slope_norm = slope * slope_window as f64 / range;
    let label = if slope_norm > 0.5 {
        "STRONG_UP"
    } else if slope_norm > 0.1 {
        "UP"
    } else if slope_norm < -0.5 {
        "STRONG_DOWN"
    } else if slope_norm < -0.1 {
        "DOWN"
    } else {
        "NEUTRAL"
    };
    ObvSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        slope_window,
        obv_value: obv[n - 1],
        obv_slope: slope,
        obv_change_pct: change_pct,
        obv_min_20: omin,
        obv_max_20: omax,
        last_close: sorted[n - 1].close,
        obv_label: label.into(),
        note: String::new(),
    }
}

/// TRIX — triple-EMA momentum (period 15, signal EMA 9).
pub fn compute_trix_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> TrixSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 15usize;
    let signal_period = 9usize;
    let min_bars = 3 * period + signal_period + 1;
    if n < min_bars {
        return TrixSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            signal_period,
            trix_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let alpha = 2.0_f64 / (period as f64 + 1.0);
    let ema = |src: &[f64]| -> Vec<f64> {
        let mut out = vec![0.0_f64; src.len()];
        out[0] = src[0];
        for i in 1..src.len() {
            out[i] = alpha * src[i] + (1.0 - alpha) * out[i - 1];
        }
        out
    };
    let e1 = ema(&closes);
    let e2 = ema(&e1);
    let e3 = ema(&e2);
    let mut trix_series = vec![0.0_f64; n];
    for i in 1..n {
        trix_series[i] = if e3[i - 1].abs() > f64::EPSILON {
            100.0 * (e3[i] / e3[i - 1] - 1.0)
        } else {
            0.0
        };
    }
    let sig_alpha = 2.0_f64 / (signal_period as f64 + 1.0);
    let mut sig = vec![0.0_f64; n];
    sig[0] = trix_series[0];
    for i in 1..n {
        sig[i] = sig_alpha * trix_series[i] + (1.0 - sig_alpha) * sig[i - 1];
    }
    let trix_now = trix_series[n - 1];
    let sig_now = sig[n - 1];
    let hist = trix_now - sig_now;
    let label = if trix_now > 0.0 && hist > 0.0 && trix_now.abs() > 0.05 {
        "STRONG_BULL"
    } else if trix_now > 0.0 {
        "BULL"
    } else if trix_now < 0.0 && hist < 0.0 && trix_now.abs() > 0.05 {
        "STRONG_BEAR"
    } else if trix_now < 0.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    TrixSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        signal_period,
        trix_value: trix_now,
        signal_value: sig_now,
        histogram: hist,
        ema3_value: e3[n - 1],
        last_close: sorted[n - 1].close,
        trix_label: label.into(),
        note: String::new(),
    }
}

/// HMA — Hull Moving Average (period 20, √p≈4).
pub fn compute_hma_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> HmaSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 20usize;
    let half = period / 2;
    let sqrt_p = (period as f64).sqrt().floor() as usize;
    let min_bars = period + sqrt_p + 5;
    if n < min_bars {
        return HmaSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            half_period: half,
            sqrt_period: sqrt_p,
            hma_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let wma = |src: &[f64], len: usize| -> Vec<f64> {
        let mut out = vec![0.0_f64; src.len()];
        let wsum: f64 = (1..=len).map(|i| i as f64).sum();
        for i in (len - 1)..src.len() {
            let mut acc = 0.0_f64;
            for k in 0..len {
                acc += src[i - (len - 1) + k] * (k + 1) as f64;
            }
            out[i] = acc / wsum;
        }
        out
    };
    let w_half = wma(&closes, half);
    let w_full = wma(&closes, period);
    let raw: Vec<f64> = (0..n).map(|i| 2.0 * w_half[i] - w_full[i]).collect();
    let hma_series = wma(&raw, sqrt_p);
    let hma_now = hma_series[n - 1];
    let back_idx = n.saturating_sub(6);
    let hma_back = hma_series[back_idx];
    let slope_pct = if hma_back.abs() > f64::EPSILON {
        (hma_now - hma_back) / hma_back.abs() * 100.0
    } else {
        0.0
    };
    let last_close = sorted[n - 1].close;
    let vs_close = if hma_now.abs() > f64::EPSILON {
        (last_close - hma_now) / hma_now * 100.0
    } else {
        0.0
    };
    let label = if slope_pct > 2.0 {
        "STRONG_UP"
    } else if slope_pct > 0.2 {
        "UP"
    } else if slope_pct < -2.0 {
        "STRONG_DOWN"
    } else if slope_pct < -0.2 {
        "DOWN"
    } else {
        "NEUTRAL"
    };
    HmaSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        half_period: half,
        sqrt_period: sqrt_p,
        hma_value: hma_now,
        hma_slope_pct: slope_pct,
        hma_vs_close_pct: vs_close,
        last_close,
        hma_label: label.into(),
        note: String::new(),
    }
}

/// PPO — Percentage Price Oscillator (fast 12, slow 26, signal 9).
pub fn compute_ppo_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> PpoSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let fast_p = 12usize;
    let slow_p = 26usize;
    let signal_p = 9usize;
    let min_bars = slow_p + signal_p + 2;
    if n < min_bars {
        return PpoSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            fast_period: fast_p,
            slow_period: slow_p,
            signal_period: signal_p,
            ppo_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let ema = |src: &[f64], p: usize| -> Vec<f64> {
        let mut out = vec![0.0_f64; src.len()];
        let k = 2.0 / (p as f64 + 1.0);
        let seed: f64 = src[..p].iter().sum::<f64>() / p as f64;
        out[p - 1] = seed;
        for i in p..src.len() {
            out[i] = src[i] * k + out[i - 1] * (1.0 - k);
        }
        out
    };
    let e_fast = ema(&closes, fast_p);
    let e_slow = ema(&closes, slow_p);
    let mut ppo_series = vec![0.0_f64; n];
    for i in (slow_p - 1)..n {
        ppo_series[i] = if e_slow[i].abs() > f64::EPSILON {
            100.0 * (e_fast[i] - e_slow[i]) / e_slow[i]
        } else {
            0.0
        };
    }
    let sig_seed_end = slow_p - 1 + signal_p - 1;
    let signal_series = {
        let mut out = vec![0.0_f64; n];
        if sig_seed_end < n {
            let k = 2.0 / (signal_p as f64 + 1.0);
            let seed: f64 =
                ppo_series[(slow_p - 1)..=sig_seed_end].iter().sum::<f64>() / signal_p as f64;
            out[sig_seed_end] = seed;
            for i in (sig_seed_end + 1)..n {
                out[i] = ppo_series[i] * k + out[i - 1] * (1.0 - k);
            }
        }
        out
    };
    let ppo_now = ppo_series[n - 1];
    let sig_now = signal_series[n - 1];
    let hist = ppo_now - sig_now;
    let label = if ppo_now > 0.0 && ppo_now > sig_now && ppo_now.abs() > 0.1 {
        "STRONG_BULL"
    } else if ppo_now > 0.0 {
        "BULL"
    } else if ppo_now < 0.0 && ppo_now < sig_now && ppo_now.abs() > 0.1 {
        "STRONG_BEAR"
    } else if ppo_now < 0.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    PpoSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        fast_period: fast_p,
        slow_period: slow_p,
        signal_period: signal_p,
        ema_fast: e_fast[n - 1],
        ema_slow: e_slow[n - 1],
        ppo_value: ppo_now,
        signal_value: sig_now,
        histogram: hist,
        last_close: sorted[n - 1].close,
        ppo_label: label.into(),
        note: String::new(),
    }
}

/// DPO — Detrended Price Oscillator (period 20).
pub fn compute_dpo_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> DpoSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 20usize;
    let shift = period / 2 + 1;
    let min_bars = period + shift + 1;
    if n < min_bars {
        return DpoSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            shift,
            dpo_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let t = n - 1;
    let sma_window_end = t;
    let sma_window_start = sma_window_end + 1 - period;
    let sma_val: f64 = closes[sma_window_start..=sma_window_end]
        .iter()
        .sum::<f64>()
        / period as f64;
    let past_idx = t.saturating_sub(shift);
    let past_close = closes[past_idx];
    let dpo_val = past_close - sma_val;
    let dpo_pct = if sma_val.abs() > f64::EPSILON {
        dpo_val / sma_val * 100.0
    } else {
        0.0
    };
    let label = if dpo_pct > 5.0 {
        "PEAK_HIGH"
    } else if dpo_pct > 0.5 {
        "BULL"
    } else if dpo_pct < -5.0 {
        "PEAK_LOW"
    } else if dpo_pct < -0.5 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    DpoSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        shift,
        sma_value: sma_val,
        dpo_value: dpo_val,
        dpo_pct,
        last_close: closes[t],
        dpo_label: label.into(),
        note: String::new(),
    }
}

/// KST — Pring Know Sure Thing (ROC(10,15,20,30) smoothed, weighted 1/2/3/4, sig=9).
pub fn compute_kst_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> KstSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let roc_periods = [10usize, 15, 20, 30];
    let sma_periods = [10usize, 10, 10, 15];
    let min_bars = 30 + 15 + 9 + 2; // 56
    if n < min_bars {
        return KstSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            kst_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let roc = |rp: usize| -> Vec<f64> {
        let mut out = vec![0.0_f64; n];
        for i in rp..n {
            let prev = closes[i - rp];
            out[i] = if prev.abs() > f64::EPSILON {
                (closes[i] - prev) / prev * 100.0
            } else {
                0.0
            };
        }
        out
    };
    let sma = |src: &[f64], p: usize, start: usize| -> Vec<f64> {
        let mut out = vec![0.0_f64; src.len()];
        for i in (start + p - 1)..src.len() {
            out[i] = src[(i + 1 - p)..=i].iter().sum::<f64>() / p as f64;
        }
        out
    };
    let r1 = roc(roc_periods[0]);
    let r2 = roc(roc_periods[1]);
    let r3 = roc(roc_periods[2]);
    let r4 = roc(roc_periods[3]);
    let rc1 = sma(&r1, sma_periods[0], roc_periods[0]);
    let rc2 = sma(&r2, sma_periods[1], roc_periods[1]);
    let rc3 = sma(&r3, sma_periods[2], roc_periods[2]);
    let rc4 = sma(&r4, sma_periods[3], roc_periods[3]);
    let mut kst_series = vec![0.0_f64; n];
    let kst_start = 30 + 15 - 1; // earliest index where all 4 RCMAs are defined (RCMA4 = SMA(ROC(30),15))
    for i in kst_start..n {
        kst_series[i] = 1.0 * rc1[i] + 2.0 * rc2[i] + 3.0 * rc3[i] + 4.0 * rc4[i];
    }
    let sig_series = sma(&kst_series, 9, kst_start);
    let kst_now = kst_series[n - 1];
    let sig_now = sig_series[n - 1];
    let hist = kst_now - sig_now;
    let label = if kst_now > 0.0 && kst_now > sig_now && kst_now.abs() > 1.0 {
        "STRONG_BULL"
    } else if kst_now > 0.0 {
        "BULL"
    } else if kst_now < 0.0 && kst_now < sig_now && kst_now.abs() > 1.0 {
        "STRONG_BEAR"
    } else if kst_now < 0.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    KstSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        rcma1: rc1[n - 1],
        rcma2: rc2[n - 1],
        rcma3: rc3[n - 1],
        rcma4: rc4[n - 1],
        kst_value: kst_now,
        signal_value: sig_now,
        histogram: hist,
        last_close: closes[n - 1],
        kst_label: label.into(),
        note: String::new(),
    }
}

/// ULTOSC — Williams Ultimate Oscillator (7/14/28, weights 4/2/1).
pub fn compute_ultosc_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> UltoscSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let ps = 7usize;
    let pm = 14usize;
    let pl = 28usize;
    let min_bars = pl + 2;
    if n < min_bars {
        return UltoscSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period_short: ps,
            period_mid: pm,
            period_long: pl,
            ultosc_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let mut bp = vec![0.0_f64; n];
    let mut tr = vec![0.0_f64; n];
    for i in 1..n {
        let c = sorted[i].close;
        let l = sorted[i].low;
        let h = sorted[i].high;
        let pc = sorted[i - 1].close;
        let min_lc = l.min(pc);
        let max_hc = h.max(pc);
        bp[i] = c - min_lc;
        tr[i] = max_hc - min_lc;
    }
    let sum_tail = |src: &[f64], p: usize| -> f64 { src[(n - p)..n].iter().sum::<f64>() };
    let sum_bp_s = sum_tail(&bp, ps);
    let sum_tr_s = sum_tail(&tr, ps);
    let sum_bp_m = sum_tail(&bp, pm);
    let sum_tr_m = sum_tail(&tr, pm);
    let sum_bp_l = sum_tail(&bp, pl);
    let sum_tr_l = sum_tail(&tr, pl);
    let avg_s = if sum_tr_s > f64::EPSILON {
        sum_bp_s / sum_tr_s
    } else {
        0.0
    };
    let avg_m = if sum_tr_m > f64::EPSILON {
        sum_bp_m / sum_tr_m
    } else {
        0.0
    };
    let avg_l = if sum_tr_l > f64::EPSILON {
        sum_bp_l / sum_tr_l
    } else {
        0.0
    };
    let uo = 100.0 * (4.0 * avg_s + 2.0 * avg_m + avg_l) / 7.0;
    let label = if uo > 70.0 {
        "OVERBOUGHT"
    } else if uo > 50.0 {
        "BULL"
    } else if uo < 30.0 {
        "OVERSOLD"
    } else if uo < 50.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    UltoscSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period_short: ps,
        period_mid: pm,
        period_long: pl,
        avg_short: avg_s,
        avg_mid: avg_m,
        avg_long: avg_l,
        ultosc_value: uo,
        last_close: sorted[n - 1].close,
        ultosc_label: label.into(),
        note: String::new(),
    }
}

/// WILLR — Larry Williams %R (period 14).
pub fn compute_willr_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> WillrSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 14usize;
    if n < period + 1 {
        return WillrSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            willr_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", period + 1, n),
            ..Default::default()
        };
    }
    let start = n - period;
    let mut hh = f64::NEG_INFINITY;
    let mut ll = f64::INFINITY;
    for r in &sorted[start..n] {
        if r.high > hh {
            hh = r.high;
        }
        if r.low < ll {
            ll = r.low;
        }
    }
    let last_close = sorted[n - 1].close;
    let range = hh - ll;
    let willr = if range > f64::EPSILON {
        (hh - last_close) / range * -100.0
    } else {
        -50.0
    };
    let label = if willr > -20.0 {
        "OVERBOUGHT"
    } else if willr > -50.0 {
        "BULL"
    } else if willr < -80.0 {
        "OVERSOLD"
    } else if willr < -50.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    WillrSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        highest_high: hh,
        lowest_low: ll,
        willr_value: willr,
        last_close,
        willr_label: label.into(),
        note: String::new(),
    }
}

/// MASS — Donald Dorsey Mass Index (EMA=9, sum window=25).
pub fn compute_mass_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MassSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let ema_p = 9usize;
    let sum_p = 25usize;
    let min_bars = 2 * ema_p + sum_p + 2;
    if n < min_bars {
        return MassSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            ema_period: ema_p,
            sum_period: sum_p,
            mass_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let hl: Vec<f64> = sorted.iter().map(|b| b.high - b.low).collect();
    let ema = |src: &[f64], p: usize, start: usize| -> Vec<f64> {
        let mut out = vec![0.0_f64; src.len()];
        if start + p > src.len() {
            return out;
        }
        let k = 2.0 / (p as f64 + 1.0);
        let seed: f64 = src[start..start + p].iter().sum::<f64>() / p as f64;
        out[start + p - 1] = seed;
        for i in (start + p)..src.len() {
            out[i] = src[i] * k + out[i - 1] * (1.0 - k);
        }
        out
    };
    let e1 = ema(&hl, ema_p, 0);
    let e2 = ema(&e1, ema_p, ema_p - 1);
    let mut ratio = vec![0.0_f64; n];
    for i in (2 * ema_p - 2)..n {
        ratio[i] = if e2[i].abs() > f64::EPSILON {
            e1[i] / e2[i]
        } else {
            0.0
        };
    }
    let start = n - sum_p;
    let mass: f64 = ratio[start..n].iter().sum();
    let single_ratio = ratio[n - 1];
    let label = if mass > 27.0 {
        "REVERSAL_BULGE"
    } else if mass > 25.0 {
        "WATCH"
    } else {
        "NEUTRAL"
    };
    MassSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        ema_period: ema_p,
        sum_period: sum_p,
        mass_value: mass,
        single_ratio,
        last_close: sorted[n - 1].close,
        mass_label: label.into(),
        note: String::new(),
    }
}

/// CHAIKOSC — Chaikin Oscillator (fast 3, slow 10 EMAs of A/D line).
pub fn compute_chaikosc_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> ChaikoscSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let fast_p = 3usize;
    let slow_p = 10usize;
    let min_bars = slow_p + 2;
    if n < min_bars {
        return ChaikoscSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            fast_period: fast_p,
            slow_period: slow_p,
            chaikosc_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let mut ad = vec![0.0_f64; n];
    for i in 0..n {
        let h = sorted[i].high;
        let l = sorted[i].low;
        let c = sorted[i].close;
        let v = sorted[i].volume as f64;
        let range = h - l;
        let mfm = if range > f64::EPSILON {
            ((c - l) - (h - c)) / range
        } else {
            0.0
        };
        let mfv = mfm * v;
        ad[i] = if i == 0 { mfv } else { ad[i - 1] + mfv };
    }
    let ema = |src: &[f64], p: usize| -> Vec<f64> {
        let mut out = vec![0.0_f64; src.len()];
        let k = 2.0 / (p as f64 + 1.0);
        let seed: f64 = src[..p].iter().sum::<f64>() / p as f64;
        out[p - 1] = seed;
        for i in p..src.len() {
            out[i] = src[i] * k + out[i - 1] * (1.0 - k);
        }
        out
    };
    let e_fast = ema(&ad, fast_p);
    let e_slow = ema(&ad, slow_p);
    let co = e_fast[n - 1] - e_slow[n - 1];
    let abs_ad = ad[n - 1].abs().max(1.0);
    let norm = co / abs_ad;
    let label = if norm > 0.02 {
        "STRONG_ACCUM"
    } else if norm > 0.002 {
        "ACCUM"
    } else if norm < -0.02 {
        "STRONG_DIST"
    } else if norm < -0.002 {
        "DIST"
    } else {
        "NEUTRAL"
    };
    ChaikoscSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        fast_period: fast_p,
        slow_period: slow_p,
        ad_last: ad[n - 1],
        ema_fast_ad: e_fast[n - 1],
        ema_slow_ad: e_slow[n - 1],
        chaikosc_value: co,
        last_close: sorted[n - 1].close,
        chaikosc_label: label.into(),
        note: String::new(),
    }
}

/// KLINGER — Volume Oscillator (fast 34, slow 55, signal 13).
pub fn compute_klinger_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> KlingerSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let fast_p = 34usize;
    let slow_p = 55usize;
    let signal_p = 13usize;
    let min_bars = slow_p + signal_p + 3;
    if n < min_bars {
        return KlingerSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            fast_period: fast_p,
            slow_period: slow_p,
            signal_period: signal_p,
            klinger_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let mut vf = vec![0.0_f64; n];
    let mut prev_hlc = 0.0_f64;
    let mut trend = 1_i32;
    let mut dm_prev = 0.0_f64;
    let mut cm_prev = 0.0_f64;
    for i in 1..n {
        let h = sorted[i].high;
        let l = sorted[i].low;
        let c = sorted[i].close;
        let v = sorted[i].volume as f64;
        let hlc = h + l + c;
        let dm = h - l;
        let cm = if (trend > 0 && i > 1) || (trend < 0 && i > 1) {
            if trend.signum() == (if hlc > prev_hlc { 1 } else { -1 }) {
                cm_prev + dm
            } else {
                dm_prev + dm
            }
        } else {
            dm
        };
        let new_trend = if hlc > prev_hlc { 1 } else { -1 };
        let signed = new_trend as f64;
        let ratio = if cm.abs() > f64::EPSILON {
            dm / cm
        } else {
            0.0
        };
        vf[i] = v * (2.0 * ratio - 1.0).abs() * signed * 100.0;
        prev_hlc = hlc;
        dm_prev = dm;
        cm_prev = cm;
        trend = new_trend;
    }
    let ema = |src: &[f64], p: usize| -> Vec<f64> {
        let mut out = vec![0.0_f64; src.len()];
        if p + 1 > src.len() {
            return out;
        }
        let k = 2.0 / (p as f64 + 1.0);
        let seed: f64 = src[1..=p].iter().sum::<f64>() / p as f64;
        out[p] = seed;
        for i in (p + 1)..src.len() {
            out[i] = src[i] * k + out[i - 1] * (1.0 - k);
        }
        out
    };
    let e_fast = ema(&vf, fast_p);
    let e_slow = ema(&vf, slow_p);
    let mut kvo = vec![0.0_f64; n];
    for i in slow_p..n {
        kvo[i] = e_fast[i] - e_slow[i];
    }
    let sig_seed_end = slow_p + signal_p - 1;
    let mut sig_series = vec![0.0_f64; n];
    if sig_seed_end < n {
        let k = 2.0 / (signal_p as f64 + 1.0);
        let seed: f64 = kvo[slow_p..=sig_seed_end].iter().sum::<f64>() / signal_p as f64;
        sig_series[sig_seed_end] = seed;
        for i in (sig_seed_end + 1)..n {
            sig_series[i] = kvo[i] * k + sig_series[i - 1] * (1.0 - k);
        }
    }
    let kvo_now = kvo[n - 1];
    let sig_now = sig_series[n - 1];
    let hist = kvo_now - sig_now;
    let abs_scale = (e_fast[n - 1].abs() + e_slow[n - 1].abs()).max(1.0);
    let norm = kvo_now / abs_scale;
    let label = if kvo_now > 0.0 && kvo_now > sig_now && norm.abs() > 0.05 {
        "STRONG_BULL"
    } else if kvo_now > 0.0 {
        "BULL"
    } else if kvo_now < 0.0 && kvo_now < sig_now && norm.abs() > 0.05 {
        "STRONG_BEAR"
    } else if kvo_now < 0.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    KlingerSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        fast_period: fast_p,
        slow_period: slow_p,
        signal_period: signal_p,
        ema_fast_vf: e_fast[n - 1],
        ema_slow_vf: e_slow[n - 1],
        kvo_value: kvo_now,
        signal_value: sig_now,
        histogram: hist,
        last_close: sorted[n - 1].close,
        klinger_label: label.into(),
        note: String::new(),
    }
}

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

/// SCHAFF — Schaff Trend Cycle: applies stochastic oscillator logic to MACD,
/// then smooths the result, then applies stochastic again, then smooths again.
/// Result is bounded [0, 100] with much tighter turning points than bare
/// MACD or bare stochastic. Schaff's original 2008 params: 23/50/10.
pub fn compute_schaff_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> SchaffSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let ema_fast = 23usize;
    let ema_slow = 50usize;
    let cycle = 10usize;
    let min_bars = ema_slow + cycle * 3;
    if n < min_bars {
        return SchaffSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            ema_fast,
            ema_slow,
            cycle,
            schaff_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let ema = |src: &[f64], p: usize| -> Vec<f64> {
        if src.len() < p {
            return Vec::new();
        }
        let alpha = 2.0 / (p as f64 + 1.0);
        let seed: f64 = src[..p].iter().sum::<f64>() / p as f64;
        let mut out = Vec::with_capacity(src.len() - p + 1);
        out.push(seed);
        for i in p..src.len() {
            let prev = *out.last().unwrap();
            out.push(alpha * src[i] + (1.0 - alpha) * prev);
        }
        out
    };
    let ema_f = ema(&closes, ema_fast);
    let ema_s = ema(&closes, ema_slow);
    if ema_f.is_empty() || ema_s.is_empty() {
        return SchaffSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            ema_fast,
            ema_slow,
            cycle,
            schaff_label: "INSUFFICIENT_DATA".into(),
            note: "ema series empty".into(),
            ..Default::default()
        };
    }
    // Align fast and slow EMAs — fast starts at index ema_fast-1, slow at ema_slow-1 in original closes.
    // MACD series: MACD[i] = ema_f[i - (ema_fast-1)] - ema_s[i - (ema_slow-1)] for i ≥ ema_slow-1.
    let macd_start = ema_slow - 1;
    let mut macd: Vec<f64> = Vec::with_capacity(n - macd_start);
    for i in macd_start..n {
        let f_idx = i - (ema_fast - 1);
        let s_idx = i - (ema_slow - 1);
        if f_idx >= ema_f.len() || s_idx >= ema_s.len() {
            break;
        }
        macd.push(ema_f[f_idx] - ema_s[s_idx]);
    }
    if macd.len() < cycle + cycle + 2 {
        return SchaffSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            ema_fast,
            ema_slow,
            cycle,
            schaff_label: "INSUFFICIENT_DATA".into(),
            note: format!("macd series {} < needed {}", macd.len(), cycle * 2 + 2),
            ..Default::default()
        };
    }
    // First stochastic pass: normalise MACD against its cycle-bar range.
    let mut stoch1: Vec<f64> = Vec::with_capacity(macd.len() - cycle + 1);
    for i in (cycle - 1)..macd.len() {
        let win = &macd[(i + 1 - cycle)..=i];
        let lo = win.iter().cloned().fold(f64::INFINITY, f64::min);
        let hi = win.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let v = if (hi - lo).abs() > 1e-12 {
            100.0 * (macd[i] - lo) / (hi - lo)
        } else {
            50.0
        };
        stoch1.push(v);
    }
    // Smoother pass 1: 0.5·stoch + 0.5·prev_pf
    let mut pf: Vec<f64> = Vec::with_capacity(stoch1.len());
    pf.push(stoch1[0]);
    for i in 1..stoch1.len() {
        let prev = *pf.last().unwrap();
        pf.push(0.5 * stoch1[i] + 0.5 * prev);
    }
    // Second stochastic pass: normalise PF against its cycle-bar range.
    if pf.len() < cycle + 2 {
        return SchaffSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            ema_fast,
            ema_slow,
            cycle,
            schaff_label: "INSUFFICIENT_DATA".into(),
            note: format!("pf series {} < {}", pf.len(), cycle + 2),
            ..Default::default()
        };
    }
    let mut stoch2: Vec<f64> = Vec::with_capacity(pf.len() - cycle + 1);
    for i in (cycle - 1)..pf.len() {
        let win = &pf[(i + 1 - cycle)..=i];
        let lo = win.iter().cloned().fold(f64::INFINITY, f64::min);
        let hi = win.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let v = if (hi - lo).abs() > 1e-12 {
            100.0 * (pf[i] - lo) / (hi - lo)
        } else {
            50.0
        };
        stoch2.push(v);
    }
    // Smoother pass 2: 0.5·stoch + 0.5·prev_stc
    let mut stc: Vec<f64> = Vec::with_capacity(stoch2.len());
    stc.push(stoch2[0]);
    for i in 1..stoch2.len() {
        let prev = *stc.last().unwrap();
        stc.push(0.5 * stoch2[i] + 0.5 * prev);
    }
    let stc_value = *stc.last().unwrap();
    let stc_prev = stc
        .get(stc.len().saturating_sub(2))
        .copied()
        .unwrap_or(stc_value);
    let last_close = sorted[n - 1].close;
    let rising = stc_value > stc_prev;
    let label = if stc_value > 75.0 && !rising {
        "OVERBOUGHT"
    } else if stc_value > 50.0 {
        "BULL"
    } else if stc_value < 25.0 && rising {
        "OVERSOLD"
    } else if stc_value < 50.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    SchaffSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        ema_fast,
        ema_slow,
        cycle,
        stc_value,
        stc_prev,
        last_close,
        schaff_label: label.into(),
        note: String::new(),
    }
}

/// STOCH — Lane's classic Stochastic Oscillator with standard 14/3/3 params.
/// Slow %K = SMA(raw %K, smoothing); %D = SMA(slow %K, d_period).
pub fn compute_stoch_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> StochSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let k_period = 14usize;
    let smoothing = 3usize;
    let d_period = 3usize;
    let min_bars = k_period + smoothing + d_period;
    if n < min_bars {
        return StochSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            k_period,
            d_period,
            smoothing,
            stoch_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    // Raw %K at each bar i ≥ k_period-1: (close - lowest_low_k) / (highest_high_k - lowest_low_k).
    let mut raw_k: Vec<f64> = Vec::with_capacity(n - k_period + 1);
    for i in (k_period - 1)..n {
        let win = &sorted[(i + 1 - k_period)..=i];
        let lo = win.iter().map(|b| b.low).fold(f64::INFINITY, f64::min);
        let hi = win.iter().map(|b| b.high).fold(f64::NEG_INFINITY, f64::max);
        let c = sorted[i].close;
        let v = if (hi - lo).abs() > 1e-12 {
            100.0 * (c - lo) / (hi - lo)
        } else {
            50.0
        };
        raw_k.push(v);
    }
    // Slow %K = SMA of raw %K over `smoothing` bars.
    if raw_k.len() < smoothing {
        return StochSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            k_period,
            d_period,
            smoothing,
            stoch_label: "INSUFFICIENT_DATA".into(),
            note: "raw %K too short for smoothing".into(),
            ..Default::default()
        };
    }
    let mut slow_k: Vec<f64> = Vec::with_capacity(raw_k.len() - smoothing + 1);
    for i in (smoothing - 1)..raw_k.len() {
        let s: f64 = raw_k[(i + 1 - smoothing)..=i].iter().sum();
        slow_k.push(s / smoothing as f64);
    }
    if slow_k.len() < d_period {
        return StochSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            k_period,
            d_period,
            smoothing,
            stoch_label: "INSUFFICIENT_DATA".into(),
            note: "slow %K too short for %D".into(),
            ..Default::default()
        };
    }
    let tail_d_sum: f64 = slow_k[slow_k.len() - d_period..].iter().sum();
    let percent_d = tail_d_sum / d_period as f64;
    let percent_k = *slow_k.last().unwrap();
    let last_close = sorted[n - 1].close;
    let label = if percent_k > 80.0 {
        "OVERBOUGHT"
    } else if percent_k > 50.0 {
        "BULL"
    } else if percent_k < 20.0 {
        "OVERSOLD"
    } else if percent_k < 50.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    StochSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        k_period,
        d_period,
        smoothing,
        percent_k,
        percent_d,
        last_close,
        stoch_label: label.into(),
        note: String::new(),
    }
}

/// MACD — Gerald Appel's 12/26/9 Moving Average Convergence Divergence.
/// Labels BULL_CROSS when histogram just turned positive, BEAR_CROSS when just
/// turned negative, otherwise BULL/BEAR/NEUTRAL by sign and magnitude.
pub fn compute_macd_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MacdSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let fast_period = 12usize;
    let slow_period = 26usize;
    let signal_period = 9usize;
    let min_bars = slow_period + signal_period + 2;
    if n < min_bars {
        return MacdSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            fast_period,
            slow_period,
            signal_period,
            macd_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let ema = |src: &[f64], p: usize| -> Vec<f64> {
        if src.len() < p {
            return Vec::new();
        }
        let alpha = 2.0 / (p as f64 + 1.0);
        let seed: f64 = src[..p].iter().sum::<f64>() / p as f64;
        let mut out = Vec::with_capacity(src.len() - p + 1);
        out.push(seed);
        for i in p..src.len() {
            let prev = *out.last().unwrap();
            out.push(alpha * src[i] + (1.0 - alpha) * prev);
        }
        out
    };
    let ema_fast = ema(&closes, fast_period);
    let ema_slow = ema(&closes, slow_period);
    if ema_fast.is_empty() || ema_slow.is_empty() {
        return MacdSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            fast_period,
            slow_period,
            signal_period,
            macd_label: "INSUFFICIENT_DATA".into(),
            note: "ema series empty".into(),
            ..Default::default()
        };
    }
    // MACD[i] = ema_fast[i - (fast-1)] - ema_slow[i - (slow-1)] for i ≥ slow-1.
    let macd_start = slow_period - 1;
    let mut macd_series: Vec<f64> = Vec::with_capacity(n - macd_start);
    for i in macd_start..n {
        let f_idx = i - (fast_period - 1);
        let s_idx = i - (slow_period - 1);
        if f_idx >= ema_fast.len() || s_idx >= ema_slow.len() {
            break;
        }
        macd_series.push(ema_fast[f_idx] - ema_slow[s_idx]);
    }
    if macd_series.len() < signal_period + 2 {
        return MacdSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            fast_period,
            slow_period,
            signal_period,
            macd_label: "INSUFFICIENT_DATA".into(),
            note: format!("macd series {} < {}", macd_series.len(), signal_period + 2),
            ..Default::default()
        };
    }
    let signal_series = ema(&macd_series, signal_period);
    if signal_series.len() < 2 {
        return MacdSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            fast_period,
            slow_period,
            signal_period,
            macd_label: "INSUFFICIENT_DATA".into(),
            note: "signal series too short".into(),
            ..Default::default()
        };
    }
    let macd_value = *macd_series.last().unwrap();
    let signal_value = *signal_series.last().unwrap();
    // Align signal's index back into macd_series: signal covers macd_series[(signal-1)..].
    let macd_prev_for_hist = macd_series[macd_series.len() - 2];
    let signal_prev = signal_series[signal_series.len() - 2];
    let histogram = macd_value - signal_value;
    let histogram_prev = macd_prev_for_hist - signal_prev;
    let last_close = sorted[n - 1].close;
    let crossed_up = histogram > 0.0 && histogram_prev <= 0.0;
    let crossed_down = histogram < 0.0 && histogram_prev >= 0.0;
    let label = if crossed_up {
        "BULL_CROSS"
    } else if crossed_down {
        "BEAR_CROSS"
    } else if histogram > 0.0 {
        "BULL"
    } else if histogram < 0.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    MacdSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        fast_period,
        slow_period,
        signal_period,
        macd_value,
        signal_value,
        histogram,
        histogram_prev,
        last_close,
        macd_label: label.into(),
        note: String::new(),
    }
}

/// VWAP — rolling Volume Weighted Average Price over `window` bars.
/// Typical price = (H+L+C)/3. Deviation buckets are ±0.5%/±2% around VWAP.
pub fn compute_vwap_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> VwapSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let window = 20usize;
    if n < window {
        return VwapSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            window,
            vwap_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", window, n),
            ..Default::default()
        };
    }
    let tail = &sorted[(n - window)..];
    let mut num = 0.0f64;
    let mut den = 0.0f64;
    for b in tail {
        let tp = (b.high + b.low + b.close) / 3.0;
        let v = b.volume.max(0.0);
        num += tp * v;
        den += v;
    }
    if den <= 0.0 {
        return VwapSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: n,
            window,
            vwap_label: "INSUFFICIENT_DATA".into(),
            note: "zero total volume in window".into(),
            ..Default::default()
        };
    }
    let vwap_value = num / den;
    let last_close = sorted[n - 1].close;
    let deviation_pct = if vwap_value > 0.0 {
        (last_close - vwap_value) / vwap_value * 100.0
    } else {
        0.0
    };
    let label = if deviation_pct > 2.0 {
        "STRONG_ABOVE"
    } else if deviation_pct > 0.5 {
        "ABOVE"
    } else if deviation_pct < -2.0 {
        "STRONG_BELOW"
    } else if deviation_pct < -0.5 {
        "BELOW"
    } else {
        "AT"
    };
    VwapSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        window,
        vwap_value,
        last_close,
        deviation_pct,
        vwap_label: label.into(),
        note: String::new(),
    }
}

/// MCGD — McGinley Dynamic adaptive MA.
/// MD[i] = MD[i-1] + (P - MD[i-1]) / (N × (P/MD[i-1])^4). Seeded from the first
/// `length`-bar SMA, then iterated across the rest of the series.
pub fn compute_mcgd_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> McgdSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 14usize;
    let min_bars = length + 2;
    if n < min_bars {
        return McgdSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            mcgd_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let seed: f64 = closes[..length].iter().sum::<f64>() / length as f64;
    let mut md = seed;
    let mut mcgd_prev = md;
    for i in length..n {
        mcgd_prev = md;
        if md.abs() < 1e-12 {
            md = closes[i];
            continue;
        }
        let ratio = closes[i] / md;
        let denom = length as f64 * ratio.powi(4);
        if denom.abs() < 1e-12 {
            md = closes[i];
        } else {
            md = md + (closes[i] - md) / denom;
        }
    }
    let last_close = sorted[n - 1].close;
    let deviation_pct = if md.abs() > 1e-12 {
        (last_close - md) / md * 100.0
    } else {
        0.0
    };
    let label = if deviation_pct > 2.5 {
        "STRONG_BULL"
    } else if deviation_pct > 0.5 {
        "BULL"
    } else if deviation_pct < -2.5 {
        "STRONG_BEAR"
    } else if deviation_pct < -0.5 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    McgdSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        mcgd_value: md,
        mcgd_prev,
        last_close,
        deviation_pct,
        mcgd_label: label.into(),
        note: String::new(),
    }
}

/// RWI — Random Walk Index (Poulos).
/// RWI_high(n) = (H[0] - L[n]) / (ATR(n) × sqrt(n))
/// RWI_low(n)  = (H[n] - L[0]) / (ATR(n) × sqrt(n))
/// Scans n = 2..=length and keeps the maximum — whichever horizon shows the
/// strongest non-random move dominates the label.
pub fn compute_rwi_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> RwiSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 14usize;
    let min_bars = length + 2;
    if n < min_bars {
        return RwiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            rwi_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    // Precompute True Range series (TR[i] for i ≥ 1).
    let mut tr: Vec<f64> = Vec::with_capacity(n);
    tr.push(sorted[0].high - sorted[0].low);
    for i in 1..n {
        let a = sorted[i].high - sorted[i].low;
        let b = (sorted[i].high - sorted[i - 1].close).abs();
        let c = (sorted[i].low - sorted[i - 1].close).abs();
        tr.push(a.max(b).max(c));
    }
    let current = n - 1;
    let mut best_up = 0.0f64;
    let mut best_dn = 0.0f64;
    for k in 2..=length {
        if current < k {
            break;
        }
        // ATR over the last k bars ending at `current`.
        let atr_sum: f64 = tr[(current + 1 - k)..=current].iter().sum();
        let atr = atr_sum / k as f64;
        if atr <= 1e-12 {
            continue;
        }
        let denom = atr * (k as f64).sqrt();
        let h_now = sorted[current].high;
        let l_then = sorted[current - k + 1].low;
        let l_now = sorted[current].low;
        let h_then = sorted[current - k + 1].high;
        let up = (h_now - l_then) / denom;
        let dn = (h_then - l_now) / denom;
        if up > best_up {
            best_up = up;
        }
        if dn > best_dn {
            best_dn = dn;
        }
    }
    let last_close = sorted[n - 1].close;
    let label = if best_up > 1.0 && best_up > best_dn {
        "TRENDING_UP"
    } else if best_dn > 1.0 && best_dn > best_up {
        "TRENDING_DOWN"
    } else {
        "RANGE_BOUND"
    };
    RwiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        rwi_high: best_up,
        rwi_low: best_dn,
        last_close,
        rwi_label: label.into(),
        note: String::new(),
    }
}
