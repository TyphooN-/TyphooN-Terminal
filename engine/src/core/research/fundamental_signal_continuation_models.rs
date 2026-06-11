use super::*;

/// DIVACC — dividend growth acceleration using annualized dividend buckets.
pub fn compute_divacc_snapshot(
    symbol: &str,
    as_of: &str,
    dividends: &[DividendRecord],
) -> DividendAccelerationSnapshot {
    let sym = symbol.to_uppercase();
    if dividends.is_empty() {
        return DividendAccelerationSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            divacc_label: "NO_HISTORY".into(),
            note: "no dividend history cached — run DVD first".into(),
            ..Default::default()
        };
    }

    let mut sorted: Vec<&DividendRecord> = dividends
        .iter()
        .filter(|d| d.amount > 0.0 && !d.ex_date.is_empty())
        .collect();
    sorted.sort_by(|a, b| a.ex_date.cmp(&b.ex_date));
    if sorted.is_empty() {
        return DividendAccelerationSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            divacc_label: "NO_HISTORY".into(),
            note: "dividend rows all zero or missing ex_date".into(),
            ..Default::default()
        };
    }

    let mut by_year: std::collections::BTreeMap<i32, (f64, usize)> =
        std::collections::BTreeMap::new();
    for d in &sorted {
        let year: i32 = match d.ex_date.splitn(2, '-').next().and_then(|y| y.parse().ok()) {
            Some(y) => y,
            None => continue,
        };
        let entry = by_year.entry(year).or_insert((0.0, 0));
        entry.0 += d.amount;
        entry.1 += 1;
    }

    let as_of_year: Option<i32> = as_of.splitn(2, '-').next().and_then(|y| y.parse().ok());
    let mut years: Vec<(i32, f64, usize)> =
        by_year.iter().map(|(y, (a, c))| (*y, *a, *c)).collect();
    if let Some(cur) = as_of_year {
        if let Some(last) = years.last() {
            if last.0 == cur {
                let prior_avg_count = if years.len() >= 2 {
                    years[..years.len() - 1]
                        .iter()
                        .rev()
                        .take(3)
                        .map(|row| row.2)
                        .sum::<usize>() as f64
                        / years.len().min(3) as f64
                } else {
                    0.0
                };
                if (last.2 as f64) < prior_avg_count.max(1.0) * 0.75 {
                    years.pop();
                }
            }
        }
    }

    let mut annual_rows: Vec<DivgAnnualRow> = Vec::with_capacity(years.len());
    for (i, (year, total, count)) in years.iter().enumerate() {
        let growth_pct = if i == 0 {
            0.0
        } else {
            let prior = years[i - 1].1;
            if prior > 0.0 {
                ((total - prior) / prior) * 100.0
            } else {
                0.0
            }
        };
        annual_rows.push(DivgAnnualRow {
            year: *year,
            total_amount: *total,
            payment_count: *count,
            growth_pct,
        });
    }

    let years_covered = annual_rows.len();
    let total_payments = sorted.len();
    let latest_year = annual_rows.last().map(|r| r.year).unwrap_or_default();
    let latest_annual_dividend = annual_rows
        .last()
        .map(|r| r.total_amount)
        .unwrap_or_default();

    let mut consecutive_growth_years = 0usize;
    for row in annual_rows.iter().rev() {
        if row.growth_pct > 0.0 {
            consecutive_growth_years += 1;
        } else {
            break;
        }
    }
    let deltas = annual_rows.iter().skip(1).count();
    let non_neg = annual_rows
        .iter()
        .skip(1)
        .filter(|r| r.growth_pct >= 0.0)
        .count();
    let consistency_score_pct = if deltas == 0 {
        0.0
    } else {
        non_neg as f64 / deltas as f64 * 100.0
    };

    if years_covered < 3 {
        return DividendAccelerationSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            total_payments,
            years_covered,
            latest_year,
            latest_annual_dividend,
            consecutive_growth_years,
            consistency_score_pct,
            annual_rows,
            divacc_label: "NO_HISTORY".into(),
            note: "need at least 3 full dividend years to compute acceleration".into(),
            ..Default::default()
        };
    }

    let latest_yoy_growth_pct = annual_rows[years_covered - 1].growth_pct;
    let prior_yoy_growth_pct = annual_rows[years_covered - 2].growth_pct;
    let acceleration_pct_pts = latest_yoy_growth_pct - prior_yoy_growth_pct;

    let recent_growth_slice = &annual_rows[years_covered.saturating_sub(3)..years_covered];
    let recent_3y_avg_growth_pct = recent_growth_slice
        .iter()
        .map(|r| r.growth_pct)
        .sum::<f64>()
        / recent_growth_slice.len() as f64;
    let prior_3y_avg_growth_pct = if years_covered >= 6 {
        let slice = &annual_rows[years_covered - 6..years_covered - 3];
        slice.iter().map(|r| r.growth_pct).sum::<f64>() / slice.len() as f64
    } else {
        0.0
    };
    let acceleration_3y_avg_pct_pts = if years_covered >= 6 {
        recent_3y_avg_growth_pct - prior_3y_avg_growth_pct
    } else {
        0.0
    };

    let divacc_label = if latest_yoy_growth_pct <= -5.0 && acceleration_pct_pts <= -3.0 {
        "CUTTING"
    } else if prior_yoy_growth_pct < 0.0
        && latest_yoy_growth_pct > 0.0
        && acceleration_pct_pts >= 5.0
    {
        "REACCELERATING"
    } else if latest_yoy_growth_pct >= 3.0 && acceleration_pct_pts >= 5.0 {
        "ACCELERATING"
    } else if acceleration_pct_pts <= -5.0 {
        "DECELERATING"
    } else {
        "STABLE"
    };

    DividendAccelerationSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        total_payments,
        years_covered,
        latest_year,
        latest_annual_dividend,
        latest_yoy_growth_pct,
        prior_yoy_growth_pct,
        acceleration_pct_pts,
        recent_3y_avg_growth_pct,
        prior_3y_avg_growth_pct,
        acceleration_3y_avg_pct_pts,
        consecutive_growth_years,
        consistency_score_pct,
        annual_rows,
        divacc_label: divacc_label.into(),
        note: String::new(),
    }
}

