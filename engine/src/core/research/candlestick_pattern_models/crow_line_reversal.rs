use super::*;

// Crow, line-strike, outside, and matching-low candlestick patterns

pub fn compute_cdl_two_crows_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlTwoCrowsSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 4 {
        return CdlTwoCrowsSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_two_crows_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥4 bars, got {}", n),
            ..Default::default()
        };
    }
    // Two Crows: strong green body, then a red candle whose real body gaps
    // above the first, followed by another red candle opening inside the
    // second body and closing back into the first real body.
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
        if !bull0 || bull1 || bull2 {
            return 0;
        }
        if body0_pct < 30.0 || body1_pct < 20.0 || body2_pct < 20.0 {
            return 0;
        }
        let (b0_lo, b0_hi) = candle_body_bounds(b0);
        let (b1_lo, b1_hi) = candle_body_bounds(b1);
        if b1_lo <= b0_hi {
            return 0;
        }
        if !(b2.open > b1_lo && b2.open < b1_hi) {
            return 0;
        }
        if b2.close >= b1.close {
            return 0;
        }
        if !(b2.close > b0_lo && b2.close < b0_hi) {
            return 0;
        }
        -100
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 2, detect);
    let b0 = sorted[n - 3];
    let b1 = sorted[n - 2];
    let b2 = sorted[n - 1];
    let (body0, _, _, _, body0_pct, _) = candle_metrics(b0);
    let (_, _, _, _, _, _) = candle_metrics(b1);
    let (_b0_lo, b0_hi) = candle_body_bounds(b0);
    let (b1_lo, _) = candle_body_bounds(b1);
    let second_gap_pct = if b0_hi.abs() > 1e-12 {
        100.0 * (b1_lo - b0_hi) / b0_hi
    } else {
        0.0
    };
    let third_penetration_pct = if body0 > 1e-12 {
        100.0 * (b0_hi - b2.close) / body0
    } else {
        0.0
    };
    let label = if last_val == -100 {
        "BEARISH_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlTwoCrowsSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        first_body_pct_range: body0_pct,
        second_gap_pct,
        third_penetration_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b2.close,
        cdl_two_crows_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_three_line_strike_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlThreeLineStrikeSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 5 {
        return CdlThreeLineStrikeSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_three_line_strike_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥5 bars, got {}", n),
            ..Default::default()
        };
    }
    // Three Line Strike: three same-direction bodies stepping forward, then
    // a large opposite-colour strike candle that opens beyond bar 3 close and
    // closes beyond bar 1 open.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        if i < 3 {
            return 0;
        }
        let b0 = s[i - 3];
        let b1 = s[i - 2];
        let b2 = s[i - 1];
        let b3 = s[i];
        let (body0, _, _, _, body0_pct, bull0) = candle_metrics(b0);
        let (body1, _, _, _, body1_pct, bull1) = candle_metrics(b1);
        let (body2, _, _, _, body2_pct, bull2) = candle_metrics(b2);
        let (body3, _, _, _, body3_pct, bull3) = candle_metrics(b3);
        if body0 < 1e-12 || body1 < 1e-12 || body2 < 1e-12 || body3 < 1e-12 {
            return 0;
        }
        if body0_pct < 30.0 || body1_pct < 30.0 || body2_pct < 30.0 || body3_pct < 40.0 {
            return 0;
        }
        if !bull0 && !bull1 && !bull2 && bull3 {
            if !(b1.close < b0.close && b2.close < b1.close) {
                return 0;
            }
            if b3.open >= b2.close {
                return 0;
            }
            if b3.close <= b0.open {
                return 0;
            }
            100
        } else if bull0 && bull1 && bull2 && !bull3 {
            if !(b1.close > b0.close && b2.close > b1.close) {
                return 0;
            }
            if b3.open <= b2.close {
                return 0;
            }
            if b3.close >= b0.open {
                return 0;
            }
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
    let (_, _, _, _, body0_pct, _) = candle_metrics(b0);
    let (_, _, _, _, body1_pct, _) = candle_metrics(b1);
    let (_, _, _, _, body2_pct, _) = candle_metrics(b2);
    let (_, _, _, _, strike_body_pct_range, _) = candle_metrics(b3);
    let avg_first_three_body_pct_range = (body0_pct + body1_pct + body2_pct) / 3.0;
    let strike_close_vs_first_open_pct = if b0.open.abs() > 1e-12 {
        100.0 * (b3.close - b0.open) / b0.open
    } else {
        0.0
    };
    let label = match last_val {
        100 => "BULLISH_PATTERN",
        -100 => "BEARISH_PATTERN",
        _ => "NO_PATTERN",
    };
    CdlThreeLineStrikeSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        avg_first_three_body_pct_range,
        strike_body_pct_range,
        strike_close_vs_first_open_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b3.close,
        cdl_three_line_strike_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_three_outside_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlThreeOutsideSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 4 {
        return CdlThreeOutsideSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_three_outside_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥4 bars, got {}", n),
            ..Default::default()
        };
    }
    // Three Outside = engulfing reversal confirmed by a third candle.
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
        if body0_pct < 20.0 {
            return 0;
        }
        if !bull0 && bull1 {
            if !(b1.open <= b0.close && b1.close >= b0.open) {
                return 0;
            }
            if !bull2 || b2.close <= b1.close {
                return 0;
            }
            100
        } else if bull0 && !bull1 {
            if !(b1.open >= b0.close && b1.close <= b0.open) {
                return 0;
            }
            if bull2 || b2.close >= b1.close {
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
    let (body0, _, _, _, first_body_pct_range, _) = candle_metrics(b0);
    let (body1, _, _, _, _, _) = candle_metrics(b1);
    let engulf_body_ratio = if body0 > 1e-12 { body1 / body0 } else { 0.0 };
    let confirmation_pct_body2 = if body1 > 1e-12 {
        100.0 * (b2.close - b1.close).abs() / body1
    } else {
        0.0
    };
    let label = match last_val {
        100 => "BULLISH_PATTERN",
        -100 => "BEARISH_PATTERN",
        _ => "NO_PATTERN",
    };
    CdlThreeOutsideSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        first_body_pct_range,
        engulf_body_ratio,
        confirmation_pct_body2,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b2.close,
        cdl_three_outside_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_matching_low_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlMatchingLowSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 3 {
        return CdlMatchingLowSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_matching_low_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 bars, got {}", n),
            ..Default::default()
        };
    }
    // Matching Low: two red candles that close at nearly the same price.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        if i < 1 {
            return 0;
        }
        let b0 = s[i - 1];
        let b1 = s[i];
        let (body0, _, _, _, body0_pct, bull0) = candle_metrics(b0);
        let (body1, _, _, _, body1_pct, bull1) = candle_metrics(b1);
        if body0 < 1e-12 || body1 < 1e-12 {
            return 0;
        }
        if bull0 || bull1 {
            return 0;
        }
        if body0_pct < 30.0 || body1_pct < 20.0 {
            return 0;
        }
        let diff_pct = 100.0 * (b1.close - b0.close).abs() / body0;
        if diff_pct <= 5.0 { 100 } else { 0 }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 1, detect);
    let b0 = sorted[n - 2];
    let b1 = sorted[n - 1];
    let (body0, _, _, _, prior_body_pct_range, _) = candle_metrics(b0);
    let (_, _, _, _, current_body_pct_range, _) = candle_metrics(b1);
    let close_match_pct_body = if body0 > 1e-12 {
        100.0 * (b1.close - b0.close).abs() / body0
    } else {
        0.0
    };
    let label = if last_val == 100 {
        "BULLISH_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlMatchingLowSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        prior_body_pct_range,
        current_body_pct_range,
        close_match_pct_body,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b1.close,
        cdl_matching_low_label: label.into(),
        note: String::new(),
    }
}
