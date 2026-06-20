use super::*;

// Relative rank and event-study compute functions

/// Simple quartile at `q ∈ [0,1]` via linear interpolation on a sorted slice.
/// Used by the rank surfaces for p25 / p75 sector markers.
pub(super) fn quantile_f64(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let idx = q * (sorted.len() as f64 - 1.0);
    let lo = idx.floor() as usize;
    let hi = idx.ceil() as usize;
    if lo == hi {
        return sorted[lo];
    }
    let frac = idx - lo as f64;
    sorted[lo] * (1.0 - frac) + sorted[hi] * frac
}

/// Percentile-rank `value` vs `others` using the
/// `(below + 0.5 × equal) / total × 100` midrank convention.
/// When `higher_is_better == false`, the returned rank is inverted so that
/// a smaller input value yields a higher percentile (used by RRK where
/// composite is higher = riskier).
pub(super) fn percentile_rank_score(value: f64, others: &[f64], higher_is_better: bool) -> f64 {
    let total = others.len() + 1;
    if total < 2 {
        return 50.0;
    }
    let (mut below, mut equal) = (0usize, 0usize);
    for &o in others {
        if (o - value).abs() < 1e-9 {
            equal += 1;
        } else if higher_is_better {
            if o < value {
                below += 1;
            }
        } else {
            if o > value {
                below += 1;
            }
        }
    }
    let raw = (below as f64 + 0.5 * equal as f64 + 0.5) / total as f64 * 100.0;
    raw.clamp(0.0, 100.0)
}

/// Standard 6-bucket rank label ladder for VRK / QRK.
pub(super) fn rank_label_for_percentile(pct: f64) -> &'static str {
    if pct >= 90.0 {
        "TOP_DECILE"
    } else if pct >= 75.0 {
        "TOP_QUARTILE"
    } else if pct >= 50.0 {
        "ABOVE_MEDIAN"
    } else if pct >= 25.0 {
        "BELOW_MEDIAN"
    } else if pct >= 10.0 {
        "BOTTOM_QUARTILE"
    } else {
        "BOTTOM_DECILE"
    }
}

/// Risk-inverted rank label ladder for RRK (higher rank = safer).
pub(super) fn risk_rank_label_for_percentile(pct: f64) -> &'static str {
    if pct >= 90.0 {
        "SAFEST_DECILE"
    } else if pct >= 75.0 {
        "SAFEST_QUARTILE"
    } else if pct >= 50.0 {
        "ABOVE_MEDIAN_SAFE"
    } else if pct >= 25.0 {
        "BELOW_MEDIAN_RISKY"
    } else if pct >= 10.0 {
        "BOTTOM_QUARTILE_RISKY"
    } else {
        "RISKIEST_DECILE"
    }
}

