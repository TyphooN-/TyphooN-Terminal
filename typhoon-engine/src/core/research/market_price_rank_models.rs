use super::*;

// ── Round 20 compute fns ──────────────────────────────────────────

/// DVDYIELDRANK compute: sector percentile rank of the subject's dividend
/// yield. Non-payers (None or 0.0) are filtered so the cohort is
/// dividend-paying names only. Needs ≥3 peers with yield data.
pub fn compute_dvdyieldrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject_yield_pct: Option<f64>,
    peers: &[(String, Option<f64>)],
) -> DividendYieldRankSnapshot {
    let sym = symbol.to_uppercase();
    let subj = match subject_yield_pct {
        Some(y) if y > 0.0 && y.is_finite() => y,
        _ => {
            return DividendYieldRankSnapshot {
                symbol: sym,
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "subject has no dividend yield (non-payer or missing data)".into(),
                ..Default::default()
            };
        }
    };
    let peer_y: Vec<f64> = peers
        .iter()
        .filter(|(s, _)| !s.eq_ignore_ascii_case(symbol))
        .filter_map(|(_, y)| y.filter(|v| *v > 0.0 && v.is_finite()))
        .collect();
    let peers_considered = peers
        .iter()
        .filter(|(s, _)| !s.eq_ignore_ascii_case(symbol))
        .count();
    let peers_with_data = peer_y.len();
    if peers_with_data < 3 {
        return DividendYieldRankSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            dividend_yield_pct: subj,
            peers_considered,
            peers_with_data,
            rank_label: "INSUFFICIENT_DATA".into(),
            note: format!(
                "need ≥3 dividend-paying sector peers, got {}",
                peers_with_data
            ),
            ..Default::default()
        };
    }
    let mut sorted = peer_y.clone();
    sorted.push(subj);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj, &peer_y, true);
    let better = peer_y.iter().filter(|&&p| p > subj).count();
    let label = rank_label_for_percentile(pct);
    DividendYieldRankSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        dividend_yield_pct: subj,
        peers_considered,
        peers_with_data,
        sector_median_yield_pct: median,
        sector_p25_yield_pct: p25,
        sector_p75_yield_pct: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// SHRANK compute: sector percentile rank of short_percent_of_float,
/// risk-inverted so a *lower* short interest earns a *higher* (safer) rank.
pub fn compute_shrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject_short_pct: Option<f64>,
    peers: &[(String, Option<f64>)],
) -> ShortInterestRankSnapshot {
    let sym = symbol.to_uppercase();
    let subj = match subject_short_pct {
        Some(s) if s.is_finite() && s >= 0.0 => s,
        _ => {
            return ShortInterestRankSnapshot {
                symbol: sym,
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "subject missing short_percent_of_float".into(),
                ..Default::default()
            };
        }
    };
    let peer_s: Vec<f64> = peers
        .iter()
        .filter(|(s, _)| !s.eq_ignore_ascii_case(symbol))
        .filter_map(|(_, v)| v.filter(|x| x.is_finite() && *x >= 0.0))
        .collect();
    let peers_considered = peers
        .iter()
        .filter(|(s, _)| !s.eq_ignore_ascii_case(symbol))
        .count();
    let peers_with_data = peer_s.len();
    if peers_with_data < 3 {
        return ShortInterestRankSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            short_pct_of_float: subj,
            peers_considered,
            peers_with_data,
            rank_label: "INSUFFICIENT_DATA".into(),
            note: format!(
                "need ≥3 sector peers with short data, got {}",
                peers_with_data
            ),
            ..Default::default()
        };
    }
    let mut sorted = peer_s.clone();
    sorted.push(subj);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    // Risk-inverted: lower short = safer = higher percentile
    let pct = percentile_rank_score(subj, &peer_s, false);
    // For risk surfaces, rank_position counts peers who are safer (lower short)
    let safer = peer_s.iter().filter(|&&p| p < subj).count();
    let label = risk_rank_label_for_percentile(pct);
    ShortInterestRankSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        short_pct_of_float: subj,
        peers_considered,
        peers_with_data,
        sector_median_short_pct: median,
        sector_p25_short_pct: p25,
        sector_p75_short_pct: p75,
        percentile_rank: pct,
        rank_position: safer + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// ATRANN compute: pure symbol-local 14-period Wilder ATR annualized, with
/// volatility regime label. Uses the most recent 253 sessions from the HP
/// cache, sorted oldest-first.
pub fn compute_atrann_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> AnnualizedAtrSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 15 {
        return AnnualizedAtrSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: bars.len(),
            regime_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥15 bars for 14-period ATR warmup".into(),
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
    // True range for each bar i (i>=1): max(high-low, |high-prev_close|, |low-prev_close|)
    let mut trs: Vec<f64> = Vec::with_capacity(window.len());
    for i in 1..window.len() {
        let h = window[i].high;
        let l = window[i].low;
        let pc = window[i - 1].close;
        if h <= 0.0 || l <= 0.0 || pc <= 0.0 {
            continue;
        }
        let tr = (h - l).max((h - pc).abs()).max((l - pc).abs());
        trs.push(tr);
    }
    if trs.len() < 14 {
        return AnnualizedAtrSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used,
            regime_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} usable TR bars after filtering", trs.len()),
            ..Default::default()
        };
    }
    // Wilder smoothing: seed = mean of first 14, then ATR_i = (prev_ATR × 13 + TR_i) / 14
    let seed: f64 = trs[..14].iter().sum::<f64>() / 14.0;
    let mut atr = seed;
    for &tr in &trs[14..] {
        atr = (atr * 13.0 + tr) / 14.0;
    }
    let latest_close = window.last().map(|r| r.close).unwrap_or(0.0);
    let atr_pct = if latest_close > 0.0 {
        atr / latest_close * 100.0
    } else {
        0.0
    };
    let atr_ann = atr_pct * (252.0f64).sqrt();
    let regime = if atr_ann < 15.0 {
        "LOW_VOL"
    } else if atr_ann < 30.0 {
        "NORMAL_VOL"
    } else if atr_ann < 60.0 {
        "HIGH_VOL"
    } else {
        "EXTREME_VOL"
    };
    AnnualizedAtrSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used,
        latest_close,
        atr14: atr,
        atr14_pct: atr_pct,
        atr_annualized_pct: atr_ann,
        regime_label: regime.into(),
        note: String::new(),
    }
}

