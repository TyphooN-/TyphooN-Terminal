use super::*;

// Separating-line, stick-sandwich, rickshaw-man, and takuri candlestick patterns

pub fn compute_cdl_separating_lines_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlSeparatingLinesSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 3 {
        return CdlSeparatingLinesSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_separating_lines_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 bars, got {}", n),
            ..Default::default()
        };
    }
    // Separating Lines: same open, opposite colours, second candle resumes
    // the broader direction.
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
        if body0_pct < 30.0 || body1_pct < 30.0 {
            return 0;
        }
        let open_match_pct = 100.0 * (b1.open - b0.open).abs() / body0;
        if open_match_pct > 10.0 {
            return 0;
        }
        if !bull0 && bull1 && b1.close > b0.open {
            100
        } else if bull0 && !bull1 && b1.close < b0.open {
            -100
        } else {
            0
        }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 1, detect);
    let b0 = sorted[n - 2];
    let b1 = sorted[n - 1];
    let (body0, _, _, _, prior_body_pct_range, _) = candle_metrics(b0);
    let (_, _, _, _, current_body_pct_range, _) = candle_metrics(b1);
    let open_match_pct_body = if body0 > 1e-12 {
        100.0 * (b1.open - b0.open).abs() / body0
    } else {
        0.0
    };
    let continuation_pct_body = if body0 > 1e-12 {
        100.0 * (b1.close - b0.open).abs() / body0
    } else {
        0.0
    };
    let label = match last_val {
        100 => "BULLISH_CONTINUATION",
        -100 => "BEARISH_CONTINUATION",
        _ => "NO_PATTERN",
    };
    CdlSeparatingLinesSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        prior_body_pct_range,
        current_body_pct_range,
        open_match_pct_body,
        continuation_pct_body,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b1.close,
        cdl_separating_lines_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_stick_sandwich_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlStickSandwichSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 4 {
        return CdlStickSandwichSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_stick_sandwich_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥4 bars, got {}", n),
            ..Default::default()
        };
    }
    // Stick Sandwich: red / green / red where the first and third closes
    // match, marking support after a rebound attempt.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        if i < 2 {
            return 0;
        }
        let b0 = s[i - 2];
        let b1 = s[i - 1];
        let b2 = s[i];
        let (body0, _, _, _, body0_pct, bull0) = candle_metrics(b0);
        let (_, _, _, _, _, bull1) = candle_metrics(b1);
        let (body2, _, _, _, body2_pct, bull2) = candle_metrics(b2);
        if body0 < 1e-12 || body2 < 1e-12 {
            return 0;
        }
        if bull0 || !bull1 || bull2 {
            return 0;
        }
        if body0_pct < 30.0 || body2_pct < 30.0 {
            return 0;
        }
        let close_match_pct = 100.0 * (b2.close - b0.close).abs() / body0;
        if close_match_pct > 5.0 {
            return 0;
        }
        if b1.close <= b0.open {
            return 0;
        }
        100
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 2, detect);
    let b0 = sorted[n - 3];
    let b1 = sorted[n - 2];
    let b2 = sorted[n - 1];
    let (body0, _, _, _, first_body_pct_range, _) = candle_metrics(b0);
    let (_, _, _, _, _, _) = candle_metrics(b1);
    let (_, _, _, _, third_body_pct_range, _) = candle_metrics(b2);
    let close_match_pct_body = if body0 > 1e-12 {
        100.0 * (b2.close - b0.close).abs() / body0
    } else {
        0.0
    };
    let middle_rebound_pct = if body0 > 1e-12 {
        100.0 * (b1.close - b0.close) / body0
    } else {
        0.0
    };
    let label = if last_val == 100 {
        "BULLISH_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlStickSandwichSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        first_body_pct_range,
        third_body_pct_range,
        close_match_pct_body,
        middle_rebound_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b2.close,
        cdl_stick_sandwich_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_rickshaw_man_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlRickshawManSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 2 {
        return CdlRickshawManSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_rickshaw_man_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥2 bars, got {}", n),
            ..Default::default()
        };
    }
    // Rickshaw Man: doji-like body centered in the range with long, roughly
    // balanced shadows on both sides.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        let b = s[i];
        let (_, range, upper, lower, body_pct, _) = candle_metrics(b);
        if range < 1e-12 {
            return 0;
        }
        let upper_pct = 100.0 * upper / range;
        let lower_pct = 100.0 * lower / range;
        let body_mid = (b.open + b.close) / 2.0;
        let range_mid = (b.high + b.low) / 2.0;
        let body_midpoint_offset_pct = 100.0 * (body_mid - range_mid).abs() / range;
        if body_pct <= 5.0
            && upper_pct >= 30.0
            && lower_pct >= 30.0
            && (upper_pct - lower_pct).abs() <= 20.0
            && body_midpoint_offset_pct <= 10.0
        {
            100
        } else {
            0
        }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 0, detect);
    let b = sorted[n - 1];
    let (_, range, upper, lower, body_pct, _) = candle_metrics(b);
    let (upper_shadow_pct, lower_shadow_pct, body_midpoint_offset_pct) = if range > 1e-12 {
        let upper_pct = 100.0 * upper / range;
        let lower_pct = 100.0 * lower / range;
        let body_mid = (b.open + b.close) / 2.0;
        let range_mid = (b.high + b.low) / 2.0;
        let offset_pct = 100.0 * (body_mid - range_mid).abs() / range;
        (upper_pct, lower_pct, offset_pct)
    } else {
        (0.0, 0.0, 0.0)
    };
    let label = if last_val == 100 {
        "RICKSHAW_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlRickshawManSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        body_pct_range: body_pct,
        upper_shadow_pct,
        lower_shadow_pct,
        body_midpoint_offset_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b.close,
        cdl_rickshaw_man_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_takuri_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlTakuriSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 2 {
        return CdlTakuriSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_takuri_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥2 bars, got {}", n),
            ..Default::default()
        };
    }
    // Takuri: dragonfly-style doji with an extreme lower shadow.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        let b = s[i];
        let (body, range, upper, lower, body_pct, _) = candle_metrics(b);
        if range < 1e-12 {
            return 0;
        }
        let upper_pct = 100.0 * upper / range;
        let lower_pct = 100.0 * lower / range;
        if body_pct <= 5.0
            && upper_pct <= 10.0
            && lower_pct >= 70.0
            && lower >= 3.0 * upper.max(body)
        {
            100
        } else {
            0
        }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 0, detect);
    let b = sorted[n - 1];
    let (body, range, upper, lower, body_pct, _) = candle_metrics(b);
    let (upper_shadow_pct, lower_shadow_pct, lower_to_upper_ratio) = if range > 1e-12 {
        let ratio = if upper > 1e-12 {
            lower / upper
        } else {
            999.0_f64.max(lower / body.max(1e-12))
        };
        (100.0 * upper / range, 100.0 * lower / range, ratio)
    } else {
        (0.0, 0.0, 0.0)
    };
    let _ = body;
    let label = if last_val == 100 {
        "BULLISH_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlTakuriSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        body_pct_range: body_pct,
        upper_shadow_pct,
        lower_shadow_pct,
        lower_to_upper_ratio,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b.close,
        cdl_takuri_label: label.into(),
        note: String::new(),
    }
}
