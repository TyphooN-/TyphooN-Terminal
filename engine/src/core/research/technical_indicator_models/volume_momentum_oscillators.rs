use super::*;

// Volume, momentum, price-oscillator, Williams, mass, and Klinger models

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