/// DDHIST compute: pure symbol-local drawdown history stat. Scans the
/// window for the deepest peak-to-trough, the longest peak-to-recovery
/// duration, the count of 5% and 10% corrections, and the current drawdown.
pub fn compute_ddhist_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DrawdownHistorySnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 20 {
        return DrawdownHistorySnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: bars.len(),
            regime_label: "INSUFFICIENT_DATA".into(),
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
    let mut running_peak = window[0].close;
    let mut running_peak_idx = 0usize;
    let mut running_peak_date = window[0].date.clone();
    let mut max_dd_pct = 0.0f64;
    let mut max_dd_peak_date = String::new();
    let mut max_dd_trough_date = String::new();
    let mut longest_dd_days = 0usize;
    let mut current_dd_start: Option<(usize, String)> = None; // (idx, date) of current peak
    let mut corrections_5 = 0usize;
    let mut corrections_10 = 0usize;
    // Track per-correction state: a "correction" is a run from a local peak to a local trough with ≥5% or ≥10% decline.
    let mut in_correction = false;
    let mut correction_peak = window[0].close;
    for (i, bar) in window.iter().enumerate() {
        let c = bar.close;
        if c <= 0.0 {
            continue;
        }
        if c >= running_peak {
            // Recovered — close the current drawdown bucket.
            if let Some((peak_idx, _)) = &current_dd_start {
                let duration = i - peak_idx;
                if duration > longest_dd_days {
                    longest_dd_days = duration;
                }
            }
            current_dd_start = None;
            running_peak = c;
            running_peak_idx = i;
            running_peak_date = bar.date.clone();
            // Close any open correction by measuring its depth.
            if in_correction {
                let depth = (c - correction_peak) / correction_peak * 100.0;
                let abs_depth = -depth; // positive number
                if abs_depth >= 10.0 {
                    corrections_10 += 1;
                }
                if abs_depth >= 5.0 {
                    corrections_5 += 1;
                }
                in_correction = false;
            }
            correction_peak = c;
        } else {
            // In a drawdown.
            if current_dd_start.is_none() {
                current_dd_start = Some((running_peak_idx, running_peak_date.clone()));
            }
            let dd = (c - running_peak) / running_peak * 100.0; // negative
            if dd < max_dd_pct {
                max_dd_pct = dd;
                max_dd_peak_date = running_peak_date.clone();
                max_dd_trough_date = bar.date.clone();
            }
            // Correction tracking: local-peak-to-trough.
            if c < correction_peak {
                in_correction = true;
            }
        }
    }
    // If we ended the window still in a drawdown, count its duration.
    if let Some((peak_idx, _)) = &current_dd_start {
        let duration = window.len().saturating_sub(*peak_idx);
        if duration > longest_dd_days {
            longest_dd_days = duration;
        }
    }
    // Close any still-open correction (open means we ended the window below the local peak).
    if in_correction {
        let last = window.last().map(|r| r.close).unwrap_or(correction_peak);
        let abs_depth = (correction_peak - last) / correction_peak * 100.0;
        if abs_depth >= 10.0 {
            corrections_10 += 1;
        }
        if abs_depth >= 5.0 {
            corrections_5 += 1;
        }
    }
    let latest = window.last().map(|r| r.close).unwrap_or(0.0);
    let current_dd = if latest > 0.0 && running_peak > 0.0 {
        (latest - running_peak) / running_peak * 100.0
    } else {
        0.0
    };
    let regime = if current_dd > -1.0 {
        "RECOVERING"
    } else if max_dd_pct > -10.0 {
        "SHALLOW"
    } else if max_dd_pct > -20.0 {
        "MEANINGFUL"
    } else if max_dd_pct > -35.0 {
        "SEVERE"
    } else {
        "CATASTROPHIC"
    };
    DrawdownHistorySnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used,
        max_drawdown_pct: max_dd_pct,
        max_drawdown_peak_date: max_dd_peak_date,
        max_drawdown_trough_date: max_dd_trough_date,
        longest_drawdown_days: longest_dd_days,
        corrections_5pct: corrections_5,
        corrections_10pct: corrections_10,
        current_drawdown_pct: current_dd,
        regime_label: regime.into(),
        note: String::new(),
    }
}

/// PRICEPERF compute: multi-horizon total return stat. Computes returns
/// over trailing 21, 63, 126, and 253 sessions plus YTD from the first
/// session of as_of's calendar year.
pub fn compute_priceperf_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> PricePerformanceSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 2 {
        return PricePerformanceSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: bars.len(),
            trend_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥2 bars".into(),
            ..Default::default()
        };
    }
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let bars_used = sorted.len();
    let latest = sorted.last().unwrap();
    let latest_close = latest.close;
    if latest_close <= 0.0 {
        return PricePerformanceSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used,
            trend_label: "INSUFFICIENT_DATA".into(),
            note: "latest close not positive".into(),
            ..Default::default()
        };
    }
    let ret_at = |offset: usize| -> f64 {
        if sorted.len() > offset {
            let past = sorted[sorted.len() - 1 - offset].close;
            if past > 0.0 {
                (latest_close - past) / past * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        }
    };
    let ret_1m = ret_at(21);
    let ret_3m = ret_at(63);
    let ret_6m = ret_at(126);
    let ret_1y = ret_at(253);
    // YTD: find first bar with date.year == latest.date.year
    let year_prefix = latest.date.get(..4).unwrap_or("");
    let ytd_ret = if !year_prefix.is_empty() {
        let ytd_start = sorted.iter().find(|r| r.date.starts_with(year_prefix));
        match ytd_start {
            Some(start_bar) if start_bar.close > 0.0 => {
                (latest_close - start_bar.close) / start_bar.close * 100.0
            }
            _ => 0.0,
        }
    } else {
        0.0
    };
    let trend = if bars_used < 20 {
        "INSUFFICIENT_DATA"
    } else if ret_1y > 30.0 && ret_3m > 10.0 {
        "STRONG_BULL"
    } else if ret_1y > 10.0 || ret_3m > 5.0 {
        "BULL"
    } else if ret_1y < -30.0 && ret_3m < -10.0 {
        "STRONG_BEAR"
    } else if ret_1y < -10.0 || ret_3m < -5.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    PricePerformanceSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used,
        latest_close,
        ret_1m_pct: ret_1m,
        ret_3m_pct: ret_3m,
        ret_6m_pct: ret_6m,
        ret_ytd_pct: ytd_ret,
        ret_1y_pct: ret_1y,
        trend_label: trend.into(),
        note: String::new(),
    }
}

