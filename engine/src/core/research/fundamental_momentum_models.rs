use super::*;

// ── Godel Parity Round 12 compute fns ──────────────────────────────

pub(super) fn parse_yyyy_mm_dd_to_days(s: &str) -> Option<i64> {
    // Crude julian-ish day number. We don't need calendar correctness — just
    // a monotone integer for sorting & window comparisons against "today".
    let parts: Vec<&str> = s.splitn(3, '-').collect();
    if parts.len() != 3 {
        return None;
    }
    let y: i64 = parts[0].parse().ok()?;
    let m: i64 = parts[1].parse().ok()?;
    let d: i64 = parts[2].parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(y * 372 + m * 31 + d)
}

/// MNGR — Insider Activity Bias score computed over a lookback window.
/// Buckets insider trades into buys/sells/other, computes gross/net values,
/// classifies bias from net-value direction and conviction from trade count.
pub fn compute_insider_activity_snapshot(
    symbol: &str,
    as_of: &str,
    trades: &[InsiderTrade],
    window_days: i32,
) -> InsiderActivitySnapshot {
    let sym = symbol.to_uppercase();
    let as_of_days = parse_yyyy_mm_dd_to_days(as_of);
    let cutoff_days = as_of_days.map(|d| d - window_days as i64);

    let in_window: Vec<&InsiderTrade> = trades
        .iter()
        .filter(|t| {
            match (cutoff_days, parse_yyyy_mm_dd_to_days(&t.transaction_date)) {
                (Some(c), Some(td)) => td >= c,
                _ => true, // if either date unparsable, include it
            }
        })
        .collect();

    if in_window.is_empty() {
        return InsiderActivitySnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            window_days,
            bias_label: "NO_ACTIVITY".to_string(),
            conviction_label: "NONE".to_string(),
            note: "no insider trades in lookback window — run INS first".to_string(),
            ..Default::default()
        };
    }

    let classify = |t: &InsiderTrade| -> &'static str {
        let upper = t.transaction_type.to_uppercase();
        let disp = t.acquisition_disposition.to_uppercase();
        if upper.contains("P-PURCHASE")
            || upper.starts_with("P ")
            || upper == "P"
            || upper.contains("PURCHASE")
        {
            "buy"
        } else if upper.contains("S-SALE")
            || upper.starts_with("S ")
            || upper == "S"
            || upper.contains("SALE")
        {
            "sell"
        } else if disp == "A" {
            "buy"
        } else if disp == "D" {
            "sell"
        } else {
            "other"
        }
    };

    let mut buy_count = 0usize;
    let mut sell_count = 0usize;
    let mut other_count = 0usize;
    let mut gross_buy_value = 0.0f64;
    let mut gross_sell_value = 0.0f64;
    let mut net_shares = 0.0f64;
    let mut insiders: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut latest_date = String::new();
    let mut latest_days: i64 = i64::MIN;

    for t in &in_window {
        let v = if t.value_usd.abs() > 0.0 {
            t.value_usd.abs()
        } else {
            (t.shares * t.price).abs()
        };
        match classify(t) {
            "buy" => {
                buy_count += 1;
                gross_buy_value += v;
                net_shares += t.shares.abs();
            }
            "sell" => {
                sell_count += 1;
                gross_sell_value += v;
                net_shares -= t.shares.abs();
            }
            _ => other_count += 1,
        }
        if !t.reporting_name.trim().is_empty() {
            insiders.insert(t.reporting_name.trim().to_lowercase());
        }
        if let Some(td) = parse_yyyy_mm_dd_to_days(&t.transaction_date) {
            if td > latest_days {
                latest_days = td;
                latest_date = t.transaction_date.clone();
            }
        }
    }

    let net_value = gross_buy_value - gross_sell_value;
    let buy_sell_ratio = if sell_count > 0 {
        buy_count as f64 / sell_count as f64
    } else {
        buy_count as f64
    };

    let total_trades = in_window.len();
    let unique = insiders.len();

    let bias = if buy_count == 0 && sell_count == 0 {
        "NO_ACTIVITY"
    } else if net_value > 0.0 && buy_count >= sell_count {
        "BULLISH"
    } else if net_value < 0.0 && sell_count > buy_count {
        "BEARISH"
    } else {
        "NEUTRAL"
    };

    let total_gross = gross_buy_value + gross_sell_value;
    let conviction = if total_gross <= 0.0 || unique == 0 {
        "NONE"
    } else if unique >= 3 && total_gross >= 1_000_000.0 {
        "HIGH"
    } else if unique >= 2 || total_gross >= 250_000.0 {
        "MEDIUM"
    } else {
        "LOW"
    };

    InsiderActivitySnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        window_days,
        total_trades,
        buy_count,
        sell_count,
        other_count,
        unique_insiders: unique,
        gross_buy_value_usd: gross_buy_value,
        gross_sell_value_usd: gross_sell_value,
        net_value_usd: net_value,
        buy_sell_ratio,
        net_shares,
        latest_trade_date: latest_date,
        bias_label: bias.to_string(),
        conviction_label: conviction.to_string(),
        note: String::new(),
    }
}

