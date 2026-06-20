use super::*;

// ── compute fns ──

/// Compute the debt-to-equity ratio for a `LeverageSnapshot`.
/// Returns `None` when equity is non-positive (shell / deficit), which is
/// handled by the LEVRANK surface as a special "NEGATIVE_EQUITY" bucket.
fn debt_to_equity_for(lev: &LeverageSnapshot) -> Option<f64> {
    if lev.total_equity > 0.0 {
        Some(lev.total_debt / lev.total_equity)
    } else {
        None
    }
}

/// LEVRANK — Leverage Rank vs sector peers.
///
/// Percentile-ranks the subject's D/E (from the cached `LeverageSnapshot`)
/// against peer snapshots pre-filtered to the same sector. Uses the
/// risk-inverted rank ladder (SAFEST_DECILE..RISKIEST_DECILE) because lower
/// D/E = safer. Negative-equity subjects get a dedicated label.
pub fn compute_levrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&LeverageSnapshot>,
    peers: &[&LeverageSnapshot],
) -> LeverageRankSnapshot {
    let subj = match subject {
        Some(s) => s,
        None => {
            return LeverageRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No LEV snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let subj_d2e = match debt_to_equity_for(subj) {
        Some(v) => v,
        None => {
            return LeverageRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                total_debt: subj.total_debt,
                total_equity: subj.total_equity,
                rank_label: "NEGATIVE_EQUITY".into(),
                note: "Subject has non-positive equity; D/E undefined".into(),
                ..Default::default()
            };
        }
    };
    let peer_d2es: Vec<f64> = peers.iter().filter_map(|p| debt_to_equity_for(p)).collect();
    let peers_considered = peers.len();
    let peers_with_data = peer_d2es.len();
    if peer_d2es.len() < 3 {
        return LeverageRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            debt_to_equity: subj_d2e,
            total_debt: subj.total_debt,
            total_equity: subj.total_equity,
            peers_considered,
            peers_with_data,
            rank_label: "NO_DATA".into(),
            note: format!(
                "Only {} LEV peers with positive equity in sector {} (need ≥3)",
                peer_d2es.len(),
                sector
            ),
            ..Default::default()
        };
    }
    let mut sorted = peer_d2es.clone();
    sorted.push(subj_d2e);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    // INVERSION: lower D/E = safer = higher rank.
    let pct = percentile_rank_score(subj_d2e, &peer_d2es, false);
    // rank_position counted by how many peers are SAFER (lower D/E).
    let safer = peer_d2es.iter().filter(|&&p| p < subj_d2e).count();
    let label = risk_rank_label_for_percentile(pct);
    LeverageRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        debt_to_equity: subj_d2e,
        total_debt: subj.total_debt,
        total_equity: subj.total_equity,
        peers_considered,
        peers_with_data,
        sector_median_d2e: median,
        sector_p25_d2e: p25,
        sector_p75_d2e: p75,
        percentile_rank: pct,
        rank_position: safer + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// OPERANK — Operating Quality Rank vs sector peers.
///
/// Percentile-ranks `MarginsSnapshot.latest_operating_margin_pct` within
/// the same sector. Higher margin = higher rank. Peers must be pre-filtered.
pub fn compute_operank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&MarginsSnapshot>,
    peers: &[&MarginsSnapshot],
) -> OperatingQualityRankSnapshot {
    let subj = match subject {
        Some(s) if s.periods_used > 0 => s,
        _ => {
            return OperatingQualityRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No MARGINS snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_margins: Vec<f64> = peers
        .iter()
        .filter(|p| p.periods_used > 0)
        .map(|p| p.latest_operating_margin_pct)
        .collect();
    let peers_considered = peers.len();
    let peers_with_data = peer_margins.len();
    if peer_margins.len() < 3 {
        return OperatingQualityRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            operating_margin_pct: subj.latest_operating_margin_pct,
            margin_trend_label: subj.overall_trend_label.clone(),
            peers_considered,
            peers_with_data,
            rank_label: "NO_DATA".into(),
            note: format!(
                "Only {} MARGINS peers in sector {} (need ≥3)",
                peer_margins.len(),
                sector
            ),
            ..Default::default()
        };
    }
    let mut sorted = peer_margins.clone();
    sorted.push(subj.latest_operating_margin_pct);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj.latest_operating_margin_pct, &peer_margins, true);
    let better = peer_margins
        .iter()
        .filter(|&&p| p > subj.latest_operating_margin_pct)
        .count();
    let label = rank_label_for_percentile(pct);
    OperatingQualityRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        operating_margin_pct: subj.latest_operating_margin_pct,
        margin_trend_label: subj.overall_trend_label.clone(),
        peers_considered,
        peers_with_data,
        sector_median_margin_pct: median,
        sector_p25_margin_pct: p25,
        sector_p75_margin_pct: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// FQMRANK — Fundamental Quality Meter Rank vs sector peers.
///
/// Percentile-ranks `FundamentalQualityMeterSnapshot.composite_score` within
/// the same sector. Filters out peers with operator_label "NO_DATA".
pub fn compute_fqmrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&FundamentalQualityMeterSnapshot>,
    peers: &[&FundamentalQualityMeterSnapshot],
) -> FqmRankSnapshot {
    let subj = match subject {
        Some(s) if s.operator_label != "NO_DATA" && s.composite_score > 0.0 => s,
        _ => {
            return FqmRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No FQM snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_scores: Vec<f64> = peers
        .iter()
        .filter(|p| p.operator_label != "NO_DATA" && p.composite_score > 0.0)
        .map(|p| p.composite_score)
        .collect();
    let peers_considered = peers.len();
    let peers_with_data = peer_scores.len();
    if peer_scores.len() < 3 {
        return FqmRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            composite_score: subj.composite_score,
            operator_label: subj.operator_label.clone(),
            peers_considered,
            peers_with_data,
            rank_label: "NO_DATA".into(),
            note: format!(
                "Only {} FQM peers in sector {} (need ≥3)",
                peer_scores.len(),
                sector
            ),
            ..Default::default()
        };
    }
    let mut sorted = peer_scores.clone();
    sorted.push(subj.composite_score);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj.composite_score, &peer_scores, true);
    let better = peer_scores
        .iter()
        .filter(|&&p| p > subj.composite_score)
        .count();
    let label = rank_label_for_percentile(pct);
    FqmRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        composite_score: subj.composite_score,
        operator_label: subj.operator_label.clone(),
        peers_considered,
        peers_with_data,
        sector_median_score: median,
        sector_p25: p25,
        sector_p75: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// LIQRANK — Liquidity Rank vs sector peers.
///
/// Percentile-ranks `LiquiditySnapshot.avg_daily_dollar_volume` within the
/// same sector. Higher ADV$ = deeper liquidity = higher rank. Filters out
/// peers with INSUFFICIENT_DATA tier.
pub fn compute_liqrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&LiquiditySnapshot>,
    peers: &[&LiquiditySnapshot],
) -> LiquidityRankSnapshot {
    let subj = match subject {
        Some(s) if s.liquidity_tier != "INSUFFICIENT_DATA" && s.avg_daily_dollar_volume > 0.0 => s,
        _ => {
            return LiquidityRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No LIQ snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_advs: Vec<f64> = peers
        .iter()
        .filter(|p| p.liquidity_tier != "INSUFFICIENT_DATA" && p.avg_daily_dollar_volume > 0.0)
        .map(|p| p.avg_daily_dollar_volume)
        .collect();
    let peers_considered = peers.len();
    let peers_with_data = peer_advs.len();
    if peer_advs.len() < 3 {
        return LiquidityRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            avg_daily_dollar_volume: subj.avg_daily_dollar_volume,
            tier_label: subj.liquidity_tier.clone(),
            peers_considered,
            peers_with_data,
            rank_label: "NO_DATA".into(),
            note: format!(
                "Only {} LIQ peers in sector {} (need ≥3)",
                peer_advs.len(),
                sector
            ),
            ..Default::default()
        };
    }
    let mut sorted = peer_advs.clone();
    sorted.push(subj.avg_daily_dollar_volume);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj.avg_daily_dollar_volume, &peer_advs, true);
    let better = peer_advs
        .iter()
        .filter(|&&p| p > subj.avg_daily_dollar_volume)
        .count();
    let label = rank_label_for_percentile(pct);
    LiquidityRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        avg_daily_dollar_volume: subj.avg_daily_dollar_volume,
        tier_label: subj.liquidity_tier.clone(),
        peers_considered,
        peers_with_data,
        sector_median_dollar_volume: median,
        sector_p25_dollar_volume: p25,
        sector_p75_dollar_volume: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// SURPSTK — Earnings Surprise Streak snapshot.
///
/// Pure time-series stat over cached `EarningsSurprise` rows. Classifies each
/// row as BEAT / MISS / INLINE using a ±2% band around the estimate, then
/// counts consecutive streaks over the series (sorted newest-first). Emits
/// a streak-strength label from the beat rate + current streak. No sector.
pub fn compute_surpstk_snapshot(
    symbol: &str,
    as_of: &str,
    surprises: &[EarningsSurprise],
) -> EarningsSurpriseStreakSnapshot {
    if surprises.is_empty() {
        return EarningsSurpriseStreakSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            streak_label: "INSUFFICIENT_DATA".into(),
            note: "No EPS rows cached for subject".into(),
            ..Default::default()
        };
    }
    // Sort newest-first by date (lexical works for YYYY-MM-DD).
    let mut rows: Vec<&EarningsSurprise> = surprises.iter().collect();
    rows.sort_by(|a, b| b.date.cmp(&a.date));
    let classify = |s: &EarningsSurprise| -> &'static str {
        if s.surprise_pct >= 2.0 {
            "BEAT"
        } else if s.surprise_pct <= -2.0 {
            "MISS"
        } else {
            "INLINE"
        }
    };
    let mut beats = 0usize;
    let mut misses = 0usize;
    let mut inlines = 0usize;
    let mut sum_surprise = 0.0f64;
    for r in &rows {
        sum_surprise += r.surprise_pct;
        match classify(r) {
            "BEAT" => beats += 1,
            "MISS" => misses += 1,
            _ => inlines += 1,
        }
    }
    let total = rows.len();
    let beat_rate = beats as f64 / total as f64 * 100.0;
    let avg_surprise = sum_surprise / total as f64;
    // Current streak: starts at rows[0] (newest) and extends while label matches.
    let current_label = classify(rows[0]);
    let mut current_len = 1usize;
    for r in rows.iter().skip(1) {
        if classify(r) == current_label {
            current_len += 1;
        } else {
            break;
        }
    }
    // Longest streaks scanned across the full series.
    let mut longest_beat = 0usize;
    let mut longest_miss = 0usize;
    let mut run_beat = 0usize;
    let mut run_miss = 0usize;
    for r in &rows {
        match classify(r) {
            "BEAT" => {
                run_beat += 1;
                run_miss = 0;
                if run_beat > longest_beat {
                    longest_beat = run_beat;
                }
            }
            "MISS" => {
                run_miss += 1;
                run_beat = 0;
                if run_miss > longest_miss {
                    longest_miss = run_miss;
                }
            }
            _ => {
                run_beat = 0;
                run_miss = 0;
            }
        }
    }
    let streak_label = if total < 4 {
        "INSUFFICIENT_DATA"
    } else if beat_rate >= 75.0 && current_label == "BEAT" && current_len >= 3 {
        "HOT_STREAK"
    } else if beat_rate >= 60.0 {
        "BEAT_TREND"
    } else if beat_rate <= 25.0 && current_label == "MISS" && current_len >= 3 {
        "COLD_STREAK"
    } else if beat_rate <= 40.0 {
        "MISS_TREND"
    } else {
        "MIXED"
    };
    let latest = rows[0];
    EarningsSurpriseStreakSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        total_events: total,
        beats,
        misses,
        inlines,
        beat_rate_pct: beat_rate,
        current_streak_type: current_label.to_string(),
        current_streak_len: current_len,
        longest_beat_streak: longest_beat,
        longest_miss_streak: longest_miss,
        avg_surprise_pct: avg_surprise,
        latest_event_date: latest.date.clone(),
        latest_event_surprise_pct: latest.surprise_pct,
        latest_event_label: classify(latest).to_string(),
        streak_label: streak_label.to_string(),
        note: String::new(),
    }
}

