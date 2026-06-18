use super::*;

mod squeeze_trend_channels;
pub use squeeze_trend_channels::*;
mod directional_flow_trend;
pub use directional_flow_trend::*;
mod volume_momentum_oscillators;
pub use volume_momentum_oscillators::*;
mod momentum_volume_pressure;
pub use momentum_volume_pressure::*;

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