fn eps_growth_pct(curr_eps: f64, prior_eps: f64) -> f64 {
    if !curr_eps.is_finite() || !prior_eps.is_finite() || prior_eps.abs() < 1e-9 {
        0.0
    } else {
        (((curr_eps - prior_eps) / prior_eps.abs()) * 100.0).clamp(-500.0, 500.0)
    }
}

fn statement_eps(stmt: &IncomeStatement) -> f64 {
    if stmt.eps_diluted.is_finite() && stmt.eps_diluted.abs() > 1e-9 {
        stmt.eps_diluted
    } else {
        stmt.eps
    }
}

/// EPSACC — EPS acceleration using cached quarterly financial statements.
pub fn compute_epsacc_snapshot(
    symbol: &str,
    as_of: &str,
    statements: &FinancialStatements,
) -> EpsAccelerationSnapshot {
    let sym = symbol.to_uppercase();
    let quarters: Vec<&IncomeStatement> = statements.income_quarterly.iter().take(12).collect();
    if quarters.len() < 6 {
        return EpsAccelerationSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            quarters_used: quarters.len(),
            epsacc_label: "INSUFFICIENT_DATA".into(),
            note: "need at least 6 quarterly statements — run FA first".into(),
            ..Default::default()
        };
    }

    let mut yoy_growths: Vec<f64> = Vec::new();
    for i in 0..quarters.len().saturating_sub(4) {
        yoy_growths.push(eps_growth_pct(
            statement_eps(quarters[i]),
            statement_eps(quarters[i + 4]),
        ));
    }
    if yoy_growths.len() < 2 {
        return EpsAccelerationSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            quarters_used: quarters.len(),
            epsacc_label: "INSUFFICIENT_DATA".into(),
            note: "need at least two quarterly y/y EPS comparisons".into(),
            ..Default::default()
        };
    }

    let latest_yoy_growth_pct = yoy_growths[0];
    let prior_yoy_growth_pct = yoy_growths[1];
    let acceleration_pct_pts = latest_yoy_growth_pct - prior_yoy_growth_pct;
    let recent_2q_avg_yoy_growth_pct =
        yoy_growths.iter().take(2).sum::<f64>() / yoy_growths.iter().take(2).count() as f64;
    let prior_2q_avg_yoy_growth_pct = if yoy_growths.len() >= 4 {
        yoy_growths[2..4].iter().sum::<f64>() / 2.0
    } else {
        0.0
    };
    let positive_yoy_quarters = yoy_growths.iter().filter(|&&v| v > 0.0).count();
    let latest_eps = statement_eps(quarters[0]);
    let prior_year_eps = statement_eps(quarters[4]);

    let epsacc_label =
        if prior_yoy_growth_pct < 0.0 && latest_yoy_growth_pct >= 0.0 && acceleration_pct_pts > 0.0
        {
            "TURNAROUND"
        } else if latest_yoy_growth_pct >= 10.0 && acceleration_pct_pts >= 5.0 {
            "ACCELERATING"
        } else if latest_yoy_growth_pct <= -10.0 && acceleration_pct_pts <= -5.0 {
            "EARNINGS_PRESSURE"
        } else if acceleration_pct_pts <= -5.0 {
            "DECELERATING"
        } else {
            "STABLE"
        };

    EpsAccelerationSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        quarters_used: quarters.len(),
        latest_period: quarters[0].date.clone(),
        latest_eps,
        prior_year_eps,
        latest_yoy_growth_pct,
        prior_yoy_growth_pct,
        acceleration_pct_pts,
        recent_2q_avg_yoy_growth_pct,
        prior_2q_avg_yoy_growth_pct,
        positive_yoy_quarters,
        epsacc_label: epsacc_label.into(),
        note: String::new(),
    }
}

