use super::*;

// Squeeze, breakout-channel, adaptive-average, cloud, and ATR trend models

/// SQUEEZE — composite short-squeeze outlier score.
/// Fuses five axes (short-float %, days-to-cover, 20d momentum, relvol, IV
/// rank) into a single 0..100 composite. Each axis is converted to a score
/// by its own saturating curve; missing axes are skipped and `inputs_present`
/// reflects how many contributed.
pub fn compute_squeeze_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
    short_interest: Option<&ShortInterestSnapshot>,
    ivol: Option<&IvolSnapshot>,
    relvol: Option<&RelVolSnapshot>,
) -> SqueezeSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));

    // 20d momentum on close-to-close
    let momentum_20d_pct = if sorted.len() >= 21 {
        let n = sorted.len();
        let last = sorted[n - 1].close;
        let prior = sorted[n - 21].close;
        if prior > 0.0 {
            (last / prior - 1.0) * 100.0
        } else {
            f64::NAN
        }
    } else {
        f64::NAN
    };

    // Axis 1: short_percent_of_float (saturate at 40% = score 100)
    let (sf_raw, sf_score, sf_present) = match short_interest.map(|s| s.short_percent_of_float) {
        Some(v) if v.is_finite() && v >= 0.0 => {
            let s = (v / 40.0 * 100.0).min(100.0);
            (v, s, true)
        }
        _ => (f64::NAN, 0.0, false),
    };

    // Axis 2: days_to_cover (saturate at 10 days = score 100)
    let (dtc_raw, dtc_score, dtc_present) = match short_interest.map(|s| s.days_to_cover) {
        Some(v) if v.is_finite() && v >= 0.0 => {
            let s = (v / 10.0 * 100.0).min(100.0);
            (v, s, true)
        }
        _ => (f64::NAN, 0.0, false),
    };

    // Axis 3: 20d momentum (positive momentum boosts squeeze; saturate at +30% = 100)
    let (mom_score, mom_present) = if momentum_20d_pct.is_finite() {
        // Clip negative to 0 (a falling stock is not squeezing)
        let s = (momentum_20d_pct.max(0.0) / 30.0 * 100.0).min(100.0);
        (s, true)
    } else {
        (0.0, false)
    };

    // Axis 4: relvol_20d (saturate at 3.0× = score 100)
    let (rv_raw, rv_score, rv_present) = match relvol.map(|r| r.rel_volume_20d) {
        Some(v) if v.is_finite() && v >= 0.0 => {
            let s = (v / 3.0 * 100.0).min(100.0);
            (v, s, true)
        }
        _ => (f64::NAN, 0.0, false),
    };

    // Axis 5: IV rank (already 0..100)
    let (iv_raw, iv_score, iv_present) = match ivol.map(|i| i.iv_rank) {
        Some(v) if v.is_finite() && v >= 0.0 => (v, v.clamp(0.0, 100.0), true),
        _ => (f64::NAN, 0.0, false),
    };

    let inputs_present = [sf_present, dtc_present, mom_present, rv_present, iv_present]
        .iter()
        .filter(|b| **b)
        .count();

    if inputs_present < 3 {
        return SqueezeSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            short_percent_of_float: sf_raw,
            days_to_cover: dtc_raw,
            momentum_20d_pct,
            relvol_20d: rv_raw,
            iv_rank: iv_raw,
            inputs_present,
            squeeze_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 axes with data, got {}", inputs_present),
            ..Default::default()
        };
    }

    // Weighted mean: short_float and days_to_cover carry 1.5× weight (core
    // squeeze mechanics); momentum, relvol, iv_rank carry 1.0×.
    let mut num = 0.0;
    let mut den = 0.0;
    if sf_present {
        num += 1.5 * sf_score;
        den += 1.5;
    }
    if dtc_present {
        num += 1.5 * dtc_score;
        den += 1.5;
    }
    if mom_present {
        num += 1.0 * mom_score;
        den += 1.0;
    }
    if rv_present {
        num += 1.0 * rv_score;
        den += 1.0;
    }
    if iv_present {
        num += 1.0 * iv_score;
        den += 1.0;
    }
    let composite = if den > 0.0 { num / den } else { 0.0 };

    let label = if composite >= 80.0 {
        "EXTREME"
    } else if composite >= 60.0 {
        "STRONG"
    } else if composite >= 40.0 {
        "ELEVATED"
    } else if composite >= 20.0 {
        "WATCH"
    } else {
        "NO_SQUEEZE"
    };

    SqueezeSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        short_percent_of_float: sf_raw,
        days_to_cover: dtc_raw,
        momentum_20d_pct,
        relvol_20d: rv_raw,
        iv_rank: iv_raw,
        short_float_score: sf_score,
        days_to_cover_score: dtc_score,
        momentum_score: mom_score,
        relvol_score: rv_score,
        iv_rank_score: iv_score,
        composite_score: composite,
        inputs_present,
        squeeze_label: label.into(),
        note: String::new(),
    }
}

