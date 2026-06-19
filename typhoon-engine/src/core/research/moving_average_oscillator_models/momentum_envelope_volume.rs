use super::*;

// Connors RSI, standard-error bands, intraday momentum, Guppy averages, envelopes, accumulation/distribution, VHF, and volume-rate models

/// Larry Connors's Connors RSI — composite of three components:
/// `CRSI = (RSI₃(close) + RSI₂(streak) + percent_rank(ROC₁, 100)) / 3`.
/// Streak is the signed run-length counter (+k on k up-days, -k on
/// k down-days, 0 on flat). Canonical extremes: > 90 (short), < 10 (long).
pub fn compute_crsi_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CrsiSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let rsi_length = 3usize;
    let streak_length = 2usize;
    let rank_lookback = 100usize;
    let min_bars = rank_lookback + rsi_length + 5;
    if n < min_bars {
        return CrsiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            rsi_length,
            streak_length,
            rank_lookback,
            crsi_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let rsi_series = |series: &[f64], length: usize| -> Vec<f64> {
        let m = series.len();
        let mut out = vec![50.0; m];
        if m < length + 1 {
            return out;
        }
        let mut gains = vec![0.0; m];
        let mut losses = vec![0.0; m];
        for i in 1..m {
            let d = series[i] - series[i - 1];
            if d > 0.0 {
                gains[i] = d;
            } else {
                losses[i] = -d;
            }
        }
        let mut avg_g: f64 = gains[1..=length].iter().sum::<f64>() / length as f64;
        let mut avg_l: f64 = losses[1..=length].iter().sum::<f64>() / length as f64;
        out[length] = if avg_l.abs() < 1e-12 {
            100.0
        } else {
            let rs = avg_g / avg_l;
            100.0 - 100.0 / (1.0 + rs)
        };
        for i in (length + 1)..m {
            avg_g = (avg_g * (length as f64 - 1.0) + gains[i]) / length as f64;
            avg_l = (avg_l * (length as f64 - 1.0) + losses[i]) / length as f64;
            out[i] = if avg_l.abs() < 1e-12 {
                100.0
            } else {
                let rs = avg_g / avg_l;
                100.0 - 100.0 / (1.0 + rs)
            };
        }
        out
    };
    let rsi_close_series = rsi_series(&closes, rsi_length);
    let rsi_close = rsi_close_series[n - 1];
    let mut streak = vec![0.0; n];
    for i in 1..n {
        let d = closes[i] - closes[i - 1];
        streak[i] = if d > 0.0 {
            if streak[i - 1] > 0.0 {
                streak[i - 1] + 1.0
            } else {
                1.0
            }
        } else if d < 0.0 {
            if streak[i - 1] < 0.0 {
                streak[i - 1] - 1.0
            } else {
                -1.0
            }
        } else {
            0.0
        };
    }
    let rsi_streak_series = rsi_series(&streak, streak_length);
    let rsi_streak = rsi_streak_series[n - 1];
    let mut roc = vec![0.0; n];
    for i in 1..n {
        roc[i] = if closes[i - 1].abs() > 1e-12 {
            (closes[i] - closes[i - 1]) / closes[i - 1] * 100.0
        } else {
            0.0
        };
    }
    let today_roc = roc[n - 1];
    let window_start = n.saturating_sub(rank_lookback);
    let window = &roc[window_start..(n - 1)];
    let below = window.iter().filter(|&&x| x < today_roc).count() as f64;
    let percent_rank = if window.is_empty() {
        0.0
    } else {
        100.0 * below / window.len() as f64
    };
    let crsi_value = (rsi_close + rsi_streak + percent_rank) / 3.0;
    let rsi_close_prev = rsi_close_series[n - 2];
    let rsi_streak_prev = rsi_streak_series[n - 2];
    let prev_roc = roc[n - 2];
    let prev_window_start = (n - 1).saturating_sub(rank_lookback);
    let prev_window = &roc[prev_window_start..(n - 2)];
    let prev_below = prev_window.iter().filter(|&&x| x < prev_roc).count() as f64;
    let prev_pr = if prev_window.is_empty() {
        0.0
    } else {
        100.0 * prev_below / prev_window.len() as f64
    };
    let crsi_prev = (rsi_close_prev + rsi_streak_prev + prev_pr) / 3.0;
    let label = if crsi_value >= 75.0 {
        "OVERBOUGHT"
    } else if crsi_value <= 25.0 {
        "OVERSOLD"
    } else if crsi_value >= 60.0 {
        "BULLISH"
    } else if crsi_value <= 40.0 {
        "BEARISH"
    } else {
        "NEUTRAL"
    };
    CrsiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        rsi_length,
        streak_length,
        rank_lookback,
        rsi_close,
        rsi_streak,
        percent_rank,
        crsi_value,
        crsi_prev,
        last_close: closes[n - 1],
        crsi_label: label.into(),
        note: String::new(),
    }
}

