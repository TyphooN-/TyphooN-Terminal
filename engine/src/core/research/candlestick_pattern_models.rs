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

// Rare multi-bar reversal candlestick patterns

pub fn compute_cdl_three_stars_in_south_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlThreeStarsInSouthSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 4 {
        return CdlThreeStarsInSouthSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_three_stars_in_south_label: "INSUFFICIENT_DATA".into(),
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
        let (body0, _range0, _upper0, lower0, body0_pct, bull0) = candle_metrics(b0);
        let (body1, _range1, _upper1, lower1, body1_pct, bull1) = candle_metrics(b1);
        let (body2, _range2, _upper2, _lower2, body2_pct, bull2) = candle_metrics(b2);
        if body0 < 1e-12 || body1 < 1e-12 || body2 < 1e-12 {
            return 0;
        }
        if bull0 || bull1 || bull2 {
            return 0;
        }
        if body0_pct < 30.0 || body1_pct > body0_pct || body1_pct < 10.0 || body2_pct > 20.0 {
            return 0;
        }
        if lower0 < body0 || lower1 < body1 {
            return 0;
        }
        if b1.low <= b0.low {
            return 0;
        }
        if !(b1.open <= b0.open && b1.open >= b0.close) {
            return 0;
        }
        if b2.high >= b1.high || b2.low <= b1.low {
            return 0;
        }
        100
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 2, detect);
    let b0 = sorted[n - 3];
    let b1 = sorted[n - 2];
    let b2 = sorted[n - 1];
    let (_body0, range0, _upper0, lower0, first_body_pct_range, _) = candle_metrics(b0);
    let (_, _, _, _, second_body_pct_range, _) = candle_metrics(b1);
    let (_, _, _, _, third_body_pct_range, _) = candle_metrics(b2);
    let first_lower_shadow_pct = if range0 > 1e-12 {
        100.0 * lower0 / range0
    } else {
        0.0
    };
    let second_range = (b1.high - b1.low).abs();
    let third_inside_pct_range = if second_range > 1e-12 && b2.high < b1.high && b2.low > b1.low {
        100.0 * (((b1.high - b2.high) + (b2.low - b1.low)) / 2.0) / second_range
    } else {
        0.0
    };
    let label = if last_val == 100 {
        "BULLISH_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlThreeStarsInSouthSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        first_body_pct_range,
        first_lower_shadow_pct,
        second_body_pct_range,
        third_body_pct_range,
        third_inside_pct_range,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b2.close,
        cdl_three_stars_in_south_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_identical_three_crows_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlIdenticalThreeCrowsSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 4 {
        return CdlIdenticalThreeCrowsSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_identical_three_crows_label: "INSUFFICIENT_DATA".into(),
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
        let (body0, _, _, _, body0_pct, bull0) = candle_metrics(b0);
        let (body1, _, _, _, body1_pct, bull1) = candle_metrics(b1);
        let (body2, _, _, _, body2_pct, bull2) = candle_metrics(b2);
        if body0 < 1e-12 || body1 < 1e-12 || body2 < 1e-12 {
            return 0;
        }
        if bull0 || bull1 || bull2 {
            return 0;
        }
        if body0_pct < 30.0 || body1_pct < 30.0 || body2_pct < 30.0 {
            return 0;
        }
        if !(b1.close < b0.close && b2.close < b1.close) {
            return 0;
        }
        if !(b1.open <= b0.open && b1.open >= b0.close) {
            return 0;
        }
        if !(b2.open <= b1.open && b2.open >= b1.close) {
            return 0;
        }
        if 100.0 * (b1.open - b0.close).abs() / body0 > 10.0 {
            return 0;
        }
        if 100.0 * (b2.open - b1.close).abs() / body1 > 10.0 {
            return 0;
        }
        -100
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 2, detect);
    let b0 = sorted[n - 3];
    let b1 = sorted[n - 2];
    let b2 = sorted[n - 1];
    let (body0, _, _, _, body0_pct, _) = candle_metrics(b0);
    let (body1, _, _, _, body1_pct, _) = candle_metrics(b1);
    let (_, _, _, _, body2_pct, _) = candle_metrics(b2);
    let avg_body_pct_range = (body0_pct + body1_pct + body2_pct) / 3.0;
    let open1_vs_close0_pct_body = if body0 > 1e-12 {
        100.0 * (b1.open - b0.close).abs() / body0
    } else {
        0.0
    };
    let open2_vs_close1_pct_body = if body1 > 1e-12 {
        100.0 * (b2.open - b1.close).abs() / body1
    } else {
        0.0
    };
    let total_close_decline_pct = if b0.open.abs() > 1e-12 {
        100.0 * (b2.close - b0.open) / b0.open
    } else {
        0.0
    };
    let label = if last_val == -100 {
        "BEARISH_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlIdenticalThreeCrowsSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        avg_body_pct_range,
        open1_vs_close0_pct_body,
        open2_vs_close1_pct_body,
        total_close_decline_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b2.close,
        cdl_identical_three_crows_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_kicking_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlKickingSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 3 {
        return CdlKickingSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_kicking_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 bars, got {}", n),
            ..Default::default()
        };
    }
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        if i < 1 {
            return 0;
        }
        let b0 = s[i - 1];
        let b1 = s[i];
        let (_, range0, upper0, lower0, body0_pct, bull0) = candle_metrics(b0);
        let (_, range1, upper1, lower1, body1_pct, bull1) = candle_metrics(b1);
        if range0 < 1e-12 || range1 < 1e-12 {
            return 0;
        }
        let upper0_pct = 100.0 * upper0 / range0;
        let lower0_pct = 100.0 * lower0 / range0;
        let upper1_pct = 100.0 * upper1 / range1;
        let lower1_pct = 100.0 * lower1 / range1;
        if body0_pct < 90.0 || body1_pct < 90.0 {
            return 0;
        }
        if upper0_pct > 5.0 || lower0_pct > 5.0 || upper1_pct > 5.0 || lower1_pct > 5.0 {
            return 0;
        }
        if bull0 == bull1 {
            return 0;
        }
        if !bull0 && bull1 && b1.low > b0.high {
            100
        } else if bull0 && !bull1 && b1.high < b0.low {
            -100
        } else {
            0
        }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 1, detect);
    let b0 = sorted[n - 2];
    let b1 = sorted[n - 1];
    let (body0, range0, _, _, first_body_pct_range, _) = candle_metrics(b0);
    let (body1, _, _, _, second_body_pct_range, _) = candle_metrics(b1);
    let gap_raw = if b1.low > b0.high {
        b1.low - b0.high
    } else if b1.high < b0.low {
        b0.low - b1.high
    } else {
        0.0
    };
    let gap_pct_range = if range0 > 1e-12 {
        100.0 * gap_raw / range0
    } else {
        0.0
    };
    let second_to_first_body_ratio = if body0 > 1e-12 { body1 / body0 } else { 0.0 };
    let label = match last_val {
        100 => "BULLISH_PATTERN",
        -100 => "BEARISH_PATTERN",
        _ => "NO_PATTERN",
    };
    CdlKickingSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        first_body_pct_range,
        second_body_pct_range,
        gap_pct_range,
        second_to_first_body_ratio,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b1.close,
        cdl_kicking_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_kicking_by_length_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlKickingByLengthSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 3 {
        return CdlKickingByLengthSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_kicking_by_length_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 bars, got {}", n),
            ..Default::default()
        };
    }
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        if i < 1 {
            return 0;
        }
        let b0 = s[i - 1];
        let b1 = s[i];
        let (body0, range0, upper0, lower0, body0_pct, bull0) = candle_metrics(b0);
        let (body1, range1, upper1, lower1, body1_pct, bull1) = candle_metrics(b1);
        if body0 < 1e-12 || body1 < 1e-12 || range0 < 1e-12 || range1 < 1e-12 {
            return 0;
        }
        let upper0_pct = 100.0 * upper0 / range0;
        let lower0_pct = 100.0 * lower0 / range0;
        let upper1_pct = 100.0 * upper1 / range1;
        let lower1_pct = 100.0 * lower1 / range1;
        if body0_pct < 90.0 || body1_pct < 90.0 {
            return 0;
        }
        if upper0_pct > 5.0 || lower0_pct > 5.0 || upper1_pct > 5.0 || lower1_pct > 5.0 {
            return 0;
        }
        if bull0 == bull1 {
            return 0;
        }
        let has_gap =
            (!bull0 && bull1 && b1.low > b0.high) || (bull0 && !bull1 && b1.high < b0.low);
        if !has_gap {
            return 0;
        }
        if body1 > body0 {
            if bull1 { 100 } else { -100 }
        } else if body0 > body1 {
            if bull0 { 100 } else { -100 }
        } else {
            0
        }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 1, detect);
    let b0 = sorted[n - 2];
    let b1 = sorted[n - 1];
    let (body0, range0, _, _, first_body_pct_range, _bull0) = candle_metrics(b0);
    let (body1, _, _, _, second_body_pct_range, _bull1) = candle_metrics(b1);
    let gap_raw = if b1.low > b0.high {
        b1.low - b0.high
    } else if b1.high < b0.low {
        b0.low - b1.high
    } else {
        0.0
    };
    let gap_pct_range = if range0 > 1e-12 {
        100.0 * gap_raw / range0
    } else {
        0.0
    };
    let (dominant_body_ratio, dominant_side) = if body0 > 1e-12 && body1 > 1e-12 {
        if body1 > body0 {
            (body1 / body0, "SECOND_BAR")
        } else if body0 > body1 {
            (body0 / body1, "FIRST_BAR")
        } else {
            (1.0, "NONE")
        }
    } else {
        (0.0, "NONE")
    };
    let label = match last_val {
        100 => "BULLISH_PATTERN",
        -100 => "BEARISH_PATTERN",
        _ => "NO_PATTERN",
    };
    CdlKickingByLengthSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        first_body_pct_range,
        second_body_pct_range,
        gap_pct_range,
        dominant_body_ratio,
        dominant_side: dominant_side.into(),
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b1.close,
        cdl_kicking_by_length_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_ladder_bottom_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlLadderBottomSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 6 {
        return CdlLadderBottomSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_ladder_bottom_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥6 bars, got {}", n),
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
        let (body1, _, _, _, body1_pct, bull1) = candle_metrics(b1);
        let (body2, _, _, _, body2_pct, bull2) = candle_metrics(b2);
        let (body3, _, upper3, _, body3_pct, bull3) = candle_metrics(b3);
        let (_, _, _, _, body4_pct, bull4) = candle_metrics(b4);
        if body0 < 1e-12 || body1 < 1e-12 || body2 < 1e-12 || body3 < 1e-12 {
            return 0;
        }
        if bull0 || bull1 || bull2 || bull3 || !bull4 {
            return 0;
        }
        if body0_pct < 30.0 || body1_pct < 30.0 || body2_pct < 30.0 || body4_pct < 30.0 {
            return 0;
        }
        if !(b1.close < b0.close && b2.close < b1.close) {
            return 0;
        }
        if !(b1.open <= b0.open && b1.open >= b0.close) {
            return 0;
        }
        if !(b2.open <= b1.open && b2.open >= b1.close) {
            return 0;
        }
        if body3_pct > 25.0 || upper3 < body3 || b3.low >= b2.low {
            return 0;
        }
        if b4.close <= b3.high || b4.close <= b2.open {
            return 0;
        }
        100
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 4, detect);
    let b0 = sorted[n - 5];
    let b1 = sorted[n - 4];
    let b2 = sorted[n - 3];
    let b3 = sorted[n - 2];
    let b4 = sorted[n - 1];
    let (_, _, _, _, body0_pct, _) = candle_metrics(b0);
    let (_, _, _, _, body1_pct, _) = candle_metrics(b1);
    let (_, _, _, _, body2_pct, _) = candle_metrics(b2);
    let (_, range3, upper3, _, fourth_body_pct_range, _) = candle_metrics(b3);
    let (_, _, _, _, fifth_body_pct_range, _) = candle_metrics(b4);
    let avg_first_three_body_pct_range = (body0_pct + body1_pct + body2_pct) / 3.0;
    let fourth_upper_shadow_pct = if range3 > 1e-12 {
        100.0 * upper3 / range3
    } else {
        0.0
    };
    let breakout_pct_vs_fourth_high = if b3.high.abs() > 1e-12 {
        100.0 * (b4.close - b3.high) / b3.high
    } else {
        0.0
    };
    let label = if last_val == 100 {
        "BULLISH_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlLadderBottomSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        avg_first_three_body_pct_range,
        fourth_body_pct_range,
        fourth_upper_shadow_pct,
        fifth_body_pct_range,
        breakout_pct_vs_fourth_high,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b4.close,
        cdl_ladder_bottom_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_unique_three_river_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlUniqueThreeRiverSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 4 {
        return CdlUniqueThreeRiverSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_unique_three_river_label: "INSUFFICIENT_DATA".into(),
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
        let (body0, _, _, _, body0_pct, bull0) = candle_metrics(b0);
        let (body1, _, _, lower1, body1_pct, bull1) = candle_metrics(b1);
        let (_, _, _, _, body2_pct, bull2) = candle_metrics(b2);
        if body0 < 1e-12 || body1 < 1e-12 {
            return 0;
        }
        if bull0 || bull1 || !bull2 {
            return 0;
        }
        if body0_pct < 30.0 || body1_pct > 30.0 || body2_pct > 25.0 {
            return 0;
        }
        if lower1 < 2.0 * body1 {
            return 0;
        }
        if b1.low >= b0.low {
            return 0;
        }
        if b1.close <= b0.close {
            return 0;
        }
        if b2.close >= b1.close {
            return 0;
        }
        if b2.high > b1.high || b2.low < b1.low {
            return 0;
        }
        100
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 2, detect);
    let b0 = sorted[n - 3];
    let b1 = sorted[n - 2];
    let b2 = sorted[n - 1];
    let (_, _, _, _, first_body_pct_range, _) = candle_metrics(b0);
    let (body1, range1, _, lower1, second_body_pct_range, _) = candle_metrics(b1);
    let (_, _, _, _, third_body_pct_range, _) = candle_metrics(b2);
    let second_lower_shadow_pct = if range1 > 1e-12 {
        100.0 * lower1 / range1
    } else {
        0.0
    };
    let third_close_vs_second_close_pct = if body1 > 1e-12 {
        100.0 * (b2.close - b1.close) / body1
    } else {
        0.0
    };
    let label = if last_val == 100 {
        "BULLISH_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlUniqueThreeRiverSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        first_body_pct_range,
        second_body_pct_range,
        second_lower_shadow_pct,
        third_body_pct_range,
        third_close_vs_second_close_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b2.close,
        cdl_unique_three_river_label: label.into(),
        note: String::new(),
    }
}