/// SQUEEZERANK — cross-symbol percentile rank of the SQUEEZE composite.
pub fn compute_squeezerank_snapshot(
    symbol: &str,
    as_of: &str,
    subject: Option<&SqueezeSnapshot>,
    all: &[SqueezeSnapshot],
) -> SqueezeRankSnapshot {
    let sym = symbol.to_uppercase();
    let subj = match subject {
        Some(s) if s.squeeze_label != "INSUFFICIENT_DATA" && s.composite_score.is_finite() => s,
        _ => {
            return SqueezeRankSnapshot {
                symbol: sym,
                as_of: as_of.into(),
                squeezerank_label: "INSUFFICIENT_DATA".into(),
                note: "subject has no SQUEEZE composite".into(),
                ..Default::default()
            };
        }
    };
    let scored: Vec<f64> = all
        .iter()
        .filter(|s| s.squeeze_label != "INSUFFICIENT_DATA" && s.composite_score.is_finite())
        .map(|s| s.composite_score)
        .collect();
    if scored.len() < 5 {
        return SqueezeRankSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            composite_score: subj.composite_score,
            peer_count: scored.len(),
            squeezerank_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥5 peers in SQUEEZE table, got {}", scored.len()),
            ..Default::default()
        };
    }
    let subj_score = subj.composite_score;
    // Rank: 1 = highest composite
    let mut sorted = scored.clone();
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let rank = sorted
        .iter()
        .position(|&x| (x - subj_score).abs() < 1e-9)
        .map(|p| p + 1)
        .unwrap_or(sorted.len());
    let below_or_equal = scored.iter().filter(|&&x| x <= subj_score).count();
    let percentile = below_or_equal as f64 / scored.len() as f64 * 100.0;
    let label = if percentile >= 99.0 {
        "TOP_1PCT"
    } else if percentile >= 95.0 {
        "TOP_5PCT"
    } else if percentile >= 90.0 {
        "TOP_10PCT"
    } else if percentile >= 50.0 {
        "ABOVE_MEDIAN"
    } else {
        "BELOW_MEDIAN"
    };
    SqueezeRankSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        composite_score: subj_score,
        peer_count: scored.len(),
        rank,
        percentile,
        squeezerank_label: label.into(),
        note: String::new(),
    }
}