/// Standard Error Bands — linear-regression endpoint fit ± k·SE channels.
/// Center = regression value at `t = N − 1`; SE = residual standard error
/// with (N − 2) degrees of freedom. Labels by close position vs bands.
pub fn compute_seb_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> SebSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 20usize;
    let num_se = 2.0f64;
    let min_bars = length + 2;
    if n < min_bars {
        return SebSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            num_se,
            seb_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let start = n - length;
    let xs: Vec<f64> = (0..length).map(|i| i as f64).collect();
    let ys: &[f64] = &closes[start..n];
    let x_mean: f64 = xs.iter().sum::<f64>() / length as f64;
    let y_mean: f64 = ys.iter().sum::<f64>() / length as f64;
    let mut sxy = 0.0;
    let mut sxx = 0.0;
    for i in 0..length {
        sxy += (xs[i] - x_mean) * (ys[i] - y_mean);
        sxx += (xs[i] - x_mean).powi(2);
    }
    let slope = if sxx.abs() < 1e-12 { 0.0 } else { sxy / sxx };
    let intercept = y_mean - slope * x_mean;
    let mut ss_res = 0.0;
    for i in 0..length {
        let yhat = slope * xs[i] + intercept;
        ss_res += (ys[i] - yhat).powi(2);
    }
    let dof = (length as f64 - 2.0).max(1.0);
    let se = (ss_res / dof).sqrt();
    let middle = slope * (length as f64 - 1.0) + intercept;
    let upper = middle + num_se * se;
    let lower = middle - num_se * se;
    let bandwidth = if middle.abs() > 1e-12 {
        (upper - lower) / middle
    } else {
        0.0
    };
    let last_close = closes[n - 1];
    let range = upper - lower;
    let position_pct = if range.abs() > 1e-12 {
        (last_close - lower) / range * 100.0
    } else {
        50.0
    };
    let label = if last_close > upper {
        "ABOVE_BAND"
    } else if last_close < lower {
        "BELOW_BAND"
    } else if position_pct >= 66.6667 {
        "UPPER_HALF"
    } else if position_pct <= 33.3333 {
        "LOWER_HALF"
    } else {
        "NEUTRAL"
    };
    SebSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        num_se,
        upper,
        middle,
        lower,
        bandwidth,
        position_pct,
        last_close,
        seb_label: label.into(),
        note: String::new(),
    }
}

/// Tushar Chande's Intraday Momentum Index — RSI-style ratio built from
/// per-bar `close − open` buying/selling pressure rather than close-to-
/// close momentum. `IMI = 100 · ΣUp / (ΣUp + ΣDown)` over N bars, where
/// Up = max(close − open, 0), Down = max(open − close, 0).
pub fn compute_imi_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> ImiSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 14usize;
    let min_bars = length + 2;
    if n < min_bars {
        return ImiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            imi_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let mut gains = vec![0.0; n];
    let mut losses = vec![0.0; n];
    for i in 0..n {
        let d = sorted[i].close - sorted[i].open;
        if d > 0.0 {
            gains[i] = d;
        } else {
            losses[i] = -d;
        }
    }
    let imi_at = |end: usize| -> (f64, f64, f64) {
        let start = end + 1 - length;
        let sg: f64 = gains[start..=end].iter().sum();
        let sl: f64 = losses[start..=end].iter().sum();
        let tot = sg + sl;
        let v = if tot > 1e-12 { 100.0 * sg / tot } else { 50.0 };
        (sg, sl, v)
    };
    let (sum_gains, sum_losses, imi_value) = imi_at(n - 1);
    let (_, _, imi_prev) = imi_at(n - 2);
    let label = if imi_value >= 70.0 {
        "OVERBOUGHT"
    } else if imi_value <= 30.0 {
        "OVERSOLD"
    } else if imi_value >= 60.0 {
        "BULL"
    } else if imi_value <= 40.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    ImiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        sum_gains,
        sum_losses,
        imi_value,
        imi_prev,
        last_close: sorted[n - 1].close,
        imi_label: label.into(),
        note: String::new(),
    }
}