// ── Round 21 compute fns ──

/// BETARANK compute: sector percentile rank of Fundamentals.beta,
/// risk-inverted so a *lower* beta earns a *higher* (safer) rank.
pub fn compute_betarank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject_beta: Option<f64>,
    peers: &[(String, Option<f64>)],
) -> BetaRankSnapshot {
    let sym = symbol.to_uppercase();
    let subj = match subject_beta {
        Some(b) if b.is_finite() => b,
        _ => {
            return BetaRankSnapshot {
                symbol: sym,
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "subject missing beta".into(),
                ..Default::default()
            };
        }
    };
    let peer_b: Vec<f64> = peers
        .iter()
        .filter(|(s, _)| !s.eq_ignore_ascii_case(symbol))
        .filter_map(|(_, v)| v.filter(|x| x.is_finite()))
        .collect();
    let peers_considered = peers
        .iter()
        .filter(|(s, _)| !s.eq_ignore_ascii_case(symbol))
        .count();
    let peers_with_data = peer_b.len();
    if peers_with_data < 3 {
        return BetaRankSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            subject_beta: Some(subj),
            peers_considered,
            peers_with_data,
            rank_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 sector peers with beta, got {}", peers_with_data),
            ..Default::default()
        };
    }
    let mut sorted = peer_b.clone();
    sorted.push(subj);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    // Risk-inverted: lower beta = safer = higher percentile
    let pct = percentile_rank_score(subj, &peer_b, false);
    let safer = peer_b.iter().filter(|&&p| p < subj).count();
    let label = risk_rank_label_for_percentile(pct);
    BetaRankSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        subject_beta: Some(subj),
        peers_considered,
        peers_with_data,
        sector_median_beta: median,
        sector_p25_beta: p25,
        sector_p75_beta: p75,
        percentile_rank: pct,
        rank_position: safer + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// PEGRANK compute: sector percentile rank of Fundamentals.peg_ratio,
/// value-inverted so a *lower* PEG (cheaper growth) earns a *higher* rank.
/// Filters out non-positive or non-finite PEG on both subject and peer sides.
pub fn compute_pegrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject_peg: Option<f64>,
    peers: &[(String, Option<f64>)],
) -> PegRankSnapshot {
    let sym = symbol.to_uppercase();
    let subj = match subject_peg {
        Some(p) if p > 0.0 && p.is_finite() => p,
        _ => {
            return PegRankSnapshot {
                symbol: sym,
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "subject has no valid PEG (negative or missing)".into(),
                ..Default::default()
            };
        }
    };
    let peer_p: Vec<f64> = peers
        .iter()
        .filter(|(s, _)| !s.eq_ignore_ascii_case(symbol))
        .filter_map(|(_, v)| v.filter(|x| *x > 0.0 && x.is_finite()))
        .collect();
    let peers_considered = peers
        .iter()
        .filter(|(s, _)| !s.eq_ignore_ascii_case(symbol))
        .count();
    let peers_with_data = peer_p.len();
    if peers_with_data < 3 {
        return PegRankSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            subject_peg: Some(subj),
            peers_considered,
            peers_with_data,
            rank_label: "INSUFFICIENT_DATA".into(),
            note: format!(
                "need ≥3 sector peers with positive PEG, got {}",
                peers_with_data
            ),
            ..Default::default()
        };
    }
    let mut sorted = peer_p.clone();
    sorted.push(subj);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    // Value-inverted: lower PEG = better value = higher percentile
    let pct = percentile_rank_score(subj, &peer_p, false);
    let better = peer_p.iter().filter(|&&p| p < subj).count();
    let label = rank_label_for_percentile(pct);
    PegRankSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        subject_peg: Some(subj),
        peers_considered,
        peers_with_data,
        sector_median_peg: median,
        sector_p25_peg: p25,
        sector_p75_peg: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// FHIGHLOW compute: 52-week high/low distance stat over cached HP bars.
/// Takes the trailing 253 sessions, tracks max close + min close + dates,
/// computes distance-from-high/low and range position, and emits a
/// proximity label band.
pub fn compute_fhighlow_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> FiftyTwoWeekHighLowSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 2 {
        return FiftyTwoWeekHighLowSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: bars.len(),
            proximity_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥2 bars".into(),
            ..Default::default()
        };
    }
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    // Trailing 253 sessions only.
    let window: Vec<&&HistoricalPriceRow> = sorted.iter().rev().take(253).collect();
    let bars_used = window.len();
    let latest = *window[0];
    let latest_close = latest.close;
    if latest_close <= 0.0 {
        return FiftyTwoWeekHighLowSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used,
            proximity_label: "INSUFFICIENT_DATA".into(),
            note: "latest close not positive".into(),
            ..Default::default()
        };
    }
    let mut high = latest_close;
    let mut high_date = latest.date.clone();
    let mut high_idx: usize = 0; // index from latest (0 = most recent)
    let mut low = latest_close;
    let mut low_date = latest.date.clone();
    let mut low_idx: usize = 0;
    for (i, row) in window.iter().enumerate() {
        if row.close > 0.0 {
            if row.close > high {
                high = row.close;
                high_date = row.date.clone();
                high_idx = i;
            }
            if row.close < low {
                low = row.close;
                low_date = row.date.clone();
                low_idx = i;
            }
        }
    }
    let pct_from_high = if high > 0.0 {
        (latest_close - high) / high * 100.0
    } else {
        0.0
    };
    let pct_from_low = if low > 0.0 {
        (latest_close - low) / low * 100.0
    } else {
        0.0
    };
    let range = high - low;
    let range_position = if range > 0.0 {
        (latest_close - low) / range * 100.0
    } else {
        50.0
    };
    let proximity = if bars_used < 20 {
        "INSUFFICIENT_DATA"
    } else if range_position >= 98.0 {
        "AT_HIGH"
    } else if range_position >= 80.0 {
        "NEAR_HIGH"
    } else if range_position >= 20.0 {
        "MID_RANGE"
    } else if range_position >= 2.0 {
        "NEAR_LOW"
    } else {
        "AT_LOW"
    };
    FiftyTwoWeekHighLowSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used,
        latest_close,
        high_52w: high,
        high_52w_date: high_date,
        days_since_high: high_idx,
        low_52w: low,
        low_52w_date: low_date,
        days_since_low: low_idx,
        pct_from_high,
        pct_from_low,
        range_position_pct: range_position,
        proximity_label: proximity.into(),
        note: String::new(),
    }
}