/// DIVG — Dividend Growth Analysis computed over cached DVD rows.
/// Buckets payments by calendar year, computes CAGRs and consistency.
pub fn compute_divg_snapshot(
    symbol: &str,
    as_of: &str,
    dividends: &[DividendRecord],
) -> DivgSnapshot {
    let sym = symbol.to_uppercase();

    if dividends.is_empty() {
        return DivgSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            trend_label: "NO_HISTORY".to_string(),
            note: "no dividend history cached — run DVD first".to_string(),
            ..Default::default()
        };
    }

    // Sort by ex_date ascending
    let mut sorted: Vec<&DividendRecord> = dividends
        .iter()
        .filter(|d| d.amount > 0.0 && !d.ex_date.is_empty())
        .collect();
    sorted.sort_by(|a, b| a.ex_date.cmp(&b.ex_date));

    if sorted.is_empty() {
        return DivgSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            trend_label: "NO_HISTORY".to_string(),
            note: "dividend rows all zero or missing ex_date".to_string(),
            ..Default::default()
        };
    }

    let first_payment_date = sorted.first().unwrap().ex_date.clone();
    let latest_payment_date = sorted.last().unwrap().ex_date.clone();
    let latest_amount = sorted.last().unwrap().amount;
    let total_payments = sorted.len();

    // Annualized = sum of most recent up-to-4 payments
    let tail_n = sorted.len().min(4);
    let annualized: f64 = sorted.iter().rev().take(tail_n).map(|d| d.amount).sum();

    // Bucket by year
    let mut by_year: std::collections::BTreeMap<i32, (f64, usize)> =
        std::collections::BTreeMap::new();
    for d in &sorted {
        let year: i32 = match d.ex_date.splitn(2, '-').next().and_then(|y| y.parse().ok()) {
            Some(y) => y,
            None => continue,
        };
        let e = by_year.entry(year).or_insert((0.0, 0));
        e.0 += d.amount;
        e.1 += 1;
    }

    // Determine current year from as_of
    let as_of_year: Option<i32> = as_of.splitn(2, '-').next().and_then(|y| y.parse().ok());

    // Exclude the in-progress current year when it's incomplete (fewer payments than prior year).
    // We still keep prior years as-is. Sort into Vec<(year, amount, count)>.
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
                        .map(|r| r.2)
                        .sum::<usize>() as f64
                        / years.len().min(3) as f64
                } else {
                    0.0
                };
                if (last.2 as f64) < prior_avg_count.max(1.0) * 0.75 {
                    years.pop(); // drop incomplete current year from growth analysis
                }
            }
        }
    }

    let mut annual_rows: Vec<DivgAnnualRow> = Vec::with_capacity(years.len());
    for (i, (y, a, c)) in years.iter().enumerate() {
        let growth = if i == 0 {
            0.0
        } else {
            let prior = years[i - 1].1;
            if prior > 0.0 {
                (a - prior) / prior * 100.0
            } else {
                0.0
            }
        };
        annual_rows.push(DivgAnnualRow {
            year: *y,
            total_amount: *a,
            payment_count: *c,
            growth_pct: growth,
        });
    }

    let years_covered = annual_rows.len();
    let cagr = |from: f64, to: f64, n: f64| -> f64 {
        if from <= 0.0 || to <= 0.0 || n <= 0.0 {
            0.0
        } else {
            ((to / from).powf(1.0 / n) - 1.0) * 100.0
        }
    };

    let cagr_1y = if years_covered >= 2 {
        annual_rows.last().unwrap().growth_pct
    } else {
        0.0
    };
    let cagr_3y = if years_covered >= 4 {
        let n = years_covered;
        cagr(
            annual_rows[n - 4].total_amount,
            annual_rows[n - 1].total_amount,
            3.0,
        )
    } else {
        0.0
    };
    let cagr_5y = if years_covered >= 6 {
        let n = years_covered;
        cagr(
            annual_rows[n - 6].total_amount,
            annual_rows[n - 1].total_amount,
            5.0,
        )
    } else {
        0.0
    };

    // Consecutive growth years counted from the latest backwards
    let mut consecutive = 0usize;
    for row in annual_rows.iter().rev() {
        if row.growth_pct > 0.0 {
            consecutive += 1;
        } else {
            break;
        }
    }
    // Consecutive counting consumes the latest `consecutive` rows whose growth > 0.
    // The earliest row always has growth_pct = 0.0 so we never count it.

    // Consistency: share of yoy deltas that were non-negative
    let deltas = annual_rows.iter().skip(1).count();
    let non_neg = annual_rows
        .iter()
        .skip(1)
        .filter(|r| r.growth_pct >= 0.0)
        .count();
    let consistency_pct = if deltas == 0 {
        0.0
    } else {
        non_neg as f64 / deltas as f64 * 100.0
    };

    let trend_label = if years_covered < 2 {
        "NO_HISTORY"
    } else if cagr_1y >= 3.0 && consistency_pct >= 70.0 {
        "GROWING"
    } else if cagr_1y <= -5.0 {
        "CUTTING"
    } else {
        "STABLE"
    };

    DivgSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        total_payments,
        first_payment_date,
        latest_payment_date,
        latest_amount,
        annualized_dividend: annualized,
        years_covered,
        cagr_1y_pct: cagr_1y,
        cagr_3y_pct: cagr_3y,
        cagr_5y_pct: cagr_5y,
        consecutive_growth_years: consecutive,
        consistency_score_pct: consistency_pct,
        annual_rows,
        trend_label: trend_label.to_string(),
        note: String::new(),
    }
}