/// BBSQUEEZE — Bollinger-Band width squeeze detector.
pub fn compute_bbsqueeze_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> BbsqueezeSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 20usize;
    let hist_window = 120usize;
    if n < period + hist_window {
        return BbsqueezeSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            bbsqueeze_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", period + hist_window, n),
            ..Default::default()
        };
    }
    // Compute a rolling BB-width series for the trailing (hist_window) bars.
    let mut widths: Vec<f64> = Vec::with_capacity(hist_window);
    for end in (n - hist_window)..n {
        let start = end + 1 - period;
        let slice: &[&HistoricalPriceRow] = &sorted[start..=end];
        let sum: f64 = slice.iter().map(|b| b.close).sum();
        let mean = sum / period as f64;
        let var: f64 = slice.iter().map(|b| (b.close - mean).powi(2)).sum::<f64>() / period as f64;
        let sd = var.sqrt();
        if mean > 0.0 {
            widths.push((2.0 * 2.0 * sd) / mean); // (upper - lower)/mid = (2*2σ)/mean
        } else {
            widths.push(0.0);
        }
    }
    let current = *widths.last().unwrap();
    let min_w = widths.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_w = widths.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let below = widths.iter().filter(|&&x| x <= current).count();
    let pct = below as f64 / widths.len() as f64 * 100.0;
    // Re-derive current bands for display
    let end = n - 1;
    let start = end + 1 - period;
    let slice = &sorted[start..=end];
    let sum: f64 = slice.iter().map(|b| b.close).sum();
    let mid = sum / period as f64;
    let var: f64 = slice.iter().map(|b| (b.close - mid).powi(2)).sum::<f64>() / period as f64;
    let sd = var.sqrt();
    let upper = mid + 2.0 * sd;
    let lower = mid - 2.0 * sd;
    let label = if pct <= 10.0 {
        "TIGHT_SQUEEZE"
    } else if pct <= 25.0 {
        "MODERATE_SQUEEZE"
    } else if pct >= 90.0 {
        "EXPANSION"
    } else {
        "NORMAL"
    };
    BbsqueezeSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        bb_width_current: current,
        bb_width_min_120: min_w,
        bb_width_max_120: max_w,
        bb_width_percentile: pct,
        upper_band: upper,
        lower_band: lower,
        mid_band: mid,
        last_close: sorted[n - 1].close,
        bbsqueeze_label: label.into(),
        note: String::new(),
    }
}

/// DONCHIAN — 20-bar Donchian channel breakout detector.
pub fn compute_donchian_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DonchianSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 20usize;
    if n < period + 1 {
        return DonchianSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            donchian_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", period + 1, n),
            ..Default::default()
        };
    }
    // Prior channel uses bars [n-period-1 .. n-2] (exclude current bar); breakout
    // is defined as current close vs that prior channel.
    let prior_slice = &sorted[(n - period - 1)..(n - 1)];
    let prior_upper = prior_slice
        .iter()
        .map(|b| b.high)
        .fold(f64::NEG_INFINITY, f64::max);
    let prior_lower = prior_slice
        .iter()
        .map(|b| b.low)
        .fold(f64::INFINITY, f64::min);
    // Display channel uses full trailing period including current.
    let disp_slice = &sorted[(n - period)..n];
    let upper = disp_slice
        .iter()
        .map(|b| b.high)
        .fold(f64::NEG_INFINITY, f64::max);
    let lower = disp_slice
        .iter()
        .map(|b| b.low)
        .fold(f64::INFINITY, f64::min);
    let mid = (upper + lower) / 2.0;
    let last_close = sorted[n - 1].close;
    let breakout_up = last_close >= prior_upper;
    let breakout_dn = last_close <= prior_lower;
    let width = (upper - lower).max(f64::EPSILON);
    let pos = ((last_close - lower) / width * 100.0).clamp(0.0, 100.0);
    let label = if breakout_up {
        "BREAKOUT_UP"
    } else if breakout_dn {
        "BREAKOUT_DOWN"
    } else if pos >= 80.0 {
        "APPROACH_UP"
    } else if pos <= 20.0 {
        "APPROACH_DOWN"
    } else {
        "NEUTRAL"
    };
    DonchianSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        upper_channel: upper,
        lower_channel: lower,
        mid_channel: mid,
        last_close,
        channel_position_pct: pos,
        breakout_upper: breakout_up,
        breakout_lower: breakout_dn,
        donchian_label: label.into(),
        note: String::new(),
    }
}