// ── compute fns ──

pub fn compute_dvdrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&DivgSnapshot>,
    peers: &[&DivgSnapshot],
) -> DividendGrowthRankSnapshot {
    let subj = match subject {
        Some(s) if s.trend_label != "NO_HISTORY" && !s.trend_label.is_empty() => s,
        _ => {
            return DividendGrowthRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No DIVG snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_cagr: Vec<f64> = peers
        .iter()
        .filter(|p| !p.symbol.eq_ignore_ascii_case(symbol))
        .filter(|p| p.trend_label != "NO_HISTORY" && !p.trend_label.is_empty())
        .map(|p| p.cagr_3y_pct)
        .collect();
    let peers_considered = peers.len();
    let peers_with_data = peer_cagr.len();
    if peer_cagr.len() < 3 {
        return DividendGrowthRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            cagr_3y_pct: subj.cagr_3y_pct,
            consecutive_growth_years: subj.consecutive_growth_years,
            trend_label: subj.trend_label.clone(),
            peers_considered,
            peers_with_data,
            rank_label: "INSUFFICIENT_DATA".into(),
            note: format!(
                "Only {} DIVG peers with history in sector {} (need ≥3)",
                peer_cagr.len(),
                sector
            ),
            ..Default::default()
        };
    }
    let mut sorted = peer_cagr.clone();
    sorted.push(subj.cagr_3y_pct);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj.cagr_3y_pct, &peer_cagr, true);
    let better = peer_cagr.iter().filter(|&&p| p > subj.cagr_3y_pct).count();
    let label = rank_label_for_percentile(pct);
    DividendGrowthRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        cagr_3y_pct: subj.cagr_3y_pct,
        consecutive_growth_years: subj.consecutive_growth_years,
        trend_label: subj.trend_label.clone(),
        peers_considered,
        peers_with_data,
        sector_median_cagr_pct: median,
        sector_p25_cagr_pct: p25,
        sector_p75_cagr_pct: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_earmrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&EarmSnapshot>,
    peers: &[&EarmSnapshot],
) -> EarningsMomentumRankSnapshot {
    let subj = match subject {
        Some(s) if s.momentum_label != "INSUFFICIENT_DATA" && !s.momentum_label.is_empty() => s,
        _ => {
            return EarningsMomentumRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No EARM snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_scores: Vec<f64> = peers
        .iter()
        .filter(|p| !p.symbol.eq_ignore_ascii_case(symbol))
        .filter(|p| p.momentum_label != "INSUFFICIENT_DATA" && !p.momentum_label.is_empty())
        .map(|p| p.composite_score)
        .collect();
    let peers_considered = peers.len();
    let peers_with_data = peer_scores.len();
    if peer_scores.len() < 3 {
        return EarningsMomentumRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            composite_score: subj.composite_score,
            momentum_label: subj.momentum_label.clone(),
            peers_considered,
            peers_with_data,
            rank_label: "INSUFFICIENT_DATA".into(),
            note: format!(
                "Only {} EARM peers with data in sector {} (need ≥3)",
                peer_scores.len(),
                sector
            ),
            ..Default::default()
        };
    }
    let mut sorted = peer_scores.clone();
    sorted.push(subj.composite_score);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj.composite_score, &peer_scores, true);
    let better = peer_scores
        .iter()
        .filter(|&&p| p > subj.composite_score)
        .count();
    let label = rank_label_for_percentile(pct);
    EarningsMomentumRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        composite_score: subj.composite_score,
        momentum_label: subj.momentum_label.clone(),
        peers_considered,
        peers_with_data,
        sector_median_score: median,
        sector_p25: p25,
        sector_p75: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_updgrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&UpdmSnapshot>,
    peers: &[&UpdmSnapshot],
) -> UpgradeDowngradeRankSnapshot {
    let subj = match subject {
        Some(s) if s.bias_label != "NO_COVERAGE" && !s.bias_label.is_empty() => s,
        _ => {
            return UpgradeDowngradeRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No UPDM snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_nets: Vec<f64> = peers
        .iter()
        .filter(|p| !p.symbol.eq_ignore_ascii_case(symbol))
        .filter(|p| p.bias_label != "NO_COVERAGE" && !p.bias_label.is_empty())
        .map(|p| p.net_90d as f64)
        .collect();
    let peers_considered = peers.len();
    let peers_with_data = peer_nets.len();
    if peer_nets.len() < 3 {
        return UpgradeDowngradeRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            net_90d: subj.net_90d,
            bias_label: subj.bias_label.clone(),
            peers_considered,
            peers_with_data,
            rank_label: "INSUFFICIENT_DATA".into(),
            note: format!(
                "Only {} UPDM peers with coverage in sector {} (need ≥3)",
                peer_nets.len(),
                sector
            ),
            ..Default::default()
        };
    }
    let subj_f = subj.net_90d as f64;
    let mut sorted = peer_nets.clone();
    sorted.push(subj_f);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj_f, &peer_nets, true);
    let better = peer_nets.iter().filter(|&&p| p > subj_f).count();
    let label = rank_label_for_percentile(pct);
    UpgradeDowngradeRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        net_90d: subj.net_90d,
        bias_label: subj.bias_label.clone(),
        peers_considered,
        peers_with_data,
        sector_median_net_90d: median,
        sector_p25_net_90d: p25,
        sector_p75_net_90d: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_gy_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> GapYearlySnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 2 {
        return GapYearlySnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: bars.len(),
            gap_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥2 bars for gap calc".into(),
            ..Default::default()
        };
    }
    // Caller passes newest-first or oldest-first; we want to scan the last
    // 252 sessions worth of "today's open vs yesterday's close" gaps. Sort by
    // date ascending (oldest first) so pairs (i-1, i) go in calendar order.
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let start = if sorted.len() > 253 {
        sorted.len() - 253
    } else {
        0
    };
    let window = &sorted[start..];
    let bars_used = window.len();
    let mut gaps_total = 0usize;
    let mut gaps_up_2 = 0usize;
    let mut gaps_down_2 = 0usize;
    let mut gaps_up_5 = 0usize;
    let mut gaps_down_5 = 0usize;
    let mut gaps_up_10 = 0usize;
    let mut gaps_down_10 = 0usize;
    let mut sum_abs = 0.0f64;
    let mut largest_up = 0.0f64;
    let mut largest_up_date = String::new();
    let mut largest_down = 0.0f64;
    let mut largest_down_date = String::new();
    for i in 1..window.len() {
        let prev_close = window[i - 1].close;
        let open = window[i].open;
        if prev_close <= 0.0 || open <= 0.0 {
            continue;
        }
        let gap_pct = (open - prev_close) / prev_close * 100.0;
        if gap_pct.abs() < 0.01 {
            continue;
        } // treat <0.01% as no gap
        gaps_total += 1;
        sum_abs += gap_pct.abs();
        if gap_pct >= 2.0 {
            gaps_up_2 += 1;
        }
        if gap_pct <= -2.0 {
            gaps_down_2 += 1;
        }
        if gap_pct >= 5.0 {
            gaps_up_5 += 1;
        }
        if gap_pct <= -5.0 {
            gaps_down_5 += 1;
        }
        if gap_pct >= 10.0 {
            gaps_up_10 += 1;
        }
        if gap_pct <= -10.0 {
            gaps_down_10 += 1;
        }
        if gap_pct > largest_up {
            largest_up = gap_pct;
            largest_up_date = window[i].date.clone();
        }
        if gap_pct < largest_down {
            largest_down = gap_pct;
            largest_down_date = window[i].date.clone();
        }
    }
    let avg_abs = if gaps_total > 0 {
        sum_abs / gaps_total as f64
    } else {
        0.0
    };
    // Gap-label ladder:
    // - EXPLOSIVE: any 10% gap OR ≥ 4 gaps at the 5% band
    // - GAPPY: ≥ 12 gaps at the 2% band OR ≥ 2 gaps at the 5% band
    // - SMOOTH: < 6 gaps at the 2% band
    // - NORMAL: anything between
    let gap_2_total = gaps_up_2 + gaps_down_2;
    let gap_5_total = gaps_up_5 + gaps_down_5;
    let gap_10_total = gaps_up_10 + gaps_down_10;
    let gap_label = if bars_used < 20 {
        "INSUFFICIENT_DATA"
    } else if gap_10_total >= 1 || gap_5_total >= 4 {
        "EXPLOSIVE"
    } else if gap_2_total >= 12 || gap_5_total >= 2 {
        "GAPPY"
    } else if gap_2_total < 6 {
        "SMOOTH"
    } else {
        "NORMAL"
    };
    GapYearlySnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used,
        gaps_total,
        gaps_up_2pct: gaps_up_2,
        gaps_down_2pct: gaps_down_2,
        gaps_up_5pct: gaps_up_5,
        gaps_down_5pct: gaps_down_5,
        gaps_up_10pct: gaps_up_10,
        gaps_down_10pct: gaps_down_10,
        largest_up_gap_pct: largest_up,
        largest_up_gap_date: largest_up_date,
        largest_down_gap_pct: largest_down,
        largest_down_gap_date: largest_down_date,
        avg_abs_gap_pct: avg_abs,
        gap_label: gap_label.to_string(),
        note: String::new(),
    }
}

