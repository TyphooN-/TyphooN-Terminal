use super::*;

// Belt-hold, marubozu, high-wave, and line-length candlestick patterns

pub fn compute_cdl_belt_hold_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlBeltHoldSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 2 {
        return CdlBeltHoldSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_belt_hold_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥2 bars, got {}", n),
            ..Default::default()
        };
    }
    // Belt-hold: long body (≥ 60% of range) with virtually no opening shadow.
    // Bullish = green candle opening at/near the low; bearish = red candle
    // opening at/near the high.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        let b = s[i];
        let (body, range, upper, lower, body_pct, bull) = candle_metrics(b);
        if body < 1e-12 || range < 1e-12 {
            return 0;
        }
        if body_pct < 60.0 {
            return 0;
        }
        let upper_pct = 100.0 * upper / range;
        let lower_pct = 100.0 * lower / range;
        if bull && lower_pct <= 5.0 && upper_pct <= 35.0 {
            100
        } else if !bull && upper_pct <= 5.0 && lower_pct <= 35.0 {
            -100
        } else {
            0
        }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 0, detect);
    let b = sorted[n - 1];
    let (_, range, upper, lower, body_pct, bull) = candle_metrics(b);
    let (opening_shadow_pct, closing_shadow_pct) = if range > 1e-12 {
        if bull {
            (100.0 * lower / range, 100.0 * upper / range)
        } else {
            (100.0 * upper / range, 100.0 * lower / range)
        }
    } else {
        (0.0, 0.0)
    };
    let label = match last_val {
        100 => "BULLISH_PATTERN",
        -100 => "BEARISH_PATTERN",
        _ => "NO_PATTERN",
    };
    CdlBeltHoldSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        body_pct_range: body_pct,
        opening_shadow_pct,
        closing_shadow_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b.close,
        cdl_belt_hold_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_closing_marubozu_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlClosingMarubozuSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 2 {
        return CdlClosingMarubozuSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_closing_marubozu_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥2 bars, got {}", n),
            ..Default::default()
        };
    }
    // Closing Marubozu: long body (≥ 60% of range) with virtually no closing
    // shadow. Bullish = green close at/near high; bearish = red close at/near low.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        let b = s[i];
        let (body, range, upper, lower, body_pct, bull) = candle_metrics(b);
        if body < 1e-12 || range < 1e-12 {
            return 0;
        }
        if body_pct < 60.0 {
            return 0;
        }
        let upper_pct = 100.0 * upper / range;
        let lower_pct = 100.0 * lower / range;
        if bull && upper_pct <= 5.0 && lower_pct <= 35.0 {
            100
        } else if !bull && lower_pct <= 5.0 && upper_pct <= 35.0 {
            -100
        } else {
            0
        }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 0, detect);
    let b = sorted[n - 1];
    let (_, range, upper, lower, body_pct, bull) = candle_metrics(b);
    let (opening_shadow_pct, closing_shadow_pct) = if range > 1e-12 {
        if bull {
            (100.0 * lower / range, 100.0 * upper / range)
        } else {
            (100.0 * upper / range, 100.0 * lower / range)
        }
    } else {
        (0.0, 0.0)
    };
    let label = match last_val {
        100 => "BULLISH_PATTERN",
        -100 => "BEARISH_PATTERN",
        _ => "NO_PATTERN",
    };
    CdlClosingMarubozuSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        body_pct_range: body_pct,
        opening_shadow_pct,
        closing_shadow_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b.close,
        cdl_closing_marubozu_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_high_wave_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlHighWaveSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 2 {
        return CdlHighWaveSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_high_wave_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥2 bars, got {}", n),
            ..Default::default()
        };
    }
    // High-Wave: small non-doji body with long shadows on both sides.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        let b = s[i];
        let (body, range, upper, lower, body_pct, bull) = candle_metrics(b);
        if body < 1e-12 || range < 1e-12 {
            return 0;
        }
        let upper_pct = 100.0 * upper / range;
        let lower_pct = 100.0 * lower / range;
        if !(5.0..=20.0).contains(&body_pct) {
            return 0;
        }
        if upper_pct < 30.0 || lower_pct < 30.0 {
            return 0;
        }
        if upper <= body || lower <= body {
            return 0;
        }
        if bull { 100 } else { -100 }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 0, detect);
    let b = sorted[n - 1];
    let (_, range, upper, lower, body_pct, _) = candle_metrics(b);
    let (upper_shadow_pct, lower_shadow_pct) = if range > 1e-12 {
        (100.0 * upper / range, 100.0 * lower / range)
    } else {
        (0.0, 0.0)
    };
    let label = match last_val {
        100 => "GREEN_BODY_PATTERN",
        -100 => "RED_BODY_PATTERN",
        _ => "NO_PATTERN",
    };
    CdlHighWaveSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        body_pct_range: body_pct,
        upper_shadow_pct,
        lower_shadow_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b.close,
        cdl_high_wave_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_long_line_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlLongLineSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 2 {
        return CdlLongLineSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_long_line_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥2 bars, got {}", n),
            ..Default::default()
        };
    }
    // Long Line: dominant body (≥ 60% of range) with relatively small shadows.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        let b = s[i];
        let (body, range, upper, lower, body_pct, bull) = candle_metrics(b);
        if body < 1e-12 || range < 1e-12 {
            return 0;
        }
        let upper_pct = 100.0 * upper / range;
        let lower_pct = 100.0 * lower / range;
        if body_pct < 60.0 {
            return 0;
        }
        if upper_pct > 20.0 || lower_pct > 20.0 {
            return 0;
        }
        if bull { 100 } else { -100 }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 0, detect);
    let b = sorted[n - 1];
    let (_, range, upper, lower, body_pct, _) = candle_metrics(b);
    let (upper_shadow_pct, lower_shadow_pct) = if range > 1e-12 {
        (100.0 * upper / range, 100.0 * lower / range)
    } else {
        (0.0, 0.0)
    };
    let label = match last_val {
        100 => "GREEN_BODY_PATTERN",
        -100 => "RED_BODY_PATTERN",
        _ => "NO_PATTERN",
    };
    CdlLongLineSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        body_pct_range: body_pct,
        upper_shadow_pct,
        lower_shadow_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b.close,
        cdl_long_line_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_short_line_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlShortLineSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 2 {
        return CdlShortLineSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_short_line_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥2 bars, got {}", n),
            ..Default::default()
        };
    }
    // Short Line: short real body (between doji and spinning-top scale)
    // with relatively small shadows.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        let b = s[i];
        let (body, range, upper, lower, body_pct, bull) = candle_metrics(b);
        if body < 1e-12 || range < 1e-12 {
            return 0;
        }
        let upper_pct = 100.0 * upper / range;
        let lower_pct = 100.0 * lower / range;
        if !(5.0..=30.0).contains(&body_pct) {
            return 0;
        }
        if upper_pct > 40.0 || lower_pct > 40.0 {
            return 0;
        }
        if bull { 100 } else { -100 }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 0, detect);
    let b = sorted[n - 1];
    let (_, range, upper, lower, body_pct, _) = candle_metrics(b);
    let (upper_shadow_pct, lower_shadow_pct) = if range > 1e-12 {
        (100.0 * upper / range, 100.0 * lower / range)
    } else {
        (0.0, 0.0)
    };
    let label = match last_val {
        100 => "GREEN_BODY_PATTERN",
        -100 => "RED_BODY_PATTERN",
        _ => "NO_PATTERN",
    };
    CdlShortLineSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        body_pct_range: body_pct,
        upper_shadow_pct,
        lower_shadow_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b.close,
        cdl_short_line_label: label.into(),
        note: String::new(),
    }
}
