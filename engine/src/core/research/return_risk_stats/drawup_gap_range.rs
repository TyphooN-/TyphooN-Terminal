use super::*;

// Draw-up, gap-statistics, volatility-cluster, close-position, and range-location computes

/// DRAWUP compute: trough-to-peak rally history over the trailing 253
/// sessions. Mirror of `compute_ddhist_snapshot` — flip peak↔trough and
/// drawdown↔drawup, keep everything else aligned.
pub fn compute_drawup_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DrawupHistorySnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 20 {
        return DrawupHistorySnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: bars.len(),
            rally_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥20 bars".into(),
            ..Default::default()
        };
    }
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let start = if sorted.len() > 253 {
        sorted.len() - 253
    } else {
        0
    };
    let window = &sorted[start..];
    let bars_used = window.len();
    let mut running_trough = window[0].close;
    let mut running_trough_idx = 0usize;
    let mut running_trough_date = window[0].date.clone();
    let mut max_du_pct = 0.0f64;
    let mut max_du_trough_date = String::new();
    let mut max_du_peak_date = String::new();
    let mut longest_du_days = 0usize;
    let mut current_du_start: Option<(usize, String)> = None;
    let mut rallies_5 = 0usize;
    let mut rallies_10 = 0usize;
    let mut in_rally = false;
    let mut rally_trough = window[0].close;
    for (i, bar) in window.iter().enumerate() {
        let c = bar.close;
        if c <= 0.0 {
            continue;
        }
        if c <= running_trough {
            if let Some((trough_idx, _)) = &current_du_start {
                let duration = i - trough_idx;
                if duration > longest_du_days {
                    longest_du_days = duration;
                }
            }
            current_du_start = None;
            running_trough = c;
            running_trough_idx = i;
            running_trough_date = bar.date.clone();
            if in_rally {
                let height = (c - rally_trough) / rally_trough * 100.0;
                if height >= 10.0 {
                    rallies_10 += 1;
                }
                if height >= 5.0 {
                    rallies_5 += 1;
                }
                in_rally = false;
            }
            rally_trough = c;
        } else {
            if current_du_start.is_none() {
                current_du_start = Some((running_trough_idx, running_trough_date.clone()));
            }
            let du = (c - running_trough) / running_trough * 100.0;
            if du > max_du_pct {
                max_du_pct = du;
                max_du_trough_date = running_trough_date.clone();
                max_du_peak_date = bar.date.clone();
            }
            if c > rally_trough {
                in_rally = true;
            }
        }
    }
    if let Some((trough_idx, _)) = &current_du_start {
        let duration = window.len().saturating_sub(*trough_idx);
        if duration > longest_du_days {
            longest_du_days = duration;
        }
    }
    if in_rally {
        let last = window.last().map(|r| r.close).unwrap_or(rally_trough);
        let height = (last - rally_trough) / rally_trough * 100.0;
        if height >= 10.0 {
            rallies_10 += 1;
        }
        if height >= 5.0 {
            rallies_5 += 1;
        }
    }
    let latest = window.last().map(|r| r.close).unwrap_or(0.0);
    let current_du = if latest > 0.0 && running_trough > 0.0 {
        (latest - running_trough) / running_trough * 100.0
    } else {
        0.0
    };
    let label = if max_du_pct < 5.0 {
        "MUTED"
    } else if max_du_pct < 10.0 {
        "MILD"
    } else if max_du_pct < 20.0 {
        "MEANINGFUL"
    } else if max_du_pct < 40.0 {
        "STRONG"
    } else {
        "EXPLOSIVE"
    };
    DrawupHistorySnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used,
        max_drawup_pct: max_du_pct,
        max_drawup_trough_date: max_du_trough_date,
        max_drawup_peak_date: max_du_peak_date,
        longest_drawup_days: longest_du_days,
        rallies_5pct: rallies_5,
        rallies_10pct: rallies_10,
        current_drawup_pct: current_du,
        rally_label: label.into(),
        note: String::new(),
    }
}