/// KAMA — Kaufman adaptive moving average + efficiency ratio.
pub fn compute_kama_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> KamaSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 10usize;
    // Need period + warmup + slope window
    if n < period + 15 {
        return KamaSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            kama_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", period + 15, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    // Efficiency Ratio at last bar
    let last = closes[n - 1];
    let prior = closes[n - 1 - period];
    let direction = (last - prior).abs();
    let mut volatility = 0.0;
    for i in (n - period)..n {
        volatility += (closes[i] - closes[i - 1]).abs();
    }
    let er = if volatility > f64::EPSILON {
        (direction / volatility).clamp(0.0, 1.0)
    } else {
        0.0
    };
    // KAMA recursion (standard fast=2, slow=30 constants).
    let fast_sc = 2.0 / (2.0 + 1.0);
    let slow_sc = 2.0 / (30.0 + 1.0);
    // Seed KAMA with SMA over the first `period` bars.
    let seed_end = period;
    let seed: f64 = closes[..seed_end].iter().sum::<f64>() / period as f64;
    let mut kama_series: Vec<f64> = vec![0.0; n];
    for i in 0..seed_end {
        kama_series[i] = seed;
    }
    for i in seed_end..n {
        let dir_i = (closes[i] - closes[i - period]).abs();
        let mut vol_i = 0.0;
        for k in (i - period + 1)..=i {
            vol_i += (closes[k] - closes[k - 1]).abs();
        }
        let er_i = if vol_i > f64::EPSILON {
            (dir_i / vol_i).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let sc = (er_i * (fast_sc - slow_sc) + slow_sc).powi(2);
        kama_series[i] = kama_series[i - 1] + sc * (closes[i] - kama_series[i - 1]);
    }
    let kama_last = kama_series[n - 1];
    let kama_prior = kama_series[n - 6];
    let slope_pct = if kama_prior.abs() > f64::EPSILON {
        (kama_last / kama_prior - 1.0) * 100.0
    } else {
        0.0
    };
    let label = if er >= 0.7 {
        "STRONG_TREND"
    } else if er >= 0.4 {
        "MODERATE_TREND"
    } else if er >= 0.2 {
        "WEAK_TREND"
    } else {
        "CHOPPY"
    };
    KamaSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        efficiency_ratio: er,
        kama_value: kama_last,
        last_close: last,
        kama_slope_pct: slope_pct,
        kama_label: label.into(),
        note: String::new(),
    }
}

/// ICHIMOKU — full Ichimoku Kinko Hyo cloud system.
pub fn compute_ichimoku_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> IchimokuSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    // Need 52 bars for Senkou B, plus 26 bars of lookback for Chikou.
    if n < 52 + 26 {
        return IchimokuSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            ichimoku_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥78 bars, got {n}"),
            ..Default::default()
        };
    }
    let midpoint = |slice: &[&HistoricalPriceRow]| -> f64 {
        let h = slice
            .iter()
            .map(|b| b.high)
            .fold(f64::NEG_INFINITY, f64::max);
        let l = slice.iter().map(|b| b.low).fold(f64::INFINITY, f64::min);
        (h + l) / 2.0
    };
    let tenkan = midpoint(&sorted[(n - 9)..n]);
    let kijun = midpoint(&sorted[(n - 26)..n]);
    let senkou_a = (tenkan + kijun) / 2.0;
    let senkou_b = midpoint(&sorted[(n - 52)..n]);
    let chikou = sorted[n - 1].close;
    let cloud_top = senkou_a.max(senkou_b);
    let cloud_bottom = senkou_a.min(senkou_b);
    let last_close = sorted[n - 1].close;
    let cloud_mid = (cloud_top + cloud_bottom) / 2.0;
    let close_vs_cloud_pct = if cloud_mid.abs() > f64::EPSILON {
        (last_close - cloud_mid) / cloud_mid * 100.0
    } else {
        0.0
    };
    let past_close = sorted[n - 1 - 26].close;
    // Chikou-past confirmation: chikou above price from 26 bars ago = bullish confirm.
    let chikou_bull = chikou > past_close;
    let chikou_bear = chikou < past_close;
    let label = if last_close > cloud_top && tenkan > kijun && chikou_bull {
        "STRONG_BULL"
    } else if last_close > cloud_top {
        "BULL"
    } else if last_close < cloud_bottom && tenkan < kijun && chikou_bear {
        "STRONG_BEAR"
    } else if last_close < cloud_bottom {
        "BEAR"
    } else {
        "IN_CLOUD"
    };
    IchimokuSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        tenkan_sen: tenkan,
        kijun_sen: kijun,
        senkou_span_a: senkou_a,
        senkou_span_b: senkou_b,
        chikou_span: chikou,
        cloud_top,
        cloud_bottom,
        last_close,
        close_vs_cloud_pct,
        ichimoku_label: label.into(),
        note: String::new(),
    }
}

