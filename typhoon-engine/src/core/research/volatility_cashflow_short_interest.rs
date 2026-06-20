use super::*;

// ── RVOL compute (realized volatility cone) ──────────────

/// Compute a `RealizedVolSnapshot` from oldest-first daily bars. Produces
/// rolling 20d / 60d / 120d / 252d realized volatility (annualized stdev of
/// daily log-returns × √252) plus a cone percentile for each window
/// (where does today's RV rank against the full history of that window?),
/// and — when `current_atm_iv_pct > 0` — an IV / RV gap and ratio.
pub fn compute_realized_vol_snapshot(
    symbol: &str,
    as_of: &str,
    bars_oldest_first: &[HistoricalPriceRow],
    current_atm_iv_pct: f64,
) -> RealizedVolSnapshot {
    if bars_oldest_first.len() < 25 {
        return RealizedVolSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            regime_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥25 daily bars; run HP first".into(),
            ..Default::default()
        };
    }

    let last_close = bars_oldest_first.last().map(|b| b.close).unwrap_or(0.0);

    let mut log_returns: Vec<f64> = Vec::with_capacity(bars_oldest_first.len() - 1);
    for w in bars_oldest_first.windows(2) {
        if w[0].close > 0.0 && w[1].close > 0.0 {
            log_returns.push((w[1].close / w[0].close).ln());
        }
    }

    let stdev = |xs: &[f64]| -> f64 {
        if xs.len() < 2 {
            return 0.0;
        }
        let mean = xs.iter().sum::<f64>() / xs.len() as f64;
        let var: f64 = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (xs.len() as f64 - 1.0);
        var.sqrt()
    };
    let ann_vol_pct = |xs: &[f64]| -> f64 { stdev(xs) * (252.0_f64).sqrt() * 100.0 };

    let rolling_vols = |window: usize| -> (f64, Vec<f64>) {
        if log_returns.len() < window {
            return (0.0, Vec::new());
        }
        let mut series: Vec<f64> = Vec::new();
        for i in window..=log_returns.len() {
            let slice = &log_returns[i - window..i];
            series.push(ann_vol_pct(slice));
        }
        let latest = *series.last().unwrap_or(&0.0);
        (latest, series)
    };

    let specs = [
        ("20d", 20usize),
        ("60d", 60usize),
        ("120d", 120usize),
        ("252d", 252usize),
    ];
    let mut windows: Vec<RealizedVolWindow> = Vec::new();
    let mut rv_20d = 0.0;
    for (label, n) in specs.iter() {
        let (latest, series) = rolling_vols(*n);
        if series.is_empty() {
            continue;
        }
        if *label == "20d" {
            rv_20d = latest;
        }
        // Percentile rank of `latest` within its own rolling history.
        let count_below = series.iter().filter(|v| **v < latest).count();
        let pct = (count_below as f64 / series.len() as f64) * 100.0;
        windows.push(RealizedVolWindow {
            label: (*label).to_string(),
            trading_days: *n,
            realized_vol_pct: latest,
            percentile: pct,
            n_observations: series.len(),
        });
    }

    let (iv_rv_gap, iv_rv_ratio, regime_label) = if current_atm_iv_pct > 0.0 && rv_20d > 0.0 {
        let gap = current_atm_iv_pct - rv_20d;
        let ratio = current_atm_iv_pct / rv_20d;
        let label = if ratio < 0.95 {
            "CHEAP_IV".to_string()
        } else if ratio > 1.15 {
            "RICH_IV".to_string()
        } else {
            "FAIR_IV".to_string()
        };
        (gap, ratio, label)
    } else if rv_20d > 0.0 {
        (0.0, 0.0, "NO_IV_REFERENCE".to_string())
    } else {
        (0.0, 0.0, "INSUFFICIENT_DATA".to_string())
    };

    let note = if windows.is_empty() {
        "need more bars for rolling windows".to_string()
    } else {
        String::new()
    };

    RealizedVolSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        last_close,
        current_atm_iv_pct,
        iv_rv_gap_pct: iv_rv_gap,
        iv_rv_ratio,
        windows,
        regime_label,
        note,
    }
}

// ── FCFY compute (FCF yield + dividend coverage) ─────────

