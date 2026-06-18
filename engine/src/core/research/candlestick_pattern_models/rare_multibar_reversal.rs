use super::*;

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