/// SUPERTREND — ATR-based trailing-stop trend indicator (period 10, multiplier 3).
pub fn compute_supertrend_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> SupertrendSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 10usize;
    let multiplier = 3.0_f64;
    if n < period + 2 {
        return SupertrendSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            multiplier,
            supertrend_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", period + 2, n),
            ..Default::default()
        };
    }
    // Wilder-smoothed ATR.
    let mut tr = Vec::<f64>::with_capacity(n);
    tr.push(sorted[0].high - sorted[0].low);
    for i in 1..n {
        let h = sorted[i].high;
        let l = sorted[i].low;
        let pc = sorted[i - 1].close;
        tr.push((h - l).max((h - pc).abs()).max((l - pc).abs()));
    }
    let mut atr = Vec::<f64>::with_capacity(n);
    // Initial ATR = simple mean of first `period` TR values.
    let first_atr: f64 = tr[..period].iter().sum::<f64>() / period as f64;
    for _ in 0..period {
        atr.push(first_atr);
    }
    for i in period..n {
        let prev = *atr.last().unwrap();
        atr.push((prev * (period as f64 - 1.0) + tr[i]) / period as f64);
    }
    // Supertrend recursion.
    let mut upper = vec![0.0_f64; n];
    let mut lower = vec![0.0_f64; n];
    let mut st = vec![0.0_f64; n];
    let mut up_trend = vec![true; n];
    for i in period..n {
        let hl2 = (sorted[i].high + sorted[i].low) / 2.0;
        let basic_upper = hl2 + multiplier * atr[i];
        let basic_lower = hl2 - multiplier * atr[i];
        if i == period {
            upper[i] = basic_upper;
            lower[i] = basic_lower;
            up_trend[i] = sorted[i].close >= basic_lower;
            st[i] = if up_trend[i] {
                basic_lower
            } else {
                basic_upper
            };
            continue;
        }
        upper[i] = if basic_upper < upper[i - 1] || sorted[i - 1].close > upper[i - 1] {
            basic_upper
        } else {
            upper[i - 1]
        };
        lower[i] = if basic_lower > lower[i - 1] || sorted[i - 1].close < lower[i - 1] {
            basic_lower
        } else {
            lower[i - 1]
        };
        let prev_st = st[i - 1];
        let prev_up = up_trend[i - 1];
        up_trend[i] = if prev_up {
            sorted[i].close >= lower[i]
        } else {
            sorted[i].close > upper[i]
        };
        st[i] = if up_trend[i] { lower[i] } else { upper[i] };
        let _ = prev_st;
    }
    let last_close = sorted[n - 1].close;
    let trend_up = up_trend[n - 1];
    let st_val = st[n - 1];
    let dist_pct = if st_val.abs() > f64::EPSILON {
        (last_close - st_val) / st_val * 100.0
    } else {
        0.0
    };
    // Bars in current trend
    let mut bars_in = 1usize;
    for i in (period + 1..n).rev() {
        if up_trend[i] == trend_up && up_trend[i - 1] == trend_up {
            bars_in += 1;
        } else {
            break;
        }
    }
    let label = if trend_up && dist_pct.abs() > 5.0 {
        "STRONG_UP"
    } else if trend_up {
        "UP"
    } else if !trend_up && dist_pct.abs() > 5.0 {
        "STRONG_DOWN"
    } else if !trend_up {
        "DOWN"
    } else {
        "FLAT"
    };
    SupertrendSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        multiplier,
        atr: atr[n - 1],
        upper_band: upper[n - 1],
        lower_band: lower[n - 1],
        supertrend_value: st_val,
        trend_is_up: trend_up,
        last_close,
        distance_pct: dist_pct,
        bars_in_trend: bars_in,
        supertrend_label: label.into(),
        note: String::new(),
    }
}