/// RVCONE compute: multi-horizon annualized realized volatility over the
/// HP cache, plus a rolling 20d RV percentile cone label.
pub fn compute_rvcone_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> RealizedVolConeSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 21 {
        return RealizedVolConeSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: bars.len(),
            cone_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥21 bars for 20-session realized vol".into(),
            ..Default::default()
        };
    }
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let bars_used = sorted.len();
    let latest_close = sorted.last().unwrap().close;
    if latest_close <= 0.0 {
        return RealizedVolConeSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used,
            cone_label: "INSUFFICIENT_DATA".into(),
            note: "latest close not positive".into(),
            ..Default::default()
        };
    }
    // Log returns.
    let mut log_rets: Vec<f64> = Vec::with_capacity(sorted.len());
    for w in sorted.windows(2) {
        let prev = w[0].close;
        let curr = w[1].close;
        if prev > 0.0 && curr > 0.0 {
            log_rets.push((curr / prev).ln());
        }
    }
    if log_rets.len() < 20 {
        return RealizedVolConeSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used,
            cone_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    // Annualized realized vol of trailing n returns, as percent.
    let ann_rv = |n: usize| -> f64 {
        if log_rets.len() < n {
            return 0.0;
        }
        let slice = &log_rets[log_rets.len() - n..];
        let mean: f64 = slice.iter().sum::<f64>() / n as f64;
        let var: f64 = slice.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n as f64;
        var.sqrt() * (252.0_f64).sqrt() * 100.0
    };
    let rv20 = ann_rv(20);
    let rv60 = ann_rv(60);
    let rv120 = ann_rv(120);
    let rv252 = ann_rv(252);
    // Rolling 20d RV distribution across the full return window.
    let mut rolling20: Vec<f64> = Vec::new();
    if log_rets.len() >= 20 {
        for end in 20..=log_rets.len() {
            let slice = &log_rets[end - 20..end];
            let mean: f64 = slice.iter().sum::<f64>() / 20.0;
            let var: f64 = slice.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / 20.0;
            rolling20.push(var.sqrt() * (252.0_f64).sqrt() * 100.0);
        }
    }
    let (rv20_min, rv20_med, rv20_max, rv20_pct) = if !rolling20.is_empty() {
        let mut sorted_r = rolling20.clone();
        sorted_r.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let min = *sorted_r.first().unwrap();
        let max = *sorted_r.last().unwrap();
        let med = quantile_f64(&sorted_r, 0.5);
        // Percentile of latest rv20 within the historical rolling distribution.
        let others: Vec<f64> = rolling20
            .iter()
            .take(rolling20.len() - 1)
            .copied()
            .collect();
        let pct = if others.is_empty() {
            50.0
        } else {
            percentile_rank_score(rv20, &others, true)
        };
        (min, med, max, pct)
    } else {
        (rv20, rv20, rv20, 50.0)
    };
    let cone = if rolling20.len() < 20 {
        "INSUFFICIENT_DATA"
    } else if rv20_pct >= 90.0 {
        "EXTREME"
    } else if rv20_pct >= 70.0 {
        "ELEVATED"
    } else if rv20_pct >= 30.0 {
        "TYPICAL"
    } else if rv20_pct >= 10.0 {
        "BELOW_AVG"
    } else {
        "COMPRESSED"
    };
    RealizedVolConeSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used,
        latest_close,
        rv20_pct: rv20,
        rv60_pct: rv60,
        rv120_pct: rv120,
        rv252_pct: rv252,
        rv20_min_pct: rv20_min,
        rv20_median_pct: rv20_med,
        rv20_max_pct: rv20_max,
        rv20_percentile: rv20_pct,
        cone_label: cone.into(),
        note: String::new(),
    }
}

