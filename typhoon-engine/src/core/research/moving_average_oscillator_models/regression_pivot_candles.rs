use super::*;

// Double/triple EMA, linear-regression, pivot-point, and Heikin-Ashi models

pub fn compute_dema_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DemaSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 20usize;
    let min_bars = 2 * length + 2;
    if n < min_bars {
        return DemaSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            dema_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let ema1 = ema_series(&closes, length);
    let ema2 = ema_series(&ema1, length);
    let dema: Vec<f64> = ema1
        .iter()
        .zip(ema2.iter())
        .map(|(a, b)| 2.0 * a - b)
        .collect();
    let dema_value = dema[n - 1];
    let dema_prev = dema[n - 2];
    let last_close = closes[n - 1];
    let dev = if dema_value.abs() > 1e-12 {
        (last_close - dema_value) / dema_value * 100.0
    } else {
        0.0
    };
    let label = if dev > 2.0 {
        "STRONG_BULL"
    } else if dev > 0.0 {
        "BULL"
    } else if dev < -2.0 {
        "STRONG_BEAR"
    } else if dev < 0.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    DemaSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        dema_value,
        dema_prev,
        deviation_pct: dev,
        last_close,
        dema_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_tema_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> TemaSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 20usize;
    let min_bars = 3 * length + 2;
    if n < min_bars {
        return TemaSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            tema_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let ema1 = ema_series(&closes, length);
    let ema2 = ema_series(&ema1, length);
    let ema3 = ema_series(&ema2, length);
    let tema: Vec<f64> = (0..n)
        .map(|i| 3.0 * ema1[i] - 3.0 * ema2[i] + ema3[i])
        .collect();
    let tema_value = tema[n - 1];
    let tema_prev = tema[n - 2];
    let last_close = closes[n - 1];
    let dev = if tema_value.abs() > 1e-12 {
        (last_close - tema_value) / tema_value * 100.0
    } else {
        0.0
    };
    let label = if dev > 2.0 {
        "STRONG_BULL"
    } else if dev > 0.0 {
        "BULL"
    } else if dev < -2.0 {
        "STRONG_BEAR"
    } else if dev < 0.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    TemaSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        tema_value,
        tema_prev,
        deviation_pct: dev,
        last_close,
        tema_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_linreg_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> LinregSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 20usize;
    if n < length {
        return LinregSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            linreg_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length, n),
            ..Default::default()
        };
    }
    // OLS on the last `length` closes with x = 0..length-1.
    let start = n - length;
    let ys: Vec<f64> = sorted[start..n].iter().map(|b| b.close).collect();
    let ln = length as f64;
    let mean_x = (ln - 1.0) / 2.0;
    let mean_y: f64 = ys.iter().sum::<f64>() / ln;
    let mut num = 0.0f64;
    let mut den_x = 0.0f64;
    let mut den_y = 0.0f64;
    for i in 0..length {
        let xi = i as f64;
        let dx = xi - mean_x;
        let dy = ys[i] - mean_y;
        num += dx * dy;
        den_x += dx * dx;
        den_y += dy * dy;
    }
    let slope = if den_x.abs() > 1e-12 {
        num / den_x
    } else {
        0.0
    };
    let intercept = mean_y - slope * mean_x;
    let r2 = if den_x.abs() > 1e-12 && den_y.abs() > 1e-12 {
        let r = num / (den_x.sqrt() * den_y.sqrt());
        r * r
    } else {
        0.0
    };
    // Residuals standard error.
    let mut ss_res = 0.0f64;
    for i in 0..length {
        let xi = i as f64;
        let yhat = intercept + slope * xi;
        ss_res += (ys[i] - yhat).powi(2);
    }
    let sigma = if length > 2 {
        (ss_res / (length as f64 - 2.0)).sqrt()
    } else {
        0.0
    };
    let fit_value = intercept + slope * (length as f64 - 1.0);
    let channel_upper = fit_value + 2.0 * sigma;
    let channel_lower = fit_value - 2.0 * sigma;
    let last_close = ys[length - 1];
    // Normalize slope as percent per bar of the fit line's mean level for labeling.
    let slope_pct = if mean_y.abs() > 1e-12 {
        slope / mean_y * 100.0
    } else {
        0.0
    };
    let label = if r2 >= 0.5 && slope_pct > 0.25 {
        "STRONG_UP_TREND"
    } else if slope_pct > 0.05 {
        "UP_TREND"
    } else if r2 >= 0.5 && slope_pct < -0.25 {
        "STRONG_DOWN_TREND"
    } else if slope_pct < -0.05 {
        "DOWN_TREND"
    } else {
        "RANGE"
    };
    LinregSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        slope,
        intercept,
        r_squared: r2,
        sigma,
        last_close,
        fit_value,
        channel_upper,
        channel_lower,
        linreg_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_pivots_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> PivotsSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 2 {
        return PivotsSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            pivots_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥2 bars, got {}", n),
            ..Default::default()
        };
    }
    let prior = sorted[n - 2];
    let current_close = sorted[n - 1].close;
    let h = prior.high;
    let l = prior.low;
    let c = prior.close;
    let pp = (h + l + c) / 3.0;
    let r1 = 2.0 * pp - l;
    let s1 = 2.0 * pp - h;
    let r2 = pp + (h - l);
    let s2 = pp - (h - l);
    let label = if current_close >= r2 {
        "ABOVE_R2"
    } else if current_close >= r1 {
        "BETWEEN_R1_R2"
    } else if current_close > pp {
        "BETWEEN_PP_R1"
    } else if (current_close - pp).abs() < 1e-9 {
        "AT_PP"
    } else if current_close >= s1 {
        "BETWEEN_S1_PP"
    } else if current_close >= s2 {
        "BETWEEN_S2_S1"
    } else {
        "BELOW_S2"
    };
    PivotsSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        pp,
        r1,
        r2,
        s1,
        s2,
        last_close: current_close,
        prior_high: h,
        prior_low: l,
        prior_close: c,
        pivots_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_heikin_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> HeikinSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 3 {
        return HeikinSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            heikin_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 bars, got {}", n),
            ..Default::default()
        };
    }
    // Build HA series.
    let mut ha_open = Vec::with_capacity(n);
    let mut ha_close = Vec::with_capacity(n);
    let mut ha_high = Vec::with_capacity(n);
    let mut ha_low = Vec::with_capacity(n);
    // Seed bar 0.
    ha_open.push((sorted[0].open + sorted[0].close) / 2.0);
    ha_close.push((sorted[0].open + sorted[0].high + sorted[0].low + sorted[0].close) / 4.0);
    ha_high.push(sorted[0].high.max(ha_open[0]).max(ha_close[0]));
    ha_low.push(sorted[0].low.min(ha_open[0]).min(ha_close[0]));
    for i in 1..n {
        let o = (ha_open[i - 1] + ha_close[i - 1]) / 2.0;
        let c = (sorted[i].open + sorted[i].high + sorted[i].low + sorted[i].close) / 4.0;
        let h = sorted[i].high.max(o).max(c);
        let l = sorted[i].low.min(o).min(c);
        ha_open.push(o);
        ha_close.push(c);
        ha_high.push(h);
        ha_low.push(l);
    }
    let idx = n - 1;
    let cur_bull = ha_close[idx] > ha_open[idx];
    let mut run = 1usize;
    let mut i = idx;
    while i > 0 {
        let prev_bull = ha_close[i - 1] > ha_open[i - 1];
        if prev_bull != cur_bull {
            break;
        }
        run += 1;
        i -= 1;
    }
    let body = (ha_close[idx] - ha_open[idx]).abs();
    let upper = ha_high[idx] - ha_open[idx].max(ha_close[idx]);
    let lower = ha_open[idx].min(ha_close[idx]) - ha_low[idx];
    let range = (ha_high[idx] - ha_low[idx]).max(1e-12);
    let doji = body / range < 0.10;
    let label = if doji {
        "DOJI"
    } else if cur_bull && run >= 3 {
        "STRONG_BULL_RUN"
    } else if cur_bull {
        "BULL"
    } else if !cur_bull && run >= 3 {
        "STRONG_BEAR_RUN"
    } else {
        "BEAR"
    };
    HeikinSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        ha_open: ha_open[idx],
        ha_high: ha_high[idx],
        ha_low: ha_low[idx],
        ha_close: ha_close[idx],
        body_abs: body,
        upper_wick: upper,
        lower_wick: lower,
        consecutive_same_color: run,
        last_close: sorted[idx].close,
        heikin_label: label.into(),
        note: String::new(),
    }
}