/// EARM — Earnings Momentum Trend computed over cached FA + EPS surprises.
pub fn compute_earm_snapshot(
    symbol: &str,
    as_of: &str,
    statements: &FinancialStatements,
    surprises: &[EarningsSurprise],
) -> EarmSnapshot {
    let sym = symbol.to_uppercase();

    let quarters: Vec<&IncomeStatement> = statements.income_quarterly.iter().take(12).collect();

    if quarters.len() < 5 {
        return EarmSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            quarters_used: quarters.len(),
            momentum_label: "INSUFFICIENT_DATA".to_string(),
            note: "need at least 5 quarterly statements — run FA first".to_string(),
            ..Default::default()
        };
    }

    // Assume income_quarterly is newest-first (consistent with other compute fns in this file).
    // quarters[0] = latest, quarters[4] = year ago.
    let mut rows: Vec<EarmQuarterRow> = Vec::with_capacity(quarters.len());
    for (i, q) in quarters.iter().enumerate() {
        let yoy_pct = if i + 4 < quarters.len() {
            let prior = quarters[i + 4].revenue;
            if prior.abs() > 0.0 {
                (q.revenue - prior) / prior.abs() * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        };
        let surprise = surprises.iter().find(|s| s.date == q.date);
        rows.push(EarmQuarterRow {
            period: q.date.clone(),
            revenue: q.revenue,
            revenue_yoy_pct: yoy_pct,
            eps_actual: surprise.map(|s| s.eps_actual).unwrap_or(q.eps),
            eps_estimate: surprise.map(|s| s.eps_estimate).unwrap_or(0.0),
            eps_surprise_pct: surprise.map(|s| s.surprise_pct).unwrap_or(0.0),
        });
    }

    // Compute revenue growth averages: latest 4Q vs prior 4Q.
    // Row indices 0..=3 are "recent", 4..=7 are "prior". If we have fewer than 8 rows,
    // use whatever overlap is available for "prior".
    let recent_count = rows
        .iter()
        .take(4)
        .filter(|r| r.revenue_yoy_pct != 0.0)
        .count();
    let recent_rev_growth: f64 = if recent_count == 0 {
        0.0
    } else {
        rows.iter().take(4).map(|r| r.revenue_yoy_pct).sum::<f64>() / recent_count as f64
    };
    let prior_slice = if rows.len() >= 8 {
        &rows[4..8]
    } else if rows.len() > 4 {
        &rows[4..]
    } else {
        &[]
    };
    let prior_count = prior_slice
        .iter()
        .filter(|r| r.revenue_yoy_pct != 0.0)
        .count();
    let prior_rev_growth: f64 = if prior_count == 0 {
        0.0
    } else {
        prior_slice.iter().map(|r| r.revenue_yoy_pct).sum::<f64>() / prior_count as f64
    };
    let rev_accel = recent_rev_growth - prior_rev_growth;

    // Similar for EPS surprise %. Pull directly from surprises array if FA/surprise alignment is sparse.
    let recent_surprises: Vec<f64> = surprises.iter().take(4).map(|s| s.surprise_pct).collect();
    let prior_surprises: Vec<f64> = surprises
        .iter()
        .skip(4)
        .take(4)
        .map(|s| s.surprise_pct)
        .collect();
    let recent_eps_surp = if recent_surprises.is_empty() {
        0.0
    } else {
        recent_surprises.iter().sum::<f64>() / recent_surprises.len() as f64
    };
    let prior_eps_surp = if prior_surprises.is_empty() {
        0.0
    } else {
        prior_surprises.iter().sum::<f64>() / prior_surprises.len() as f64
    };
    let eps_accel = recent_eps_surp - prior_eps_surp;

    // Composite 0..100: combine growth level, growth acceleration, surprise level, surprise acceleration.
    // Each component clamped and scaled.
    let clamp = |x: f64, lo: f64, hi: f64| -> f64 { x.max(lo).min(hi) };
    let g1 = (clamp(recent_rev_growth, -30.0, 30.0) + 30.0) / 60.0 * 100.0;
    let g2 = (clamp(rev_accel, -15.0, 15.0) + 15.0) / 30.0 * 100.0;
    let g3 = (clamp(recent_eps_surp, -30.0, 30.0) + 30.0) / 60.0 * 100.0;
    let g4 = (clamp(eps_accel, -15.0, 15.0) + 15.0) / 30.0 * 100.0;
    let composite = (g1 * 0.35 + g2 * 0.25 + g3 * 0.25 + g4 * 0.15)
        .max(0.0)
        .min(100.0);

    let momentum = if composite >= 65.0 && (rev_accel > 0.0 || eps_accel > 0.0) {
        "ACCELERATING"
    } else if composite <= 35.0 && (rev_accel < 0.0 || eps_accel < 0.0) {
        "DECELERATING"
    } else {
        "STABLE"
    };

    EarmSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        quarters_used: quarters.len(),
        recent_revenue_growth_pct: recent_rev_growth,
        prior_revenue_growth_pct: prior_rev_growth,
        revenue_acceleration_pct: rev_accel,
        recent_eps_surprise_pct: recent_eps_surp,
        prior_eps_surprise_pct: prior_eps_surp,
        eps_surprise_acceleration_pct: eps_accel,
        composite_score: composite,
        momentum_label: momentum.to_string(),
        quarters: rows,
        note: String::new(),
    }
}

