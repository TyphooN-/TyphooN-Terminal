//! Fundamental leverage and earnings-quality research snapshot computations.

use super::*;

// ── LEV compute (leverage & coverage ratios) ─────────────

/// Compute a `LeverageSnapshot` from cached financial statements and the
/// Fundamentals row. Pulls trailing-12-month EBITDA + interest expense from
/// quarterly income statements, total debt / equity / cash from the most
/// recent annual balance sheet, and produces a standard battery of ratios.
pub fn compute_leverage_snapshot(
    symbol: &str,
    as_of: &str,
    statements: &FinancialStatements,
    total_debt_fund: f64,
    cash_fund: f64,
) -> LeverageSnapshot {
    // Prefer the latest annual balance sheet; fall back to the most recent quarter.
    let bal = statements
        .balance_annual
        .first()
        .or_else(|| statements.balance_quarterly.first());

    let total_debt = bal
        .map(|b| b.total_debt)
        .filter(|v| *v > 0.0)
        .unwrap_or(total_debt_fund);
    let cash = bal
        .map(|b| b.cash_and_equiv)
        .filter(|v| *v > 0.0)
        .unwrap_or(cash_fund);
    let net_debt = (total_debt - cash).max(0.0);
    let total_equity = bal.map(|b| b.total_equity).unwrap_or(0.0);

    // TTM roll-ups from quarterly income statements (last 4 quarters).
    let q = &statements.income_quarterly;
    let take = q.iter().take(4);
    let ebitda_ttm: f64 = take.clone().map(|i| i.ebitda).sum();
    let interest_ttm: f64 = take.clone().map(|i| i.interest_expense.abs()).sum();
    let op_inc_ttm: f64 = take.clone().map(|i| i.operating_income).sum();

    let cur_assets = bal.map(|b| b.total_current_assets).unwrap_or(0.0);
    let cur_liab = bal.map(|b| b.total_current_liabilities).unwrap_or(0.0);
    let inventory = bal.map(|b| b.inventory).unwrap_or(0.0);

    let mut ratios: Vec<LeverageRatio> = Vec::new();

    // Debt / EBITDA
    if ebitda_ttm > 0.0 && total_debt > 0.0 {
        let v = total_debt / ebitda_ttm;
        let sig = if v < 2.5 {
            "HEALTHY"
        } else if v < 4.0 {
            "ELEVATED"
        } else {
            "STRETCHED"
        };
        ratios.push(LeverageRatio {
            name: "Debt / EBITDA".into(),
            value: v,
            peer_median: 0.0,
            signal: sig.into(),
            note: "lower is safer; >4× typically flags high leverage".into(),
        });
    }

    // Net Debt / EBITDA
    if ebitda_ttm > 0.0 {
        let v = net_debt / ebitda_ttm;
        let sig = if v < 2.0 {
            "HEALTHY"
        } else if v < 3.5 {
            "ELEVATED"
        } else {
            "STRETCHED"
        };
        ratios.push(LeverageRatio {
            name: "Net Debt / EBITDA".into(),
            value: v,
            peer_median: 0.0,
            signal: sig.into(),
            note: "net of cash; negative when cash > debt".into(),
        });
    }

    // Debt / Equity
    if total_equity > 0.0 && total_debt > 0.0 {
        let v = total_debt / total_equity;
        let sig = if v < 1.0 {
            "HEALTHY"
        } else if v < 2.0 {
            "ELEVATED"
        } else {
            "STRETCHED"
        };
        ratios.push(LeverageRatio {
            name: "Debt / Equity".into(),
            value: v,
            peer_median: 0.0,
            signal: sig.into(),
            note: "gearing ratio; varies by sector".into(),
        });
    }

    // Interest Coverage (EBIT / Interest)
    if interest_ttm > 0.0 {
        let v = op_inc_ttm / interest_ttm;
        let sig = if v >= 5.0 {
            "HEALTHY"
        } else if v >= 2.0 {
            "ELEVATED"
        } else {
            "STRETCHED"
        };
        ratios.push(LeverageRatio {
            name: "Interest Coverage".into(),
            value: v,
            peer_median: 0.0,
            signal: sig.into(),
            note: "EBIT / interest expense; higher is safer; <2× distress signal".into(),
        });
    }

    // Current Ratio
    if cur_liab > 0.0 && cur_assets > 0.0 {
        let v = cur_assets / cur_liab;
        let sig = if v >= 1.5 {
            "HEALTHY"
        } else if v >= 1.0 {
            "ELEVATED"
        } else {
            "STRETCHED"
        };
        ratios.push(LeverageRatio {
            name: "Current Ratio".into(),
            value: v,
            peer_median: 0.0,
            signal: sig.into(),
            note: "short-term liquidity; <1 flags near-term squeeze".into(),
        });
    }

    // Quick Ratio
    if cur_liab > 0.0 && cur_assets > 0.0 {
        let v = (cur_assets - inventory) / cur_liab;
        let sig = if v >= 1.0 {
            "HEALTHY"
        } else if v >= 0.7 {
            "ELEVATED"
        } else {
            "STRETCHED"
        };
        ratios.push(LeverageRatio {
            name: "Quick Ratio".into(),
            value: v,
            peer_median: 0.0,
            signal: sig.into(),
            note: "excludes inventory; more conservative than current ratio".into(),
        });
    }

    // Solvency summary: count HEALTHY vs STRETCHED signals.
    let n_health = ratios.iter().filter(|r| r.signal == "HEALTHY").count();
    let n_stretch = ratios.iter().filter(|r| r.signal == "STRETCHED").count();
    let solvency_summary = if ratios.is_empty() {
        "insufficient data — run FA + EVSCRAPE first".to_string()
    } else if n_stretch >= 2 {
        format!("STRETCHED — {}/{} ratios flagged", n_stretch, ratios.len())
    } else if n_health >= ratios.len() / 2 + 1 {
        format!(
            "HEALTHY — {}/{} ratios in safe zone",
            n_health,
            ratios.len()
        )
    } else {
        "MIXED — some pressure points but no widespread stress".to_string()
    };

    let note = if ratios.is_empty() {
        "no cached financial statements — run FA".to_string()
    } else {
        String::new()
    };

    LeverageSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        total_debt,
        net_debt,
        ebitda_ttm,
        interest_expense_ttm: interest_ttm,
        total_equity,
        ratios,
        solvency_summary,
        note,
    }
}

