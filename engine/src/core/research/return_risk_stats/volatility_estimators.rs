use super::*;

// Parkinson, Garman-Klass, Rogers-Satchell, CVaR, and day-of-week computes

fn annualized_vol_label(annualized_pct: f64) -> &'static str {
    if annualized_pct < 10.0 {
        "VERY_LOW"
    } else if annualized_pct < 20.0 {
        "LOW"
    } else if annualized_pct < 40.0 {
        "NORMAL"
    } else if annualized_pct < 60.0 {
        "HIGH"
    } else {
        "VERY_HIGH"
    }
}

pub fn compute_parkinson_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> ParkinsonVolSnapshot {
    let sym = symbol.to_uppercase();
    let window: Vec<&HistoricalPriceRow> = bars
        .iter()
        .rev()
        .take(253)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if window.len() < 30 {
        return ParkinsonVolSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            vol_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 bars, got {}", window.len()),
            ..Default::default()
        };
    }
    let mut sum_sq = 0.0_f64;
    let mut sum_ln = 0.0_f64;
    let mut n = 0usize;
    for b in &window {
        if b.high <= 0.0 || b.low <= 0.0 || b.high < b.low {
            continue;
        }
        let r = (b.high / b.low).ln();
        sum_sq += r * r;
        sum_ln += r;
        n += 1;
    }
    if n < 30 {
        return ParkinsonVolSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            vol_label: "INSUFFICIENT_DATA".into(),
            note: format!("valid bars {n} < 30"),
            ..Default::default()
        };
    }
    let variance = sum_sq / (4.0 * 2f64.ln() * n as f64);
    let daily_sigma = variance.max(0.0).sqrt();
    let daily_pct = daily_sigma * 100.0;
    let annualized_pct = daily_sigma * (252.0_f64).sqrt() * 100.0;
    let mean_hl = sum_ln / n as f64;
    ParkinsonVolSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        daily_vol_pct: daily_pct,
        annualized_vol_pct: annualized_pct,
        mean_hl_log_ratio: mean_hl,
        vol_label: annualized_vol_label(annualized_pct).into(),
        note: String::new(),
    }
}

pub fn compute_gkvol_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> GarmanKlassVolSnapshot {
    let sym = symbol.to_uppercase();
    let window: Vec<&HistoricalPriceRow> = bars
        .iter()
        .rev()
        .take(253)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if window.len() < 30 {
        return GarmanKlassVolSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            vol_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 bars, got {}", window.len()),
            ..Default::default()
        };
    }
    let k = 2.0 * 2f64.ln() - 1.0;
    let mut sum_range = 0.0_f64;
    let mut sum_co = 0.0_f64;
    let mut n = 0usize;
    for b in &window {
        if b.high <= 0.0 || b.low <= 0.0 || b.open <= 0.0 || b.close <= 0.0 || b.high < b.low {
            continue;
        }
        let hl = (b.high / b.low).ln();
        let co = (b.close / b.open).ln();
        sum_range += 0.5 * hl * hl;
        sum_co += k * co * co;
        n += 1;
    }
    if n < 30 {
        return GarmanKlassVolSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            vol_label: "INSUFFICIENT_DATA".into(),
            note: format!("valid bars {n} < 30"),
            ..Default::default()
        };
    }
    let nf = n as f64;
    let range_component = sum_range / nf;
    let co_component = sum_co / nf;
    let variance = (range_component - co_component).max(0.0);
    let daily_sigma = variance.sqrt();
    let daily_pct = daily_sigma * 100.0;
    let annualized_pct = daily_sigma * (252.0_f64).sqrt() * 100.0;
    GarmanKlassVolSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        daily_vol_pct: daily_pct,
        annualized_vol_pct: annualized_pct,
        range_component,
        co_component,
        vol_label: annualized_vol_label(annualized_pct).into(),
        note: String::new(),
    }
}

pub fn compute_rsvol_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> RogersSatchellVolSnapshot {
    let sym = symbol.to_uppercase();
    let window: Vec<&HistoricalPriceRow> = bars
        .iter()
        .rev()
        .take(253)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if window.len() < 30 {
        return RogersSatchellVolSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            vol_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 bars, got {}", window.len()),
            ..Default::default()
        };
    }
    let mut sum = 0.0_f64;
    let mut n = 0usize;
    for b in &window {
        if b.high <= 0.0 || b.low <= 0.0 || b.open <= 0.0 || b.close <= 0.0 || b.high < b.low {
            continue;
        }
        let hc = (b.high / b.close).ln();
        let ho = (b.high / b.open).ln();
        let lc = (b.low / b.close).ln();
        let lo = (b.low / b.open).ln();
        sum += hc * ho + lc * lo;
        n += 1;
    }
    if n < 30 {
        return RogersSatchellVolSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            vol_label: "INSUFFICIENT_DATA".into(),
            note: format!("valid bars {n} < 30"),
            ..Default::default()
        };
    }
    let variance = (sum / n as f64).max(0.0);
    let daily_sigma = variance.sqrt();
    let daily_pct = daily_sigma * 100.0;
    let annualized_pct = daily_sigma * (252.0_f64).sqrt() * 100.0;
    RogersSatchellVolSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        daily_vol_pct: daily_pct,
        annualized_vol_pct: annualized_pct,
        vol_label: annualized_vol_label(annualized_pct).into(),
        note: String::new(),
    }
}

