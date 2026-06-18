use super::*;

// Rolling min/max value, index, and recency transform family

/// Walk the trailing `period`-bar window ending at `end_idx` and return
/// `(min_val, min_idx_in_series, max_val, max_idx_in_series)`. The two
/// indices are absolute positions in the sorted array (so `end_idx -
/// idx_in_series` gives a "bars ago" recency).
pub(super) fn window_extrema(
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
