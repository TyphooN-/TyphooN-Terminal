use super::*;

// Counterattack, homing-pigeon, neck-line, and thrusting candlestick patterns

pub fn compute_cdl_counterattack_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlCounterattackSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 3 {
        return CdlCounterattackSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_counterattack_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 bars, got {}", n),
            ..Default::default()
        };
    }
    // Counterattack: opposite-colour long bodies with a directional gap and a
    // close back at/near the prior close.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
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
        let tol = 0.10 * body0;
        if !bull0 && bull1 {
            if b1.open >= b0.low {
                return 0;
            }
            if (b1.close - b0.close).abs() > tol {
                return 0;
            }
            100
        } else if bull0 && !bull1 {
            if b1.open <= b0.high {
                return 0;
            }
            if (b1.close - b0.close).abs() > tol {
                return 0;
            }
            -100
        } else {
            0
        }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 1, detect);
    let b0 = sorted[n - 2];
    let b1 = sorted[n - 1];
    let (body0, _, _, _, body0_pct, bull0) = candle_metrics(b0);
    let (_, _, _, _, body1_pct, _) = candle_metrics(b1);
    let gap_open_pct = match last_val {
        100 if b0.low.abs() > 1e-12 => 100.0 * (b0.low - b1.open) / b0.low,
        -100 if b0.high.abs() > 1e-12 => 100.0 * (b1.open - b0.high) / b0.high,
        _ => 0.0,
    };
    let close_diff_pct_body = if body0 > 1e-12 {
        100.0 * (b1.close - b0.close).abs() / body0
    } else {
        0.0
    };
    let label = match last_val {
        100 => "BULLISH_PATTERN",
        -100 => "BEARISH_PATTERN",
        _ => "NO_PATTERN",
    };
    let _ = bull0;
    CdlCounterattackSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        prior_body_pct_range: body0_pct,
        current_body_pct_range: body1_pct,
        gap_open_pct,
        close_diff_pct_body,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b1.close,
        cdl_counterattack_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_homing_pigeon_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlHomingPigeonSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 3 {
        return CdlHomingPigeonSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_homing_pigeon_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 bars, got {}", n),
            ..Default::default()
        };
    }
    // Homing Pigeon: bearish harami variant with both bars red.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        let b0 = s[i - 1];
        let b1 = s[i];
        let (body0, _, _, _, body0_pct, bull0) = candle_metrics(b0);
        let (body1, _, _, _, _, bull1) = candle_metrics(b1);
        if body0 < 1e-12 || body1 < 1e-12 {
            return 0;
        }
        if bull0 || bull1 {
            return 0;
        }
        if body0_pct < 30.0 {
            return 0;
        }
        let b0_low_body = b0.open.min(b0.close);
        let b0_high_body = b0.open.max(b0.close);
        let b1_low_body = b1.open.min(b1.close);
        let b1_high_body = b1.open.max(b1.close);
        if body1 >= body0 {
            return 0;
        }
        if !(b1_low_body >= b0_low_body && b1_high_body <= b0_high_body) {
            return 0;
        }
        100
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 1, detect);
    let b0 = sorted[n - 2];
    let b1 = sorted[n - 1];
    let (body0, _, _, _, body0_pct, _) = candle_metrics(b0);
    let (body1, _, _, _, body1_pct, _) = candle_metrics(b1);
    let b0_low_body = b0.open.min(b0.close);
    let b0_high_body = b0.open.max(b0.close);
    let b1_low_body = b1.open.min(b1.close);
    let b1_high_body = b1.open.max(b1.close);
    let body_size_ratio = if body0 > 1e-12 { body1 / body0 } else { 0.0 };
    let inner_body_margin_pct = if body0 > 1e-12 {
        let lower_margin = b1_low_body - b0_low_body;
        let upper_margin = b0_high_body - b1_high_body;
        100.0 * lower_margin.min(upper_margin) / body0
    } else {
        0.0
    };
    let label = if last_val == 100 {
        "BULLISH_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlHomingPigeonSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        prior_body_pct_range: body0_pct,
        current_body_pct_range: body1_pct,
        body_size_ratio,
        inner_body_margin_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b1.close,
        cdl_homing_pigeon_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_in_neck_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlInNeckSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 3 {
        return CdlInNeckSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_in_neck_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 bars, got {}", n),
            ..Default::default()
        };
    }
    // In-Neck: long red bar, gap-down green bar, close slightly into the
    // prior body but still below the midpoint.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        let b0 = s[i - 1];
        let b1 = s[i];
        let (body0, _, _, _, body0_pct, bull0) = candle_metrics(b0);
        let (_, _, _, _, _, bull1) = candle_metrics(b1);
        if body0 < 1e-12 {
            return 0;
        }
        if bull0 || !bull1 {
            return 0;
        }
        if body0_pct < 30.0 {
            return 0;
        }
        if b1.open >= b0.low {
            return 0;
        }
        let pen = 100.0 * (b1.close - b0.close) / body0;
        let midpoint0 = (b0.open + b0.close) / 2.0;
        if pen <= 5.0 || pen > 25.0 {
            return 0;
        }
        if b1.close >= midpoint0 {
            return 0;
        }
        -100
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 1, detect);
    let b0 = sorted[n - 2];
    let b1 = sorted[n - 1];
    let (body0, _, _, _, body0_pct, _) = candle_metrics(b0);
    let (_, _, _, _, body1_pct, _) = candle_metrics(b1);
    let gap_open_pct = if b0.low.abs() > 1e-12 {
        100.0 * (b0.low - b1.open) / b0.low
    } else {
        0.0
    };
    let penetration_pct = if body0 > 1e-12 {
        100.0 * (b1.close - b0.close) / body0
    } else {
        0.0
    };
    let label = if last_val == -100 {
        "BEARISH_CONTINUATION"
    } else {
        "NO_PATTERN"
    };
    CdlInNeckSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        prior_body_pct_range: body0_pct,
        current_body_pct_range: body1_pct,
        gap_open_pct,
        penetration_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b1.close,
        cdl_in_neck_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_on_neck_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlOnNeckSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 3 {
        return CdlOnNeckSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_on_neck_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 bars, got {}", n),
            ..Default::default()
        };
    }
    // On-Neck: long red bar, gap-down green bar, close back at/near prior close.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        let b0 = s[i - 1];
        let b1 = s[i];
        let (body0, _, _, _, body0_pct, bull0) = candle_metrics(b0);
        if body0 < 1e-12 {
            return 0;
        }
        if bull0 {
            return 0;
        }
        if body0_pct < 30.0 {
            return 0;
        }
        let (_, _, _, _, _, bull1) = candle_metrics(b1);
        if !bull1 {
            return 0;
        }
        if b1.open >= b0.low {
            return 0;
        }
        let diff_pct = 100.0 * (b1.close - b0.close).abs() / body0;
        if diff_pct <= 5.0 { -100 } else { 0 }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 1, detect);
    let b0 = sorted[n - 2];
    let b1 = sorted[n - 1];
    let (body0, _, _, _, body0_pct, _) = candle_metrics(b0);
    let (_, _, _, _, body1_pct, _) = candle_metrics(b1);
    let gap_open_pct = if b0.low.abs() > 1e-12 {
        100.0 * (b0.low - b1.open) / b0.low
    } else {
        0.0
    };
    let close_match_pct = if body0 > 1e-12 {
        100.0 * (b1.close - b0.close).abs() / body0
    } else {
        0.0
    };
    let label = if last_val == -100 {
        "BEARISH_CONTINUATION"
    } else {
        "NO_PATTERN"
    };
    CdlOnNeckSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        prior_body_pct_range: body0_pct,
        current_body_pct_range: body1_pct,
        gap_open_pct,
        close_match_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b1.close,
        cdl_on_neck_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_thrusting_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlThrustingSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 3 {
        return CdlThrustingSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_thrusting_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 bars, got {}", n),
            ..Default::default()
        };
    }
    // Thrusting: same setup as In-Neck, but deeper penetration into the
    // prior body while still staying below the midpoint.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        let b0 = s[i - 1];
        let b1 = s[i];
        let (body0, _, _, _, body0_pct, bull0) = candle_metrics(b0);
        let (_, _, _, _, _, bull1) = candle_metrics(b1);
        if body0 < 1e-12 {
            return 0;
        }
        if bull0 || !bull1 {
            return 0;
        }
        if body0_pct < 30.0 {
            return 0;
        }
        if b1.open >= b0.low {
            return 0;
        }
        let pen = 100.0 * (b1.close - b0.close) / body0;
        let midpoint0 = (b0.open + b0.close) / 2.0;
        if pen <= 25.0 || pen >= 50.0 {
            return 0;
        }
        if b1.close >= midpoint0 {
            return 0;
        }
        -100
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 1, detect);
    let b0 = sorted[n - 2];
    let b1 = sorted[n - 1];
    let (body0, _, _, _, body0_pct, _) = candle_metrics(b0);
    let (_, _, _, _, body1_pct, _) = candle_metrics(b1);
    let gap_open_pct = if b0.low.abs() > 1e-12 {
        100.0 * (b0.low - b1.open) / b0.low
    } else {
        0.0
    };
    let penetration_pct = if body0 > 1e-12 {
        100.0 * (b1.close - b0.close) / body0
    } else {
        0.0
    };
    let label = if last_val == -100 {
        "BEARISH_CONTINUATION"
    } else {
        "NO_PATTERN"
    };
    CdlThrustingSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        prior_body_pct_range: body0_pct,
        current_body_pct_range: body1_pct,
        gap_open_pct,
        penetration_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b1.close,
        cdl_thrusting_label: label.into(),
        note: String::new(),
    }
}