/// GAPSTATS compute: gap frequency and magnitude over trailing 253 sessions.
/// A "gap" is `(open_t - close_{t-1}) / close_{t-1}`. Counts only |gap| > 0.5%
/// as a real gap (avoids counting normal micro-noise).
pub fn compute_gapstats_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> GapStatsSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let window: Vec<&HistoricalPriceRow> = sorted.iter().rev().take(253).rev().copied().collect();
    if window.len() < 20 {
        return GapStatsSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            bias_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} bars", window.len()),
            ..Default::default()
        };
    }
    let mut all_gaps: Vec<f64> = Vec::new();
    let mut up_gaps: Vec<f64> = Vec::new();
    let mut down_gaps: Vec<f64> = Vec::new();
    let mut largest_up = 0.0f64;
    let mut largest_down = 0.0f64;
    for w in window.windows(2) {
        let prev_close = w[0].close;
        let curr_open = w[1].open;
        if prev_close <= 0.0 || curr_open <= 0.0 {
            continue;
        }
        let gap_pct = (curr_open - prev_close) / prev_close * 100.0;
        all_gaps.push(gap_pct);
        if gap_pct > 0.5 {
            up_gaps.push(gap_pct);
            if gap_pct > largest_up {
                largest_up = gap_pct;
            }
        } else if gap_pct < -0.5 {
            down_gaps.push(gap_pct);
            if gap_pct < largest_down {
                largest_down = gap_pct;
            }
        }
    }
    if all_gaps.is_empty() {
        return GapStatsSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            bias_label: "INSUFFICIENT_DATA".into(),
            note: "no usable open/close pairs".into(),
            ..Default::default()
        };
    }
    let avg_all: f64 = all_gaps.iter().sum::<f64>() / all_gaps.len() as f64;
    let avg_up = if up_gaps.is_empty() {
        0.0
    } else {
        up_gaps.iter().sum::<f64>() / up_gaps.len() as f64
    };
    let avg_down = if down_gaps.is_empty() {
        0.0
    } else {
        down_gaps.iter().sum::<f64>() / down_gaps.len() as f64
    };
    let gap_freq = ((up_gaps.len() + down_gaps.len()) as f64) / all_gaps.len() as f64 * 100.0;
    let label = if avg_all <= -0.15 {
        "DOWN_BIAS"
    } else if avg_all <= -0.05 {
        "SLIGHT_DOWN"
    } else if avg_all < 0.05 {
        "NEUTRAL"
    } else if avg_all < 0.15 {
        "SLIGHT_UP"
    } else {
        "UP_BIAS"
    };
    GapStatsSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        gap_up_count: up_gaps.len(),
        gap_down_count: down_gaps.len(),
        avg_gap_pct: avg_all,
        avg_gap_up_pct: avg_up,
        avg_gap_down_pct: avg_down,
        largest_gap_up_pct: largest_up,
        largest_gap_down_pct: largest_down,
        gap_frequency_pct: gap_freq,
        bias_label: label.into(),
        note: String::new(),
    }
}

/// VOLCLUSTER compute: ACF of r² and |r| at lags 1/5/20. Classical ARCH test.
pub fn compute_volcluster_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> VolClusterSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return VolClusterSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            cluster_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} returns", log_rets.len()),
            ..Default::default()
        };
    }
    let sq: Vec<f64> = log_rets.iter().map(|r| r * r).collect();
    let abs: Vec<f64> = log_rets.iter().map(|r| r.abs()).collect();
    let sq1 = acf_at_lag(&sq, 1);
    let sq5 = acf_at_lag(&sq, 5);
    let sq20 = acf_at_lag(&sq, 20);
    let a1 = acf_at_lag(&abs, 1);
    let a5 = acf_at_lag(&abs, 5);
    let a20 = acf_at_lag(&abs, 20);
    let label = if a1 < 0.05 {
        "NONE"
    } else if a1 < 0.15 {
        "MILD"
    } else if a1 < 0.25 {
        "MODERATE"
    } else if a1 < 0.40 {
        "STRONG"
    } else {
        "VERY_STRONG"
    };
    VolClusterSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        sq_acf_lag1: sq1,
        sq_acf_lag5: sq5,
        sq_acf_lag20: sq20,
        abs_acf_lag1: a1,
        abs_acf_lag5: a5,
        abs_acf_lag20: a20,
        cluster_label: label.into(),
        note: String::new(),
    }
}

