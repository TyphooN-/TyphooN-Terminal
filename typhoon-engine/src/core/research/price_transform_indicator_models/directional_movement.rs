use super::*;

// Directional-movement indicator transform family

/// Build Wilder-smoothed +DM / −DM / TR / +DI / −DI series for the DMI
/// family. Returns (plus_di, minus_di, atr, plus_smoothed, minus_smoothed,
/// tr_smoothed, plus_dm_raw, minus_dm_raw) at the last bar, plus the
/// previous-bar +DI / −DI / +DM smoothed / −DM smoothed for *_prev fields.
/// Returns None if bars are insufficient.
fn compute_dmi_series(
    sorted: &[&HistoricalPriceRow],
    period: usize,
) -> Option<(
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
)> {
    let n = sorted.len();
    // Need period+2 bars: period bars for Wilder seed + current + prior-smoothed history.
    if n < period + 2 {
        return None;
    }
    let mut tr = vec![0.0_f64; n];
    let mut plus_dm = vec![0.0_f64; n];
    let mut minus_dm = vec![0.0_f64; n];
    for i in 1..n {
        let hi = sorted[i].high;
        let lo = sorted[i].low;
        let pc = sorted[i - 1].close;
        tr[i] = (hi - lo).max((hi - pc).abs()).max((lo - pc).abs());
        let up_move = hi - sorted[i - 1].high;
        let dn_move = sorted[i - 1].low - lo;
        plus_dm[i] = if up_move > dn_move && up_move > 0.0 {
            up_move
        } else {
            0.0
        };
        minus_dm[i] = if dn_move > up_move && dn_move > 0.0 {
            dn_move
        } else {
            0.0
        };
    }
    let p_f = period as f64;
    let mut tr_smooth = vec![0.0_f64; n];
    let mut plus_smooth = vec![0.0_f64; n];
    let mut minus_smooth = vec![0.0_f64; n];
    let mut plus_di = vec![0.0_f64; n];
    let mut minus_di = vec![0.0_f64; n];
    let mut atr = vec![0.0_f64; n];
    tr_smooth[period] = tr[1..=period].iter().sum();
    plus_smooth[period] = plus_dm[1..=period].iter().sum();
    minus_smooth[period] = minus_dm[1..=period].iter().sum();
    atr[period] = tr_smooth[period] / p_f;
    if tr_smooth[period] > 0.0 {
        plus_di[period] = 100.0 * plus_smooth[period] / tr_smooth[period];
        minus_di[period] = 100.0 * minus_smooth[period] / tr_smooth[period];
    }
    for i in (period + 1)..n {
        tr_smooth[i] = tr_smooth[i - 1] - tr_smooth[i - 1] / p_f + tr[i];
        plus_smooth[i] = plus_smooth[i - 1] - plus_smooth[i - 1] / p_f + plus_dm[i];
        minus_smooth[i] = minus_smooth[i - 1] - minus_smooth[i - 1] / p_f + minus_dm[i];
        atr[i] = tr_smooth[i] / p_f;
        if tr_smooth[i] > 0.0 {
            plus_di[i] = 100.0 * plus_smooth[i] / tr_smooth[i];
            minus_di[i] = 100.0 * minus_smooth[i] / tr_smooth[i];
        }
    }
    Some((
        plus_di,
        minus_di,
        atr,
        plus_smooth,
        minus_smooth,
        tr_smooth,
        plus_dm,
        minus_dm,
    ))
}

/// Compute TA-Lib PLUS_DI — Wilder's Positive Directional Indicator (period 14).
pub fn compute_plus_di_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> PlusDiSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 14usize;
    let min_bars = period + 2;
    let series = compute_dmi_series(&sorted, period);
    if n < min_bars || series.is_none() {
        return PlusDiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            plus_di_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let (plus_di, minus_di, atr, _ps, _ms, _ts, _pr, _mr) = series.unwrap();
    let pdi_now = plus_di[n - 1];
    let pdi_prev = plus_di[n - 2];
    let mdi_now = minus_di[n - 1];
    let diff = pdi_now - mdi_now;
    let label = if diff > 10.0 {
        "BULL_DOMINANT"
    } else if diff > 2.0 {
        "BULL_LEAN"
    } else if diff < -10.0 {
        "BEAR_LEAN"
    } else if diff < -2.0 {
        "BEAR_LEAN"
    } else {
        "NEUTRAL"
    };
    PlusDiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        plus_di: pdi_now,
        plus_di_prev: pdi_prev,
        minus_di: mdi_now,
        atr: atr[n - 1],
        last_close: sorted[n - 1].close,
        plus_di_label: label.into(),
        note: String::new(),
    }
}

/// Compute TA-Lib MINUS_DI — Wilder's Negative Directional Indicator (period 14).
pub fn compute_minus_di_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MinusDiSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 14usize;
    let min_bars = period + 2;
    let series = compute_dmi_series(&sorted, period);
    if n < min_bars || series.is_none() {
        return MinusDiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            minus_di_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let (plus_di, minus_di, atr, _ps, _ms, _ts, _pr, _mr) = series.unwrap();
    let mdi_now = minus_di[n - 1];
    let mdi_prev = minus_di[n - 2];
    let pdi_now = plus_di[n - 1];
    let diff = mdi_now - pdi_now;
    let label = if diff > 10.0 {
        "BEAR_DOMINANT"
    } else if diff > 2.0 {
        "BEAR_LEAN"
    } else if diff < -10.0 {
        "BULL_LEAN"
    } else if diff < -2.0 {
        "BULL_LEAN"
    } else {
        "NEUTRAL"
    };
    MinusDiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        minus_di: mdi_now,
        minus_di_prev: mdi_prev,
        plus_di: pdi_now,
        atr: atr[n - 1],
        last_close: sorted[n - 1].close,
        minus_di_label: label.into(),
        note: String::new(),
    }
}