/// Compute an `FcfYieldSnapshot` from cached financial statements + market cap.
/// Builds per-annual FCF yield / dividend coverage rows, rolls TTM from the last
/// 4 quarterly cash flow statements, computes a 5-year FCF CAGR when enough
/// annual rows exist, and emits a dividend-sustainability label.
pub fn compute_fcf_yield_snapshot(
    symbol: &str,
    as_of: &str,
    statements: &FinancialStatements,
    market_cap: f64,
    stock_price: f64,
) -> FcfYieldSnapshot {
    let mut periods: Vec<FcfYieldPeriod> = Vec::new();

    for cf in statements.cashflow_annual.iter().take(5) {
        let ni = statements
            .income_annual
            .iter()
            .find(|i| i.date == cf.date)
            .map(|i| i.net_income)
            .unwrap_or(0.0);
        let div = cf.dividends_paid.abs();
        let payout_fcf = if cf.free_cash_flow > 0.0 {
            div / cf.free_cash_flow * 100.0
        } else {
            0.0
        };
        let payout_ni = if ni > 0.0 { div / ni * 100.0 } else { 0.0 };
        let yield_pct = if market_cap > 0.0 {
            cf.free_cash_flow / market_cap * 100.0
        } else {
            0.0
        };
        periods.push(FcfYieldPeriod {
            period: cf.period.clone(),
            date: cf.date.clone(),
            free_cash_flow: cf.free_cash_flow,
            dividends_paid: div,
            payout_from_fcf_pct: payout_fcf,
            payout_from_ni_pct: payout_ni,
            fcf_yield_pct: yield_pct,
        });
    }

    // TTM roll-up from the last 4 quarterly cash flow statements.
    let q_cf = &statements.cashflow_quarterly;
    let ttm_fcf: f64 = q_cf.iter().take(4).map(|c| c.free_cash_flow).sum();
    let ttm_div: f64 = q_cf.iter().take(4).map(|c| c.dividends_paid.abs()).sum();
    let ttm_ni: f64 = statements
        .income_quarterly
        .iter()
        .take(4)
        .map(|i| i.net_income)
        .sum();
    let ttm_fcf_yield = if market_cap > 0.0 {
        ttm_fcf / market_cap * 100.0
    } else {
        0.0
    };
    let ttm_div_yield = if market_cap > 0.0 {
        ttm_div / market_cap * 100.0
    } else {
        0.0
    };
    let ttm_payout_fcf = if ttm_fcf > 0.0 {
        ttm_div / ttm_fcf * 100.0
    } else {
        0.0
    };
    let ttm_payout_ni = if ttm_ni > 0.0 {
        ttm_div / ttm_ni * 100.0
    } else {
        0.0
    };

    // 5-year FCF CAGR (oldest → newest) when we have ≥5 annual rows.
    let fcf_cagr = if statements.cashflow_annual.len() >= 5 {
        let sorted_rev: Vec<&CashFlowStatement> = {
            let mut v: Vec<&CashFlowStatement> =
                statements.cashflow_annual.iter().take(5).collect();
            v.sort_by(|a, b| a.date.cmp(&b.date));
            v
        };
        let start = sorted_rev.first().map(|c| c.free_cash_flow).unwrap_or(0.0);
        let end = sorted_rev.last().map(|c| c.free_cash_flow).unwrap_or(0.0);
        if start > 0.0 && end > 0.0 {
            ((end / start).powf(1.0 / 4.0) - 1.0) * 100.0
        } else {
            0.0
        }
    } else {
        0.0
    };

    let sustainability_label = if ttm_div <= 0.0 {
        "NO_DIVIDEND".to_string()
    } else if ttm_fcf <= 0.0 || ttm_payout_fcf > 100.0 {
        "UNSUSTAINABLE".to_string()
    } else if ttm_payout_fcf > 75.0 {
        "STRETCHED".to_string()
    } else {
        "SAFE".to_string()
    };

    let note = if periods.is_empty() && ttm_fcf == 0.0 {
        "no cached cash-flow statements — run FA".to_string()
    } else if market_cap <= 0.0 {
        format!(
            "market cap missing — yield pct not computed (last ${:.2})",
            stock_price
        )
    } else {
        String::new()
    };

    FcfYieldSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        market_cap,
        ttm_free_cash_flow: ttm_fcf,
        ttm_dividends_paid: ttm_div,
        ttm_fcf_yield_pct: ttm_fcf_yield,
        ttm_dividend_yield_pct: ttm_div_yield,
        ttm_payout_from_fcf_pct: ttm_payout_fcf,
        ttm_payout_from_ni_pct: ttm_payout_ni,
        fcf_cagr_5y_pct: fcf_cagr,
        periods,
        sustainability_label,
        note,
    }
}

// ── SHRT compute (short interest + days-to-cover) ────────

/// Compute a `ShortInterestSnapshot` from the Fundamentals short fields plus
/// daily HP bars. Days-to-cover comes from `short_shares / avg_daily_volume_20d`.
pub fn compute_short_interest_snapshot(
    symbol: &str,
    as_of: &str,
    shares_outstanding: f64,
    shares_float: f64,
    short_percent_of_float: f64,
    short_ratio_reported: f64,
    bars_oldest_first: &[HistoricalPriceRow],
) -> ShortInterestSnapshot {
    let short_shares = if shares_float > 0.0 && short_percent_of_float > 0.0 {
        shares_float * (short_percent_of_float / 100.0)
    } else {
        0.0
    };

    // 20-day average daily volume from the tail of the bar series.
    let avg_dv_20d = if bars_oldest_first.len() >= 20 {
        let tail = &bars_oldest_first[bars_oldest_first.len() - 20..];
        tail.iter().map(|b| b.volume).sum::<f64>() / 20.0
    } else if !bars_oldest_first.is_empty() {
        bars_oldest_first.iter().map(|b| b.volume).sum::<f64>() / bars_oldest_first.len() as f64
    } else {
        0.0
    };

    let days_to_cover = if avg_dv_20d > 0.0 && short_shares > 0.0 {
        short_shares / avg_dv_20d
    } else {
        0.0
    };

    let squeeze_risk_label = if short_shares <= 0.0 || avg_dv_20d <= 0.0 {
        "INSUFFICIENT_DATA".to_string()
    } else if short_percent_of_float >= 30.0 || days_to_cover >= 10.0 {
        "EXTREME".to_string()
    } else if short_percent_of_float >= 20.0 || days_to_cover >= 7.0 {
        "HIGH".to_string()
    } else if short_percent_of_float >= 10.0 || days_to_cover >= 4.0 {
        "ELEVATED".to_string()
    } else {
        "LOW".to_string()
    };

    let note = if short_shares <= 0.0 {
        "no short data in Fundamentals — run EVSCRAPE".to_string()
    } else if bars_oldest_first.is_empty() {
        "no bar volumes — run HP first".to_string()
    } else {
        String::new()
    };

    ShortInterestSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        shares_outstanding,
        shares_float,
        short_shares,
        short_percent_of_float,
        avg_daily_volume_20d: avg_dv_20d,
        days_to_cover,
        short_ratio_reported,
        utilization_proxy_pct: short_percent_of_float,
        squeeze_risk_label,
        note,
    }
}