/// VRP — combine IVOL and RVCONE into a focused implied-vs-realized-vol
/// premium view.
pub fn compute_vrp_snapshot(
    symbol: &str,
    as_of: &str,
    ivol: Option<&IvolSnapshot>,
    rvcone: Option<&RealizedVolConeSnapshot>,
) -> VolRiskPremiumSnapshot {
    let sym = symbol.to_uppercase();
    let iv = match ivol {
        Some(s) if s.current_atm_iv_pct > 0.0 => s,
        _ => {
            return VolRiskPremiumSnapshot {
                symbol: sym,
                as_of: as_of.to_string(),
                premium_label: "INSUFFICIENT_DATA".into(),
                note: "Need a cached IVOL snapshot with a current ATM IV reading".into(),
                ..Default::default()
            };
        }
    };
    let rv = match rvcone {
        Some(s) if s.rv20_pct > 0.0 && s.rv252_pct > 0.0 => s,
        _ => {
            return VolRiskPremiumSnapshot {
                symbol: sym,
                as_of: as_of.to_string(),
                current_atm_iv_pct: iv.current_atm_iv_pct,
                iv_rank: iv.iv_rank,
                iv_percentile: iv.iv_percentile,
                iv_observation_count: iv.observation_count,
                premium_label: "INSUFFICIENT_DATA".into(),
                note: "Need a cached RVCONE snapshot with usable realized-vol levels".into(),
                ..Default::default()
            };
        }
    };

    let iv_minus_rv20_pct = iv.current_atm_iv_pct - rv.rv20_pct;
    let iv_to_rv20_ratio = if rv.rv20_pct > 0.0 {
        iv.current_atm_iv_pct / rv.rv20_pct
    } else {
        0.0
    };
    let iv_minus_rv252_pct = iv.current_atm_iv_pct - rv.rv252_pct;
    let iv_to_rv252_ratio = if rv.rv252_pct > 0.0 {
        iv.current_atm_iv_pct / rv.rv252_pct
    } else {
        0.0
    };

    let premium_label = if iv_to_rv20_ratio >= 1.50
        || iv_minus_rv20_pct >= 15.0
        || (iv.iv_rank >= 80.0 && rv.rv20_percentile <= 40.0 && iv_to_rv20_ratio >= 1.25)
    {
        "EXTREME_RICH"
    } else if iv_to_rv20_ratio >= 1.15 || iv_minus_rv20_pct >= 5.0 {
        "RICH_IV"
    } else if iv_to_rv20_ratio <= 0.85 || iv_minus_rv20_pct <= -5.0 {
        "CHEAP_IV"
    } else {
        "FAIR_IV"
    };

    let mut note_parts: Vec<String> = Vec::new();
    if iv.observation_count < 20 && !iv.note.is_empty() {
        note_parts.push(iv.note.clone());
    }
    if !rv.note.is_empty() {
        note_parts.push(rv.note.clone());
    }

    VolRiskPremiumSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        current_atm_iv_pct: iv.current_atm_iv_pct,
        iv_rank: iv.iv_rank,
        iv_percentile: iv.iv_percentile,
        iv_observation_count: iv.observation_count,
        rv20_pct: rv.rv20_pct,
        rv60_pct: rv.rv60_pct,
        rv252_pct: rv.rv252_pct,
        rv20_percentile: rv.rv20_percentile,
        rv_cone_label: rv.cone_label.clone(),
        iv_minus_rv20_pct,
        iv_to_rv20_ratio,
        iv_minus_rv252_pct,
        iv_to_rv252_ratio,
        premium_label: premium_label.into(),
        note: note_parts.join(" | "),
    }
}

