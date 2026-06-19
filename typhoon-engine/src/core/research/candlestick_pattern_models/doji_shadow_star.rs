use super::*;

// Piercing, doji-shadow, hanging/inverted hammer, and star candlestick patterns

pub fn compute_cdl_piercing_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlPiercingSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 3 {
        return CdlPiercingSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_piercing_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 bars, got {}", n),
            ..Default::default()
        };
    }
    // Piercing Line (2-bar bullish reversal — mirror of Dark Cloud Cover):
    //   prior bar: red with body ≥ 30% of range,
    //   current bar: green, opens below prior low, closes above prior midpoint.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        if i < 1 {
            return 0;
        }
        let b0 = s[i - 1];
        let b1 = s[i];
        let (body0, _, _, _, body0_pct, bull0) = candle_metrics(b0);
        let (body1, _, _, _, _body1_pct, bull1) = candle_metrics(b1);
        if body0 < 1e-12 || body1 < 1e-12 {
            return 0;
        }
        if bull0 {
            return 0;
        } // prior must be red
        if !bull1 {
            return 0;
        } // current must be green
        if body0_pct < 30.0 {
            return 0;
        }
        if b1.open >= b0.low {
            return 0;
        } // current opens below prior low
        let midpoint0 = (b0.open + b0.close) / 2.0;
        if b1.close <= midpoint0 {
            return 0;
        } // current closes above prior midpoint
        if b1.close >= b0.open {
            return 0;
        } // but not above prior open (engulfing)
        100
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 1, detect);
    let b0 = sorted[n - 2];
    let b1 = sorted[n - 1];
    let (body0, _, _, _, body0_pct, _) = candle_metrics(b0);
    let (_, _, _, _, body1_pct, _) = candle_metrics(b1);
    let pen = if body0 > 1e-12 {
        100.0 * (b1.close - b0.close) / body0
    } else {
        0.0
    };
    let label = if last_match {
        "BULLISH_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlPiercingSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        penetration_pct: pen,
        prior_body_pct_range: body0_pct,
        current_body_pct_range: body1_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b1.close,
        cdl_piercing_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_dragonfly_doji_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlDragonflyDojiSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 2 {
        return CdlDragonflyDojiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_dragonfly_doji_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥2 bars, got {}", n),
            ..Default::default()
        };
    }
    // Dragonfly Doji: body ≤ 5% of range, upper shadow ≤ 5% of range,
    // lower shadow ≥ 60% of range (T-shape).
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        let b = s[i];
        let (_, range, upper, lower, body_pct, _) = candle_metrics(b);
        if range < 1e-12 {
            return 0;
        }
        let upper_pct = 100.0 * upper / range;
        let lower_pct = 100.0 * lower / range;
        if body_pct > 5.0 {
            return 0;
        }
        if upper_pct > 5.0 {
            return 0;
        }
        if lower_pct < 60.0 {
            return 0;
        }
        100
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 0, detect);
    let b = sorted[n - 1];
    let (_, range, upper, lower, body_pct, _) = candle_metrics(b);
    let (upper_pct, lower_pct) = if range > 1e-12 {
        (100.0 * upper / range, 100.0 * lower / range)
    } else {
        (0.0, 0.0)
    };
    let label = if last_match {
        "BULLISH_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlDragonflyDojiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        body_pct_range: body_pct,
        upper_shadow_pct: upper_pct,
        lower_shadow_pct: lower_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b.close,
        cdl_dragonfly_doji_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_gravestone_doji_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlGravestoneDojiSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 2 {
        return CdlGravestoneDojiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_gravestone_doji_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥2 bars, got {}", n),
            ..Default::default()
        };
    }
    // Gravestone Doji: body ≤ 5% of range, lower shadow ≤ 5% of range,
    // upper shadow ≥ 60% of range (inverted-T shape).
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        let b = s[i];
        let (_, range, upper, lower, body_pct, _) = candle_metrics(b);
        if range < 1e-12 {
            return 0;
        }
        let upper_pct = 100.0 * upper / range;
        let lower_pct = 100.0 * lower / range;
        if body_pct > 5.0 {
            return 0;
        }
        if lower_pct > 5.0 {
            return 0;
        }
        if upper_pct < 60.0 {
            return 0;
        }
        -100
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 0, detect);
    let b = sorted[n - 1];
    let (_, range, upper, lower, body_pct, _) = candle_metrics(b);
    let (upper_pct, lower_pct) = if range > 1e-12 {
        (100.0 * upper / range, 100.0 * lower / range)
    } else {
        (0.0, 0.0)
    };
    let label = if last_match {
        "BEARISH_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlGravestoneDojiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        body_pct_range: body_pct,
        upper_shadow_pct: upper_pct,
        lower_shadow_pct: lower_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b.close,
        cdl_gravestone_doji_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_hanging_man_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlHangingManSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 2 {
        return CdlHangingManSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_hanging_man_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥2 bars, got {}", n),
            ..Default::default()
        };
    }
    // Hanging Man (geometrically identical to Hammer): small body in
    // upper third, lower shadow ≥ 2 × body, upper shadow ≤ body.
    // Emits -100 (bearish at tops) per TA-Lib convention.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        let b = s[i];
        let (body, _, upper, lower, body_pct, _) = candle_metrics(b);
        if body < 1e-12 {
            return 0;
        }
        if body_pct > 30.0 {
            return 0;
        }
        if lower < 2.0 * body {
            return 0;
        }
        if upper > body {
            return 0;
        }
        -100
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 0, detect);
    let b = sorted[n - 1];
    let (_, range, upper, lower, body_pct, _) = candle_metrics(b);
    let (upper_pct, lower_pct) = if range > 1e-12 {
        (100.0 * upper / range, 100.0 * lower / range)
    } else {
        (0.0, 0.0)
    };
    let label = if last_match {
        "BEARISH_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlHangingManSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        body_pct_range: body_pct,
        upper_shadow_pct: upper_pct,
        lower_shadow_pct: lower_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b.close,
        cdl_hanging_man_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_inverted_hammer_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlInvertedHammerSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 2 {
        return CdlInvertedHammerSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_inverted_hammer_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥2 bars, got {}", n),
            ..Default::default()
        };
    }
    // Inverted Hammer (geometrically identical to Shooting Star): small
    // body in lower third, upper shadow ≥ 2 × body, lower shadow ≤ body.
    // Emits +100 (bullish at bottoms) per TA-Lib convention.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        let b = s[i];
        let (body, _, upper, lower, body_pct, _) = candle_metrics(b);
        if body < 1e-12 {
            return 0;
        }
        if body_pct > 30.0 {
            return 0;
        }
        if upper < 2.0 * body {
            return 0;
        }
        if lower > body {
            return 0;
        }
        100
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 0, detect);
    let b = sorted[n - 1];
    let (_, range, upper, lower, body_pct, _) = candle_metrics(b);
    let (upper_pct, lower_pct) = if range > 1e-12 {
        (100.0 * upper / range, 100.0 * lower / range)
    } else {
        (0.0, 0.0)
    };
    let label = if last_match {
        "BULLISH_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlInvertedHammerSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        body_pct_range: body_pct,
        upper_shadow_pct: upper_pct,
        lower_shadow_pct: lower_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b.close,
        cdl_inverted_hammer_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_harami_cross_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlHaramiCrossSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 3 {
        return CdlHaramiCrossSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_harami_cross_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 bars, got {}", n),
            ..Default::default()
        };
    }
    // Harami Cross = Harami where inside bar is a doji (body ≤ 5% of range).
    // Prior bar large body, current body contained in prior body AND current
    // body ≤ 5% of range. Bullish when prior red, bearish when prior green.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        let prior = s[i - 1];
        let cur = s[i];
        let (p_body, _, _, _, p_body_pct, p_is_bull) = candle_metrics(prior);
        let (_, _, _, _, c_body_pct, _) = candle_metrics(cur);
        if p_body < 1e-12 || p_body_pct < 30.0 {
            return 0;
        }
        if c_body_pct > 5.0 {
            return 0;
        } // current must be doji
        let p_low_body = prior.open.min(prior.close);
        let p_high_body = prior.open.max(prior.close);
        let c_low_body = cur.open.min(cur.close);
        let c_high_body = cur.open.max(cur.close);
        if !(c_low_body >= p_low_body && c_high_body <= p_high_body) {
            return 0;
        }
        if p_is_bull { -100 } else { 100 }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 1, detect);
    let prior = sorted[n - 2];
    let cur = sorted[n - 1];
    let (p_body, _, _, _, p_body_pct, _) = candle_metrics(prior);
    let (c_body, _, _, _, c_body_pct, _) = candle_metrics(cur);
    let ratio = if p_body > 1e-12 { c_body / p_body } else { 0.0 };
    let label = match last_val {
        100 => "BULLISH_PATTERN",
        -100 => "BEARISH_PATTERN",
        _ => "NO_PATTERN",
    };
    CdlHaramiCrossSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        prior_body_pct_range: p_body_pct,
        current_body_pct_range: c_body_pct,
        body_size_ratio: ratio,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: cur.close,
        cdl_harami_cross_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_long_legged_doji_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlLongLeggedDojiSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 2 {
        return CdlLongLeggedDojiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_long_legged_doji_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥2 bars, got {}", n),
            ..Default::default()
        };
    }
    // Long-legged doji: body ≤ 5% of range, BOTH shadows ≥ 30% of range.
    // TA-Lib emits +100 for pattern present (treated as neutral indecision).
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        let b = s[i];
        let (_, range, upper, lower, body_pct, _) = candle_metrics(b);
        if range < 1e-12 {
            return 0;
        }
        if body_pct > 5.0 {
            return 0;
        }
        let upper_pct = 100.0 * upper / range;
        let lower_pct = 100.0 * lower / range;
        if upper_pct < 30.0 || lower_pct < 30.0 {
            return 0;
        }
        100
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 0, detect);
    let b = sorted[n - 1];
    let (_, range, upper, lower, body_pct, _) = candle_metrics(b);
    let (upper_pct, lower_pct) = if range > 1e-12 {
        (100.0 * upper / range, 100.0 * lower / range)
    } else {
        (0.0, 0.0)
    };
    let label = if last_match {
        "DOJI_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlLongLeggedDojiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        body_pct_range: body_pct,
        upper_shadow_pct: upper_pct,
        lower_shadow_pct: lower_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b.close,
        cdl_long_legged_doji_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_marubozu_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlMarubozuSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 2 {
        return CdlMarubozuSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_marubozu_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥2 bars, got {}", n),
            ..Default::default()
        };
    }
    // Marubozu: body ≥ 90% of range, BOTH shadows ≤ 5%. Bullish when green
    // (open ≈ low, close ≈ high), bearish when red (open ≈ high, close ≈ low).
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        let b = s[i];
        let (_, range, upper, lower, body_pct, is_bull) = candle_metrics(b);
        if range < 1e-12 {
            return 0;
        }
        if body_pct < 90.0 {
            return 0;
        }
        let upper_pct = 100.0 * upper / range;
        let lower_pct = 100.0 * lower / range;
        if upper_pct > 5.0 || lower_pct > 5.0 {
            return 0;
        }
        if is_bull { 100 } else { -100 }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 0, detect);
    let b = sorted[n - 1];
    let (_, range, upper, lower, body_pct, _) = candle_metrics(b);
    let (upper_pct, lower_pct) = if range > 1e-12 {
        (100.0 * upper / range, 100.0 * lower / range)
    } else {
        (0.0, 0.0)
    };
    let label = match last_val {
        100 => "BULLISH_PATTERN",
        -100 => "BEARISH_PATTERN",
        _ => "NO_PATTERN",
    };
    CdlMarubozuSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        body_pct_range: body_pct,
        upper_shadow_pct: upper_pct,
        lower_shadow_pct: lower_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b.close,
        cdl_marubozu_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_spinning_top_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlSpinningTopSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 2 {
        return CdlSpinningTopSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_spinning_top_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥2 bars, got {}", n),
            ..Default::default()
        };
    }
    // Spinning Top: small body (≤ 30% of range), BOTH shadows > body.
    // TA-Lib sign convention: +100 (green body) or -100 (red body).
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        let b = s[i];
        let (body, _, upper, lower, body_pct, is_bull) = candle_metrics(b);
        if body < 1e-12 {
            return 0;
        }
        if body_pct > 30.0 {
            return 0;
        }
        if upper <= body || lower <= body {
            return 0;
        }
        if is_bull { 100 } else { -100 }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 0, detect);
    let b = sorted[n - 1];
    let (_, range, upper, lower, body_pct, _) = candle_metrics(b);
    let (upper_pct, lower_pct) = if range > 1e-12 {
        (100.0 * upper / range, 100.0 * lower / range)
    } else {
        (0.0, 0.0)
    };
    let label = match last_val {
        100 => "GREEN_BODY_PATTERN",
        -100 => "RED_BODY_PATTERN",
        _ => "NO_PATTERN",
    };
    CdlSpinningTopSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        body_pct_range: body_pct,
        upper_shadow_pct: upper_pct,
        lower_shadow_pct: lower_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b.close,
        cdl_spinning_top_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_tristar_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlTristarSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 4 {
        return CdlTristarSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_tristar_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥4 bars, got {}", n),
            ..Default::default()
        };
    }
    // Tri-Star: three consecutive doji bars. Bullish when middle doji gaps
    // below the outer two (reversal at bottom); bearish when middle gaps
    // above (reversal at top).
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        let b0 = s[i - 2];
        let b1 = s[i - 1];
        let b2 = s[i];
        let (_, _, _, _, p0, _) = candle_metrics(b0);
        let (_, _, _, _, p1, _) = candle_metrics(b1);
        let (_, _, _, _, p2, _) = candle_metrics(b2);
        if p0 > 5.0 || p1 > 5.0 || p2 > 5.0 {
            return 0;
        }
        let mid0 = 0.5 * (b0.open + b0.close);
        let mid1 = 0.5 * (b1.open + b1.close);
        let mid2 = 0.5 * (b2.open + b2.close);
        // Bullish tri-star: middle gaps below, third closes back up
        if mid1 < mid0 && mid1 < mid2 && b1.high < b0.low.min(b2.low) {
            return 100;
        }
        // Bearish tri-star: middle gaps above, third closes back down
        if mid1 > mid0 && mid1 > mid2 && b1.low > b0.high.max(b2.high) {
            return -100;
        }
        0
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 2, detect);
    let b2 = sorted[n - 1];
    let b1 = sorted[n - 2];
    let b0 = sorted[n - 3];
    let (_, _, _, _, p0, _) = candle_metrics(b0);
    let (_, _, _, _, p1, _) = candle_metrics(b1);
    let (_, _, _, _, p2, _) = candle_metrics(b2);
    let avg_body = (p0 + p1 + p2) / 3.0;
    let mid0 = 0.5 * (b0.open + b0.close);
    let mid1 = 0.5 * (b1.open + b1.close);
    let mid2 = 0.5 * (b2.open + b2.close);
    let outer_avg = 0.5 * (mid0 + mid2);
    let gap_pct = if outer_avg.abs() > 1e-12 {
        100.0 * (mid1 - outer_avg) / outer_avg
    } else {
        0.0
    };
    let label = match last_val {
        100 => "BULLISH_PATTERN",
        -100 => "BEARISH_PATTERN",
        _ => "NO_PATTERN",
    };
    CdlTristarSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        avg_body_pct_range: avg_body,
        middle_gap_pct: gap_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b2.close,
        cdl_tristar_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_doji_star_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlDojiStarSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 3 {
        return CdlDojiStarSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_doji_star_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 bars, got {}", n),
            ..Default::default()
        };
    }
    // Doji Star: prior bar with real body (≥ 30% of range); current is a
    // doji (body ≤ 5% of range) whose entire body gaps away from prior
    // close. Bearish (-100) when prior green + gap up; bullish (+100) when
    // prior red + gap down.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        let prior = s[i - 1];
        let cur = s[i];
        let (p_body, _, _, _, p_body_pct, p_is_bull) = candle_metrics(prior);
        let (_, _, _, _, c_body_pct, _) = candle_metrics(cur);
        if p_body < 1e-12 || p_body_pct < 30.0 {
            return 0;
        }
        if c_body_pct > 5.0 {
            return 0;
        } // current must be doji
        let c_low_body = cur.open.min(cur.close);
        let c_high_body = cur.open.max(cur.close);
        if p_is_bull {
            // prior green → bearish top star: doji body above prior close
            if c_low_body > prior.close { -100 } else { 0 }
        } else {
            // prior red → bullish bottom star: doji body below prior close
            if c_high_body < prior.close { 100 } else { 0 }
        }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 1, detect);
    let prior = sorted[n - 2];
    let cur = sorted[n - 1];
    let (_, _, _, _, p_body_pct, p_is_bull) = candle_metrics(prior);
    let (_, _, _, _, c_body_pct, _) = candle_metrics(cur);
    let c_low_body = cur.open.min(cur.close);
    let c_high_body = cur.open.max(cur.close);
    // signed gap: positive = gap up (bearish), negative = gap down (bullish)
    let gap_pct = if prior.close.abs() > 1e-12 {
        if p_is_bull {
            100.0 * (c_low_body - prior.close) / prior.close
        } else {
            100.0 * (c_high_body - prior.close) / prior.close
        }
    } else {
        0.0
    };
    let label = match last_val {
        100 => "BULLISH_PATTERN",
        -100 => "BEARISH_PATTERN",
        _ => "NO_PATTERN",
    };
    CdlDojiStarSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        prior_body_pct_range: p_body_pct,
        current_body_pct_range: c_body_pct,
        gap_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: cur.close,
        cdl_doji_star_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_morning_doji_star_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlMorningDojiStarSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 4 {
        return CdlMorningDojiStarSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_morning_doji_star_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥4 bars, got {}", n),
            ..Default::default()
        };
    }
    // Morning Doji Star (3-bar bullish): bar 1 long red, bar 2 doji
    // gapping below bar 1 close, bar 3 green closing above bar 1 midpoint.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        if i < 2 {
            return 0;
        }
        let b0 = s[i - 2];
        let b1 = s[i - 1];
        let b2 = s[i];
        let (body0, _, _, _, body0_pct, bull0) = candle_metrics(b0);
        let (_, _, _, _, body1_pct, _) = candle_metrics(b1);
        let (body2, _, _, _, _, bull2) = candle_metrics(b2);
        if body0 < 1e-12 || body2 < 1e-12 {
            return 0;
        }
        if bull0 {
            return 0;
        } // bar 1 must be red
        if body0_pct < 30.0 {
            return 0;
        } // bar 1 long body
        if body1_pct > 5.0 {
            return 0;
        } // bar 2 doji
        if !bull2 {
            return 0;
        } // bar 3 must be green
        // bar 2 body gaps below bar 1 close
        let b1_high_body = b1.open.max(b1.close);
        if b1_high_body >= b0.close {
            return 0;
        }
        let midpoint_b0 = (b0.open + b0.close) / 2.0;
        if b2.close > midpoint_b0 { 100 } else { 0 }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 2, detect);
    let b0 = sorted[n - 3];
    let b1 = sorted[n - 2];
    let b2 = sorted[n - 1];
    let (_, _, _, _, body0_pct, _) = candle_metrics(b0);
    let (_, _, _, _, body1_pct, _) = candle_metrics(b1);
    let mid0 = (b0.open + b0.close) / 2.0;
    let pct_above = if mid0.abs() > 1e-12 {
        100.0 * (b2.close - mid0) / mid0
    } else {
        0.0
    };
    let label = if last_match {
        "BULLISH_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlMorningDojiStarSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        bar1_body_pct_range: body0_pct,
        bar2_body_pct_range: body1_pct,
        bar3_close_vs_bar1_mid_pct: pct_above,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b2.close,
        cdl_morning_doji_star_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_evening_doji_star_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlEveningDojiStarSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 4 {
        return CdlEveningDojiStarSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_evening_doji_star_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥4 bars, got {}", n),
            ..Default::default()
        };
    }
    // Evening Doji Star (3-bar bearish): bar 1 long green, bar 2 doji
    // gapping above bar 1 close, bar 3 red closing below bar 1 midpoint.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        if i < 2 {
            return 0;
        }
        let b0 = s[i - 2];
        let b1 = s[i - 1];
        let b2 = s[i];
        let (body0, _, _, _, body0_pct, bull0) = candle_metrics(b0);
        let (_, _, _, _, body1_pct, _) = candle_metrics(b1);
        let (body2, _, _, _, _, bull2) = candle_metrics(b2);
        if body0 < 1e-12 || body2 < 1e-12 {
            return 0;
        }
        if !bull0 {
            return 0;
        } // bar 1 must be green
        if body0_pct < 30.0 {
            return 0;
        } // bar 1 long body
        if body1_pct > 5.0 {
            return 0;
        } // bar 2 doji
        if bull2 {
            return 0;
        } // bar 3 must be red
        // bar 2 body gaps above bar 1 close
        let b1_low_body = b1.open.min(b1.close);
        if b1_low_body <= b0.close {
            return 0;
        }
        let midpoint_b0 = (b0.open + b0.close) / 2.0;
        if b2.close < midpoint_b0 { -100 } else { 0 }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 2, detect);
    let b0 = sorted[n - 3];
    let b1 = sorted[n - 2];
    let b2 = sorted[n - 1];
    let (_, _, _, _, body0_pct, _) = candle_metrics(b0);
    let (_, _, _, _, body1_pct, _) = candle_metrics(b1);
    let mid0 = (b0.open + b0.close) / 2.0;
    let pct_below = if mid0.abs() > 1e-12 {
        100.0 * (b2.close - mid0) / mid0
    } else {
        0.0
    };
    let label = if last_match {
        "BEARISH_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlEveningDojiStarSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        bar1_body_pct_range: body0_pct,
        bar2_body_pct_range: body1_pct,
        bar3_close_vs_bar1_mid_pct: pct_below,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b2.close,
        cdl_evening_doji_star_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_abandoned_baby_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlAbandonedBabySnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 4 {
        return CdlAbandonedBabySnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_abandoned_baby_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥4 bars, got {}", n),
            ..Default::default()
        };
    }
    // Abandoned Baby: doji isolated by full-shadow gaps on BOTH sides.
    // Bullish: bar 1 long red, bar 2 doji with bar2.high < bar1.low,
    // bar 3 green with bar3.low > bar2.high.
    // Bearish: bar 1 long green, bar 2 doji with bar2.low > bar1.high,
    // bar 3 red with bar3.high < bar2.low.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        if i < 2 {
            return 0;
        }
        let b0 = s[i - 2];
        let b1 = s[i - 1];
        let b2 = s[i];
        let (body0, _, _, _, body0_pct, bull0) = candle_metrics(b0);
        let (_, _, _, _, body1_pct, _) = candle_metrics(b1);
        let (body2, _, _, _, _, bull2) = candle_metrics(b2);
        if body0 < 1e-12 || body2 < 1e-12 {
            return 0;
        }
        if body0_pct < 30.0 {
            return 0;
        } // bar 1 long body
        if body1_pct > 5.0 {
            return 0;
        } // bar 2 doji
        if !bull0 && bull2 {
            // bullish path: bar 1 red, bar 3 green, with full-shadow gaps
            if b1.high < b0.low && b2.low > b1.high {
                return 100;
            }
        } else if bull0 && !bull2 {
            // bearish path: bar 1 green, bar 3 red, with full-shadow gaps
            if b1.low > b0.high && b2.high < b1.low {
                return -100;
            }
        }
        0
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 2, detect);
    let b0 = sorted[n - 3];
    let b1 = sorted[n - 2];
    let b2 = sorted[n - 1];
    let (_, _, _, _, body0_pct, _) = candle_metrics(b0);
    let (_, _, _, _, body1_pct, _) = candle_metrics(b1);
    // Report signed gap magnitudes as computed on the last triplet.
    let (gap_down_pct, gap_up_pct) = match last_val {
        100 => {
            let gd = if b0.low.abs() > 1e-12 {
                100.0 * (b0.low - b1.high) / b0.low
            } else {
                0.0
            };
            let gu = if b1.high.abs() > 1e-12 {
                100.0 * (b2.low - b1.high) / b1.high
            } else {
                0.0
            };
            (gd, gu)
        }
        -100 => {
            let gd = if b0.high.abs() > 1e-12 {
                100.0 * (b0.high - b1.low) / b0.high
            } else {
                0.0
            };
            let gu = if b1.low.abs() > 1e-12 {
                100.0 * (b2.high - b1.low) / b1.low
            } else {
                0.0
            };
            (gd, gu)
        }
        _ => (0.0, 0.0),
    };
    let label = match last_val {
        100 => "BULLISH_PATTERN",
        -100 => "BEARISH_PATTERN",
        _ => "NO_PATTERN",
    };
    CdlAbandonedBabySnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        bar1_body_pct_range: body0_pct,
        bar2_body_pct_range: body1_pct,
        gap_down_pct,
        gap_up_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b2.close,
        cdl_abandoned_baby_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_three_inside_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlThreeInsideSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 4 {
        return CdlThreeInsideSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_three_inside_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥4 bars, got {}", n),
            ..Default::default()
        };
    }
    // Three Inside Up/Down: Harami in bars 1-2, confirmation close in bar 3.
    // Bullish: bar 1 red long, bar 2 small green contained inside bar 1 body,
    //   bar 3 closes above bar 1 open.
    // Bearish: bar 1 green long, bar 2 small red contained inside bar 1 body,
    //   bar 3 closes below bar 1 open.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        if i < 2 {
            return 0;
        }
        let b0 = s[i - 2];
        let b1 = s[i - 1];
        let b2 = s[i];
        let (body0, _, _, _, body0_pct, bull0) = candle_metrics(b0);
        let (_, _, _, _, _, bull1) = candle_metrics(b1);
        if body0 < 1e-12 || body0_pct < 30.0 {
            return 0;
        }
        if bull0 == bull1 {
            return 0;
        } // opposite colours for Harami
        let b0_low_body = b0.open.min(b0.close);
        let b0_high_body = b0.open.max(b0.close);
        let b1_low_body = b1.open.min(b1.close);
        let b1_high_body = b1.open.max(b1.close);
        if !(b1_low_body >= b0_low_body && b1_high_body <= b0_high_body) {
            return 0;
        }
        if !bull0 {
            // bullish: bar 1 red, bar 3 closes above bar 1 open (above body top)
            if b2.close > b0.open {
                return 100;
            }
        } else {
            // bearish: bar 1 green, bar 3 closes below bar 1 open (below body bottom)
            if b2.close < b0.open {
                return -100;
            }
        }
        0
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 2, detect);
    let b0 = sorted[n - 3];
    let b1 = sorted[n - 2];
    let b2 = sorted[n - 1];
    let (body0, _, _, _, body0_pct, _) = candle_metrics(b0);
    let (body1, _, _, _, _, _) = candle_metrics(b1);
    let ratio = if body0 > 1e-12 { body1 / body0 } else { 0.0 };
    let pct_from_b0_open = if b0.open.abs() > 1e-12 {
        100.0 * (b2.close - b0.open) / b0.open
    } else {
        0.0
    };
    let label = match last_val {
        100 => "BULLISH_PATTERN",
        -100 => "BEARISH_PATTERN",
        _ => "NO_PATTERN",
    };
    CdlThreeInsideSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        bar1_body_pct_range: body0_pct,
        body_size_ratio: ratio,
        bar3_close_vs_bar1_open_pct: pct_from_b0_open,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b2.close,
        cdl_three_inside_label: label.into(),
        note: String::new(),
    }
}
