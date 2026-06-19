use super::*;

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