/// CALPB compute: calendar-aligned period breakdowns over the HP cache.
/// Uses year-prefix / month-prefix string matching on `date` (assumes
/// ISO-8601 YYYY-MM-DD), like PRICEPERF's YTD shortcut.
pub fn compute_calpb_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CalendarPeriodBreakdownSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 2 {
        return CalendarPeriodBreakdownSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: bars.len(),
            momentum_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥2 bars".into(),
            ..Default::default()
        };
    }
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let bars_used = sorted.len();
    let latest = sorted.last().unwrap();
    let latest_close = latest.close;
    if latest_close <= 0.0 {
        return CalendarPeriodBreakdownSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used,
            momentum_label: "INSUFFICIENT_DATA".into(),
            note: "latest close not positive".into(),
            ..Default::default()
        };
    }
    // Parse latest date as YYYY-MM-DD.
    let year: i32 = latest
        .date
        .get(..4)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let month: u32 = latest
        .date
        .get(5..7)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    if year == 0 || month == 0 {
        return CalendarPeriodBreakdownSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used,
            momentum_label: "INSUFFICIENT_DATA".into(),
            note: "cannot parse latest bar date".into(),
            ..Default::default()
        };
    }
    let quarter = ((month - 1) / 3) + 1; // 1..=4
    let q_first_month = ((quarter - 1) * 3) + 1;
    // Helpers.
    let pct_from_first_in = |prefix: &str| -> f64 {
        if let Some(start) = sorted.iter().find(|r| r.date.starts_with(prefix)) {
            if start.close > 0.0 {
                return (latest_close - start.close) / start.close * 100.0;
            }
        }
        0.0
    };
    let full_period_return = |start_prefix: &str, end_prefix: &str| -> f64 {
        let first = sorted.iter().find(|r| r.date.starts_with(start_prefix));
        let last = sorted.iter().rev().find(|r| r.date.starts_with(end_prefix));
        match (first, last) {
            (Some(a), Some(b)) if a.close > 0.0 && b.close > 0.0 => {
                (b.close - a.close) / a.close * 100.0
            }
            _ => 0.0,
        }
    };
    // MTD — bars with year-month prefix matching latest.
    let ym_prefix = format!("{:04}-{:02}", year, month);
    let mtd = pct_from_first_in(&ym_prefix);
    // QTD — bars from q_first_month of current year onwards.
    // Use inclusion filter across the 3 month-prefixes in current quarter.
    let qtd = {
        let q_prefixes: Vec<String> = (0..3)
            .map(|i| format!("{:04}-{:02}", year, q_first_month + i))
            .collect();
        let first = sorted
            .iter()
            .find(|r| q_prefixes.iter().any(|p| r.date.starts_with(p)));
        match first {
            Some(bar) if bar.close > 0.0 => (latest_close - bar.close) / bar.close * 100.0,
            _ => 0.0,
        }
    };
    // YTD — first bar of current year to latest.
    let y_prefix = format!("{:04}", year);
    let ytd = pct_from_first_in(&y_prefix);
    // Prior quarter — full return over the quarter before the current one.
    let (prior_q_year, prior_q) = if quarter == 1 {
        (year - 1, 4u32)
    } else {
        (year, quarter - 1)
    };
    let prior_q_first_month = ((prior_q - 1) * 3) + 1;
    let prior_q_prefixes: Vec<String> = (0..3)
        .map(|i| format!("{:04}-{:02}", prior_q_year, prior_q_first_month + i))
        .collect();
    let prior_quarter = {
        let first = sorted
            .iter()
            .find(|r| prior_q_prefixes.iter().any(|p| r.date.starts_with(p)));
        let last = sorted
            .iter()
            .rev()
            .find(|r| prior_q_prefixes.iter().any(|p| r.date.starts_with(p)));
        match (first, last) {
            (Some(a), Some(b)) if a.close > 0.0 && b.close > 0.0 => {
                (b.close - a.close) / a.close * 100.0
            }
            _ => 0.0,
        }
    };
    // Prior year — full-year return of year-1.
    let prior_year_str = format!("{:04}", year - 1);
    let prior_year = full_period_return(&prior_year_str, &prior_year_str);
    // Momentum label: compare QTD vs prior_quarter.
    let momentum = if bars_used < 20 {
        "INSUFFICIENT_DATA"
    } else if qtd > prior_quarter + 5.0 && qtd > 0.0 {
        "ACCELERATING"
    } else if (qtd - prior_quarter).abs() <= 5.0 {
        "STEADY"
    } else if qtd < prior_quarter - 5.0 && qtd < 0.0 && prior_quarter < 0.0 {
        "DECELERATING"
    } else if qtd.signum() != prior_quarter.signum() && prior_quarter != 0.0 {
        "REVERSING"
    } else {
        "DECELERATING"
    };
    CalendarPeriodBreakdownSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used,
        latest_close,
        mtd_pct: mtd,
        qtd_pct: qtd,
        ytd_pct: ytd,
        prior_quarter_pct: prior_quarter,
        prior_year_pct: prior_year,
        current_year: format!("{:04}", year),
        current_quarter: format!("Q{}", quarter),
        momentum_label: momentum.into(),
        note: String::new(),
    }
}

/// MOMRANK_MULTI compute: sector-relative momentum rank using cached
/// PRICEPERF snapshots. Each horizon is percentile-ranked against peers, then
/// blended into a composite percentile.
pub fn compute_momrank_multi_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&PricePerformanceSnapshot>,
    peers: &[(String, Option<PricePerformanceSnapshot>)],
) -> MomentumRankMultiSnapshot {
    #[derive(Clone)]
    struct Row {
        symbol: String,
        ret_1m: f64,
        ret_3m: f64,
        ret_6m: f64,
        ret_ytd: f64,
        ret_1y: f64,
    }

    let sym = symbol.to_uppercase();
    let Some(subject) = subject else {
        return MomentumRankMultiSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            rank_label: "NO_DATA".into(),
            note: "subject missing PRICEPERF snapshot".into(),
            ..Default::default()
        };
    };

    let mut rows: Vec<Row> = Vec::new();
    rows.push(Row {
        symbol: sym.clone(),
        ret_1m: subject.ret_1m_pct,
        ret_3m: subject.ret_3m_pct,
        ret_6m: subject.ret_6m_pct,
        ret_ytd: subject.ret_ytd_pct,
        ret_1y: subject.ret_1y_pct,
    });
    let peers_considered = peers
        .iter()
        .filter(|(peer_sym, _)| !peer_sym.eq_ignore_ascii_case(symbol))
        .count();

    for (peer_sym, snap) in peers {
        if peer_sym.eq_ignore_ascii_case(symbol) {
            continue;
        }
        let Some(snap) = snap.as_ref() else {
            continue;
        };
        rows.push(Row {
            symbol: peer_sym.to_uppercase(),
            ret_1m: snap.ret_1m_pct,
            ret_3m: snap.ret_3m_pct,
            ret_6m: snap.ret_6m_pct,
            ret_ytd: snap.ret_ytd_pct,
            ret_1y: snap.ret_1y_pct,
        });
    }
    let peers_with_data = rows.len().saturating_sub(1);
    if peers_with_data < 3 {
        return MomentumRankMultiSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            peers_considered,
            peers_with_data,
            ret_1m_pct: subject.ret_1m_pct,
            ret_3m_pct: subject.ret_3m_pct,
            ret_6m_pct: subject.ret_6m_pct,
            ret_ytd_pct: subject.ret_ytd_pct,
            ret_1y_pct: subject.ret_1y_pct,
            rank_label: "INSUFFICIENT_DATA".into(),
            note: format!(
                "need ≥3 sector peers with PRICEPERF, got {}",
                peers_with_data
            ),
            ..Default::default()
        };
    }

    let mut composite_rows: Vec<(String, f64)> = Vec::with_capacity(rows.len());
    let mut subject_pct = (0.0, 0.0, 0.0, 0.0, 0.0);
    for row in &rows {
        let p1 = percentile_rank_score(
            row.ret_1m,
            &rows
                .iter()
                .filter(|r| r.symbol != row.symbol)
                .map(|r| r.ret_1m)
                .collect::<Vec<_>>(),
            true,
        );
        let p3 = percentile_rank_score(
            row.ret_3m,
            &rows
                .iter()
                .filter(|r| r.symbol != row.symbol)
                .map(|r| r.ret_3m)
                .collect::<Vec<_>>(),
            true,
        );
        let p6 = percentile_rank_score(
            row.ret_6m,
            &rows
                .iter()
                .filter(|r| r.symbol != row.symbol)
                .map(|r| r.ret_6m)
                .collect::<Vec<_>>(),
            true,
        );
        let pytd = percentile_rank_score(
            row.ret_ytd,
            &rows
                .iter()
                .filter(|r| r.symbol != row.symbol)
                .map(|r| r.ret_ytd)
                .collect::<Vec<_>>(),
            true,
        );
        let p1y = percentile_rank_score(
            row.ret_1y,
            &rows
                .iter()
                .filter(|r| r.symbol != row.symbol)
                .map(|r| r.ret_1y)
                .collect::<Vec<_>>(),
            true,
        );
        let composite = p1 * 0.15 + p3 * 0.20 + p6 * 0.25 + pytd * 0.15 + p1y * 0.25;
        if row.symbol.eq_ignore_ascii_case(&sym) {
            subject_pct = (p1, p3, p6, pytd, p1y);
        }
        composite_rows.push((row.symbol.clone(), composite));
    }
    composite_rows.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });

    let composite_percentile = composite_rows
        .iter()
        .find(|(row_sym, _)| row_sym.eq_ignore_ascii_case(&sym))
        .map(|(_, score)| *score)
        .unwrap_or(50.0);
    let rank_position = composite_rows
        .iter()
        .position(|(row_sym, _)| row_sym.eq_ignore_ascii_case(&sym))
        .map(|i| i + 1)
        .unwrap_or(0);
    let horizons_above_median = [
        subject_pct.0,
        subject_pct.1,
        subject_pct.2,
        subject_pct.3,
        subject_pct.4,
    ]
    .iter()
    .filter(|&&pct| pct >= 50.0)
    .count();

    MomentumRankMultiSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        peers_considered,
        peers_with_data,
        ret_1m_pct: subject.ret_1m_pct,
        ret_3m_pct: subject.ret_3m_pct,
        ret_6m_pct: subject.ret_6m_pct,
        ret_ytd_pct: subject.ret_ytd_pct,
        ret_1y_pct: subject.ret_1y_pct,
        pct_1m: subject_pct.0,
        pct_3m: subject_pct.1,
        pct_6m: subject_pct.2,
        pct_ytd: subject_pct.3,
        pct_1y: subject_pct.4,
        composite_percentile,
        horizons_above_median,
        rank_position,
        rank_label: rank_label_for_percentile(composite_percentile).into(),
        note: String::new(),
    }
}