/// Compute TA-Lib PLUS_DM — Wilder's raw Positive Directional Movement (period 14).
pub fn compute_plus_dm_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> PlusDmSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 14usize;
    let min_bars = period + 2;
    let series = compute_dmi_series(&sorted, period);
    if n < min_bars || series.is_none() {
        return PlusDmSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            plus_dm_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let (_pdi, _mdi, _atr, plus_smooth, _ms, _ts, plus_dm_raw_v, minus_dm_raw_v) = series.unwrap();
    let pdm_s_now = plus_smooth[n - 1];
    let pdm_s_prev = plus_smooth[n - 2];
    let pdm_raw_now = plus_dm_raw_v[n - 1];
    let mdm_raw_now = minus_dm_raw_v[n - 1];
    let up = sorted[n - 1].high - sorted[n - 2].high;
    let dn = sorted[n - 2].low - sorted[n - 1].low;
    let label = if pdm_raw_now > 0.0 && pdm_raw_now > mdm_raw_now * 2.0 {
        "BULL_PRESSURE"
    } else if pdm_raw_now > 0.0 && pdm_raw_now > mdm_raw_now {
        "BULL_SOFT"
    } else if mdm_raw_now > pdm_raw_now && mdm_raw_now > 0.0 {
        "BEAR_PRESSURE"
    } else {
        "NEUTRAL"
    };
    PlusDmSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        plus_dm_raw: pdm_raw_now,
        plus_dm_smoothed: pdm_s_now,
        plus_dm_smoothed_prev: pdm_s_prev,
        up_move: up,
        down_move: dn,
        last_close: sorted[n - 1].close,
        plus_dm_label: label.into(),
        note: String::new(),
    }
}

/// Compute TA-Lib MINUS_DM — Wilder's raw Negative Directional Movement (period 14).
pub fn compute_minus_dm_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MinusDmSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 14usize;
    let min_bars = period + 2;
    let series = compute_dmi_series(&sorted, period);
    if n < min_bars || series.is_none() {
        return MinusDmSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            minus_dm_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let (_pdi, _mdi, _atr, _ps, minus_smooth, _ts, plus_dm_raw_v, minus_dm_raw_v) = series.unwrap();
    let mdm_s_now = minus_smooth[n - 1];
    let mdm_s_prev = minus_smooth[n - 2];
    let pdm_raw_now = plus_dm_raw_v[n - 1];
    let mdm_raw_now = minus_dm_raw_v[n - 1];
    let up = sorted[n - 1].high - sorted[n - 2].high;
    let dn = sorted[n - 2].low - sorted[n - 1].low;
    let label = if mdm_raw_now > 0.0 && mdm_raw_now > pdm_raw_now * 2.0 {
        "BEAR_PRESSURE"
    } else if mdm_raw_now > 0.0 && mdm_raw_now > pdm_raw_now {
        "BEAR_SOFT"
    } else if pdm_raw_now > mdm_raw_now && pdm_raw_now > 0.0 {
        "BULL_PRESSURE"
    } else {
        "NEUTRAL"
    };
    MinusDmSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        minus_dm_raw: mdm_raw_now,
        minus_dm_smoothed: mdm_s_now,
        minus_dm_smoothed_prev: mdm_s_prev,
        up_move: up,
        down_move: dn,
        last_close: sorted[n - 1].close,
        minus_dm_label: label.into(),
        note: String::new(),
    }
}

/// Compute TA-Lib DX — Wilder's Directional Movement Index (period 14).
/// `DX = 100 · |+DI − −DI| / (+DI + −DI)`; the raw (unsmoothed)
/// directional-purity signal that ADX then Wilder-smooths.
pub fn compute_dx_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> DxSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let period = 14usize;
    let min_bars = period + 2;
    let series = compute_dmi_series(&sorted, period);
    if n < min_bars || series.is_none() {
        return DxSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            period,
            dx_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", min_bars, n),
            ..Default::default()
        };
    }
    let (plus_di, minus_di, _atr, _ps, _ms, _ts, _pr, _mr) = series.unwrap();
    let dx_at = |i: usize| -> f64 {
        let s = plus_di[i] + minus_di[i];
        if s > 0.0 {
            100.0 * (plus_di[i] - minus_di[i]).abs() / s
        } else {
            0.0
        }
    };
    let dx_now = dx_at(n - 1);
    let dx_prev = dx_at(n - 2);
    let label = if dx_now >= 40.0 {
        "STRONG_DIR"
    } else if dx_now >= 25.0 {
        "DIR"
    } else if dx_now >= 15.0 {
        "WEAK_DIR"
    } else {
        "NO_DIR"
    };
    DxSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        dx: dx_now,
        dx_prev,
        plus_di: plus_di[n - 1],
        minus_di: minus_di[n - 1],
        last_close: sorted[n - 1].close,
        dx_label: label.into(),
        note: String::new(),
    }
}