// Advance-block, breakaway, gap, and concealed-baby-swallow candlestick patterns

pub fn compute_cdl_advance_block_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlAdvanceBlockSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 4 {
        return CdlAdvanceBlockSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_advance_block_label: "INSUFFICIENT_DATA".into(),
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
        let (body0, range0, upper0, _, body0_pct, bull0) = candle_metrics(b0);
        let (body1, _range1, upper1, _, body1_pct, bull1) = candle_metrics(b1);
        let (body2, range2, upper2, _, body2_pct, bull2) = candle_metrics(b2);
        if body0 < 1e-12 || body1 < 1e-12 || body2 < 1e-12 {
            return 0;
        }
        if !(bull0 && bull1 && bull2) {
            return 0;
        }
        if body0_pct < 30.0 || body1_pct < 25.0 || body2_pct < 20.0 {
            return 0;
        }
        if !(b1.close > b0.close && b2.close > b1.close) {
            return 0;
        }
        if !(b1.open >= b0.open && b1.open <= b0.close) {
            return 0;
        }
        if !(b2.open >= b1.open && b2.open <= b1.close) {
            return 0;
        }
        let upper0_pct = if range0 > 1e-12 {
            100.0 * upper0 / range0
        } else {
            0.0
        };
        let upper1_pct = if (b1.high - b1.low).abs() > 1e-12 {
            100.0 * upper1 / (b1.high - b1.low).abs()
        } else {
            0.0
        };
        let upper2_pct = if range2 > 1e-12 {
            100.0 * upper2 / range2
        } else {
            0.0
        };
        let shrinking = body2 < body1 && body1 <= body0 * 1.05;
        let rising_shadows = upper2_pct >= upper1_pct && upper1_pct >= upper0_pct;
        if !shrinking || !rising_shadows || upper2_pct < 15.0 {
            return 0;
        }
        -100
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 2, detect);
    let b0 = sorted[n - 3];
    let b1 = sorted[n - 2];
    let b2 = sorted[n - 1];
    let (_, _, _, _, first_body_pct_range, _) = candle_metrics(b0);
    let (_, _, _, _, second_body_pct_range, _) = candle_metrics(b1);
    let (_, range2, upper2, _, third_body_pct_range, _) = candle_metrics(b2);
    let third_upper_shadow_pct = if range2 > 1e-12 {
        100.0 * upper2 / range2
    } else {
        0.0
    };
    let total_close_gain_pct = if b0.close.abs() > 1e-12 {
        100.0 * (b2.close - b0.close) / b0.close
    } else {
        0.0
    };
    let label = if last_val == -100 {
        "BEARISH_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlAdvanceBlockSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        first_body_pct_range,
        second_body_pct_range,
        third_body_pct_range,
        third_upper_shadow_pct,
        total_close_gain_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b2.close,
        cdl_advance_block_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_breakaway_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlBreakawaySnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 6 {
        return CdlBreakawaySnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_breakaway_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥6 bars, got {}", n),
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
        let (_, _, _, _, _, bull1) = candle_metrics(b1);
        let (_, _, _, _, _, bull2) = candle_metrics(b2);
        let (_, _, _, _, _, bull3) = candle_metrics(b3);
        let (_, _, _, _, body4_pct, bull4) = candle_metrics(b4);
        if body0 < 1e-12 {
            return 0;
        }
        if body0_pct < 30.0 || body4_pct < 30.0 {
            return 0;
        }
        if !bull0 && !bull1 && !bull2 && !bull3 && bull4 {
            if b1.high >= b0.low {
                return 0;
            }
            if !(b2.low < b1.low && b3.low <= b2.low) {
                return 0;
            }
            if !(b4.close > b1.high && b4.close < b0.low) {
                return 0;
            }
            100
        } else if bull0 && bull1 && bull2 && bull3 && !bull4 {
            if b1.low <= b0.high {
                return 0;
            }
            if !(b2.high > b1.high && b3.high >= b2.high) {
                return 0;
            }
            if !(b4.close < b1.low && b4.close > b0.high) {
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
    let b4 = sorted[n - 1];
    let (_, range0, _, _, first_body_pct_range, bull0) = candle_metrics(b0);
    let (_, _, _, _, fifth_body_pct_range, _) = candle_metrics(b4);
    let initial_gap = if !bull0 && b1.high < b0.low {
        b0.low - b1.high
    } else if bull0 && b1.low > b0.high {
        b1.low - b0.high
    } else {
        0.0
    };
    let initial_gap_pct_range = if range0 > 1e-12 {
        100.0 * initial_gap / range0
    } else {
        0.0
    };
    let gap_retracement_pct = if initial_gap > 1e-12 {
        if !bull0 {
            100.0 * (b4.close - b1.high) / initial_gap
        } else {
            100.0 * (b1.low - b4.close) / initial_gap
        }
    } else {
        0.0
    };
    let label = match last_val {
        100 => "BULLISH_PATTERN",
        -100 => "BEARISH_PATTERN",
        _ => "NO_PATTERN",
    };
    CdlBreakawaySnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        first_body_pct_range,
        initial_gap_pct_range,
        fifth_body_pct_range,
        gap_retracement_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b4.close,
        cdl_breakaway_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_gap_side_side_white_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlGapSideSideWhiteSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 4 {
        return CdlGapSideSideWhiteSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_gap_side_side_white_label: "INSUFFICIENT_DATA".into(),
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
        let (body1, _range1, _, _, body1_pct, bull1) = candle_metrics(b1);
        let (body2, _, _, _, body2_pct, bull2) = candle_metrics(b2);
        if body1 < 1e-12 || body2 < 1e-12 {
            return 0;
        }
        if !(bull1 && bull2) {
            return 0;
        }
        if body1_pct < 25.0 || body2_pct < 25.0 {
            return 0;
        }
        let open_diff = 100.0 * (b2.open - b1.open).abs() / body1;
        let close_diff = 100.0 * (b2.close - b1.close).abs() / body1;
        if open_diff > 25.0 || close_diff > 35.0 {
            return 0;
        }
        if b1.low > b0.high && b2.low > b0.high {
            100
        } else if b1.high < b0.low && b2.high < b0.low {
            -100
        } else {
            0
        }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 2, detect);
    let b0 = sorted[n - 3];
    let b1 = sorted[n - 2];
    let b2 = sorted[n - 1];
    let (body1, range1, _, _, second_body_pct_range, _) = candle_metrics(b1);
    let (_, _, _, _, third_body_pct_range, _) = candle_metrics(b2);
    let gap_raw = if b1.low > b0.high {
        b1.low - b0.high
    } else if b1.high < b0.low {
        b0.low - b1.high
    } else {
        0.0
    };
    let gap_pct_range = if range1 > 1e-12 {
        100.0 * gap_raw / range1
    } else {
        0.0
    };
    let open_similarity_pct_body = if body1 > 1e-12 {
        100.0 * (b2.open - b1.open).abs() / body1
    } else {
        0.0
    };
    let close_similarity_pct_body = if body1 > 1e-12 {
        100.0 * (b2.close - b1.close).abs() / body1
    } else {
        0.0
    };
    let label = match last_val {
        100 => "BULLISH_CONTINUATION",
        -100 => "BEARISH_CONTINUATION",
        _ => "NO_PATTERN",
    };
    CdlGapSideSideWhiteSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        gap_pct_range,
        second_body_pct_range,
        third_body_pct_range,
        open_similarity_pct_body,
        close_similarity_pct_body,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b2.close,
        cdl_gap_side_side_white_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_upside_gap_two_crows_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlUpsideGapTwoCrowsSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 4 {
        return CdlUpsideGapTwoCrowsSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_upside_gap_two_crows_label: "INSUFFICIENT_DATA".into(),
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
        let (body0, _, _, _, body0_pct, bull0) = candle_metrics(b0);
        let (body1, _, _, _, _, bull1) = candle_metrics(b1);
        let (_, _, _, _, _, bull2) = candle_metrics(b2);
        if body0 < 1e-12 || body1 < 1e-12 {
            return 0;
        }
        if !bull0 || bull1 || bull2 {
            return 0;
        }
        if body0_pct < 30.0 {
            return 0;
        }
        if b1.low <= b0.high {
            return 0;
        }
        if b2.open <= b1.open {
            return 0;
        }
        if !(b2.close < b1.close && b2.close > b0.high && b2.close < b1.low) {
            return 0;
        }
        -100
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 2, detect);
    let b0 = sorted[n - 3];
    let b1 = sorted[n - 2];
    let b2 = sorted[n - 1];
    let (body0, range0, _, _, first_body_pct_range, _) = candle_metrics(b0);
    let (body1, _, _, _, _, _) = candle_metrics(b1);
    let upside_gap = if b1.low > b0.high {
        b1.low - b0.high
    } else {
        0.0
    };
    let upside_gap_pct_range = if range0 > 1e-12 {
        100.0 * upside_gap / range0
    } else {
        0.0
    };
    let third_open_above_second_pct_body = if body1 > 1e-12 {
        100.0 * (b2.open - b1.open) / body1
    } else {
        0.0
    };
    let third_close_into_gap_pct = if upside_gap > 1e-12 {
        100.0 * (b1.low - b2.close) / upside_gap
    } else {
        0.0
    };
    let _ = body0;
    let label = if last_val == -100 {
        "BEARISH_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlUpsideGapTwoCrowsSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        first_body_pct_range,
        upside_gap_pct_range,
        third_open_above_second_pct_body,
        third_close_into_gap_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b2.close,
        cdl_upside_gap_two_crows_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_xside_gap_three_methods_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlXSideGapThreeMethodsSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 4 {
        return CdlXSideGapThreeMethodsSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_xside_gap_three_methods_label: "INSUFFICIENT_DATA".into(),
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
        let (_, _, _, _, body1_pct, bull1) = candle_metrics(b1);
        let (_, _, _, _, body2_pct, bull2) = candle_metrics(b2);
        if body1_pct < 25.0 || body2_pct < 20.0 {
            return 0;
        }
        if b1.low > b0.high && bull1 && !bull2 {
            if !(b2.open < b1.close && b2.open > b1.open) {
                return 0;
            }
            if !(b2.close < b1.open && b2.close > b0.high) {
                return 0;
            }
            100
        } else if b1.high < b0.low && !bull1 && bull2 {
            if !(b2.open > b1.close && b2.open < b1.open) {
                return 0;
            }
            if !(b2.close > b1.open && b2.close < b0.low) {
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
    let (_, range1, _, _, second_body_pct_range, bull1) = candle_metrics(b1);
    let (_, _, _, _, third_body_pct_range, _) = candle_metrics(b2);
    let gap_raw = if b1.low > b0.high {
        b1.low - b0.high
    } else if b1.high < b0.low {
        b0.low - b1.high
    } else {
        0.0
    };
    let gap_pct_range = if range1 > 1e-12 {
        100.0 * gap_raw / range1
    } else {
        0.0
    };
    let gap_fill_pct = if gap_raw > 1e-12 {
        if bull1 {
            100.0 * (b1.low - b2.close) / gap_raw
        } else {
            100.0 * (b2.close - b1.high) / gap_raw
        }
    } else {
        0.0
    };
    let label = match last_val {
        100 => "BULLISH_CONTINUATION",
        -100 => "BEARISH_CONTINUATION",
        _ => "NO_PATTERN",
    };
    CdlXSideGapThreeMethodsSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        gap_pct_range,
        second_body_pct_range,
        third_body_pct_range,
        gap_fill_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b2.close,
        cdl_xside_gap_three_methods_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_conceal_baby_swallow_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlConcealBabySwallowSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 5 {
        return CdlConcealBabySwallowSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_conceal_baby_swallow_label: "INSUFFICIENT_DATA".into(),
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
        let (_, range0, upper0, lower0, body0_pct, bull0) = candle_metrics(b0);
        let (_, range1, upper1, lower1, body1_pct, bull1) = candle_metrics(b1);
        let (_, range2, upper2, _, body2_pct, bull2) = candle_metrics(b2);
        let (_, _, _, _, _, bull3) = candle_metrics(b3);
        if bull0 || bull1 || bull2 || bull3 {
            return 0;
        }
        let marubozu0 = body0_pct >= 90.0
            && range0 > 1e-12
            && 100.0 * upper0 / range0 <= 5.0
            && 100.0 * lower0 / range0 <= 5.0;
        let marubozu1 = body1_pct >= 90.0
            && range1 > 1e-12
            && 100.0 * upper1 / range1 <= 5.0
            && 100.0 * lower1 / range1 <= 5.0;
        if !marubozu0 || !marubozu1 || body2_pct > 50.0 {
            return 0;
        }
        if b2.high >= b1.low {
            return 0;
        }
        if range2 <= 1e-12 || 100.0 * upper2 / range2 < 20.0 {
            return 0;
        }
        if !(b3.high > b2.high && b3.low < b2.low) {
            return 0;
        }
        100
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 3, detect);
    let b0 = sorted[n - 4];
    let b1 = sorted[n - 3];
    let b2 = sorted[n - 2];
    let b3 = sorted[n - 1];
    let (_, _, _, _, first_body_pct_range, _) = candle_metrics(b0);
    let (_, _, _, _, second_body_pct_range, _) = candle_metrics(b1);
    let (_, range2, upper2, _, _, _) = candle_metrics(b2);
    let third_upper_shadow_pct = if range2 > 1e-12 {
        100.0 * upper2 / range2
    } else {
        0.0
    };
    let third_range = (b2.high - b2.low).abs();
    let fourth_range_engulf_pct = if third_range > 1e-12 && b3.high > b2.high && b3.low < b2.low {
        100.0 * ((b3.high - b3.low) - third_range) / third_range
    } else {
        0.0
    };
    let label = if last_val == 100 {
        "BULLISH_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlConcealBabySwallowSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        first_body_pct_range,
        second_body_pct_range,
        third_upper_shadow_pct,
        fourth_range_engulf_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b3.close,
        cdl_conceal_baby_swallow_label: label.into(),
        note: String::new(),
    }
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