fn normalize_short_interest_history_date(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.len() < 10 {
        return None;
    }
    let cand = &trimmed[..10];
    chrono::NaiveDate::parse_from_str(cand, "%Y-%m-%d")
        .ok()
        .map(|_| cand.to_string())
}

fn same_short_interest_point(a: &ShortInterestHistoryPoint, b: &ShortInterestHistoryPoint) -> bool {
    (a.short_percent_of_float - b.short_percent_of_float).abs() < 1e-9
        && (a.short_ratio - b.short_ratio).abs() < 1e-9
        && (a.shares_outstanding - b.shares_outstanding).abs() < 1e-6
}

pub(super) fn merge_short_interest_history_rows(
    existing: &[ShortInterestHistoryPoint],
    new_rows: &[ShortInterestHistoryPoint],
) -> Vec<ShortInterestHistoryPoint> {
    let mut by_date: std::collections::BTreeMap<String, ShortInterestHistoryPoint> =
        std::collections::BTreeMap::new();

    for row in existing.iter().chain(new_rows.iter()) {
        let as_of = match normalize_short_interest_history_date(&row.as_of) {
            Some(v) => v,
            None => continue,
        };
        if !row.short_percent_of_float.is_finite() || row.short_percent_of_float < 0.0 {
            continue;
        }
        let normalized = ShortInterestHistoryPoint {
            as_of: as_of.clone(),
            short_percent_of_float: row.short_percent_of_float,
            short_ratio: if row.short_ratio.is_finite() && row.short_ratio >= 0.0 {
                row.short_ratio
            } else {
                0.0
            },
            shares_outstanding: if row.shares_outstanding.is_finite()
                && row.shares_outstanding >= 0.0
            {
                row.shares_outstanding
            } else {
                0.0
            },
        };
        by_date.insert(as_of, normalized);
    }

    let mut compacted: Vec<ShortInterestHistoryPoint> = Vec::new();
    for row in by_date.into_values() {
        if compacted
            .last()
            .map(|prev| same_short_interest_point(prev, &row))
            .unwrap_or(false)
        {
            continue;
        }
        compacted.push(row);
    }

    let keep_from = compacted.len().saturating_sub(256);
    compacted.into_iter().skip(keep_from).collect()
}

fn short_interest_trend_label(delta_pct_pts: f64) -> &'static str {
    if delta_pct_pts <= -5.0 {
        "HEAVY_COVERING"
    } else if delta_pct_pts <= -1.5 {
        "COVERING"
    } else if delta_pct_pts < 1.5 {
        "STABLE"
    } else if delta_pct_pts < 5.0 {
        "BUILDING"
    } else {
        "HEAVY_BUILD"
    }
}

#[derive(Debug, Clone)]
struct ShortInterestDeltaContext {
    history_points_used: usize,
    history_start_date: String,
    history_end_date: String,
    prior_short_pct_of_float: f64,
    latest_short_pct_of_float: f64,
    prior_short_ratio: f64,
    latest_short_ratio: f64,
    delta_short_pct_points: f64,
}