/// CORRSTK compute: rolling correlation of a symbol's daily log returns vs
/// SPY and an optional sector ETF benchmark.
pub fn compute_corrstk_snapshot(
    symbol: &str,
    as_of: &str,
    symbol_sector: &str,
    market_benchmark: &str,
    subject_bars: &[HistoricalPriceRow],
    market_bars: &[HistoricalPriceRow],
    sector_benchmark: Option<&str>,
    sector_bars: &[HistoricalPriceRow],
) -> CorrStkSnapshot {
    let sym = symbol.to_uppercase();
    let spy_pairs = aligned_log_return_pairs(subject_bars, market_bars);
    let sector_pairs = if sector_benchmark.is_some() {
        aligned_log_return_pairs(subject_bars, sector_bars)
    } else {
        Vec::new()
    };

    let (corr_spy_20d, _, _, overlaps_spy_20d) = rolling_corr_stats(&spy_pairs, 20);
    let (corr_spy_60d, _, _, overlaps_spy_60d) = rolling_corr_stats(&spy_pairs, 60);
    let (corr_spy_252d, beta_spy_252d, r_squared_spy_252d, overlaps_spy_252d) =
        rolling_corr_stats(&spy_pairs, 252);
    let (corr_sector_20d, _, _, overlaps_sector_20d) = rolling_corr_stats(&sector_pairs, 20);
    let (corr_sector_60d, _, _, overlaps_sector_60d) = rolling_corr_stats(&sector_pairs, 60);
    let (corr_sector_252d, beta_sector_252d, r_squared_sector_252d, overlaps_sector_252d) =
        rolling_corr_stats(&sector_pairs, 252);

    let sector_ticker = sector_benchmark.unwrap_or("").to_string();
    let (dominant_benchmark, correlation_label, note) =
        if overlaps_spy_252d < 20 && overlaps_sector_252d < 20 {
            (
                "NONE".to_string(),
                "INSUFFICIENT_DATA".to_string(),
                "need ≥20 overlapping benchmark returns".to_string(),
            )
        } else {
            let use_sector =
                overlaps_sector_252d >= 20 && corr_sector_252d.abs() > corr_spy_252d.abs() + 0.05;
            let dominant = if use_sector {
                sector_ticker.clone()
            } else {
                market_benchmark.to_string()
            };
            let dominant_corr = if use_sector {
                corr_sector_252d
            } else {
                corr_spy_252d
            };
            let label = if dominant_corr <= -0.70 {
                if use_sector {
                    "INVERSE_SECTOR"
                } else {
                    "INVERSE_INDEX"
                }
            } else if dominant_corr >= 0.75 {
                if use_sector {
                    "SECTOR_LOCKSTEP"
                } else {
                    "INDEX_LOCKSTEP"
                }
            } else if corr_spy_252d.abs() >= 0.55 || corr_sector_252d.abs() >= 0.55 {
                "MIXED"
            } else {
                "DIVERGENT"
            };
            (dominant, label.to_string(), String::new())
        };

    CorrStkSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        symbol_sector: symbol_sector.to_string(),
        market_benchmark: market_benchmark.to_string(),
        sector_benchmark: sector_ticker,
        overlaps_spy_20d,
        overlaps_spy_60d,
        overlaps_spy_252d,
        overlaps_sector_20d,
        overlaps_sector_60d,
        overlaps_sector_252d,
        corr_spy_20d,
        corr_spy_60d,
        corr_spy_252d,
        beta_spy_252d,
        r_squared_spy_252d,
        corr_sector_20d,
        corr_sector_60d,
        corr_sector_252d,
        beta_sector_252d,
        r_squared_sector_252d,
        dominant_benchmark,
        correlation_label,
        note,
    }
}

