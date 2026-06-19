use super::*;

// Average-price, median-price, typical-price, weighted-close, and variance transforms

/// Compute TA-Lib AVGPRICE — `(open + high + low + close) / 4`.
pub fn compute_avgprice_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> AvgpriceSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 1 {
        return AvgpriceSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            avgprice_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥1 bar, got 0".into(),
            ..Default::default()
        };
    }
    let b = sorted[n - 1];
    let avg_now = (b.open + b.high + b.low + b.close) / 4.0;
    let avg_prev = if n >= 2 {
        let p = sorted[n - 2];
        (p.open + p.high + p.low + p.close) / 4.0
    } else {
        avg_now
    };
    let delta_pct = if b.close.abs() > 1e-12 {
        (avg_now - b.close) / b.close * 100.0
    } else {
        0.0
    };
    let label = if delta_pct > 0.3 {
        "ABOVE_CLOSE"
    } else if delta_pct < -0.3 {
        "BELOW_CLOSE"
    } else {
        "NEAR_CLOSE"
    };
    AvgpriceSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        avgprice: avg_now,
        avgprice_prev: avg_prev,
        open: b.open,
        high: b.high,
        low: b.low,
        close: b.close,
        delta_pct,
        avgprice_label: label.into(),
        note: String::new(),
    }
}

/// Compute TA-Lib MEDPRICE — `(high + low) / 2`.
pub fn compute_medprice_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MedpriceSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 1 {
        return MedpriceSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            medprice_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥1 bar, got 0".into(),
            ..Default::default()
        };
    }
    let b = sorted[n - 1];
    let med_now = 0.5 * (b.high + b.low);
    let med_prev = if n >= 2 {
        let p = sorted[n - 2];
        0.5 * (p.high + p.low)
    } else {
        med_now
    };
    let delta_pct = if b.close.abs() > 1e-12 {
        (med_now - b.close) / b.close * 100.0
    } else {
        0.0
    };
    let label = if delta_pct > 0.2 {
        "ABOVE_MID"
    } else if delta_pct < -0.2 {
        "BELOW_MID"
    } else {
        "AT_MID"
    };
    MedpriceSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        medprice: med_now,
        medprice_prev: med_prev,
        high: b.high,
        low: b.low,
        close: b.close,
        delta_pct,
        medprice_label: label.into(),
        note: String::new(),
    }
}

/// Compute TA-Lib TYPPRICE — `(high + low + close) / 3`.
pub fn compute_typprice_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> TypPriceSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 1 {
        return TypPriceSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            typprice_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥1 bar, got 0".into(),
            ..Default::default()
        };
    }
    let b = sorted[n - 1];
    let typ_now = (b.high + b.low + b.close) / 3.0;
    let typ_prev = if n >= 2 {
        let p = sorted[n - 2];
        (p.high + p.low + p.close) / 3.0
    } else {
        typ_now
    };
    let delta_pct = if b.close.abs() > 1e-12 {
        (typ_now - b.close) / b.close * 100.0
    } else {
        0.0
    };
    let label = if delta_pct > 0.2 {
        "ABOVE_CLOSE"
    } else if delta_pct < -0.2 {
        "BELOW_CLOSE"
    } else {
        "NEAR_CLOSE"
    };
    TypPriceSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        typprice: typ_now,
        typprice_prev: typ_prev,
        high: b.high,
        low: b.low,
        close: b.close,
        delta_pct,
        typprice_label: label.into(),
        note: String::new(),
    }
}

/// Compute TA-Lib WCLPRICE — `(high + low + 2 × close) / 4`.
pub fn compute_wclprice_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> WclPriceSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 1 {
        return WclPriceSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            wclprice_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥1 bar, got 0".into(),
            ..Default::default()
        };
    }
    let b = sorted[n - 1];
    let wcl_now = (b.high + b.low + 2.0 * b.close) / 4.0;
    let wcl_prev = if n >= 2 {
        let p = sorted[n - 2];
        (p.high + p.low + 2.0 * p.close) / 4.0
    } else {
        wcl_now
    };
    let delta_pct = if b.close.abs() > 1e-12 {
        (wcl_now - b.close) / b.close * 100.0
    } else {
        0.0
    };
    let label = if delta_pct > 0.15 {
        "ABOVE_CLOSE"
    } else if delta_pct < -0.15 {
        "BELOW_CLOSE"
    } else {
        "NEAR_CLOSE"
    };
    WclPriceSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        wclprice: wcl_now,
        wclprice_prev: wcl_prev,
        high: b.high,
        low: b.low,
        close: b.close,
        delta_pct,
        wclprice_label: label.into(),
        note: String::new(),
    }
}

/// Compute TA-Lib VARIANCE — population variance of close over `period` bars (default 5).
pub fn compute_variance_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> VarianceSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 5usize;
    if n < period + 1 {
        return VarianceSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            variance_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", period + 1, n),
            ..Default::default()
        };
    }
    let var_at = |end_idx: usize| -> (f64, f64) {
        let start = end_idx + 1 - period;
        let slice: Vec<f64> = sorted[start..=end_idx].iter().map(|r| r.close).collect();
        let mu: f64 = slice.iter().sum::<f64>() / period as f64;
        let v: f64 = slice.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / period as f64;
        (mu, v)
    };
    let (mean_now, var_now) = var_at(n - 1);
    let (_, var_prev) = var_at(n - 2);
    let stddev = var_now.sqrt();
    let last_close = sorted[n - 1].close;
    let cv = if mean_now.abs() > 1e-12 {
        stddev / mean_now.abs() * 100.0
    } else {
        0.0
    };
    let label = if cv > 5.0 {
        "HIGH_VOL"
    } else if cv > 2.0 {
        "ELEVATED"
    } else if cv < 0.5 {
        "LOW_VOL"
    } else {
        "NORMAL"
    };
    VarianceSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        mean: mean_now,
        variance: var_now,
        variance_prev: var_prev,
        stddev,
        cv,
        last_close,
        variance_label: label.into(),
        note: String::new(),
    }
}
