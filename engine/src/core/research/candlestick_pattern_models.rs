use super::*;

// ── Round 72 CDL* candlestick patterns ─────────────────────────────

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

pub fn compute_cdl_doji_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlDojiSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 2 {
        return CdlDojiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_doji_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥2 bars, got {}", n),
            ..Default::default()
        };
    }
    // Doji: body_pct_range ≤ 5% — body is negligibly small relative to range.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        let (_body, range, _u, _l, body_pct, _bull) = candle_metrics(s[i]);
        if range < 1e-12 {
            return 0;
        }
        if body_pct <= 5.0 { 100 } else { 0 }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 0, detect);
    let (_b, range, upper, lower, body_pct, _bull) = candle_metrics(sorted[n - 1]);
    let upper_pct = if range > 1e-12 {
        100.0 * upper / range
    } else {
        0.0
    };
    let lower_pct = if range > 1e-12 {
        100.0 * lower / range
    } else {
        0.0
    };
    let label = if last_match {
        "DOJI_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlDojiSnapshot {
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
        last_close: sorted[n - 1].close,
        cdl_doji_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_hammer_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlHammerSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 2 {
        return CdlHammerSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_hammer_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥2 bars, got {}", n),
            ..Default::default()
        };
    }
    // Hammer: body_pct ≤ 30%, lower_shadow ≥ 2 × body, upper_shadow ≤ body.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        let (body, range, upper, lower, body_pct, _bull) = candle_metrics(s[i]);
        if range < 1e-12 || body < 1e-12 {
            return 0;
        }
        if body_pct <= 30.0 && lower >= 2.0 * body && upper <= body {
            100
        } else {
            0
        }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 0, detect);
    let (_b, range, upper, lower, body_pct, _bull) = candle_metrics(sorted[n - 1]);
    let upper_pct = if range > 1e-12 {
        100.0 * upper / range
    } else {
        0.0
    };
    let lower_pct = if range > 1e-12 {
        100.0 * lower / range
    } else {
        0.0
    };
    let label = if last_match {
        "BULLISH_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlHammerSnapshot {
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
        last_close: sorted[n - 1].close,
        cdl_hammer_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_shooting_star_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlShootingStarSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 2 {
        return CdlShootingStarSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_shooting_star_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥2 bars, got {}", n),
            ..Default::default()
        };
    }
    // Shooting star: body_pct ≤ 30%, upper_shadow ≥ 2 × body, lower_shadow ≤ body.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        let (body, range, upper, lower, body_pct, _bull) = candle_metrics(s[i]);
        if range < 1e-12 || body < 1e-12 {
            return 0;
        }
        if body_pct <= 30.0 && upper >= 2.0 * body && lower <= body {
            -100
        } else {
            0
        }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 0, detect);
    let (_b, range, upper, lower, body_pct, _bull) = candle_metrics(sorted[n - 1]);
    let upper_pct = if range > 1e-12 {
        100.0 * upper / range
    } else {
        0.0
    };
    let lower_pct = if range > 1e-12 {
        100.0 * lower / range
    } else {
        0.0
    };
    let label = if last_match {
        "BEARISH_PATTERN"
    } else {
        "NO_PATTERN"
    };
    CdlShootingStarSnapshot {
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
        last_close: sorted[n - 1].close,
        cdl_shooting_star_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_engulfing_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlEngulfingSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 3 {
        return CdlEngulfingSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_engulfing_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 bars, got {}", n),
            ..Default::default()
        };
    }
    // Engulfing: current body fully engulfs prior body AND direction is opposite.
    // Bullish: prior red (close<open), current green (close>open),
    //   current_open ≤ prior_close AND current_close ≥ prior_open.
    // Bearish: prior green, current red,
    //   current_open ≥ prior_close AND current_close ≤ prior_open.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        if i < 1 {
            return 0;
        }
        let prev = s[i - 1];
        let cur = s[i];
        let prev_bull = prev.close > prev.open;
        let cur_bull = cur.close > cur.open;
        let prev_body = (prev.close - prev.open).abs();
        let cur_body = (cur.close - cur.open).abs();
        if prev_body < 1e-12 || cur_body < 1e-12 {
            return 0;
        }
        if cur_body <= prev_body {
            return 0;
        }
        if !prev_bull && cur_bull && cur.open <= prev.close && cur.close >= prev.open {
            100
        } else if prev_bull && !cur_bull && cur.open >= prev.close && cur.close <= prev.open {
            -100
        } else {
            0
        }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 1, detect);
    let cur = sorted[n - 1];
    let prev = sorted[n - 2];
    let prev_body = (prev.close - prev.open).abs();
    let cur_body = (cur.close - cur.open).abs();
    let ratio = if prev_body > 1e-12 {
        cur_body / prev_body
    } else {
        0.0
    };
    let (_pb, prev_range, _pu, _pl, prev_body_pct, _) = candle_metrics(prev);
    let (_cb, cur_range, _cu, _cl, cur_body_pct, _) = candle_metrics(cur);
    let _ = prev_range;
    let _ = cur_range;
    let label = match last_val {
        100 => "BULLISH_PATTERN",
        -100 => "BEARISH_PATTERN",
        _ => "NO_PATTERN",
    };
    CdlEngulfingSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        body_size_ratio: ratio,
        prior_body_pct_range: prev_body_pct,
        current_body_pct_range: cur_body_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: cur.close,
        cdl_engulfing_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_cdl_harami_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CdlHaramiSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 3 {
        return CdlHaramiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cdl_harami_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 bars, got {}", n),
            ..Default::default()
        };
    }
    // Harami: current body contained within prior body AND direction is opposite.
    // Bullish: prior red, current green,
    //   current_open ≥ prior_close AND current_close ≤ prior_open.
    // Bearish: prior green, current red,
    //   current_open ≤ prior_close AND current_close ≥ prior_open.
    let detect = |s: &[&HistoricalPriceRow], i: usize| -> i32 {
        if i < 1 {
            return 0;
        }
        let prev = s[i - 1];
        let cur = s[i];
        let prev_bull = prev.close > prev.open;
        let cur_bull = cur.close > cur.open;
        let prev_body = (prev.close - prev.open).abs();
        let cur_body = (cur.close - cur.open).abs();
        if prev_body < 1e-12 || cur_body < 1e-12 {
            return 0;
        }
        if cur_body >= prev_body {
            return 0;
        }
        if !prev_bull && cur_bull && cur.open >= prev.close && cur.close <= prev.open {
            100
        } else if prev_bull && !cur_bull && cur.open <= prev.close && cur.close >= prev.open {
            -100
        } else {
            0
        }
    };
    let (last_match, days_since, last_val, prev_val) = cdl_scan(&sorted, 1, detect);
    let cur = sorted[n - 1];
    let prev = sorted[n - 2];
    let prev_body = (prev.close - prev.open).abs();
    let cur_body = (cur.close - cur.open).abs();
    let ratio = if prev_body > 1e-12 {
        cur_body / prev_body
    } else {
        0.0
    };
    let (_pb, _pr, _pu, _pl, prev_body_pct, _) = candle_metrics(prev);
    let (_cb, _cr, _cu, _cl, cur_body_pct, _) = candle_metrics(cur);
    let label = match last_val {
        100 => "BULLISH_PATTERN",
        -100 => "BEARISH_PATTERN",
        _ => "NO_PATTERN",
    };
    CdlHaramiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pattern_value: last_val,
        pattern_value_prev: prev_val,
        body_size_ratio: ratio,
        prior_body_pct_range: prev_body_pct,
        current_body_pct_range: cur_body_pct,
        last_bar_match: last_match,
        days_since_pattern: days_since,
        last_close: cur.close,
        cdl_harami_label: label.into(),
        note: String::new(),
    }
}