/// Guppy Multiple Moving Average — fan of 6 short + 6 long EMAs.
/// Reports group averages, spread, and trend label (STRONG_UPTREND when
/// short-avg > long-avg and both groups fanned; COMPRESSION when short
/// group width < 0.25 · long group width).
pub fn compute_gmma_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> GmmaSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let short_lengths: [usize; 6] = [3, 5, 8, 10, 12, 15];
    let long_lengths: [usize; 6] = [30, 35, 40, 45, 50, 60];
    let min_bars = 60 + 2;
    if n < min_bars {
        return GmmaSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            gmma_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|r| r.close).collect();
    let ema = |period: usize| -> f64 {
        let alpha = 2.0 / (period as f64 + 1.0);
        let mut e = closes[0];
        for &c in &closes[1..] {
            e = alpha * c + (1.0 - alpha) * e;
        }
        e
    };
    let shorts: Vec<f64> = short_lengths.iter().map(|&p| ema(p)).collect();
    let longs: Vec<f64> = long_lengths.iter().map(|&p| ema(p)).collect();
    let short_ema_avg = shorts.iter().sum::<f64>() / shorts.len() as f64;
    let long_ema_avg = longs.iter().sum::<f64>() / longs.len() as f64;
    let short_min = shorts.iter().cloned().fold(f64::INFINITY, f64::min);
    let short_max = shorts.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let long_min = longs.iter().cloned().fold(f64::INFINITY, f64::min);
    let long_max = longs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let last_close = closes[n - 1];
    let short_compression_pct = if last_close > 0.0 {
        (short_max - short_min) / last_close * 100.0
    } else {
        0.0
    };
    let long_compression_pct = if last_close > 0.0 {
        (long_max - long_min) / last_close * 100.0
    } else {
        0.0
    };
    let group_gap_pct = if last_close > 0.0 {
        (short_ema_avg - long_ema_avg) / last_close * 100.0
    } else {
        0.0
    };
    let fanned_up = short_min > long_max;
    let fanned_down = short_max < long_min;
    let compressed = short_compression_pct < 0.25 * long_compression_pct.max(1e-6);
    let label = if fanned_up && group_gap_pct > 1.0 {
        "STRONG_UPTREND"
    } else if short_ema_avg > long_ema_avg {
        "UPTREND"
    } else if fanned_down && group_gap_pct < -1.0 {
        "STRONG_DOWNTREND"
    } else if short_ema_avg < long_ema_avg {
        "DOWNTREND"
    } else if compressed {
        "COMPRESSION"
    } else {
        "NEUTRAL"
    };
    GmmaSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        short_ema_avg,
        long_ema_avg,
        short_min,
        short_max,
        long_min,
        long_max,
        short_compression_pct,
        long_compression_pct,
        group_gap_pct,
        last_close,
        gmma_label: label.into(),
        note: String::new(),
    }
}

/// Moving Average Envelope — SMA(N) ± k%. Labels the close's position
/// relative to the envelope.
pub fn compute_maenv_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MaenvSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 20usize;
    let pct_band = 2.5_f64;
    if n < length + 1 {
        return MaenvSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            pct_band,
            maenv_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length + 1, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|r| r.close).collect();
    let start = n - length;
    let sma: f64 = closes[start..].iter().sum::<f64>() / length as f64;
    let middle = sma;
    let factor = pct_band / 100.0;
    let upper = middle * (1.0 + factor);
    let lower = middle * (1.0 - factor);
    let last_close = closes[n - 1];
    let bandwidth_pct = 2.0 * pct_band;
    let position_pct = if upper > lower {
        (last_close - lower) / (upper - lower) * 100.0
    } else {
        50.0
    };
    let label = if last_close > upper {
        "ABOVE_BAND"
    } else if last_close < lower {
        "BELOW_BAND"
    } else if position_pct >= 75.0 {
        "UPPER_HALF"
    } else if position_pct <= 25.0 {
        "LOWER_HALF"
    } else {
        "NEUTRAL"
    };
    MaenvSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        pct_band,
        upper,
        middle,
        lower,
        bandwidth_pct,
        position_pct,
        last_close,
        maenv_label: label.into(),
        note: String::new(),
    }
}

