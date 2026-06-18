use super::*;

mod basic_reversal;
pub use basic_reversal::*;
mod multibar_reversal;
pub use multibar_reversal::*;
mod doji_shadow_star;
pub use doji_shadow_star::*;
mod body_line_shapes;
pub use body_line_shapes::*;
mod neck_line_reversal;
pub use neck_line_reversal::*;
mod crow_line_reversal;
pub use crow_line_reversal::*;
mod separating_sandwich_doji;
pub use separating_sandwich_doji::*;
mod rare_multibar_reversal;
pub use rare_multibar_reversal::*;
mod gap_breakaway_reversal;
pub use gap_breakaway_reversal::*;

// Candlestick pattern storage/helpers

/// Candle metrics for a single bar: (body, range, upper_shadow,
/// lower_shadow, body_pct_range, is_bullish). body = |close - open|;
/// range = high - low; upper_shadow = high - max(open, close);
/// lower_shadow = min(open, close) - low. body_pct_range = 100 ·
/// body / range (0 when range == 0 to avoid div-by-zero).
#[allow(dead_code)]
fn candle_metrics(bar: &HistoricalPriceRow) -> (f64, f64, f64, f64, f64, bool) {
    let body = (bar.close - bar.open).abs();
    let range = (bar.high - bar.low).max(0.0);
    let top = bar.open.max(bar.close);
    let bot = bar.open.min(bar.close);
    let upper = (bar.high - top).max(0.0);
    let lower = (bot - bar.low).max(0.0);
    let body_pct = if range > 1e-12 {
        100.0 * body / range
    } else {
        0.0
    };
    let bullish = bar.close >= bar.open;
    (body, range, upper, lower, body_pct, bullish)
}

#[allow(dead_code)]
fn candle_body_bounds(bar: &HistoricalPriceRow) -> (f64, f64) {
    (bar.open.min(bar.close), bar.open.max(bar.close))
}

/// Shared helper: scan sorted bars back from end to find the most recent
/// bar matching a predicate, and return (last_bar_match, days_since_pattern,
/// pattern_value_on_last_bar, pattern_value_prev_bar).
#[allow(dead_code)]
fn cdl_scan<F>(sorted: &[&HistoricalPriceRow], min_i: usize, detector: F) -> (bool, usize, i32, i32)
where
    F: Fn(&[&HistoricalPriceRow], usize) -> i32,
{
    let n = sorted.len();
    let last_val = detector(sorted, n - 1);
    let prev_val = if n >= 2 { detector(sorted, n - 2) } else { 0 };
    let last_match = last_val != 0;
    let mut days_since: usize = 0;
    if !last_match {
        let mut idx = n - 1;
        while idx > min_i {
            if detector(sorted, idx) != 0 {
                days_since = (n - 1) - idx;
                break;
            }
            idx -= 1;
        }
        if days_since == 0 {
            days_since = (n - 1) - min_i;
        }
    }
    (last_match, days_since, last_val, prev_val)
}

// Hikkake, mat-hold, and rise/fall continuation candlestick patterns