// ── Round 73 CDL* 3-bar and additional 2-bar patterns ──────────────

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

// ── Round 74 compute fns — CDLPIERCING / CDLDRAGONFLYDOJI /
//    CDLGRAVESTONEDOJI / CDLHANGINGMAN / CDLINVERTEDHAMMER ──

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

// ── Round 77 compute fns — CDLBELTHOLD / CDLCLOSINGMARUBOZU /
//    CDLHIGHWAVE / CDLLONGLINE / CDLSHORTLINE ──

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

// ── Round 78 compute fns — CDLCOUNTERATTACK / CDLHOMINGPIGEON /
//    CDLINNECK / CDLONNECK / CDLTHRUSTING ──

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

// ── Round 79 compute fns — CDL2CROWS / CDL3LINESTRIKE /
//    CDL3OUTSIDE / CDLMATCHINGLOW ──

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

// ── Round 80 compute fns — CDLSEPARATINGLINES / CDLSTICKSANDWICH /
//    CDLRICKSHAWMAN / CDLTAKURI ──

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

// ── Round 81/82 compute fns — CDL3STARSINSOUTH /
//    CDLIDENTICAL3CROWS / CDLKICKING / CDLKICKINGBYLENGTH /
//    CDLLADDERBOTTOM / CDLUNIQUE3RIVER ──

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

// ── Round 83/84 compute fns — CDLADVANCEBLOCK /
//    CDLBREAKAWAY / CDLGAPSIDESIDEWHITE / CDLUPSIDEGAP2CROWS /
//    CDLXSIDEGAP3METHODS / CDLCONCEALBABYSWALL ──

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

// ── Round 85/86 compute fns — CDLHIKKAKE / CDLHIKKAKEMOD /
//    CDLMATHOLD / CDLRISEFALL3METHODS ──

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

// ── Round 87/88 compute fns — CDLSTALLEDPATTERN /
//    CDLTASUKIGAP ──

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
