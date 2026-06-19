use super::*;

// Omega, DFA, Burke, monthly-seasonality, and roll-spread computes

pub fn compute_omega_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> OmegaRatioSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return OmegaRatioSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            omega_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let mut gains = 0.0_f64;
    let mut losses = 0.0_f64;
    let mut gain_days = 0usize;
    let mut loss_days = 0usize;
    for &r in &log_rets {
        if r > 0.0 {
            gains += r;
            gain_days += 1;
        } else if r < 0.0 {
            losses += -r;
            loss_days += 1;
        }
    }
    let omega = if losses < f64::EPSILON {
        f64::INFINITY
    } else {
        gains / losses
    };
    let total_directional = gain_days + loss_days;
    let win_rate = if total_directional == 0 {
        0.0
    } else {
        gain_days as f64 / total_directional as f64 * 100.0
    };
    let label = if !omega.is_finite() {
        "EXCELLENT"
    } else if omega < 0.5 {
        "VERY_POOR"
    } else if omega < 0.9 {
        "POOR"
    } else if omega < 1.1 {
        "NEUTRAL"
    } else if omega < 1.5 {
        "GOOD"
    } else {
        "EXCELLENT"
    };
    OmegaRatioSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: log_rets.len(),
        gains_sum: gains,
        losses_sum: losses,
        gain_days,
        loss_days,
        omega_ratio: omega,
        win_rate_pct: win_rate,
        omega_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_dfa_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DetrendedFluctuationSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    let n = log_rets.len();
    if n < 100 {
        return DetrendedFluctuationSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            dfa_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥100 returns, got {n}"),
            ..Default::default()
        };
    }
    // Profile Y_i = Σ (r_j - mean)
    let mean = log_rets.iter().sum::<f64>() / n as f64;
    let mut profile: Vec<f64> = Vec::with_capacity(n);
    let mut acc = 0.0_f64;
    for &r in &log_rets {
        acc += r - mean;
        profile.push(acc);
    }
    // Geometric scale grid: s = 8, 10, 12, ..., up to n/4
    let max_s = (n / 4).max(16);
    let mut scales: Vec<usize> = Vec::new();
    let mut s = 8usize;
    while s <= max_s {
        scales.push(s);
        let next = ((s as f64) * 1.3).round() as usize;
        s = if next <= s { s + 1 } else { next };
    }
    if scales.len() < 4 {
        return DetrendedFluctuationSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            dfa_label: "INSUFFICIENT_DATA".into(),
            note: format!("not enough scales: {}", scales.len()),
            ..Default::default()
        };
    }
    // For each scale s: split profile into floor(n/s) non-overlapping windows,
    // linearly detrend each, accumulate squared residuals.
    let mut log_s: Vec<f64> = Vec::new();
    let mut log_f: Vec<f64> = Vec::new();
    for &s in &scales {
        let n_win = n / s;
        if n_win < 2 {
            continue;
        }
        let mut sq_resid_total = 0.0_f64;
        let mut count = 0usize;
        for w in 0..n_win {
            let start = w * s;
            let end = start + s;
            // Linear fit y = a + b*x against x = 0..s-1
            let slen = s as f64;
            let sum_x = (s - 1) as f64 * slen / 2.0;
            let sum_xx: f64 = (0..s).map(|i| (i as f64) * (i as f64)).sum();
            let mut sum_y = 0.0_f64;
            let mut sum_xy = 0.0_f64;
            for i in 0..s {
                let y = profile[start + i];
                sum_y += y;
                sum_xy += (i as f64) * y;
            }
            let denom = slen * sum_xx - sum_x * sum_x;
            if denom.abs() < f64::EPSILON {
                continue;
            }
            let b = (slen * sum_xy - sum_x * sum_y) / denom;
            let a = (sum_y - b * sum_x) / slen;
            for i in 0..s {
                let y = profile[start + i];
                let fitted = a + b * (i as f64);
                let resid = y - fitted;
                sq_resid_total += resid * resid;
                count += 1;
            }
            let _ = end;
        }
        if count == 0 {
            continue;
        }
        let f_s = (sq_resid_total / count as f64).sqrt();
        if f_s < f64::EPSILON {
            continue;
        }
        log_s.push((s as f64).ln());
        log_f.push(f_s.ln());
    }
    if log_s.len() < 4 {
        return DetrendedFluctuationSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            dfa_label: "INSUFFICIENT_DATA".into(),
            note: format!("insufficient scales after filtering: {}", log_s.len()),
            ..Default::default()
        };
    }
    // OLS slope = alpha
    let k = log_s.len() as f64;
    let mean_x = log_s.iter().sum::<f64>() / k;
    let mean_y = log_f.iter().sum::<f64>() / k;
    let mut num = 0.0_f64;
    let mut den = 0.0_f64;
    for i in 0..log_s.len() {
        let dx = log_s[i] - mean_x;
        let dy = log_f[i] - mean_y;
        num += dx * dy;
        den += dx * dx;
    }
    if den < f64::EPSILON {
        return DetrendedFluctuationSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            dfa_label: "INSUFFICIENT_DATA".into(),
            note: "scale variance zero".into(),
            ..Default::default()
        };
    }
    let alpha = num / den;
    // R² of log-log fit
    let mut ss_tot = 0.0_f64;
    let mut ss_res = 0.0_f64;
    let intercept = mean_y - alpha * mean_x;
    for i in 0..log_s.len() {
        let y = log_f[i];
        let dy = y - mean_y;
        ss_tot += dy * dy;
        let pred = intercept + alpha * log_s[i];
        let r = y - pred;
        ss_res += r * r;
    }
    let r_sq = if ss_tot < f64::EPSILON {
        0.0
    } else {
        1.0 - ss_res / ss_tot
    };
    let label = if alpha < 0.35 {
        "ANTI_PERSISTENT"
    } else if alpha < 0.45 {
        "MEAN_REVERTING"
    } else if alpha < 0.55 {
        "RANDOM_WALK"
    } else if alpha < 0.65 {
        "PERSISTENT"
    } else {
        "STRONGLY_PERSISTENT"
    };
    DetrendedFluctuationSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        alpha,
        num_scales: log_s.len(),
        r_squared: r_sq,
        dfa_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_burke_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> BurkeRatioSnapshot {
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
        return BurkeRatioSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            burke_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 bars, got {}", window.len()),
            ..Default::default()
        };
    }
    let first = window.first().map(|b| b.close).unwrap_or(0.0);
    let last = window.last().map(|b| b.close).unwrap_or(0.0);
    if first < f64::EPSILON {
        return BurkeRatioSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            burke_label: "INSUFFICIENT_DATA".into(),
            note: "zero starting price".into(),
            ..Default::default()
        };
    }
    let total_ret = (last / first - 1.0) * 100.0;
    let ann_ret = total_ret * (252.0 / window.len() as f64);
    // Detect distinct drawdown events: from peak → trough → recovery to (or above) peak.
    // Walk prices, track running peak. When price < peak, we are "in drawdown". Track
    // the min drawdown pct within the episode. On recovery to peak (or at end-of-window),
    // close the episode with its trough pct.
    let mut peak = first;
    let mut in_dd = false;
    let mut worst_in_ep = 0.0_f64; // positive %; 0 = not yet in dd
    let mut dd_events: Vec<f64> = Vec::new();
    for b in window.iter().skip(1) {
        let p = b.close;
        if p >= peak {
            if in_dd {
                if worst_in_ep > 0.0 {
                    dd_events.push(worst_in_ep);
                }
                in_dd = false;
                worst_in_ep = 0.0;
            }
            peak = p;
        } else {
            in_dd = true;
            let dd = (peak - p) / peak * 100.0;
            if dd > worst_in_ep {
                worst_in_ep = dd;
            }
        }
    }
    if in_dd && worst_in_ep > 0.0 {
        dd_events.push(worst_in_ep);
    }
    if dd_events.is_empty() {
        let label = if ann_ret > 0.0 {
            "EXCELLENT"
        } else {
            "NEUTRAL"
        };
        return BurkeRatioSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: window.len(),
            annualized_return_pct: ann_ret,
            dd_event_count: 0,
            sum_sq_drawdowns: 0.0,
            worst_event_dd_pct: 0.0,
            burke_ratio: 0.0,
            burke_label: label.into(),
            note: "no drawdown events in window".into(),
        };
    }
    let sum_sq: f64 = dd_events.iter().map(|d| d * d).sum();
    let worst = dd_events.iter().cloned().fold(0.0_f64, f64::max);
    let burke = if sum_sq < f64::EPSILON {
        0.0
    } else {
        ann_ret / sum_sq.sqrt()
    };
    let label = if burke < -0.5 {
        "VERY_POOR"
    } else if burke < 0.0 {
        "POOR"
    } else if burke < 0.5 {
        "NEUTRAL"
    } else if burke < 1.5 {
        "GOOD"
    } else {
        "EXCELLENT"
    };
    BurkeRatioSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: window.len(),
        annualized_return_pct: ann_ret,
        dd_event_count: dd_events.len(),
        sum_sq_drawdowns: sum_sq,
        worst_event_dd_pct: worst,
        burke_ratio: burke,
        burke_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_monthseas_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MonthlySeasonalitySnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 120 {
        return MonthlySeasonalitySnapshot {
            symbol: sym,
            as_of: as_of.into(),
            season_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥120 bars, got {}", bars.len()),
            ..Default::default()
        };
    }
    // Group bars by (year, month). Use last-close-of-month as month close.
    // Month return = (close_this / close_prev - 1) × 100 where prev is prior calendar month's last close.
    use std::collections::BTreeMap;
    let mut month_last: BTreeMap<(i32, u32), f64> = BTreeMap::new();
    for b in bars {
        let d = b.date.as_str();
        if d.len() < 7 {
            continue;
        }
        let year: i32 = match d[0..4].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let month: u32 = match d[5..7].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        if !(1..=12).contains(&month) {
            continue;
        }
        month_last.insert((year, month), b.close);
    }
    if month_last.len() < 13 {
        return MonthlySeasonalitySnapshot {
            symbol: sym,
            as_of: as_of.into(),
            season_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥13 months, got {}", month_last.len()),
            ..Default::default()
        };
    }
    // Walk sorted keys to compute close-to-close month returns. Only pair consecutive calendar months.
    let keys: Vec<(i32, u32)> = month_last.keys().cloned().collect();
    let mut rets_by_month: [Vec<f64>; 12] = Default::default();
    for i in 1..keys.len() {
        let (py, pm) = keys[i - 1];
        let (cy, cm) = keys[i];
        // Consecutive if (cy, cm) follows (py, pm) by 1 month
        let expected_next = if pm == 12 { (py + 1, 1) } else { (py, pm + 1) };
        if (cy, cm) != expected_next {
            continue;
        }
        let prev_c = month_last[&(py, pm)];
        let cur_c = month_last[&(cy, cm)];
        if prev_c < f64::EPSILON {
            continue;
        }
        let r = (cur_c / prev_c - 1.0) * 100.0;
        rets_by_month[(cm as usize) - 1].push(r);
    }
    let total_rets: usize = rets_by_month.iter().map(|v| v.len()).sum();
    if total_rets < 12 {
        return MonthlySeasonalitySnapshot {
            symbol: sym,
            as_of: as_of.into(),
            season_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥12 monthly returns, got {total_rets}"),
            ..Default::default()
        };
    }
    let mut hit_pct = [0.0_f64; 12];
    let mut mean_pct = [0.0_f64; 12];
    for m in 0..12 {
        let v = &rets_by_month[m];
        if v.is_empty() {
            continue;
        }
        let pos = v.iter().filter(|r| **r > 0.0).count();
        hit_pct[m] = pos as f64 / v.len() as f64 * 100.0;
        mean_pct[m] = v.iter().sum::<f64>() / v.len() as f64;
    }
    // Best/worst by hit rate, tiebreak by mean return
    let mut best_idx = 0usize;
    let mut worst_idx = 0usize;
    for m in 1..12 {
        if hit_pct[m] > hit_pct[best_idx]
            || (hit_pct[m] == hit_pct[best_idx] && mean_pct[m] > mean_pct[best_idx])
        {
            best_idx = m;
        }
        if hit_pct[m] < hit_pct[worst_idx]
            || (hit_pct[m] == hit_pct[worst_idx] && mean_pct[m] < mean_pct[worst_idx])
        {
            worst_idx = m;
        }
    }
    let spread = hit_pct[best_idx] - hit_pct[worst_idx];
    let label = if spread >= 40.0 {
        "STRONG_SEASONAL"
    } else if spread >= 25.0 {
        "MILD_SEASONAL"
    } else if spread >= 15.0 {
        "NEUTRAL"
    } else {
        "INCONSISTENT"
    };
    let years = {
        let mut ys: std::collections::BTreeSet<i32> = std::collections::BTreeSet::new();
        for (y, _) in &keys {
            ys.insert(*y);
        }
        ys.len()
    };
    MonthlySeasonalitySnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: bars.len(),
        years_covered: years,
        month_hit_pct: hit_pct,
        month_mean_ret_pct: mean_pct,
        best_month_idx: best_idx,
        worst_month_idx: worst_idx,
        best_month_hit_pct: hit_pct[best_idx],
        worst_month_hit_pct: hit_pct[worst_idx],
        season_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_rollsprd_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> RollSpreadSnapshot {
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
        return RollSpreadSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            roll_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 bars, got {}", window.len()),
            ..Default::default()
        };
    }
    // Δp_t = close_t - close_{t-1}
    let mut dp: Vec<f64> = Vec::with_capacity(window.len());
    for i in 1..window.len() {
        dp.push(window[i].close - window[i - 1].close);
    }
    if dp.len() < 20 {
        return RollSpreadSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            roll_label: "INSUFFICIENT_DATA".into(),
            note: "too few price changes".into(),
            ..Default::default()
        };
    }
    let n = dp.len() as f64;
    let mean_dp = dp.iter().sum::<f64>() / n;
    let mean_price = window.iter().map(|b| b.close).sum::<f64>() / window.len() as f64;
    // Sample cov of consecutive Δp pairs: Cov(Δp_t, Δp_{t-1})
    let m = dp.len();
    if m < 2 {
        return RollSpreadSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            roll_label: "INSUFFICIENT_DATA".into(),
            note: "too few price changes".into(),
            ..Default::default()
        };
    }
    let mut cov_num = 0.0_f64;
    for i in 1..m {
        cov_num += (dp[i] - mean_dp) * (dp[i - 1] - mean_dp);
    }
    let first_lag_cov = cov_num / (m - 1) as f64;
    if first_lag_cov >= 0.0 {
        return RollSpreadSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bars_used: window.len(),
            first_lag_cov,
            mean_price,
            implicit_spread: 0.0,
            implicit_spread_bps: 0.0,
            roll_label: "INVALID_POSITIVE_COV".into(),
            note: "first-lag cov non-negative; Roll model undefined".into(),
        };
    }
    let spread = 2.0 * (-first_lag_cov).sqrt();
    let spread_bps = if mean_price < f64::EPSILON {
        0.0
    } else {
        spread / mean_price * 1e4
    };
    let label = if spread_bps < 10.0 {
        "TIGHT"
    } else if spread_bps < 30.0 {
        "NORMAL"
    } else if spread_bps < 75.0 {
        "WIDE"
    } else {
        "VERY_WIDE"
    };
    RollSpreadSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: window.len(),
        first_lag_cov,
        mean_price,
        implicit_spread: spread,
        implicit_spread_bps: spread_bps,
        roll_label: label.into(),
        note: String::new(),
    }
}