pub fn compute_cdl_hikkake_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlHikkakeSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 4 {
        return CdlHikkakeSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_hikkake_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥4 bars, got {}", n),
            ..Default::default()
        };
    }
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        if i < 2 {
            return 0;
        }
        let b0 = s[i - 2];
        let b1 = s[i - 1];
        let b2 = s[i];
        if !(b1.high < b0.high && b1.low > b0.low) {
            return 0;
        }
        if b2.low < b1.low && b2.high <= b1.high && b2.close > b1.low {
            100
        } else if b2.high > b1.high && b2.low >= b1.low && b2.close < b1.high {
            -100
        } else {
            0
        }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 2, detect);
    let b0 = sorted[n - 3];
    let b1 = sorted[n - 2];
    let b2 = sorted[n - 1];
    let mother_range = (b0.high - b0.low).abs();
    let inside_range = (b1.high - b1.low).abs();
    let (_, trigger_range, _, _, trigger_body_pct_range, _) = candle_metrics(b2);
    let inside_width_pct_mother = if mother_range > 1e-12 {
        100.0 * inside_range / mother_range
    } else {
        0.0
    };
    let false_break_extension_pct = if inside_range > 1e-12 {
        if b2.low < b1.low {
            100.0 * (b1.low - b2.low) / inside_range
        } else if b2.high > b1.high {
            100.0 * (b2.high - b1.high) / inside_range
        } else {
            0.0
        }
    } else {
        0.0
    };
    let _ = trigger_range;
    let label = match last_val {
        100 => "BULLISH_PATTERN",
        -100 => "BEARISH_PATTERN",
        _ => "NO_PATTERN",
    };
    CdlHikkakeSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        inside_width_pct_mother,
        false_break_extension_pct,
        trigger_body_pct_range,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b2.close,
        cdl_hikkake_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_hikkake_mod_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlHikkakeModSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 5 {
        return CdlHikkakeModSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_hikkake_mod_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥5 bars, got {}", n),
            ..Default::default()
        };
    }
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        if i < 3 {
            return 0;
        }
        let b0 = s[i - 3];
        let b1 = s[i - 2];
        let b2 = s[i - 1];
        let b3 = s[i];
        if !(b1.high < b0.high && b1.low > b0.low) {
            return 0;
        }
        if b2.low < b1.low
            && b2.high <= b1.high
            && b2.close > b1.low
            && b3.close > b1.high
            && b3.close > b2.high
        {
            100
        } else if b2.high > b1.high
            && b2.low >= b1.low
            && b2.close < b1.high
            && b3.close < b1.low
            && b3.close < b2.low
        {
            -100
        } else {
            0
        }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 3, detect);
    let b0 = sorted[n - 4];
    let b1 = sorted[n - 3];
    let b2 = sorted[n - 2];
    let b3 = sorted[n - 1];
    let mother_range = (b0.high - b0.low).abs();
    let inside_range = (b1.high - b1.low).abs();
    let inside_width_pct_mother = if mother_range > 1e-12 {
        100.0 * inside_range / mother_range
    } else {
        0.0
    };
    let false_break_extension_pct = if inside_range > 1e-12 {
        if b2.low < b1.low {
            100.0 * (b1.low - b2.low) / inside_range
        } else if b2.high > b1.high {
            100.0 * (b2.high - b1.high) / inside_range
        } else {
            0.0
        }
    } else {
        0.0
    };
    let confirmation_extension_pct = if inside_range > 1e-12 {
        if b2.low < b1.low {
            100.0 * (b3.close - b1.high) / inside_range
        } else if b2.high > b1.high {
            100.0 * (b1.low - b3.close) / inside_range
        } else {
            0.0
        }
    } else {
        0.0
    };
    let label = match last_val {
        100 => "BULLISH_PATTERN",
        -100 => "BEARISH_PATTERN",
        _ => "NO_PATTERN",
    };
    CdlHikkakeModSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        inside_width_pct_mother,
        false_break_extension_pct,
        confirmation_extension_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b3.close,
        cdl_hikkake_mod_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_mat_hold_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlMatHoldSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 5 {
        return CdlMatHoldSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_mat_hold_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥5 bars, got {}", n),
            ..Default::default()
        };
    }
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        if i < 4 {
            return 0;
        }
        let b0 = s[i - 4];
        let b1 = s[i - 3];
        let b2 = s[i - 2];
        let b3 = s[i - 1];
        let b4 = s[i];
        let (body0, _, _, _, body0_pct, bull0) = candle_metrics(b0);
        let (_, _, _, _, body1_pct, bull1) = candle_metrics(b1);
        let (_, _, _, _, body2_pct, bull2) = candle_metrics(b2);
        let (_, _, _, _, body3_pct, bull3) = candle_metrics(b3);
        let (_, _, _, _, body4_pct, bull4) = candle_metrics(b4);
        if body0 < 1e-12 {
            return 0;
        }
        let middle_small = body1_pct <= 35.0 && body2_pct <= 35.0 && body3_pct <= 35.0;
        if !middle_small || body0_pct < 30.0 || body4_pct < 30.0 {
            return 0;
        }
        if bull0 && bull4 {
            if b1.low <= b0.high {
                return 0;
            }
            if !(b2.low > b0.open && b3.low > b0.open) {
                return 0;
            }
            if !(b2.high < b1.high && b3.high < b1.high) {
                return 0;
            }
            if !(b4.close > b1.high) {
                return 0;
            }
            100
        } else if !bull0 && !bull4 {
            if b1.high >= b0.low {
                return 0;
            }
            if !(b2.high < b0.open && b3.high < b0.open) {
                return 0;
            }
            if !(b2.low > b1.low && b3.low > b1.low) {
                return 0;
            }
            if !(b4.close < b1.low) {
                return 0;
            }
            -100
        } else {
            let _ = (bull1, bull2, bull3);
            0
        }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 4, detect);
    let b0 = sorted[n - 5];
    let b1 = sorted[n - 4];
    let b2 = sorted[n - 3];
    let b3 = sorted[n - 2];
    let b4 = sorted[n - 1];
    let (body0, range0, _, _, first_body_pct_range, bull0) = candle_metrics(b0);
    let (_, _, _, _, body1_pct, _) = candle_metrics(b1);
    let (_, _, _, _, body2_pct, _) = candle_metrics(b2);
    let (_, _, _, _, body3_pct, _) = candle_metrics(b3);
    let (_, _, _, _, final_body_pct_range, _) = candle_metrics(b4);
    let middle_avg_body_pct_range = (body1_pct + body2_pct + body3_pct) / 3.0;
    let initial_gap_raw = if bull0 && b1.low > b0.high {
        b1.low - b0.high
    } else if !bull0 && b1.high < b0.low {
        b0.low - b1.high
    } else {
        0.0
    };
    let initial_gap_pct_range = if range0 > 1e-12 {
        100.0 * initial_gap_raw / range0
    } else {
        0.0
    };
    let hold_depth_pct_body = if body0 > 1e-12 {
        if bull0 {
            let min_low = b1.low.min(b2.low.min(b3.low));
            100.0 * (min_low - b0.open) / body0
        } else {
            let max_high = b1.high.max(b2.high.max(b3.high));
            100.0 * (b0.open - max_high) / body0
        }
    } else {
        0.0
    };
    let label = match last_val {
        100 => "BULLISH_CONTINUATION",
        -100 => "BEARISH_CONTINUATION",
        _ => "NO_PATTERN",
    };
    CdlMatHoldSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        first_body_pct_range,
        middle_avg_body_pct_range,
        initial_gap_pct_range,
        hold_depth_pct_body,
        final_body_pct_range,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b4.close,
        cdl_mat_hold_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_rise_fall_three_methods_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlRiseFallThreeMethodsSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 5 {
        return CdlRiseFallThreeMethodsSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_rise_fall_three_methods_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥5 bars, got {}", n),
            ..Default::default()
        };
    }
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        if i < 4 {
            return 0;
        }
        let b0 = s[i - 4];
        let b1 = s[i - 3];
        let b2 = s[i - 2];
        let b3 = s[i - 1];
        let b4 = s[i];
        let (_, _, _, _, body0_pct, bull0) = candle_metrics(b0);
        let (_, _, _, _, body1_pct, bull1) = candle_metrics(b1);
        let (_, _, _, _, body2_pct, bull2) = candle_metrics(b2);
        let (_, _, _, _, body3_pct, bull3) = candle_metrics(b3);
        let (_, _, _, _, body4_pct, bull4) = candle_metrics(b4);
        if body0_pct < 30.0 || body4_pct < 30.0 {
            return 0;
        }
        if !(body1_pct <= 35.0 && body2_pct <= 35.0 && body3_pct <= 35.0) {
            return 0;
        }
        if bull0 && !bull1 && !bull2 && !bull3 && bull4 {
            if !(b1.high < b0.close && b2.high < b0.close && b3.high < b0.close) {
                return 0;
            }
            if !(b1.low > b0.open && b2.low > b0.open && b3.low > b0.open) {
                return 0;
            }
            if b4.close <= b0.close {
                return 0;
            }
            100
        } else if !bull0 && bull1 && bull2 && bull3 && !bull4 {
            if !(b1.low > b0.close && b2.low > b0.close && b3.low > b0.close) {
                return 0;
            }
            if !(b1.high < b0.open && b2.high < b0.open && b3.high < b0.open) {
                return 0;
            }
            if b4.close >= b0.close {
                return 0;
            }
            -100
        } else {
            0
        }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 4, detect);
    let b0 = sorted[n - 5];
    let b1 = sorted[n - 4];
    let b2 = sorted[n - 3];
    let b3 = sorted[n - 2];
    let b4 = sorted[n - 1];
    let (body0, _, _, _, first_body_pct_range, bull0) = candle_metrics(b0);
    let (_, _, _, _, body1_pct, _) = candle_metrics(b1);
    let (_, _, _, _, body2_pct, _) = candle_metrics(b2);
    let (_, _, _, _, body3_pct, _) = candle_metrics(b3);
    let (_, _, _, _, final_body_pct_range, _) = candle_metrics(b4);
    let middle_avg_body_pct_range = (body1_pct + body2_pct + body3_pct) / 3.0;
    let containment_pct_body = if body0 > 1e-12 {
        if bull0 {
            let min_low = b1.low.min(b2.low.min(b3.low));
            100.0 * (min_low - b0.open) / body0
        } else {
            let max_high = b1.high.max(b2.high.max(b3.high));
            100.0 * (b0.open - max_high) / body0
        }
    } else {
        0.0
    };
    let label = match last_val {
        100 => "BULLISH_CONTINUATION",
        -100 => "BEARISH_CONTINUATION",
        _ => "NO_PATTERN",
    };
    CdlRiseFallThreeMethodsSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        first_body_pct_range,
        middle_avg_body_pct_range,
        containment_pct_body,
        final_body_pct_range,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b4.close,
        cdl_rise_fall_three_methods_label: label.into(),
        note: String::new(),
    }
}