/// CLOSEPLC compute: average `(close - low) / (high - low)` placement.
pub fn compute_closeplc_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> ClosePlacementSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let window: Vec<&HistoricalPriceRow> = sorted.iter().rev().take(253).rev().copied().collect();
    if window.len() < 20 {
        return ClosePlacementSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: 0,
            placement_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} bars", window.len()),
            ..Default::default()
        };
    }
    let mut positions: Vec<f64> = Vec::new();
    let mut latest_placement = 0.5;
    for bar in &window {
        if bar.high > bar.low {
            let pos = (bar.close - bar.low) / (bar.high - bar.low);
            positions.push(pos);
            latest_placement = pos;
        }
    }
    if positions.len() < 20 {
        return ClosePlacementSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: positions.len(),
            placement_label: "INSUFFICIENT_DATA".into(),
            note: "not enough non-flat bars".into(),
            ..Default::default()
        };
    }
    let avg: f64 = positions.iter().sum::<f64>() / positions.len() as f64;
    let mut sorted_pos = positions.clone();
    sorted_pos.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted_pos, 0.5);
    let near_high =
        positions.iter().filter(|p| **p > 0.8).count() as f64 / positions.len() as f64 * 100.0;
    let near_low =
        positions.iter().filter(|p| **p < 0.2).count() as f64 / positions.len() as f64 * 100.0;
    let label = if avg < 0.3 {
        "STRONG_BEAR"
    } else if avg < 0.45 {
        "BEAR"
    } else if avg < 0.55 {
        "NEUTRAL"
    } else if avg < 0.7 {
        "BULL"
    } else {
        "STRONG_BULL"
    };
    ClosePlacementSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: positions.len(),
        avg_placement: avg,
        median_placement: median,
        latest_placement,
        pct_near_high: near_high,
        pct_near_low: near_low,
        placement_label: label.into(),
        note: String::new(),
    }
}

/// MRHL compute: AR(1) fit `r_t = α + β r_{t-1} + ε` and derive half-life.
pub fn compute_mrhl_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MeanReversionHalfLifeSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return MeanReversionHalfLifeSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            regime_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} returns", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len() - 1;
    let x: Vec<f64> = log_rets[..n].to_vec();
    let y: Vec<f64> = log_rets[1..].to_vec();
    let nf = n as f64;
    let mx: f64 = x.iter().sum::<f64>() / nf;
    let my: f64 = y.iter().sum::<f64>() / nf;
    let mut sxy = 0.0f64;
    let mut sxx = 0.0f64;
    let mut syy = 0.0f64;
    for i in 0..n {
        let dx = x[i] - mx;
        let dy = y[i] - my;
        sxy += dx * dy;
        sxx += dx * dx;
        syy += dy * dy;
    }
    if sxx < f64::EPSILON {
        return MeanReversionHalfLifeSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            regime_label: "INSUFFICIENT_DATA".into(),
            note: "zero variance in lagged series".into(),
            ..Default::default()
        };
    }
    let beta = sxy / sxx;
    let alpha = my - beta * mx;
    let r_squared = if syy > f64::EPSILON {
        (sxy * sxy) / (sxx * syy)
    } else {
        0.0
    };
    let (half_life, label) = if beta <= 0.0 {
        (0.0, "FAST_REVERT")
    } else if beta >= 1.0 {
        (0.0, "INSUFFICIENT_DATA")
    } else {
        let hl = -std::f64::consts::LN_2 / beta.ln();
        let lbl = if beta < 0.15 {
            "MEAN_REVERTING"
        } else if beta < 0.35 {
            "NEUTRAL"
        } else if beta < 0.60 {
            "PERSISTENT"
        } else {
            "STRONG_PERSISTENT"
        };
        (hl, lbl)
    };
    MeanReversionHalfLifeSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        beta,
        alpha,
        half_life_days: half_life,
        r_squared,
        regime_label: label.into(),
        note: String::new(),
    }
}