/// VRK — Value Rank vs sector peers.
///
/// Takes the subject's `ValueSnapshot` and a slice of peer snapshots
/// (caller filters to the same sector). Returns a percentile rank with the
/// standard 6-bucket label ladder. Higher percentile = better value.
pub fn compute_vrk_snapshot(
    symbol: &str,
    as_of: &str,
    subject: Option<&ValueSnapshot>,
    peers: &[&ValueSnapshot],
) -> ValueRankSnapshot {
    let subj = match subject {
        Some(s) if s.value_label != "NO_DATA" && s.composite_score > 0.0 => s,
        _ => {
            return ValueRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No VAL snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_scores: Vec<f64> = peers
        .iter()
        .filter(|p| p.value_label != "NO_DATA" && p.composite_score > 0.0)
        .map(|p| p.composite_score)
        .collect();
    if peer_scores.len() < 3 {
        return ValueRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: subj.sector.clone(),
            composite_score: subj.composite_score,
            peers_considered: peer_scores.len(),
            peers_with_data: peer_scores.len(),
            rank_label: "NO_DATA".into(),
            note: format!(
                "Only {} VAL peers in sector {} (need ≥3)",
                peer_scores.len(),
                subj.sector
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
    // 1-based rank position: count peers strictly better than subject + 1.
    let better = peer_scores
        .iter()
        .filter(|&&p| p > subj.composite_score)
        .count();
    let rank_position = better + 1;
    let label = rank_label_for_percentile(pct);
    ValueRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: subj.sector.clone(),
        composite_score: subj.composite_score,
        peers_considered: peer_scores.len(),
        peers_with_data: peer_scores.len(),
        sector_median_score: median,
        sector_p25: p25,
        sector_p75: p75,
        percentile_rank: pct,
        rank_position,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// QRK — Quality Rank vs sector peers.
///
/// `QualitySnapshot` does not carry sector — caller must supply it (typically
/// from `fundamentals::get_fundamentals(symbol).sector`), and peers must be
/// pre-filtered to the same sector.
pub fn compute_qrk_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&QualitySnapshot>,
    peers: &[&QualitySnapshot],
) -> QualityRankSnapshot {
    let subj = match subject {
        Some(s) if s.quality_label != "NO_DATA" && s.composite_score > 0.0 => s,
        _ => {
            return QualityRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No QUAL snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_scores: Vec<f64> = peers
        .iter()
        .filter(|p| p.quality_label != "NO_DATA" && p.composite_score > 0.0)
        .map(|p| p.composite_score)
        .collect();
    if peer_scores.len() < 3 {
        return QualityRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            composite_score: subj.composite_score,
            peers_considered: peer_scores.len(),
            peers_with_data: peer_scores.len(),
            rank_label: "NO_DATA".into(),
            note: format!(
                "Only {} QUAL peers in sector {} (need ≥3)",
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
    QualityRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        composite_score: subj.composite_score,
        peers_considered: peer_scores.len(),
        peers_with_data: peer_scores.len(),
        sector_median_score: median,
        sector_p25: p25,
        sector_p75: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// RRK — Risk Rank vs sector peers.
///
/// Percentile rank is *inverted* relative to VRK/QRK: RISK composite is
/// higher = riskier, so this surface treats a **lower** composite as **better**
/// and reports "higher percentile = safer." Label ladder uses
/// SAFEST_DECILE..RISKIEST_DECILE phrasing so the inversion is explicit.
pub fn compute_rrk_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&RiskSnapshot>,
    peers: &[&RiskSnapshot],
) -> RiskRankSnapshot {
    let subj = match subject {
        Some(s) if s.risk_label != "NO_DATA" && s.composite_score > 0.0 => s,
        _ => {
            return RiskRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No RISK snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_scores: Vec<f64> = peers
        .iter()
        .filter(|p| p.risk_label != "NO_DATA" && p.composite_score > 0.0)
        .map(|p| p.composite_score)
        .collect();
    if peer_scores.len() < 3 {
        return RiskRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            composite_score: subj.composite_score,
            peers_considered: peer_scores.len(),
            peers_with_data: peer_scores.len(),
            rank_label: "NO_DATA".into(),
            note: format!(
                "Only {} RISK peers in sector {} (need ≥3)",
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
    // INVERSION: higher_is_better = false because RISK composite is higher = riskier.
    let pct = percentile_rank_score(subj.composite_score, &peer_scores, false);
    // 1-based: rank position counted by how many peers are SAFER (lower composite).
    let safer = peer_scores
        .iter()
        .filter(|&&p| p < subj.composite_score)
        .count();
    let label = risk_rank_label_for_percentile(pct);
    RiskRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        composite_score: subj.composite_score,
        peers_considered: peer_scores.len(),
        peers_with_data: peer_scores.len(),
        sector_median_score: median,
        sector_p25: p25,
        sector_p75: p75,
        percentile_rank: pct,
        rank_position: safer + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// Compute 3-year EPS CAGR from a `FinancialStatements`.
/// Prefers annual rows (latest[0] vs latest[3] → 3y CAGR). Returns
/// `(latest_eps, earliest_eps, years_used, cagr_pct)` where `cagr_pct` is
/// `f64::NAN` if the sign-rule rejects the series.
fn eps_cagr_3y_from_statements(statements: &FinancialStatements) -> (f64, f64, usize, f64) {
    let annuals = &statements.income_annual;
    if annuals.len() < 4 {
        return (0.0, 0.0, 0, f64::NAN);
    }
    // Rows are assumed newest-first per the Finnhub fetcher convention.
    let latest = annuals[0].eps;
    let earliest = annuals[3].eps;
    let years = 3usize;
    // CAGR only valid when both endpoints are strictly positive.
    if latest > 0.0 && earliest > 0.0 {
        let cagr = ((latest / earliest).powf(1.0 / years as f64) - 1.0) * 100.0;
        (latest, earliest, years, cagr)
    } else if latest.is_finite() && earliest.is_finite() && earliest.abs() > 1e-9 {
        // Degrade gracefully to a linear annualised growth when signs cross:
        // this is the "CAGR_NEGATIVE" path — the snapshot label captures it.
        let linear = (latest - earliest) / earliest.abs() / years as f64 * 100.0;
        (latest, earliest, years, linear)
    } else {
        (latest, earliest, years, f64::NAN)
    }
}

/// RELEPSGR — Relative 3-year EPS CAGR vs sector median.
///
/// Computes the subject's 3y EPS CAGR and the median CAGR of the peer slice,
/// then labels the subject relative to the sector median. Labels:
/// FAR_ABOVE (≥ +15pp), ABOVE (≥ +5pp), INLINE (within ±5pp), BELOW (≤ -5pp),
/// FAR_BELOW (≤ -15pp), CAGR_NEGATIVE (sign-crossed subject EPS),
/// NO_DATA (insufficient annual rows or empty peer set).
pub fn compute_relepsgr_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&FinancialStatements>,
    peer_statements: &[(String, FinancialStatements)],
) -> RelativeEpsGrowthSnapshot {
    let subj = match subject {
        Some(s) if s.income_annual.len() >= 4 => s,
        _ => {
            return RelativeEpsGrowthSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                relative_label: "NO_DATA".into(),
                note: "Subject has < 4 annual income rows".into(),
                ..Default::default()
            };
        }
    };
    let (latest, earliest, years, subj_cagr) = eps_cagr_3y_from_statements(subj);
    let mut peer_cagrs: Vec<f64> = Vec::new();
    for (_, st) in peer_statements {
        if st.income_annual.len() < 4 {
            continue;
        }
        let (_, _, _, c) = eps_cagr_3y_from_statements(st);
        if c.is_finite() {
            peer_cagrs.push(c);
        }
    }
    let peers_considered = peer_statements.len();
    let peers_with_data = peer_cagrs.len();
    if peer_cagrs.len() < 3 {
        return RelativeEpsGrowthSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            latest_eps: latest,
            earliest_eps: earliest,
            years_used: years,
            symbol_cagr_pct: if subj_cagr.is_finite() {
                subj_cagr
            } else {
                0.0
            },
            peers_considered,
            peers_with_data,
            relative_label: "NO_DATA".into(),
            note: format!(
                "Only {} peers with ≥4 annual rows (need ≥3)",
                peer_cagrs.len()
            ),
            ..Default::default()
        };
    }
    let mut sorted = peer_cagrs.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    if !subj_cagr.is_finite() || latest <= 0.0 || earliest <= 0.0 {
        return RelativeEpsGrowthSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            latest_eps: latest,
            earliest_eps: earliest,
            years_used: years,
            symbol_cagr_pct: if subj_cagr.is_finite() {
                subj_cagr
            } else {
                0.0
            },
            peers_considered,
            peers_with_data,
            sector_median_cagr_pct: median,
            sector_p25_cagr_pct: p25,
            sector_p75_cagr_pct: p75,
            relative_label: "CAGR_NEGATIVE".into(),
            note: "Subject EPS crosses zero; using linear proxy".into(),
            ..Default::default()
        };
    }
    let gap = subj_cagr - median;
    let label = if gap >= 15.0 {
        "FAR_ABOVE"
    } else if gap >= 5.0 {
        "ABOVE"
    } else if gap >= -5.0 {
        "INLINE"
    } else if gap >= -15.0 {
        "BELOW"
    } else {
        "FAR_BELOW"
    };
    RelativeEpsGrowthSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        latest_eps: latest,
        earliest_eps: earliest,
        years_used: years,
        symbol_cagr_pct: subj_cagr,
        peers_considered,
        peers_with_data,
        sector_median_cagr_pct: median,
        sector_p25_cagr_pct: p25,
        sector_p75_cagr_pct: p75,
        gap_to_median_pp: gap,
        relative_label: label.into(),
        note: String::new(),
    }
}

/// Locate the index of the first bar with `date >= target_date` in a
/// newest-first HP bar slice. Returns `None` if no such bar exists.
fn find_t0_index_newest_first(bars: &[HistoricalPriceRow], target_date: &str) -> Option<usize> {
    // Scan from oldest to newest (reverse iteration) and return the first
    // bar that is on-or-after the target. "newest-first" means bars[0] is
    // the most recent trading day.
    let mut best: Option<usize> = None;
    for (i, b) in bars.iter().enumerate() {
        if b.date.as_str() >= target_date {
            best = Some(i);
        } else {
            break;
        }
    }
    best
}

/// PEAD — Post-Earnings-Announcement Drift snapshot.
///
/// For each surprise row, locate `T0` in the HP bar slice (first trading day
/// at or after the announcement date), then compute forward drift over 1 / 3 /
/// 5 / 10 trading days. Averages over all successfully-matched events.
/// Returns INSUFFICIENT_DATA if fewer than 3 events match.
pub fn compute_pead_snapshot(
    symbol: &str,
    as_of: &str,
    surprises: &[EarningsSurprise],
    bars_newest_first: &[HistoricalPriceRow],
) -> PeadSnapshot {
    let num_events = surprises.len();
    if num_events == 0 || bars_newest_first.len() < 11 {
        return PeadSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            num_events,
            events_used: 0,
            drift_direction_label: "INSUFFICIENT_DATA".into(),
            note: if num_events == 0 {
                "No earnings surprises cached".into()
            } else {
                format!("Need ≥11 HP bars, have {}", bars_newest_first.len())
            },
            ..Default::default()
        };
    }
    let mut rows: Vec<PeadEventRow> = Vec::new();
    let mut beat_drifts_5d: Vec<f64> = Vec::new();
    let mut miss_drifts_5d: Vec<f64> = Vec::new();
    let mut all_1d: Vec<f64> = Vec::new();
    let mut all_3d: Vec<f64> = Vec::new();
    let mut all_5d: Vec<f64> = Vec::new();
    let mut all_10d: Vec<f64> = Vec::new();
    for surprise in surprises {
        let t0_idx = match find_t0_index_newest_first(bars_newest_first, &surprise.date) {
            Some(i) => i,
            None => continue,
        };
        // drift_Nd: close(t0 - N days back in newest-first ordering) vs close(t0).
        // Because bars are newest-first, "N trading days forward" means a
        // *smaller* index. Subtract N from t0_idx.
        if t0_idx < 10 {
            continue;
        }
        let t0_close = bars_newest_first[t0_idx].close;
        if t0_close <= 0.0 {
            continue;
        }
        let drift = |n: usize| {
            let fwd = &bars_newest_first[t0_idx - n];
            (fwd.close / t0_close - 1.0) * 100.0
        };
        let d1 = drift(1);
        let d3 = drift(3);
        let d5 = drift(5);
        let d10 = drift(10);
        let classification = if surprise.surprise_pct > 2.0 {
            "BEAT"
        } else if surprise.surprise_pct < -2.0 {
            "MISS"
        } else {
            "INLINE"
        };
        match classification {
            "BEAT" => beat_drifts_5d.push(d5),
            "MISS" => miss_drifts_5d.push(d5),
            _ => {}
        }
        all_1d.push(d1);
        all_3d.push(d3);
        all_5d.push(d5);
        all_10d.push(d10);
        rows.push(PeadEventRow {
            event_date: surprise.date.clone(),
            surprise_pct: surprise.surprise_pct,
            classification: classification.into(),
            drift_1d_pct: d1,
            drift_3d_pct: d3,
            drift_5d_pct: d5,
            drift_10d_pct: d10,
        });
    }
    let events_used = rows.len();
    if events_used < 3 {
        return PeadSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            num_events,
            events_used,
            drift_direction_label: "INSUFFICIENT_DATA".into(),
            note: format!("Matched only {} events to HP bars (need ≥3)", events_used),
            rows,
            ..Default::default()
        };
    }
    let mean = |v: &[f64]| {
        if v.is_empty() {
            0.0
        } else {
            v.iter().sum::<f64>() / v.len() as f64
        }
    };
    let avg_1d = mean(&all_1d);
    let avg_3d = mean(&all_3d);
    let avg_5d = mean(&all_5d);
    let avg_10d = mean(&all_10d);
    let beat_5d = mean(&beat_drifts_5d);
    let miss_5d = mean(&miss_drifts_5d);
    // Sort rows newest-first (highest event_date string first) for stable display.
    let mut sorted_rows = rows.clone();
    sorted_rows.sort_by(|a, b| b.event_date.cmp(&a.event_date));
    let latest = sorted_rows.first().cloned().unwrap_or_default();
    let label = if avg_5d >= 2.0 {
        "DRIFT_UP"
    } else if avg_5d <= -2.0 {
        "DRIFT_DOWN"
    } else {
        "MIXED"
    };
    let display_rows: Vec<PeadEventRow> = sorted_rows.into_iter().take(8).collect();
    PeadSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        num_events,
        events_used,
        avg_drift_1d_pct: avg_1d,
        avg_drift_3d_pct: avg_3d,
        avg_drift_5d_pct: avg_5d,
        avg_drift_10d_pct: avg_10d,
        beat_event_drift_5d_pct: beat_5d,
        miss_event_drift_5d_pct: miss_5d,
        latest_event_date: latest.event_date.clone(),
        latest_event_surprise_pct: latest.surprise_pct,
        latest_event_drift_5d_pct: latest.drift_5d_pct,
        drift_direction_label: label.into(),
        rows: display_rows,
        note: String::new(),
    }
}

// ── rank surfaces + FQM + revenue growth ────────────────

/// Market-cap tier classifier (absolute dollar thresholds).
fn size_tier_label(market_cap: f64) -> &'static str {
    if market_cap >= 200_000_000_000.0 {
        "MEGA_CAP"
    } else if market_cap >= 10_000_000_000.0 {
        "LARGE_CAP"
    } else if market_cap >= 2_000_000_000.0 {
        "MID_CAP"
    } else if market_cap >= 300_000_000.0 {
        "SMALL_CAP"
    } else if market_cap > 0.0 {
        "MICRO_CAP"
    } else {
        "NO_DATA"
    }
}

/// SIZEF — Size Factor Rank vs sector peers.
///
/// Callers pass the subject's market cap + sector and a slice of
/// `(symbol, market_cap)` tuples for sector peers. Returns a percentile
/// rank (higher = larger) plus a tier label derived from absolute cap.
pub fn compute_sizef_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject_market_cap: Option<f64>,
    peers: &[(String, f64)],
) -> SizeFactorSnapshot {
    let cap = match subject_market_cap {
        Some(c) if c > 0.0 => c,
        _ => {
            return SizeFactorSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                tier_label: "NO_DATA".into(),
                rank_label: "NO_DATA".into(),
                note: "No market cap on file for subject".into(),
                ..Default::default()
            };
        }
    };
    let tier = size_tier_label(cap);
    let peer_caps: Vec<f64> = peers
        .iter()
        .filter(|(_, c)| *c > 0.0)
        .map(|(_, c)| *c)
        .collect();
    if peer_caps.len() < 3 {
        return SizeFactorSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            market_cap: cap,
            log_market_cap: cap.ln(),
            tier_label: tier.into(),
            peers_considered: peer_caps.len(),
            peers_with_data: peer_caps.len(),
            rank_label: "NO_DATA".into(),
            note: format!(
                "Only {} peers with market cap in sector {} (need ≥3)",
                peer_caps.len(),
                sector
            ),
            ..Default::default()
        };
    }
    let mut sorted = peer_caps.clone();
    sorted.push(cap);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(cap, &peer_caps, true);
    let better = peer_caps.iter().filter(|&&c| c > cap).count();
    let label = rank_label_for_percentile(pct);
    SizeFactorSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        market_cap: cap,
        log_market_cap: cap.ln(),
        tier_label: tier.into(),
        peers_considered: peer_caps.len(),
        peers_with_data: peer_caps.len(),
        sector_median_cap: median,
        sector_p25_cap: p25,
        sector_p75_cap: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// MOMF — Momentum Factor Rank vs sector peers.
///
/// `MomentumSnapshot` does not carry sector — caller must supply it and
/// pre-filter peers to the same sector.
pub fn compute_momf_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&MomentumSnapshot>,
    peers: &[&MomentumSnapshot],
) -> MomentumRankSnapshot {
    let subj = match subject {
        Some(s) if s.regime_label != "INSUFFICIENT_DATA" && s.composite_score > 0.0 => s,
        _ => {
            return MomentumRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No MOMENTUM snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_scores: Vec<f64> = peers
        .iter()
        .filter(|p| p.regime_label != "INSUFFICIENT_DATA" && p.composite_score > 0.0)
        .map(|p| p.composite_score)
        .collect();
    if peer_scores.len() < 3 {
        return MomentumRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            composite_score: subj.composite_score,
            peers_considered: peer_scores.len(),
            peers_with_data: peer_scores.len(),
            rank_label: "NO_DATA".into(),
            note: format!(
                "Only {} MOMENTUM peers in sector {} (need ≥3)",
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
    MomentumRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        composite_score: subj.composite_score,
        peers_considered: peer_scores.len(),
        peers_with_data: peer_scores.len(),
        sector_median_score: median,
        sector_p25: p25,
        sector_p75: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// PEADRANK — Post-Earnings Drift Rank vs sector peers.
///
/// Peers must have a valid PEAD snapshot (`drift_direction_label !=
/// "INSUFFICIENT_DATA"` and `events_used >= 3`). Higher percentile =
/// stronger positive drift.
pub fn compute_peadrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&PeadSnapshot>,
    peers: &[&PeadSnapshot],
) -> PeadRankSnapshot {
    let subj = match subject {
        Some(s) if s.drift_direction_label != "INSUFFICIENT_DATA" && s.events_used >= 3 => s,
        _ => {
            return PeadRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No valid PEAD snapshot for subject (need ≥3 events)".into(),
                ..Default::default()
            };
        }
    };
    let peer_drifts: Vec<f64> = peers
        .iter()
        .filter(|p| p.drift_direction_label != "INSUFFICIENT_DATA" && p.events_used >= 3)
        .map(|p| p.avg_drift_5d_pct)
        .collect();
    if peer_drifts.len() < 3 {
        return PeadRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            avg_drift_5d_pct: subj.avg_drift_5d_pct,
            peers_considered: peer_drifts.len(),
            peers_with_data: peer_drifts.len(),
            rank_label: "NO_DATA".into(),
            note: format!(
                "Only {} valid PEAD peers in sector {} (need ≥3)",
                peer_drifts.len(),
                sector
            ),
            ..Default::default()
        };
    }
    let mut sorted = peer_drifts.clone();
    sorted.push(subj.avg_drift_5d_pct);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj.avg_drift_5d_pct, &peer_drifts, true);
    let better = peer_drifts
        .iter()
        .filter(|&&d| d > subj.avg_drift_5d_pct)
        .count();
    let label = rank_label_for_percentile(pct);
    PeadRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        avg_drift_5d_pct: subj.avg_drift_5d_pct,
        peers_considered: peer_drifts.len(),
        peers_with_data: peer_drifts.len(),
        sector_median_drift_5d_pct: median,
        sector_p25_drift_5d_pct: p25,
        sector_p75_drift_5d_pct: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// FQM — Fundamental Quality Meter.
///
/// One-layer composite over PTFS + MARGINS + ACRL (weights 40/30/30).
/// Deliberately excludes LEV so the score reflects **operational** quality
/// (does the business machine convert sales into durable cash?) rather than
/// capital-structure strength. A highly-levered business with elite margins
/// and strong cash conversion will FQM-high and QUAL-mid — that's the
/// intended divergence from QUAL.
pub fn compute_fqm_snapshot(
    symbol: &str,
    as_of: &str,
    piotroski: Option<&PiotroskiSnapshot>,
    margins: Option<&MarginsSnapshot>,
    accruals: Option<&AccrualsSnapshot>,
) -> FundamentalQualityMeterSnapshot {
    let sym = symbol.to_uppercase();
    let mut components: Vec<FactorComponent> = Vec::new();
    let mut total_weight = 0.0;
    let mut weighted_sum = 0.0;
    let mut inputs_available = 0i32;

    let mut piotroski_score = 0;
    let mut piotroski_label = String::new();
    let mut operating_margin_pct = 0.0;
    let mut margin_trend_label = String::new();
    let mut cash_conversion_pct = 0.0;
    let mut accruals_trend_label = String::new();

    // PTFS — weight 40.
    if let Some(p) = piotroski {
        if p.strength_label != "INSUFFICIENT_DATA" && !p.strength_label.is_empty() {
            piotroski_score = p.f_score;
            piotroski_label = p.strength_label.clone();
            let score = (p.f_score as f64 / 9.0 * 100.0).clamp(0.0, 100.0);
            let w = 40.0;
            components.push(FactorComponent {
                name: "Piotroski F".to_string(),
                value: format!("{}/9 ({})", p.f_score, p.strength_label),
                score,
                weight: w,
                contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // MARGINS — weight 30.
    if let Some(m) = margins {
        if m.quality_label != "INSUFFICIENT_DATA" && !m.quality_label.is_empty() {
            operating_margin_pct = m.latest_operating_margin_pct;
            margin_trend_label = m.overall_trend_label.clone();
            let mut score: f64 = match m.quality_label.as_str() {
                "HIGH" => 85.0,
                "MEDIUM" => 60.0,
                "LOW" => 30.0,
                _ => 50.0,
            };
            match m.overall_trend_label.as_str() {
                "EXPANDING" => score = (score + 10.0).min(100.0),
                "CONTRACTING" => score = (score - 10.0).max(0.0),
                _ => {}
            }
            let w = 30.0;
            components.push(FactorComponent {
                name: "Margins".to_string(),
                value: format!(
                    "{} op {:.1}% ({})",
                    m.quality_label, m.latest_operating_margin_pct, m.overall_trend_label
                ),
                score,
                weight: w,
                contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // ACRL — weight 30.
    if let Some(ac) = accruals {
        if !ac.trend_label.is_empty() {
            accruals_trend_label = ac.trend_label.clone();
            cash_conversion_pct = ac.ttm_cash_conversion_pct;
            let mut score: f64 = match ac.trend_label.as_str() {
                "IMPROVING" => 80.0,
                "STABLE" => 60.0,
                "MIXED" => 50.0,
                "DETERIORATING" => 30.0,
                _ => 50.0,
            };
            if ac.ttm_cash_conversion_pct >= 100.0 {
                score = (score + 10.0).min(100.0);
            } else if ac.ttm_cash_conversion_pct < 50.0 && ac.ttm_cash_conversion_pct != 0.0 {
                score = (score - 10.0).max(0.0);
            }
            let w = 30.0;
            components.push(FactorComponent {
                name: "Accruals".to_string(),
                value: format!(
                    "{} ({:.0}% cash conv)",
                    ac.trend_label, ac.ttm_cash_conversion_pct
                ),
                score,
                weight: w,
                contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    if inputs_available == 0 || total_weight <= 0.0 {
        return FundamentalQualityMeterSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            operator_label: "NO_DATA".to_string(),
            note: "need at least one of PTFS / MARGINS / ACRL cached".to_string(),
            ..Default::default()
        };
    }

    let composite = (weighted_sum / total_weight).clamp(0.0, 100.0);
    let label = if composite >= 85.0 {
        "ELITE_OPERATOR"
    } else if composite >= 70.0 {
        "STRONG_OPERATOR"
    } else if composite >= 50.0 {
        "AVERAGE_OPERATOR"
    } else if composite >= 30.0 {
        "WEAK_OPERATOR"
    } else {
        "BROKEN_OPERATOR"
    };

    FundamentalQualityMeterSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        piotroski_score,
        piotroski_label,
        operating_margin_pct,
        margin_trend_label,
        cash_conversion_pct,
        accruals_trend_label,
        composite_score: composite,
        operator_label: label.to_string(),
        inputs_available,
        components,
        note: String::new(),
    }
}

/// Compute 3-year revenue CAGR from a `FinancialStatements`.
/// Returns `(latest_rev, earliest_rev, years_used, cagr_pct)`. CAGR is
/// `f64::NAN` if the sign rule rejects the series (revenue must both be
/// strictly positive — revenue rarely crosses zero, so NAN usually signals
/// missing data).
fn revenue_cagr_3y_from_statements(statements: &FinancialStatements) -> (f64, f64, usize, f64) {
    let annuals = &statements.income_annual;
    if annuals.len() < 4 {
        return (0.0, 0.0, 0, f64::NAN);
    }
    let latest = annuals[0].revenue;
    let earliest = annuals[3].revenue;
    let years = 3usize;
    if latest > 0.0 && earliest > 0.0 {
        let cagr = ((latest / earliest).powf(1.0 / years as f64) - 1.0) * 100.0;
        (latest, earliest, years, cagr)
    } else if latest.is_finite() && earliest.is_finite() && earliest.abs() > 1e-9 {
        let linear = (latest - earliest) / earliest.abs() / years as f64 * 100.0;
        (latest, earliest, years, linear)
    } else {
        (latest, earliest, years, f64::NAN)
    }
}

/// REVRANK — Relative Revenue Growth Rank.
///
/// Mirrors RELEPSGR but over `IncomeStatement.revenue` instead of EPS.
/// Label ladder: FAR_ABOVE (≥+15pp), ABOVE (≥+5pp), INLINE (±5pp),
/// BELOW (≤-5pp), FAR_BELOW (≤-15pp), CAGR_NEGATIVE (subject endpoints
/// non-positive), NO_DATA.
pub fn compute_revrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&FinancialStatements>,
    peer_statements: &[(String, FinancialStatements)],
) -> RevenueGrowthRankSnapshot {
    let subj = match subject {
        Some(s) if s.income_annual.len() >= 4 => s,
        _ => {
            return RevenueGrowthRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                relative_label: "NO_DATA".into(),
                note: "Subject has < 4 annual income rows".into(),
                ..Default::default()
            };
        }
    };
    let (latest, earliest, years, subj_cagr) = revenue_cagr_3y_from_statements(subj);
    let mut peer_cagrs: Vec<f64> = Vec::new();
    for (_, st) in peer_statements {
        if st.income_annual.len() < 4 {
            continue;
        }
        let (_, _, _, c) = revenue_cagr_3y_from_statements(st);
        if c.is_finite() {
            peer_cagrs.push(c);
        }
    }
    let peers_considered = peer_statements.len();
    let peers_with_data = peer_cagrs.len();
    if peer_cagrs.len() < 3 {
        return RevenueGrowthRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            latest_revenue: latest,
            earliest_revenue: earliest,
            years_used: years,
            symbol_cagr_pct: if subj_cagr.is_finite() {
                subj_cagr
            } else {
                0.0
            },
            peers_considered,
            peers_with_data,
            relative_label: "NO_DATA".into(),
            note: format!(
                "Only {} peers with ≥4 annual rows (need ≥3)",
                peer_cagrs.len()
            ),
            ..Default::default()
        };
    }
    let mut sorted = peer_cagrs.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    if !subj_cagr.is_finite() || latest <= 0.0 || earliest <= 0.0 {
        return RevenueGrowthRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            latest_revenue: latest,
            earliest_revenue: earliest,
            years_used: years,
            symbol_cagr_pct: if subj_cagr.is_finite() {
                subj_cagr
            } else {
                0.0
            },
            peers_considered,
            peers_with_data,
            sector_median_cagr_pct: median,
            sector_p25_cagr_pct: p25,
            sector_p75_cagr_pct: p75,
            relative_label: "CAGR_NEGATIVE".into(),
            note: "Subject revenue crosses zero; using linear proxy".into(),
            ..Default::default()
        };
    }
    let gap = subj_cagr - median;
    let label = if gap >= 15.0 {
        "FAR_ABOVE"
    } else if gap >= 5.0 {
        "ABOVE"
    } else if gap >= -5.0 {
        "INLINE"
    } else if gap >= -15.0 {
        "BELOW"
    } else {
        "FAR_BELOW"
    };
    RevenueGrowthRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        latest_revenue: latest,
        earliest_revenue: earliest,
        years_used: years,
        symbol_cagr_pct: subj_cagr,
        peers_considered,
        peers_with_data,
        sector_median_cagr_pct: median,
        sector_p25_cagr_pct: p25,
        sector_p75_cagr_pct: p75,
        gap_to_median_pp: gap,
        relative_label: label.into(),
        note: String::new(),
    }
}