fn short_interest_delta_context(
    as_of: &str,
    rows: &[ShortInterestHistoryPoint],
    lookback_days: i64,
) -> Option<ShortInterestDeltaContext> {
    let ref_date = normalize_short_interest_history_date(as_of)
        .and_then(|d| chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())
        .or_else(|| {
            rows.iter()
                .rev()
                .find_map(|row| normalize_short_interest_history_date(&row.as_of))
                .and_then(|d| chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())
        })?;

    let min_date = ref_date - chrono::Duration::days(lookback_days.max(1));
    let mut window: Vec<(chrono::NaiveDate, &ShortInterestHistoryPoint)> = rows
        .iter()
        .filter_map(|row| {
            let parsed = normalize_short_interest_history_date(&row.as_of)
                .and_then(|d| chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())?;
            if parsed < min_date || parsed > ref_date {
                None
            } else {
                Some((parsed, row))
            }
        })
        .collect();
    if window.len() < 2 {
        return None;
    }
    window.sort_by_key(|(date, _)| *date);

    let (_, start) = window.first()?;
    let (_, end) = window.last()?;
    Some(ShortInterestDeltaContext {
        history_points_used: window.len(),
        history_start_date: start.as_of.clone(),
        history_end_date: end.as_of.clone(),
        prior_short_pct_of_float: start.short_percent_of_float,
        latest_short_pct_of_float: end.short_percent_of_float,
        prior_short_ratio: start.short_ratio,
        latest_short_ratio: end.short_ratio,
        delta_short_pct_points: end.short_percent_of_float - start.short_percent_of_float,
    })
}

fn json_value_number(row: &serde_json::Value, keys: &[&str]) -> Option<f64> {
    for key in keys {
        let value = match row.get(*key) {
            Some(v) => v,
            None => continue,
        };
        if let Some(num) = value.as_f64() {
            if num.is_finite() {
                return Some(num);
            }
        }
        if let Some(s) = value.as_str() {
            let cleaned = s.trim().replace(',', "");
            if let Ok(num) = cleaned.parse::<f64>() {
                if num.is_finite() {
                    return Some(num);
                }
            }
        }
    }
    None
}