// Stalled-pattern and tasuki-gap candlestick patterns

pub fn compute_cdl_stalled_pattern_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlStalledPatternSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 3 {
        return CdlStalledPatternSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_stalled_pattern_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 bars, got {}", n),
            ..Default::default()
        };
    }
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        if i < 2 {
            return 0;
        }
        let b0 = s[i - 2];
        let b1 = s[i - 1];
        let b2 = s[i];
        let (_body0, _, _, _, body0_pct, bull0) = candle_metrics(b0);
        let (body1, _, _, _, body1_pct, bull1) = candle_metrics(b1);
        let (body2, range2, upper2, _, body2_pct, bull2) = candle_metrics(b2);
        let upper2_pct = if range2 > 1e-12 {
            100.0 * upper2 / range2
        } else {
            0.0
        };
        if !(bull0 && bull1 && bull2) {
            return 0;
        }
        if body0_pct < 30.0 || body1_pct < 30.0 || body2_pct < 12.0 {
            return 0;
        }
        if body2 >= body1 * 0.8 {
            return 0;
        }
        let advance1 = b1.close - b0.close;
        let advance2 = b2.close - b1.close;
        if advance1 <= 0.0 || advance2 <= 0.0 {
            return 0;
        }
        if advance2 > advance1 * 0.6 {
            return 0;
        }
        if upper2_pct < 15.0 {
            return 0;
        }
        -100
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 2, detect);
    let b0 = sorted[n - 3];
    let b1 = sorted[n - 2];
    let b2 = sorted[n - 1];
    let (_, _, _, _, first_body_pct_range, _) = candle_metrics(b0);
    let (_, range1, _, _, second_body_pct_range, _) = candle_metrics(b1);
    let (_, range2, upper2, _, third_body_pct_range, _) = candle_metrics(b2);
    let third_upper_shadow_pct = if range2 > 1e-12 {
        100.0 * upper2 / range2
    } else {
        0.0
    };
    let third_open_gap_pct_range = if range1 > 1e-12 {
        100.0 * (b2.open - b1.close).max(0.0) / range1
    } else {
        0.0
    };
    let advance1 = b1.close - b0.close;
    let advance2 = b2.close - b1.close;
    let close_progress_pct_prev_leg = if advance1.abs() > 1e-12 {
        100.0 * advance2 / advance1
    } else {
        0.0
    };
    let label = match last_val {
        -100 => "BEARISH_PATTERN",
        _ => "NO_PATTERN",
    };
    CdlStalledPatternSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        first_body_pct_range,
        second_body_pct_range,
        third_body_pct_range,
        third_open_gap_pct_range,
        third_upper_shadow_pct,
        close_progress_pct_prev_leg,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b2.close,
        cdl_stalled_pattern_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_tasuki_gap_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlTasukiGapSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 3 {
        return CdlTasukiGapSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_tasuki_gap_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 bars, got {}", n),
            ..Default::default()
        };
    }
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        if i < 2 {
            return 0;
        }
        let b0 = s[i - 2];
        let b1 = s[i - 1];
        let b2 = s[i];
        let (_, _, _, _, body0_pct, bull0) = candle_metrics(b0);
        let (_, _, _, _, body1_pct, bull1) = candle_metrics(b1);
        let (_, _, _, _, body2_pct, bull2) = candle_metrics(b2);
        if body0_pct < 30.0 || body1_pct < 30.0 || body2_pct < 20.0 {
            return 0;
        }
        if bull0 && bull1 && !bull2 {
            if !(b1.low > b0.high) {
                return 0;
            }
            if !(b2.open > b1.open && b2.open < b1.close) {
                return 0;
            }
            if !(b2.close < b1.low && b2.close > b0.high) {
                return 0;
            }
            100
        } else if !bull0 && !bull1 && bull2 {
            if !(b1.high < b0.low) {
                return 0;
            }
            if !(b2.open > b1.close && b2.open < b1.open) {
                return 0;
            }
            if !(b2.close > b1.high && b2.close < b0.low) {
                return 0;
            }
            -100
        } else {
            0
        }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 2, detect);
    let b0 = sorted[n - 3];
    let b1 = sorted[n - 2];
    let b2 = sorted[n - 1];
    let (_, range0, _, _, first_body_pct_range, bull0) = candle_metrics(b0);
    let (_, _, _, _, second_body_pct_range, _) = candle_metrics(b1);
    let (_, _, _, _, third_body_pct_range, _) = candle_metrics(b2);
    let gap_raw = if bull0 && b1.low > b0.high {
        b1.low - b0.high
    } else if !bull0 && b1.high < b0.low {
        b0.low - b1.high
    } else {
        0.0
    };
    let gap_pct_range = if range0 > 1e-12 {
        100.0 * gap_raw / range0
    } else {
        0.0
    };
    let gap_fill_pct = if gap_raw > 1e-12 {
        if bull0 && b1.low > b0.high {
            100.0 * (b1.low - b2.close).clamp(0.0, gap_raw) / gap_raw
        } else if !bull0 && b1.high < b0.low {
            100.0 * (b2.close - b1.high).clamp(0.0, gap_raw) / gap_raw
        } else {
            0.0
        }
    } else {
        0.0
    };
    let body1 = (b1.close - b1.open).abs();
    let third_open_pct_second_body = if body1 > 1e-12 {
        if bull0 {
            100.0 * (b2.open - b1.open) / body1
        } else {
            100.0 * (b1.open - b2.open) / body1
        }
    } else {
        0.0
    };
    let label = match last_val {
        100 => "BULLISH_CONTINUATION",
        -100 => "BEARISH_CONTINUATION",
        _ => "NO_PATTERN",
    };
    CdlTasukiGapSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        first_body_pct_range,
        second_body_pct_range,
        third_body_pct_range,
        gap_pct_range,
        gap_fill_pct,
        third_open_pct_second_body,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b2.close,
        cdl_tasuki_gap_label: label.into(),
        note: String::new(),
    }
}
