use super::*;

// Multi-bar star, crow, soldier, and cloud-cover candlestick patterns

pub fn compute_cdl_morning_star_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlMorningStarSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 4 {
        return CdlMorningStarSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_morning_star_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥4 bars, got {}", n),
            ..Default::default()
        };
    }
    // Morning Star (3-bar bullish reversal):
    //   bar 0: large red body (close < open, body ≥ 30% of range)
    //   bar 1: small body (body ≤ 30% of range) — the "star"
    //   bar 2: large green body (close > open, body ≥ 30% of range),
    //          close > midpoint(bar 0 open, bar 0 close).
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        if i < 2 {
            return 0;
        }
        let b0 = s[i - 2];
        let b1 = s[i - 1];
        let b2 = s[i];
        let (body0, _r0, _u0, _l0, body0_pct, bull0) = candle_metrics(b0);
        let (_body1, _r1, _u1, _l1, body1_pct, _bull1) = candle_metrics(b1);
        let (body2, _r2, _u2, _l2, body2_pct, bull2) = candle_metrics(b2);
        if body0 < 1e-12 || body2 < 1e-12 {
            return 0;
        }
        if bull0 {
            return 0;
        } // bar 0 must be red
        if !bull2 {
            return 0;
        } // bar 2 must be green
        if body0_pct < 30.0 || body2_pct < 30.0 {
            return 0;
        }
        if body1_pct > 30.0 {
            return 0;
        } // bar 1 is a small star
        let midpoint_b0 = (b0.open + b0.close) / 2.0;
        if b2.close > midpoint_b0 { 100 } else { 0 }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 2, detect);
    let b0 = sorted[n - 3];
    let b1 = sorted[n - 2];
    let b2 = sorted[n - 1];
    let (body0, _, _, _, body0_pct, _) = candle_metrics(b0);
    let (_, _, _, _, body1_pct, _) = candle_metrics(b1);
    let (_, _, _, _, body2_pct, _) = candle_metrics(b2);
    let mid0 = (b0.open + b0.close) / 2.0;
    let pen = if body0 > 1e-12 {
        100.0 * (b2.close - mid0) / body0
    } else {
        0.0
    };
    let label = if last_match {
        "BULLISH_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlMorningStarSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        penetration_pct: pen,
        star_body_pct_range: body1_pct,
        first_body_pct_range: body0_pct,
        last_body_pct_range: body2_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b2.close,
        cdl_morning_star_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_evening_star_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlEveningStarSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 4 {
        return CdlEveningStarSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_evening_star_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥4 bars, got {}", n),
            ..Default::default()
        };
    }
    // Evening Star (3-bar bearish reversal):
    //   bar 0: large green body, bar 1: small body, bar 2: large red body,
    //   bar-2 close < midpoint(bar 0 open, bar 0 close).
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        if i < 2 {
            return 0;
        }
        let b0 = s[i - 2];
        let b1 = s[i - 1];
        let b2 = s[i];
        let (body0, _, _, _, body0_pct, bull0) = candle_metrics(b0);
        let (_, _, _, _, body1_pct, _) = candle_metrics(b1);
        let (body2, _, _, _, body2_pct, bull2) = candle_metrics(b2);
        if body0 < 1e-12 || body2 < 1e-12 {
            return 0;
        }
        if !bull0 {
            return 0;
        } // bar 0 must be green
        if bull2 {
            return 0;
        } // bar 2 must be red
        if body0_pct < 30.0 || body2_pct < 30.0 {
            return 0;
        }
        if body1_pct > 30.0 {
            return 0;
        }
        let midpoint_b0 = (b0.open + b0.close) / 2.0;
        if b2.close < midpoint_b0 { -100 } else { 0 }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 2, detect);
    let b0 = sorted[n - 3];
    let b1 = sorted[n - 2];
    let b2 = sorted[n - 1];
    let (body0, _, _, _, body0_pct, _) = candle_metrics(b0);
    let (_, _, _, _, body1_pct, _) = candle_metrics(b1);
    let (_, _, _, _, body2_pct, _) = candle_metrics(b2);
    let mid0 = (b0.open + b0.close) / 2.0;
    let pen = if body0 > 1e-12 {
        100.0 * (mid0 - b2.close) / body0
    } else {
        0.0
    };
    let label = if last_match {
        "BEARISH_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlEveningStarSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        penetration_pct: pen,
        star_body_pct_range: body1_pct,
        first_body_pct_range: body0_pct,
        last_body_pct_range: body2_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b2.close,
        cdl_evening_star_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_three_black_crows_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlThreeBlackCrowsSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 4 {
        return CdlThreeBlackCrowsSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_three_black_crows_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥4 bars, got {}", n),
            ..Default::default()
        };
    }
    // Three Black Crows: three consecutive red bars, each opens within
    // prior body, closes below prior close, body ≥ 30% of range.
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
        } // all must be red
        if body0_pct < 30.0 || body1_pct < 30.0 || body2_pct < 30.0 {
            return 0;
        }
        // Each bar opens within prior body: prior_close ≤ open ≤ prior_open (red prior).
        if !(b1.open <= b0.open && b1.open >= b0.close) {
            return 0;
        }
        if !(b2.open <= b1.open && b2.open >= b1.close) {
            return 0;
        }
        // Each bar closes below prior close.
        if b1.close >= b0.close || b2.close >= b1.close {
            return 0;
        }
        -100
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 2, detect);
    let b0 = sorted[n - 3];
    let b1 = sorted[n - 2];
    let b2 = sorted[n - 1];
    let (_, _, _, _, body0_pct, _) = candle_metrics(b0);
    let (_, _, _, _, body1_pct, _) = candle_metrics(b1);
    let (_, _, _, _, body2_pct, _) = candle_metrics(b2);
    let avg_body_pct = (body0_pct + body1_pct + body2_pct) / 3.0;
    let total_decl = if b0.open.abs() > 1e-12 {
        100.0 * (b2.close - b0.open) / b0.open
    } else {
        0.0
    };
    let label = if last_match {
        "BEARISH_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlThreeBlackCrowsSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        avg_body_pct_range: avg_body_pct,
        total_close_decline_pct: total_decl,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b2.close,
        cdl_three_black_crows_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_three_white_soldiers_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlThreeWhiteSoldiersSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 4 {
        return CdlThreeWhiteSoldiersSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_three_white_soldiers_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥4 bars, got {}", n),
            ..Default::default()
        };
    }
    // Three White Soldiers: three consecutive green bars, each opens within
    // prior body, closes above prior close, body ≥ 30% of range.
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
        if !bull0 || !bull1 || !bull2 {
            return 0;
        }
        if body0_pct < 30.0 || body1_pct < 30.0 || body2_pct < 30.0 {
            return 0;
        }
        // Each bar opens within prior body: prior_open ≤ open ≤ prior_close (green prior).
        if !(b1.open >= b0.open && b1.open <= b0.close) {
            return 0;
        }
        if !(b2.open >= b1.open && b2.open <= b1.close) {
            return 0;
        }
        // Each bar closes above prior close.
        if b1.close <= b0.close || b2.close <= b1.close {
            return 0;
        }
        100
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 2, detect);
    let b0 = sorted[n - 3];
    let b1 = sorted[n - 2];
    let b2 = sorted[n - 1];
    let (_, _, _, _, body0_pct, _) = candle_metrics(b0);
    let (_, _, _, _, body1_pct, _) = candle_metrics(b1);
    let (_, _, _, _, body2_pct, _) = candle_metrics(b2);
    let avg_body_pct = (body0_pct + body1_pct + body2_pct) / 3.0;
    let total_adv = if b0.open.abs() > 1e-12 {
        100.0 * (b2.close - b0.open) / b0.open
    } else {
        0.0
    };
    let label = if last_match {
        "BULLISH_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlThreeWhiteSoldiersSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        avg_body_pct_range: avg_body_pct,
        total_close_advance_pct: total_adv,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: b2.close,
        cdl_three_white_soldiers_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_dark_cloud_cover_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlDarkCloudCoverSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 3 {
        return CdlDarkCloudCoverSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_dark_cloud_cover_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 bars, got {}", n),
            ..Default::default()
        };
    }
    // Dark Cloud Cover (2-bar bearish reversal):
    //   prior bar: green with body ≥ 30% of range,
    //   current bar: red, opens above prior high, closes below prior midpoint.
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
        if !bull0 {
            return 0;
        } // prior must be green
        if bull1 {
            return 0;
        } // current must be red
        if body0_pct < 30.0 {
            return 0;
        }
        if b1.open <= b0.high {
            return 0;
        } // current opens above prior high
        let midpoint0 = (b0.open + b0.close) / 2.0;
        if b1.close >= midpoint0 {
            return 0;
        } // current closes below prior midpoint
        if b1.close <= b0.open {
            return 0;
        } // but not below prior open (that'd be engulfing)
        -100
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 1, detect);
    let b0 = sorted[n - 2];
    let b1 = sorted[n - 1];
    let (body0, _, _, _, body0_pct, _) = candle_metrics(b0);
    let (_, _, _, _, body1_pct, _) = candle_metrics(b1);
    let pen = if body0 > 1e-12 {
        100.0 * (b0.close - b1.close) / body0
    } else {
        0.0
    };
    let label = if last_match {
        "BEARISH_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlDarkCloudCoverSnapshot {
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
        cdl_dark_cloud_cover_label: label.into(),
        note: String::new(),
    }
}