fn json_value_string(row: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        let value = match row.get(*key) {
            Some(v) => v,
            None => continue,
        };
        if let Some(s) = value.as_str() {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

/// Parse vendor short-interest rows into the compact per-symbol history format.
/// The parser is intentionally tolerant because upstream providers use slightly
/// different field names for the same concept.
pub fn short_interest_history_points_from_json_rows(
    rows: &[serde_json::Value],
) -> Vec<ShortInterestHistoryPoint> {
    let parsed: Vec<ShortInterestHistoryPoint> = rows
        .iter()
        .filter_map(|row| {
            let as_of = json_value_string(row, &["date", "settlementDate", "reportDate"])
                .and_then(|s| normalize_short_interest_history_date(&s))?;
            let short_percent_of_float = json_value_number(
                row,
                &[
                    "shortPercentOfFloat",
                    "short_percent_of_float",
                    "shortPercentFloat",
                    "shortInterestPct",
                    "percentOfFloat",
                ],
            )
            .or_else(|| {
                let short_interest =
                    json_value_number(row, &["shortInterest", "shortShares", "sharesShort"])?;
                let float_shares =
                    json_value_number(row, &["shareFloat", "floatShares", "sharesFloat"])?;
                if float_shares > 0.0 {
                    Some(short_interest / float_shares * 100.0)
                } else {
                    None
                }
            })?;
            let short_ratio = json_value_number(
                row,
                &["shortRatio", "daysToCover", "short_ratio", "days_to_cover"],
            )
            .unwrap_or(0.0);
            let shares_outstanding =
                json_value_number(row, &["sharesOutstanding", "shares_outstanding"]).unwrap_or(0.0);

            Some(ShortInterestHistoryPoint {
                as_of,
                short_percent_of_float,
                short_ratio,
                shares_outstanding,
            })
        })
        .collect();

    merge_short_interest_history_rows(&[], &parsed)
}

/// SHORTRANK_DELTA — rank the 180-day change in short_percent_of_float vs
/// same-sector peers. More negative delta (short covering) is safer.
pub fn compute_shortrank_delta_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject_history: &[ShortInterestHistoryPoint],
    peers: &[(String, Vec<ShortInterestHistoryPoint>)],
) -> ShortInterestDeltaRankSnapshot {
    const LOOKBACK_DAYS: i64 = 180;

    let sym = symbol.to_uppercase();
    let subject_ctx = match short_interest_delta_context(as_of, subject_history, LOOKBACK_DAYS) {
        Some(ctx) => ctx,
        None => {
            return ShortInterestDeltaRankSnapshot {
                symbol: sym,
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                lookback_days: LOOKBACK_DAYS as i32,
                rank_label: "NO_DATA".into(),
                note: "Need at least 2 short-interest history points for the subject in the trailing 180d window".into(),
                ..Default::default()
            };
        }
    };

    let peer_contexts: Vec<ShortInterestDeltaContext> = peers
        .iter()
        .filter_map(|(_, rows)| short_interest_delta_context(as_of, rows, LOOKBACK_DAYS))
        .collect();
    let peer_deltas: Vec<f64> = peer_contexts
        .iter()
        .map(|ctx| ctx.delta_short_pct_points)
        .collect();
    let peers_considered = peers.len();
    let peers_with_data = peer_deltas.len();
    if peers_with_data < 3 {
        return ShortInterestDeltaRankSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            lookback_days: LOOKBACK_DAYS as i32,
            history_points_used: subject_ctx.history_points_used,
            history_start_date: subject_ctx.history_start_date,
            history_end_date: subject_ctx.history_end_date,
            latest_short_pct_of_float: subject_ctx.latest_short_pct_of_float,
            prior_short_pct_of_float: subject_ctx.prior_short_pct_of_float,
            delta_short_pct_points: subject_ctx.delta_short_pct_points,
            latest_short_ratio: subject_ctx.latest_short_ratio,
            prior_short_ratio: subject_ctx.prior_short_ratio,
            subject_trend_label: short_interest_trend_label(subject_ctx.delta_short_pct_points)
                .into(),
            peers_considered,
            peers_with_data,
            rank_label: "INSUFFICIENT_DATA".into(),
            note: format!(
                "Only {} sector peers have usable short-interest trend history (need ≥3)",
                peers_with_data
            ),
            ..Default::default()
        };
    }

    let mut sorted = peer_deltas.clone();
    sorted.push(subject_ctx.delta_short_pct_points);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let percentile_rank =
        percentile_rank_score(subject_ctx.delta_short_pct_points, &peer_deltas, false);
    let rank_position = peer_deltas
        .iter()
        .filter(|&&delta| delta < subject_ctx.delta_short_pct_points)
        .count()
        + 1;

    ShortInterestDeltaRankSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        lookback_days: LOOKBACK_DAYS as i32,
        history_points_used: subject_ctx.history_points_used,
        history_start_date: subject_ctx.history_start_date,
        history_end_date: subject_ctx.history_end_date,
        latest_short_pct_of_float: subject_ctx.latest_short_pct_of_float,
        prior_short_pct_of_float: subject_ctx.prior_short_pct_of_float,
        delta_short_pct_points: subject_ctx.delta_short_pct_points,
        latest_short_ratio: subject_ctx.latest_short_ratio,
        prior_short_ratio: subject_ctx.prior_short_ratio,
        subject_trend_label: short_interest_trend_label(subject_ctx.delta_short_pct_points).into(),
        peers_considered,
        peers_with_data,
        sector_median_delta_pct_pts: quantile_f64(&sorted, 0.5),
        sector_p25_delta_pct_pts: quantile_f64(&sorted, 0.25),
        sector_p75_delta_pct_pts: quantile_f64(&sorted, 0.75),
        percentile_rank,
        rank_position,
        rank_label: risk_rank_label_for_percentile(percentile_rank).into(),
        note: String::new(),
    }
}

fn normalize_insider_trade_date(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.len() < 10 {
        return None;
    }
    let cand = &trimmed[..10];
    chrono::NaiveDate::parse_from_str(cand, "%Y-%m-%d")
        .ok()
        .map(|_| cand.to_string())
}

#[derive(Debug, Clone)]
struct InsiderConcentrationContext {
    latest_holdings_date: String,
    trade_rows_used: usize,
    reporters_covered: usize,
    reporters_holding_shares: usize,
    total_estimated_insider_shares: f64,
    estimated_insider_pct_held: f64,
    largest_reporter: String,
    largest_reporter_shares: f64,
    largest_reporter_weight_pct: f64,
}