pub fn compute_des_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DailyEventStreakSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 2 {
        return DailyEventStreakSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: bars.len(),
            streak_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥2 bars for streak calc".into(),
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
    let mut up_days = 0usize;
    let mut down_days = 0usize;
    let mut flat_days = 0usize;
    let mut sum_up = 0.0f64;
    let mut sum_down = 0.0f64;
    let mut dirs: Vec<i8> = Vec::with_capacity(window.len());
    for i in 1..window.len() {
        let prev = window[i - 1].close;
        let cur = window[i].close;
        if prev <= 0.0 || cur <= 0.0 {
            dirs.push(0);
            continue;
        }
        let pct = (cur - prev) / prev * 100.0;
        if pct > 0.0 {
            up_days += 1;
            sum_up += pct;
            dirs.push(1);
        } else if pct < 0.0 {
            down_days += 1;
            sum_down += pct;
            dirs.push(-1);
        } else {
            flat_days += 1;
            dirs.push(0);
        }
    }
    let mut longest_up = 0usize;
    let mut longest_down = 0usize;
    let mut run_up = 0usize;
    let mut run_down = 0usize;
    for d in &dirs {
        match *d {
            1 => {
                run_up += 1;
                run_down = 0;
                if run_up > longest_up {
                    longest_up = run_up;
                }
            }
            -1 => {
                run_down += 1;
                run_up = 0;
                if run_down > longest_down {
                    longest_down = run_down;
                }
            }
            _ => {
                run_up = 0;
                run_down = 0;
            }
        }
    }
    // Current streak: trailing run at the end of `dirs`.
    let (current_type, current_len) = if let Some(last) = dirs.last().copied() {
        let mut len = 0usize;
        if last != 0 {
            for d in dirs.iter().rev() {
                if *d == last {
                    len += 1;
                } else {
                    break;
                }
            }
        }
        match last {
            1 => ("UP".to_string(), len),
            -1 => ("DOWN".to_string(), len),
            0 => ("FLAT".to_string(), 0usize),
            _ => ("NONE".to_string(), 0usize),
        }
    } else {
        ("NONE".to_string(), 0usize)
    };
    let total_directional = up_days + down_days;
    let up_day_rate = if total_directional > 0 {
        up_days as f64 / total_directional as f64 * 100.0
    } else {
        0.0
    };
    let avg_up = if up_days > 0 {
        sum_up / up_days as f64
    } else {
        0.0
    };
    let avg_down = if down_days > 0 {
        sum_down / down_days as f64
    } else {
        0.0
    };
    let streak_label = if bars_used < 20 {
        "INSUFFICIENT_DATA"
    } else if up_day_rate >= 60.0 && longest_up >= 5 {
        "STRONG_UPTREND"
    } else if up_day_rate >= 55.0 {
        "UPTREND_BIAS"
    } else if up_day_rate <= 40.0 && longest_down >= 5 {
        "STRONG_DOWNTREND"
    } else if up_day_rate <= 45.0 {
        "DOWNTREND_BIAS"
    } else {
        "NEUTRAL"
    };
    DailyEventStreakSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used,
        current_streak_type: current_type,
        current_streak_len: current_len,
        longest_up_streak: longest_up,
        longest_down_streak: longest_down,
        up_days,
        down_days,
        flat_days,
        up_day_rate_pct: up_day_rate,
        avg_up_move_pct: avg_up,
        avg_down_move_pct: avg_down,
        streak_label: streak_label.to_string(),
        note: String::new(),
    }
}