/// KELTNER — Keltner Channels (EMA 20 ± 2·ATR 10) + TTM-squeeze flag.
pub fn compute_keltner_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> KeltnerSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let ema_period = 20usize;
    let atr_period = 10usize;
    let multiplier = 2.0_f64;
    let need = ema_period.max(atr_period) + 2;
    if n < need {
        return KeltnerSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            ema_period,
            atr_period,
            multiplier,
            keltner_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", need, n),
            ..Default::default()
        };
    }
    // EMA of close.
    let k = 2.0 / (ema_period as f64 + 1.0);
    let seed: f64 = sorted[..ema_period].iter().map(|b| b.close).sum::<f64>() / ema_period as f64;
    let mut ema = seed;
    for i in ema_period..n {
        ema = sorted[i].close * k + ema * (1.0 - k);
    }
    // Wilder ATR.
    let mut tr = Vec::<f64>::with_capacity(n);
    tr.push(sorted[0].high - sorted[0].low);
    for i in 1..n {
        let h = sorted[i].high;
        let l = sorted[i].low;
        let pc = sorted[i - 1].close;
        tr.push((h - l).max((h - pc).abs()).max((l - pc).abs()));
    }
    let first_atr: f64 = tr[..atr_period].iter().sum::<f64>() / atr_period as f64;
    let mut atr = first_atr;
    for i in atr_period..n {
        atr = (atr * (atr_period as f64 - 1.0) + tr[i]) / atr_period as f64;
    }
    let upper = ema + multiplier * atr;
    let lower = ema - multiplier * atr;
    let width = upper - lower;
    let width_pct = if ema.abs() > f64::EPSILON {
        width / ema * 100.0
    } else {
        0.0
    };
    let last_close = sorted[n - 1].close;
    let pos_pct = if width > f64::EPSILON {
        ((last_close - lower) / width * 100.0).clamp(0.0, 100.0)
    } else {
        50.0
    };
    // TTM squeeze: BB 20/2σ inside KC.
    let slice = &sorted[(n - ema_period)..n];
    let mean: f64 = slice.iter().map(|b| b.close).sum::<f64>() / ema_period as f64;
    let var: f64 = slice.iter().map(|b| (b.close - mean).powi(2)).sum::<f64>() / ema_period as f64;
    let sd = var.sqrt();
    let bb_upper = mean + 2.0 * sd;
    let bb_lower = mean - 2.0 * sd;
    let ttm_squeeze = bb_upper <= upper && bb_lower >= lower;
    let label = if last_close > upper {
        "BREAKOUT_UP"
    } else if pos_pct >= 80.0 {
        "NEAR_UPPER"
    } else if last_close < lower {
        "BREAKOUT_DOWN"
    } else if pos_pct <= 20.0 {
        "NEAR_LOWER"
    } else {
        "IN_CHANNEL"
    };
    KeltnerSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        ema_period,
        atr_period,
        multiplier,
        ema_value: ema,
        atr,
        upper_channel: upper,
        lower_channel: lower,
        last_close,
        channel_width: width,
        width_pct_of_mid: width_pct,
        channel_position_pct: pos_pct,
        ttm_squeeze_on: ttm_squeeze,
        keltner_label: label.into(),
        note: String::new(),
    }
}
