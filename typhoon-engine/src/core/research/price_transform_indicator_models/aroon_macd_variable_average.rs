use super::*;

// Aroon, min/max index, MACD-extension, and variable-period moving-average transforms

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