fn insider_concentration_context(
    shares_outstanding: f64,
    trades: &[InsiderTrade],
) -> Option<InsiderConcentrationContext> {
    if !shares_outstanding.is_finite() || shares_outstanding <= 0.0 {
        return None;
    }

    #[derive(Debug, Clone)]
    struct LatestHolding {
        reporter_display: String,
        effective_date: chrono::NaiveDate,
        effective_date_str: String,
        filing_date: Option<chrono::NaiveDate>,
        shares_owned_after: f64,
    }

    let mut by_reporter: std::collections::BTreeMap<String, LatestHolding> =
        std::collections::BTreeMap::new();
    let mut trade_rows_used = 0usize;

    for trade in trades {
        let reporter_display = trade.reporting_name.trim();
        if reporter_display.is_empty() {
            continue;
        }
        if !trade.shares_owned_after.is_finite() || trade.shares_owned_after < 0.0 {
            continue;
        }
        let effective_date_str = normalize_insider_trade_date(&trade.transaction_date)
            .or_else(|| normalize_insider_trade_date(&trade.filing_date));
        let Some(effective_date_str) = effective_date_str else {
            continue;
        };
        let Some(effective_date) =
            chrono::NaiveDate::parse_from_str(&effective_date_str, "%Y-%m-%d").ok()
        else {
            continue;
        };
        let filing_date = normalize_insider_trade_date(&trade.filing_date)
            .and_then(|d| chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok());
        trade_rows_used += 1;

        let entry = LatestHolding {
            reporter_display: reporter_display.to_string(),
            effective_date,
            effective_date_str,
            filing_date,
            shares_owned_after: trade.shares_owned_after,
        };
        let key = reporter_display.to_lowercase();
        match by_reporter.get_mut(&key) {
            Some(existing) => {
                let take_new = entry.effective_date > existing.effective_date
                    || (entry.effective_date == existing.effective_date
                        && entry.filing_date > existing.filing_date)
                    || (entry.effective_date == existing.effective_date
                        && entry.filing_date == existing.filing_date
                        && entry.shares_owned_after > existing.shares_owned_after);
                if take_new {
                    *existing = entry;
                }
            }
            None => {
                by_reporter.insert(key, entry);
            }
        }
    }

    if by_reporter.is_empty() {
        return None;
    }

    let reporters_covered = by_reporter.len();
    let reporters_holding_shares = by_reporter
        .values()
        .filter(|row| row.shares_owned_after > 0.0)
        .count();
    let total_estimated_insider_shares: f64 =
        by_reporter.values().map(|row| row.shares_owned_after).sum();
    let estimated_insider_pct_held = total_estimated_insider_shares / shares_outstanding * 100.0;

    let (largest_reporter, largest_reporter_shares) = by_reporter
        .values()
        .max_by(|a, b| {
            a.shares_owned_after
                .partial_cmp(&b.shares_owned_after)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|row| (row.reporter_display.clone(), row.shares_owned_after))
        .unwrap_or_default();
    let largest_reporter_weight_pct = if total_estimated_insider_shares > 0.0 {
        largest_reporter_shares / total_estimated_insider_shares * 100.0
    } else {
        0.0
    };
    let latest_holdings_date = by_reporter
        .values()
        .map(|row| row.effective_date_str.as_str())
        .max()
        .unwrap_or("")
        .to_string();

    Some(InsiderConcentrationContext {
        latest_holdings_date,
        trade_rows_used,
        reporters_covered,
        reporters_holding_shares,
        total_estimated_insider_shares,
        estimated_insider_pct_held,
        largest_reporter,
        largest_reporter_shares,
        largest_reporter_weight_pct,
    })
}

/// INSIDERCONC — estimate insider-held % from cached INS rows, then rank that
/// vs same-sector peers. Higher insider concentration earns a higher rank.
pub fn compute_insiderconc_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    shares_outstanding: Option<f64>,
    subject_trades: &[InsiderTrade],
    peers: &[(String, Option<f64>, Vec<InsiderTrade>)],
) -> InsiderConcentrationSnapshot {
    let sym = symbol.to_uppercase();
    let Some(subject_shares_outstanding) = shares_outstanding else {
        return InsiderConcentrationSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            rank_label: "NO_DATA".into(),
            note: "Need Fundamentals.shares_outstanding and cached INS rows for the subject".into(),
            ..Default::default()
        };
    };

    let Some(subject_ctx) =
        insider_concentration_context(subject_shares_outstanding, subject_trades)
    else {
        return InsiderConcentrationSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            shares_outstanding: if subject_shares_outstanding.is_finite()
                && subject_shares_outstanding > 0.0
            {
                subject_shares_outstanding
            } else {
                0.0
            },
            rank_label: "NO_DATA".into(),
            note: "Need at least one dated INS row with shares_owned_after for the subject".into(),
            ..Default::default()
        };
    };

    let peer_contexts: Vec<InsiderConcentrationContext> = peers
        .iter()
        .filter_map(|(_, peer_shares_outstanding, peer_trades)| {
            peer_shares_outstanding
                .and_then(|shares| insider_concentration_context(shares, peer_trades))
        })
        .collect();
    let peer_pcts: Vec<f64> = peer_contexts
        .iter()
        .map(|ctx| ctx.estimated_insider_pct_held)
        .collect();
    let peers_considered = peers.len();
    let peers_with_data = peer_pcts.len();
    if peers_with_data < 3 {
        return InsiderConcentrationSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            latest_holdings_date: subject_ctx.latest_holdings_date,
            trade_rows_used: subject_ctx.trade_rows_used,
            reporters_covered: subject_ctx.reporters_covered,
            reporters_holding_shares: subject_ctx.reporters_holding_shares,
            shares_outstanding: subject_shares_outstanding,
            total_estimated_insider_shares: subject_ctx.total_estimated_insider_shares,
            estimated_insider_pct_held: subject_ctx.estimated_insider_pct_held,
            largest_reporter: subject_ctx.largest_reporter.clone(),
            largest_reporter_shares: subject_ctx.largest_reporter_shares,
            largest_reporter_pct_of_outstanding: subject_ctx.largest_reporter_shares
                / subject_shares_outstanding
                * 100.0,
            largest_reporter_weight_pct: subject_ctx.largest_reporter_weight_pct,
            peers_considered,
            peers_with_data,
            rank_label: "INSUFFICIENT_DATA".into(),
            note: format!(
                "Only {} sector peers have usable INS holdings coverage (need ≥3)",
                peers_with_data
            ),
            ..Default::default()
        };
    }

    let mut sorted = peer_pcts.clone();
    sorted.push(subject_ctx.estimated_insider_pct_held);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let percentile_rank =
        percentile_rank_score(subject_ctx.estimated_insider_pct_held, &peer_pcts, true);
    let rank_position = peer_pcts
        .iter()
        .filter(|&&pct| pct > subject_ctx.estimated_insider_pct_held)
        .count()
        + 1;

    InsiderConcentrationSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        latest_holdings_date: subject_ctx.latest_holdings_date,
        trade_rows_used: subject_ctx.trade_rows_used,
        reporters_covered: subject_ctx.reporters_covered,
        reporters_holding_shares: subject_ctx.reporters_holding_shares,
        shares_outstanding: subject_shares_outstanding,
        total_estimated_insider_shares: subject_ctx.total_estimated_insider_shares,
        estimated_insider_pct_held: subject_ctx.estimated_insider_pct_held,
        largest_reporter: subject_ctx.largest_reporter.clone(),
        largest_reporter_shares: subject_ctx.largest_reporter_shares,
        largest_reporter_pct_of_outstanding: subject_ctx.largest_reporter_shares
            / subject_shares_outstanding
            * 100.0,
        largest_reporter_weight_pct: subject_ctx.largest_reporter_weight_pct,
        peers_considered,
        peers_with_data,
        sector_median_pct_held: quantile_f64(&sorted, 0.5),
        sector_p25_pct_held: quantile_f64(&sorted, 0.25),
        sector_p75_pct_held: quantile_f64(&sorted, 0.75),
        percentile_rank,
        rank_position,
        rank_label: rank_label_for_percentile(percentile_rank).into(),
        note: String::new(),
    }
}
