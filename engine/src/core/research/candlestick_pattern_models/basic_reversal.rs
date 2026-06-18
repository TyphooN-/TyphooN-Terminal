use super::*;

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