/// Chaikin Accumulation/Distribution Line — cumulative ∑(MFM · volume).
/// Reports ADL, 20-bar SMA, OLS slope of last 20 ADL points, and
/// accumulation/distribution label.
pub fn compute_adl_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> AdlSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let adl_sma_length = 20usize;
    if n < adl_sma_length + 2 {
        return AdlSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            adl_sma_length,
            adl_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", adl_sma_length + 2, n),
            ..Default::default()
        };
    }
    let mut adl = vec![0.0; n];
    let mut running = 0.0_f64;
    for i in 0..n {
        let r = sorted[i];
        let range = r.high - r.low;
        let mfm = if range > 1e-12 {
            ((r.close - r.low) - (r.high - r.close)) / range
        } else {
            0.0
        };
        running += mfm * r.volume;
        adl[i] = running;
    }
    let adl_value = adl[n - 1];
    let adl_prev = adl[n - 2];
    let sma_start = n - adl_sma_length;
    let adl_sma: f64 = adl[sma_start..].iter().sum::<f64>() / adl_sma_length as f64;
    // OLS slope of last 20 points
    let nf = adl_sma_length as f64;
    let xs: Vec<f64> = (0..adl_sma_length).map(|i| i as f64).collect();
    let ys: &[f64] = &adl[sma_start..];
    let mx: f64 = xs.iter().sum::<f64>() / nf;
    let my: f64 = ys.iter().sum::<f64>() / nf;
    let mut num = 0.0;
    let mut den = 0.0;
    for i in 0..adl_sma_length {
        let dx = xs[i] - mx;
        num += dx * (ys[i] - my);
        den += dx * dx;
    }
    let slope_per_bar = if den > 1e-12 { num / den } else { 0.0 };
    let last_close = sorted[n - 1].close;
    let price_past = sorted[sma_start].close;
    let price_delta_pct = if price_past > 0.0 {
        (last_close - price_past) / price_past * 100.0
    } else {
        0.0
    };
    let norm_slope = if last_close > 0.0 {
        slope_per_bar / last_close
    } else {
        0.0
    };
    let label = if norm_slope > 1_000_000.0 {
        "STRONG_ACCUMULATION"
    } else if norm_slope > 100_000.0 {
        "ACCUMULATION"
    } else if norm_slope < -1_000_000.0 {
        "STRONG_DISTRIBUTION"
    } else if norm_slope < -100_000.0 {
        "DISTRIBUTION"
    } else {
        "NEUTRAL"
    };
    AdlSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        adl_value,
        adl_prev,
        adl_sma_length,
        adl_sma,
        slope_per_bar,
        last_close,
        price_delta_pct,
        adl_label: label.into(),
        note: String::new(),
    }
}

/// Vertical Horizontal Filter — (HHV − LLV) / Σ|Δclose| over N=28.
/// High = trending, low = ranging.
pub fn compute_vhf_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> VhfSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 28usize;
    if n < length + 2 {
        return VhfSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            vhf_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length + 2, n),
            ..Default::default()
        };
    }
    let vhf_at = |end: usize| -> (f64, f64, f64, f64) {
        let start = end + 1 - length;
        let mut hh = f64::NEG_INFINITY;
        let mut ll = f64::INFINITY;
        for i in start..=end {
            if sorted[i].high > hh {
                hh = sorted[i].high;
            }
            if sorted[i].low < ll {
                ll = sorted[i].low;
            }
        }
        let mut sum_abs = 0.0;
        for i in start..=end {
            sum_abs += (sorted[i].close - sorted[i - 1].close).abs();
        }
        let v = if sum_abs > 1e-12 {
            (hh - ll) / sum_abs
        } else {
            0.0
        };
        (hh, ll, sum_abs, v)
    };
    let (highest_high, lowest_low, sum_abs_delta, vhf_value) = vhf_at(n - 1);
    let (_, _, _, vhf_prev) = vhf_at(n - 2);
    let label = if vhf_value >= 0.6 {
        "STRONG_TREND"
    } else if vhf_value >= 0.4 {
        "TREND"
    } else if vhf_value <= 0.2 {
        "STRONG_RANGING"
    } else if vhf_value <= 0.3 {
        "RANGING"
    } else {
        "NEUTRAL"
    };
    VhfSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        highest_high,
        lowest_low,
        sum_abs_delta,
        vhf_value,
        vhf_prev,
        last_close: sorted[n - 1].close,
        vhf_label: label.into(),
        note: String::new(),
    }
}

/// Volume Rate of Change — 14-bar ROC of volume.
pub fn compute_vroc_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> VrocSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 14usize;
    if n < length + 2 {
        return VrocSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            vroc_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length + 2, n),
            ..Default::default()
        };
    }
    let vroc_at = |end: usize| -> (f64, f64, f64) {
        let then = sorted[end - length].volume;
        let now = sorted[end].volume;
        let v = if then > 1e-12 {
            (now - then) / then * 100.0
        } else {
            0.0
        };
        (now, then, v)
    };
    let (volume_now, volume_then, vroc_value) = vroc_at(n - 1);
    let (_, _, vroc_prev) = vroc_at(n - 2);
    let label = if vroc_value >= 100.0 {
        "SURGE"
    } else if vroc_value >= 30.0 {
        "ELEVATED"
    } else if vroc_value <= -50.0 {
        "COLLAPSE"
    } else if vroc_value <= -20.0 {
        "QUIET"
    } else {
        "NEUTRAL"
    };
    VrocSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        volume_now,
        volume_then,
        vroc_value,
        vroc_prev,
        last_close: sorted[n - 1].close,
        vroc_label: label.into(),
        note: String::new(),
    }
}