pub fn compute_cvar_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CVaRSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if window.len() < 100 || log_rets.len() < 100 {
        return CVaRSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cvar_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥100 log returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let mut sorted: Vec<f64> = log_rets.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    let idx5 = ((n as f64) * 0.05).floor() as usize;
    let idx1 = ((n as f64) * 0.01).floor() as usize;
    let idx5 = idx5.max(1).min(n - 1);
    let idx1 = idx1.max(1).min(n - 1);
    let var5 = sorted[idx5];
    let var1 = sorted[idx1];
    let tail5: Vec<f64> = sorted.iter().take(idx5 + 1).copied().collect();
    let tail1: Vec<f64> = sorted.iter().take(idx1 + 1).copied().collect();
    let cvar5 = if tail5.is_empty() {
        0.0
    } else {
        tail5.iter().sum::<f64>() / tail5.len() as f64
    };
    let cvar1 = if tail1.is_empty() {
        0.0
    } else {
        tail1.iter().sum::<f64>() / tail1.len() as f64
    };
    let cvar5_pct = (cvar5.exp() - 1.0) * 100.0;
    let cvar1_pct = (cvar1.exp() - 1.0) * 100.0;
    let var5_pct = (var5.exp() - 1.0) * 100.0;
    let var1_pct = (var1.exp() - 1.0) * 100.0;
    let abs5 = cvar5_pct.abs();
    let label = if abs5 < 1.0 {
        "MINIMAL"
    } else if abs5 < 2.5 {
        "LOW"
    } else if abs5 < 5.0 {
        "MODERATE"
    } else if abs5 < 10.0 {
        "HIGH"
    } else {
        "EXTREME"
    };
    CVaRSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: window.len(),
        var_5pct_ret_pct: var5_pct,
        cvar_5pct_ret_pct: cvar5_pct,
        var_1pct_ret_pct: var1_pct,
        cvar_1pct_ret_pct: cvar1_pct,
        tail_days_5pct: tail5.len(),
        tail_days_1pct: tail1.len(),
        cvar_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_doweffect_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DayOfWeekEffectSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 100 {
        return DayOfWeekEffectSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            dow_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥100 bars, got {}", bars.len()),
            ..Default::default()
        };
    }
    use chrono::{Datelike, NaiveDate};
    let mut hits: [usize; 5] = [0; 5];
    let mut counts: [usize; 5] = [0; 5];
    let mut sum_ret: [f64; 5] = [0.0; 5];
    let mut used = 0usize;
    let mut min_date: Option<NaiveDate> = None;
    let mut max_date: Option<NaiveDate> = None;
    for b in bars {
        let d = match NaiveDate::parse_from_str(&b.date, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => continue,
        };
        let w = d.weekday().num_days_from_monday();
        if w >= 5 {
            continue;
        } // Skip weekends defensively
        let wi = w as usize;
        if b.open <= 0.0 || b.close <= 0.0 {
            continue;
        }
        let r = (b.close / b.open - 1.0) * 100.0;
        sum_ret[wi] += r;
        counts[wi] += 1;
        if r > 0.0 {
            hits[wi] += 1;
        }
        used += 1;
        min_date = Some(min_date.map_or(d, |m| m.min(d)));
        max_date = Some(max_date.map_or(d, |m| m.max(d)));
    }
    if used < 100 || counts.iter().any(|c| *c < 10) {
        return DayOfWeekEffectSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            dow_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥100 bars and ≥10 per weekday, used {used}"),
            ..Default::default()
        };
    }
    let mut dow_hit_pct = [0.0_f64; 5];
    let mut dow_mean = [0.0_f64; 5];
    for i in 0..5 {
        if counts[i] > 0 {
            dow_hit_pct[i] = hits[i] as f64 / counts[i] as f64 * 100.0;
            dow_mean[i] = sum_ret[i] / counts[i] as f64;
        }
    }
    let mut best = 0usize;
    let mut worst = 0usize;
    for i in 1..5 {
        if dow_hit_pct[i] > dow_hit_pct[best]
            || (dow_hit_pct[i] == dow_hit_pct[best] && dow_mean[i] > dow_mean[best])
        {
            best = i;
        }
        if dow_hit_pct[i] < dow_hit_pct[worst]
            || (dow_hit_pct[i] == dow_hit_pct[worst] && dow_mean[i] < dow_mean[worst])
        {
            worst = i;
        }
    }
    let spread = dow_hit_pct[best] - dow_hit_pct[worst];
    let label = if spread >= 20.0 {
        "STRONG_EFFECT"
    } else if spread >= 10.0 {
        "MILD_EFFECT"
    } else if spread >= 5.0 {
        "NEUTRAL"
    } else {
        "INCONSISTENT"
    };
    let weeks_covered = match (min_date, max_date) {
        (Some(a), Some(b)) => ((b - a).num_days().max(0) / 7) as usize,
        _ => 0,
    };
    DayOfWeekEffectSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: used,
        weeks_covered,
        dow_hit_pct,
        dow_mean_ret_pct: dow_mean,
        dow_sample_count: counts,
        best_dow_idx: best,
        worst_dow_idx: worst,
        best_dow_hit_pct: dow_hit_pct[best],
        worst_dow_hit_pct: dow_hit_pct[worst],
        dow_label: label.into(),
        note: String::new(),
    }
}
