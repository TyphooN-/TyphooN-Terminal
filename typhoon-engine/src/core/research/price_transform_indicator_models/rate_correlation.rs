use super::*;

// Rate-of-change, ratio, and correlation transform family

fn roc_label(pct: f64) -> &'static str {
    if pct >= 5.0 {
        "STRONG_UP"
    } else if pct >= 1.0 {
        "UP"
    } else if pct <= -5.0 {
        "STRONG_DOWN"
    } else if pct <= -1.0 {
        "DOWN"
    } else {
        "NEUTRAL"
    }
}

pub fn compute_roc_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> RocSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 10usize;
    let min_bars = period + 2;
    if n < min_bars {
        return RocSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            roc_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let close_now = sorted[n - 1].close;
    let close_lag = sorted[n - 1 - period].close;
    let close_prev = sorted[n - 2].close;
    let close_lag_prev = sorted[n - 2 - period].close;
    let roc = close_now - close_lag;
    let roc_prev = close_prev - close_lag_prev;
    let pct = if close_lag.abs() > 1e-12 {
        (roc / close_lag) * 100.0
    } else {
        0.0
    };
    RocSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        roc,
        roc_prev,
        close_now,
        close_lag,
        last_close: close_now,
        roc_label: roc_label(pct).into(),
        note: String::new(),
    }
}

pub fn compute_rocp_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> RocpSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 10usize;
    let min_bars = period + 2;
    if n < min_bars {
        return RocpSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            rocp_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let close_now = sorted[n - 1].close;
    let close_lag = sorted[n - 1 - period].close;
    let close_prev = sorted[n - 2].close;
    let close_lag_prev = sorted[n - 2 - period].close;
    let rocp = if close_lag.abs() > 1e-12 {
        (close_now - close_lag) / close_lag
    } else {
        0.0
    };
    let rocp_prev = if close_lag_prev.abs() > 1e-12 {
        (close_prev - close_lag_prev) / close_lag_prev
    } else {
        0.0
    };
    let rocp_pct = rocp * 100.0;
    RocpSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        rocp,
        rocp_prev,
        rocp_pct,
        close_now,
        close_lag,
        last_close: close_now,
        rocp_label: roc_label(rocp_pct).into(),
        note: String::new(),
    }
}

pub fn compute_rocr_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> RocrSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 10usize;
    let min_bars = period + 2;
    if n < min_bars {
        return RocrSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            rocr_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let close_now = sorted[n - 1].close;
    let close_lag = sorted[n - 1 - period].close;
    let close_prev = sorted[n - 2].close;
    let close_lag_prev = sorted[n - 2 - period].close;
    let rocr = if close_lag.abs() > 1e-12 {
        close_now / close_lag
    } else {
        1.0
    };
    let rocr_prev = if close_lag_prev.abs() > 1e-12 {
        close_prev / close_lag_prev
    } else {
        1.0
    };
    let pct = (rocr - 1.0) * 100.0;
    RocrSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        rocr,
        rocr_prev,
        close_now,
        close_lag,
        last_close: close_now,
        rocr_label: roc_label(pct).into(),
        note: String::new(),
    }
}

pub fn compute_rocr100_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> Rocr100Snapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 10usize;
    let min_bars = period + 2;
    if n < min_bars {
        return Rocr100Snapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            rocr100_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let close_now = sorted[n - 1].close;
    let close_lag = sorted[n - 1 - period].close;
    let close_prev = sorted[n - 2].close;
    let close_lag_prev = sorted[n - 2 - period].close;
    let rocr100 = if close_lag.abs() > 1e-12 {
        100.0 * close_now / close_lag
    } else {
        100.0
    };
    let rocr100_prev = if close_lag_prev.abs() > 1e-12 {
        100.0 * close_prev / close_lag_prev
    } else {
        100.0
    };
    let pct = rocr100 - 100.0;
    Rocr100Snapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        rocr100,
        rocr100_prev,
        close_now,
        close_lag,
        last_close: close_now,
        rocr100_label: roc_label(pct).into(),
        note: String::new(),
    }
}

/// Pearson correlation of (close_t, close_{t-1}) over the last `period`
/// bars — a lag-1 autocorrelation for single-symbol momentum / mean-
/// reversion classification. `ρ → +1` is strong momentum,
/// `ρ → 0` is a random walk, `ρ → −1` is strong mean reversion.
pub fn compute_correl_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CorrelSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 30usize;
    let min_bars = period + 2;
    if n < min_bars {
        return CorrelSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            correl_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let correl_at = |end_idx: usize| -> (f64, f64, f64, f64, f64) {
        let start = end_idx + 1 - period;
        let xs: Vec<f64> = (start..=end_idx).map(|i| sorted[i].close).collect();
        let ys: Vec<f64> = (start..=end_idx).map(|i| sorted[i - 1].close).collect();
        let mx: f64 = xs.iter().sum::<f64>() / period as f64;
        let my: f64 = ys.iter().sum::<f64>() / period as f64;
        let mut sxx = 0.0f64;
        let mut syy = 0.0f64;
        let mut sxy = 0.0f64;
        for i in 0..period {
            let dx = xs[i] - mx;
            let dy = ys[i] - my;
            sxx += dx * dx;
            syy += dy * dy;
            sxy += dx * dy;
        }
        let sdx = (sxx / period as f64).sqrt();
        let sdy = (syy / period as f64).sqrt();
        let rho = if sxx > 0.0 && syy > 0.0 {
            sxy / (sxx.sqrt() * syy.sqrt())
        } else {
            0.0
        };
        (rho, mx, my, sdx, sdy)
    };
    let (rho_now, mx, my, sdx, sdy) = correl_at(n - 1);
    let (rho_prev, _, _, _, _) = correl_at(n - 2);
    let label = if rho_now >= 0.7 {
        "STRONG_MOMO"
    } else if rho_now >= 0.2 {
        "MOMO"
    } else if rho_now <= -0.7 {
        "STRONG_MEAN_REVERT"
    } else if rho_now <= -0.2 {
        "MEAN_REVERT"
    } else {
        "RANDOM_WALK"
    };
    CorrelSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        correl: rho_now,
        correl_prev: rho_prev,
        mean_x: mx,
        mean_y: my,
        stddev_x: sdx,
        stddev_y: sdy,
        last_close: sorted[n - 1].close,
        correl_label: label.into(),
        note: String::new(),
    }
}