// ── ACRL compute (earnings quality / accruals) ───────────

/// Compute an `AccrualsSnapshot` from cached financial statements. Walks the
/// last 4 quarterly income + cash-flow pairs, producing an FCF/NI ratio per
/// period plus a TTM roll-up and trend label.
pub fn compute_accruals_snapshot(
    symbol: &str,
    as_of: &str,
    statements: &FinancialStatements,
) -> AccrualsSnapshot {
    let mut periods: Vec<AccrualPeriod> = Vec::new();

    // Match income rows to cashflow rows by date. Keep order as-provided (newest first).
    for inc in statements.income_quarterly.iter().take(8) {
        let cf = statements
            .cashflow_quarterly
            .iter()
            .find(|c| c.date == inc.date);
        let ni = inc.net_income;
        let fcf = cf.map(|c| c.free_cash_flow).unwrap_or(0.0);
        if ni == 0.0 && fcf == 0.0 {
            continue;
        }
        let ratio = if ni != 0.0 { fcf / ni } else { 0.0 };
        let conv_pct = ratio * 100.0;
        let accruals = ni - fcf;
        let quality_label = if ni <= 0.0 {
            "NEGATIVE_NI".to_string()
        } else if conv_pct >= 90.0 {
            "HIGH".to_string()
        } else if conv_pct >= 60.0 {
            "MEDIUM".to_string()
        } else {
            "LOW".to_string()
        };
        periods.push(AccrualPeriod {
            period: inc.period.clone(),
            date: inc.date.clone(),
            net_income: ni,
            free_cash_flow: fcf,
            fcf_to_ni_ratio: ratio,
            cash_conversion_pct: conv_pct,
            accruals,
            quality_label,
        });
    }

    // TTM roll-up from the last 4 quarters.
    let ttm_ni: f64 = periods.iter().take(4).map(|p| p.net_income).sum();
    let ttm_fcf: f64 = periods.iter().take(4).map(|p| p.free_cash_flow).sum();
    let ttm_conv_pct = if ttm_ni != 0.0 {
        ttm_fcf / ttm_ni * 100.0
    } else {
        0.0
    };

    let avg_conv_pct: f64 = if !periods.is_empty() {
        periods.iter().map(|p| p.cash_conversion_pct).sum::<f64>() / periods.len() as f64
    } else {
        0.0
    };

    // Trend: compare recent-2 average vs older-2 average.
    let trend_label = if periods.len() < 4 {
        "INSUFFICIENT".to_string()
    } else {
        let recent: f64 = periods
            .iter()
            .take(2)
            .map(|p| p.cash_conversion_pct)
            .sum::<f64>()
            / 2.0;
        let older: f64 = periods
            .iter()
            .skip(2)
            .take(2)
            .map(|p| p.cash_conversion_pct)
            .sum::<f64>()
            / 2.0;
        let delta = recent - older;
        if delta.abs() < 5.0 {
            "STABLE".to_string()
        } else if delta > 0.0 {
            "IMPROVING".to_string()
        } else {
            "DETERIORATING".to_string()
        }
    };

    let note = if periods.is_empty() {
        "no cached quarterly statements — run FA".to_string()
    } else {
        String::new()
    };

    AccrualsSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        ttm_net_income: ttm_ni,
        ttm_free_cash_flow: ttm_fcf,
        ttm_cash_conversion_pct: ttm_conv_pct,
        avg_cash_conversion_pct: avg_conv_pct,
        periods,
        trend_label,
        note,
    }
}