fn liquidity_tier_for_avg_dollar_volume(avg_dollar: f64) -> &'static str {
    if avg_dollar >= 5.0e8 {
        "DEEP"
    } else if avg_dollar >= 5.0e7 {
        "LIQUID"
    } else if avg_dollar >= 5.0e6 {
        "MODERATE"
    } else if avg_dollar >= 5.0e5 {
        "THIN"
    } else {
        "ILLIQUID"
    }
}

fn trailing_avg_dollar_volume_window(
    bars: &[HistoricalPriceRow],
    window_days: usize,
) -> Option<(f64, usize)> {
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let window: Vec<&HistoricalPriceRow> =
        sorted.into_iter().rev().take(window_days.max(20)).collect();
    let dollar_vols: Vec<f64> = window
        .iter()
        .filter_map(|b| {
            if b.close > 0.0 && b.volume > 0.0 {
                Some(b.close * b.volume)
            } else {
                None
            }
        })
        .collect();
    if dollar_vols.len() < 20 {
        None
    } else {
        Some((
            dollar_vols.iter().sum::<f64>() / dollar_vols.len() as f64,
            dollar_vols.len(),
        ))
    }
}

/// TLRANK — rank trailing 30-session average dollar volume vs sector peers.
pub fn compute_tlrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject_bars: &[HistoricalPriceRow],
    peers: &[(String, Vec<HistoricalPriceRow>)],
) -> ThirtyDayLiquidityRankSnapshot {
    let sym = symbol.to_uppercase();
    let (subject_adv, bars_used) = match trailing_avg_dollar_volume_window(subject_bars, 30) {
        Some(v) => v,
        None => {
            return ThirtyDayLiquidityRankSnapshot {
                symbol: sym,
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                window_days: 30,
                rank_label: "NO_DATA".into(),
                note: "Need ≥20 valid historical-price bars for the subject".into(),
                ..Default::default()
            };
        }
    };

    let peer_advs: Vec<f64> = peers
        .iter()
        .filter_map(|(_, bars)| trailing_avg_dollar_volume_window(bars, 30).map(|(adv, _)| adv))
        .collect();
    let peers_considered = peers.len();
    let peers_with_data = peer_advs.len();
    if peer_advs.len() < 3 {
        return ThirtyDayLiquidityRankSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            window_days: 30,
            bars_used,
            avg_30d_dollar_volume: subject_adv,
            tier_label: liquidity_tier_for_avg_dollar_volume(subject_adv).into(),
            peers_considered,
            peers_with_data,
            rank_label: "NO_DATA".into(),
            note: format!(
                "Only {} sector peers have ≥20 valid bars for trailing ADV$ (need ≥3)",
                peer_advs.len()
            ),
            ..Default::default()
        };
    }

    let mut sorted = peer_advs.clone();
    sorted.push(subject_adv);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let pct = percentile_rank_score(subject_adv, &peer_advs, true);
    let rank_position = peer_advs.iter().filter(|&&adv| adv > subject_adv).count() + 1;

    ThirtyDayLiquidityRankSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        window_days: 30,
        bars_used,
        avg_30d_dollar_volume: subject_adv,
        tier_label: liquidity_tier_for_avg_dollar_volume(subject_adv).into(),
        peers_considered,
        peers_with_data,
        sector_median_dollar_volume: quantile_f64(&sorted, 0.5),
        sector_p25_dollar_volume: quantile_f64(&sorted, 0.25),
        sector_p75_dollar_volume: quantile_f64(&sorted, 0.75),
        percentile_rank: pct,
        rank_position,
        rank_label: rank_label_for_percentile(pct).into(),
        note: String::new(),
    }
}

fn corrrank_metric_for_snapshot(
    snap: &CorrStkSnapshot,
    use_sector_benchmark: bool,
    benchmark_name: &str,
) -> Option<(f64, f64, f64)> {
    if use_sector_benchmark {
        if benchmark_name.is_empty()
            || snap.sector_benchmark.is_empty()
            || !snap.sector_benchmark.eq_ignore_ascii_case(benchmark_name)
            || snap.overlaps_sector_252d < 20
            || !snap.corr_sector_252d.is_finite()
        {
            None
        } else {
            Some((
                snap.corr_sector_252d,
                snap.beta_sector_252d,
                snap.r_squared_sector_252d,
            ))
        }
    } else if snap.market_benchmark.eq_ignore_ascii_case(benchmark_name)
        && snap.overlaps_spy_252d >= 20
        && snap.corr_spy_252d.is_finite()
    {
        Some((
            snap.corr_spy_252d,
            snap.beta_spy_252d,
            snap.r_squared_spy_252d,
        ))
    } else {
        None
    }
}