/// SECTR — Sector Rotation Strength for a symbol, using the latest INDU snapshot.
pub fn compute_sector_rotation_snapshot(
    symbol: &str,
    as_of: &str,
    symbol_sector: &str,
    sectors: &[SectorPerformance],
) -> SectorRotationSnapshot {
    let sym = symbol.to_uppercase();

    if sectors.is_empty() {
        return SectorRotationSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            symbol_sector: symbol_sector.to_string(),
            strength_label: "NO_DATA".to_string(),
            note: "no sector performance cached — run INDU first".to_string(),
            ..Default::default()
        };
    }

    let mut ranked: Vec<&SectorPerformance> = sectors.iter().collect();
    ranked.sort_by(|a, b| {
        b.change_pct
            .partial_cmp(&a.change_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let sectors_total = ranked.len() as i32;
    let avg_change = ranked.iter().map(|s| s.change_pct).sum::<f64>() / ranked.len() as f64;

    let mut sorted_pcts: Vec<f64> = ranked.iter().map(|s| s.change_pct).collect();
    sorted_pcts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_change = if sorted_pcts.is_empty() {
        0.0
    } else if sorted_pcts.len() % 2 == 1 {
        sorted_pcts[sorted_pcts.len() / 2]
    } else {
        (sorted_pcts[sorted_pcts.len() / 2 - 1] + sorted_pcts[sorted_pcts.len() / 2]) / 2.0
    };

    let breadth =
        ranked.iter().filter(|s| s.change_pct > 0.0).count() as f64 / ranked.len() as f64 * 100.0;

    let strongest = ranked.first().unwrap();
    let weakest = ranked.last().unwrap();

    // Locate symbol's sector. Fuzzy-match: exact, case-insensitive, contains.
    let target = symbol_sector.trim();
    let target_lower = target.to_lowercase();
    let (symbol_rank, symbol_change) = if target.is_empty() {
        (0i32, 0.0f64)
    } else {
        let mut rank = 0i32;
        let mut change = 0.0f64;
        for (i, s) in ranked.iter().enumerate() {
            let a = s.sector.to_lowercase();
            if a == target_lower || a.contains(&target_lower) || target_lower.contains(&a) {
                rank = (i + 1) as i32;
                change = s.change_pct;
                break;
            }
        }
        (rank, change)
    };

    let rel_strength = symbol_change - avg_change;

    let strength = if symbol_rank == 0 {
        "NO_DATA"
    } else if symbol_rank <= (sectors_total / 3).max(1) && rel_strength > 0.0 {
        "LEADER"
    } else if symbol_rank > sectors_total - (sectors_total / 3).max(1) && rel_strength < 0.0 {
        "LAGGARD"
    } else {
        "NEUTRAL"
    };

    let note = if symbol_rank == 0 && !target.is_empty() {
        format!(
            "symbol sector '{}' not found in cached INDU snapshot",
            target
        )
    } else {
        String::new()
    };

    SectorRotationSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        symbol_sector: symbol_sector.to_string(),
        symbol_sector_change_pct: symbol_change,
        sector_rank: symbol_rank,
        sectors_total,
        avg_sector_change_pct: avg_change,
        median_sector_change_pct: median_change,
        relative_strength_pct: rel_strength,
        breadth_pct: breadth,
        strongest_sector: strongest.sector.clone(),
        strongest_sector_pct: strongest.change_pct,
        weakest_sector: weakest.sector.clone(),
        weakest_sector_pct: weakest.change_pct,
        strength_label: strength.to_string(),
        note,
    }
}

/// UPDM — Upgrade/Downgrade Momentum snapshot for a symbol.
pub fn compute_updm_snapshot(symbol: &str, as_of: &str, actions: &[RatingChange]) -> UpdmSnapshot {
    let sym = symbol.to_uppercase();
    let as_of_days = parse_yyyy_mm_dd_to_days(as_of);

    if actions.is_empty() || as_of_days.is_none() {
        return UpdmSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bias_label: "NO_COVERAGE".to_string(),
            trend_label: "STABLE".to_string(),
            note: "no rating change history cached — run UPDG first".to_string(),
            ..Default::default()
        };
    }
    let as_of_days = as_of_days.unwrap();

    let (mut up30, mut dn30, mut up90, mut dn90, mut up180, mut dn180) = (0, 0, 0, 0, 0, 0);
    let mut init90 = 0;
    let mut maint90 = 0;
    let mut total = 0;
    let mut latest_days: i64 = i64::MIN;
    let mut latest: Option<&RatingChange> = None;

    for a in actions {
        total += 1;
        let ad = match parse_yyyy_mm_dd_to_days(&a.date) {
            Some(d) => d,
            None => continue,
        };
        let delta = as_of_days - ad;
        if delta < 0 {
            continue;
        }
        let act = a.action.to_lowercase();
        let is_up = act.contains("upgrade");
        let is_dn = act.contains("downgrade");
        let is_init = act.contains("init");
        let is_maint = act.contains("maintain") || act.contains("reiterat");

        if delta <= 30 {
            if is_up {
                up30 += 1;
            }
            if is_dn {
                dn30 += 1;
            }
        }
        if delta <= 90 {
            if is_up {
                up90 += 1;
            }
            if is_dn {
                dn90 += 1;
            }
            if is_init {
                init90 += 1;
            }
            if is_maint {
                maint90 += 1;
            }
        }
        if delta <= 180 {
            if is_up {
                up180 += 1;
            }
            if is_dn {
                dn180 += 1;
            }
        }

        if ad > latest_days {
            latest_days = ad;
            latest = Some(a);
        }
    }

    let net_30 = up30 as i32 - dn30 as i32;
    let net_90 = up90 as i32 - dn90 as i32;
    let net_180 = up180 as i32 - dn180 as i32;

    let bias = if up90 == 0 && dn90 == 0 && init90 == 0 && maint90 == 0 {
        "NO_COVERAGE"
    } else if net_90 > 0 {
        "BULLISH"
    } else if net_90 < 0 {
        "BEARISH"
    } else {
        "NEUTRAL"
    };

    let trend = if net_30 > 0 && net_30 as i64 * 3 >= net_90 as i64 {
        "IMPROVING"
    } else if net_30 < 0 && net_30 as i64 * 3 <= net_90 as i64 {
        "DETERIORATING"
    } else {
        "STABLE"
    };

    let (latest_date, latest_action, latest_firm, latest_grade) = latest
        .map(|l| {
            (
                l.date.clone(),
                l.action.clone(),
                l.firm.clone(),
                l.to_grade.clone(),
            )
        })
        .unwrap_or_default();

    UpdmSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        total_actions: total,
        upgrades_30d: up30,
        downgrades_30d: dn30,
        upgrades_90d: up90,
        downgrades_90d: dn90,
        upgrades_180d: up180,
        downgrades_180d: dn180,
        initiations_90d: init90,
        maintains_90d: maint90,
        net_30d: net_30,
        net_90d: net_90,
        net_180d: net_180,
        latest_date,
        latest_action,
        latest_firm,
        latest_to_grade: latest_grade,
        bias_label: bias.to_string(),
        trend_label: trend.to_string(),
        note: String::new(),
    }
}