/// CORRRANK — rank one symbol's benchmark linkage vs same-sector peers.
///
/// The subject chooses one benchmark basis from its cached CORRSTK row:
/// dominant sector ETF when available and valid, otherwise the market
/// benchmark (usually SPY). Peers are then ranked on the same 252d absolute
/// correlation basis.
pub fn compute_corrrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&CorrStkSnapshot>,
    peers: &[&CorrStkSnapshot],
) -> CorrelationRankSnapshot {
    let sym = symbol.to_uppercase();
    let subj = match subject {
        Some(s) if s.correlation_label != "INSUFFICIENT_DATA" => s,
        _ => {
            return CorrelationRankSnapshot {
                symbol: sym,
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No cached CORRSTK snapshot for subject".into(),
                ..Default::default()
            };
        }
    };

    let (benchmark_name, benchmark_kind, use_sector_benchmark) =
        if !subj.sector_benchmark.is_empty()
            && subj
                .dominant_benchmark
                .eq_ignore_ascii_case(&subj.sector_benchmark)
            && subj.overlaps_sector_252d >= 20
        {
            (
                subj.sector_benchmark.clone(),
                "SECTOR_ETF".to_string(),
                true,
            )
        } else if subj.overlaps_spy_252d >= 20 && !subj.market_benchmark.is_empty() {
            (subj.market_benchmark.clone(), "MARKET".to_string(), false)
        } else if subj.overlaps_sector_252d >= 20 && !subj.sector_benchmark.is_empty() {
            (
                subj.sector_benchmark.clone(),
                "SECTOR_ETF".to_string(),
                true,
            )
        } else {
            return CorrelationRankSnapshot {
                symbol: sym,
                as_of: as_of.to_string(),
                sector: if !sector.is_empty() {
                    sector.to_string()
                } else {
                    subj.symbol_sector.clone()
                },
                rank_label: "NO_DATA".into(),
                note: "Subject CORRSTK snapshot lacks a usable 252d benchmark overlap".into(),
                ..Default::default()
            };
        };

    let (subject_corr, subject_beta, subject_r_squared) =
        match corrrank_metric_for_snapshot(subj, use_sector_benchmark, &benchmark_name) {
            Some(v) => v,
            None => {
                return CorrelationRankSnapshot {
                    symbol: sym,
                    as_of: as_of.to_string(),
                    sector: if !sector.is_empty() {
                        sector.to_string()
                    } else {
                        subj.symbol_sector.clone()
                    },
                    benchmark_name,
                    benchmark_kind,
                    rank_label: "NO_DATA".into(),
                    note: "Subject CORRSTK snapshot is missing the selected benchmark series"
                        .into(),
                    ..Default::default()
                };
            }
        };
    let subject_abs = subject_corr.abs();

    let peer_abs_corrs: Vec<f64> = peers
        .iter()
        .filter_map(|p| {
            corrrank_metric_for_snapshot(p, use_sector_benchmark, &benchmark_name)
                .map(|(corr, _, _)| corr.abs())
        })
        .collect();
    let peers_considered = peers.len();
    let peers_with_data = peer_abs_corrs.len();
    if peer_abs_corrs.len() < 3 {
        return CorrelationRankSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            sector: if !sector.is_empty() {
                sector.to_string()
            } else {
                subj.symbol_sector.clone()
            },
            benchmark_name,
            benchmark_kind,
            subject_corr_252d: subject_corr,
            subject_abs_corr_252d: subject_abs,
            subject_beta_252d: subject_beta,
            subject_r_squared_252d: subject_r_squared,
            subject_correlation_label: subj.correlation_label.clone(),
            peers_considered,
            peers_with_data,
            rank_label: "NO_DATA".into(),
            note: format!(
                "Only {} sector peers have cached {} correlation data (need ≥3)",
                peer_abs_corrs.len(),
                if use_sector_benchmark {
                    "sector-ETF"
                } else {
                    "market"
                }
            ),
            ..Default::default()
        };
    }

    let mut sorted = peer_abs_corrs.clone();
    sorted.push(subject_abs);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let pct = percentile_rank_score(subject_abs, &peer_abs_corrs, true);
    let rank_position = peer_abs_corrs
        .iter()
        .filter(|&&corr_abs| corr_abs > subject_abs)
        .count()
        + 1;

    CorrelationRankSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        sector: if !sector.is_empty() {
            sector.to_string()
        } else {
            subj.symbol_sector.clone()
        },
        benchmark_name,
        benchmark_kind,
        subject_corr_252d: subject_corr,
        subject_abs_corr_252d: subject_abs,
        subject_beta_252d: subject_beta,
        subject_r_squared_252d: subject_r_squared,
        subject_correlation_label: subj.correlation_label.clone(),
        peers_considered,
        peers_with_data,
        sector_median_abs_corr_252d: quantile_f64(&sorted, 0.5),
        sector_p25_abs_corr_252d: quantile_f64(&sorted, 0.25),
        sector_p75_abs_corr_252d: quantile_f64(&sorted, 0.75),
        percentile_rank: pct,
        rank_position,
        rank_label: rank_label_for_percentile(pct).into(),
        note: String::new(),
    }
}

/// OPERANK_DELTA — rank one symbol's operating-margin expansion/contraction
/// vs same-sector peers using cached MARGINS snapshots.
pub fn compute_operank_delta_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&MarginsSnapshot>,
    peers: &[&MarginsSnapshot],
) -> OperatingMarginDeltaRankSnapshot {
    let sym = symbol.to_uppercase();
    let subj = match subject {
        Some(s) if s.periods_used >= 2 => s,
        _ => {
            return OperatingMarginDeltaRankSnapshot {
                symbol: sym,
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No cached MARGINS trend snapshot for subject".into(),
                ..Default::default()
            };
        }
    };

    let peer_deltas: Vec<f64> = peers
        .iter()
        .filter(|p| p.periods_used >= 2)
        .map(|p| p.operating_margin_change_pct)
        .collect();
    let peers_considered = peers.len();
    let peers_with_data = peer_deltas.len();
    if peers_with_data < 3 {
        return OperatingMarginDeltaRankSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            basis: subj.basis.clone(),
            latest_period: subj.latest_period.clone(),
            operating_margin_pct: subj.latest_operating_margin_pct,
            operating_margin_change_pct: subj.operating_margin_change_pct,
            operating_trend_label: subj.operating_trend_label.clone(),
            peers_considered,
            peers_with_data,
            rank_label: "INSUFFICIENT_DATA".into(),
            note: format!(
                "Only {} sector peers have usable MARGINS trend data (need ≥3)",
                peers_with_data
            ),
            ..Default::default()
        };
    }

    let mut sorted = peer_deltas.clone();
    sorted.push(subj.operating_margin_change_pct);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let pct = percentile_rank_score(subj.operating_margin_change_pct, &peer_deltas, true);
    let rank_position = peer_deltas
        .iter()
        .filter(|&&delta| delta > subj.operating_margin_change_pct)
        .count()
        + 1;

    OperatingMarginDeltaRankSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        basis: subj.basis.clone(),
        latest_period: subj.latest_period.clone(),
        operating_margin_pct: subj.latest_operating_margin_pct,
        operating_margin_change_pct: subj.operating_margin_change_pct,
        operating_trend_label: subj.operating_trend_label.clone(),
        peers_considered,
        peers_with_data,
        sector_median_change_pct: quantile_f64(&sorted, 0.5),
        sector_p25_change_pct: quantile_f64(&sorted, 0.25),
        sector_p75_change_pct: quantile_f64(&sorted, 0.75),
        percentile_rank: pct,
        rank_position,
        rank_label: rank_label_for_percentile(pct).into(),
        note: String::new(),
    }
}
