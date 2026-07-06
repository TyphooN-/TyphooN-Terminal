//! Finviz-style snapshot derivations (ADR-116 gap closure).
//!
//! Finviz's stock page is mostly fields the terminal already caches; the ADR's
//! finding was that reaching parity is a **derivation + presentation**
//! exercise. This module derives the missing fields from stored research
//! tables + fundamentals and exposes one `FinvizSnapshot` the packet renders
//! as `### Finviz-Style Snapshot`. Everything is computed on snapshot build —
//! never per-frame (ADR-098 discipline).

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::core::fundamentals::Fundamentals;

use super::HistoricalPriceRow;

/// Perf-window returns computed from stored daily closes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerfWindows {
    pub week: Option<f64>,
    pub month: Option<f64>,
    pub quarter: Option<f64>,
    pub half_year: Option<f64>,
    pub ytd: Option<f64>,
    pub year: Option<f64>,
    pub three_year: Option<f64>,
    pub five_year: Option<f64>,
    pub ten_year: Option<f64>,
}

/// One consolidated Finviz-style snapshot (ADR-116). `None` = the input the
/// derivation needs is not cached for this symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FinvizSnapshot {
    pub symbol: String,
    pub perf: PerfWindows,
    // Valuation derivations
    pub price_to_cash: Option<f64>,
    pub price_to_fcf: Option<f64>,
    pub ev_to_sales: Option<f64>,
    pub roic: Option<f64>,
    pub book_per_share: Option<f64>,
    pub cash_per_share: Option<f64>,
    pub current_ratio: Option<f64>,
    pub quick_ratio: Option<f64>,
    pub payout_ratio: Option<f64>,
    // Growth derivations (percent)
    pub sales_yoy_ttm: Option<f64>,
    pub sales_qoq: Option<f64>,
    pub eps_yoy_ttm: Option<f64>,
    pub eps_qoq: Option<f64>,
    pub sales_growth_3y: Option<f64>,
    pub sales_growth_5y: Option<f64>,
    // Technicals from stored closes
    pub sma20_dist_pct: Option<f64>,
    pub sma50_dist_pct: Option<f64>,
    pub sma200_dist_pct: Option<f64>,
    pub rsi14: Option<f64>,
    pub w52_high_dist_pct: Option<f64>,
    pub w52_low_dist_pct: Option<f64>,
    // Profile extras
    pub employees: Option<f64>,
    pub optionable: Option<bool>,
    pub shortable: Option<bool>,
}

fn pct(from: f64, to: f64) -> Option<f64> {
    (from > 0.0 && from.is_finite() && to.is_finite()).then(|| (to / from - 1.0) * 100.0)
}

/// Return over the window ending at the newest close, `days` calendar days
/// back (closest stored close at or before the anchor date). `rows` are
/// newest-first, as `get_historical_price` returns them.
fn window_return(rows: &[HistoricalPriceRow], days: i64) -> Option<f64> {
    let newest = rows.first()?;
    let newest_date = chrono::NaiveDate::parse_from_str(&newest.date, "%Y-%m-%d").ok()?;
    let anchor = newest_date - chrono::Duration::days(days);
    let base = rows.iter().find(|r| {
        chrono::NaiveDate::parse_from_str(&r.date, "%Y-%m-%d")
            .map(|d| d <= anchor)
            .unwrap_or(false)
    })?;
    pct(base.close, newest.close)
}

fn ytd_return(rows: &[HistoricalPriceRow]) -> Option<f64> {
    let newest = rows.first()?;
    let year = newest.date.get(0..4)?;
    let base = rows.iter().find(|r| !r.date.starts_with(year))?;
    pct(base.close, newest.close)
}

/// All perf windows from newest-first daily rows. Windows deeper than the
/// stored history come back `None` rather than guessing.
pub fn perf_windows(rows: &[HistoricalPriceRow]) -> PerfWindows {
    PerfWindows {
        week: window_return(rows, 7),
        month: window_return(rows, 30),
        quarter: window_return(rows, 91),
        half_year: window_return(rows, 182),
        ytd: ytd_return(rows),
        year: window_return(rows, 365),
        three_year: window_return(rows, 365 * 3),
        five_year: window_return(rows, 365 * 5),
        ten_year: window_return(rows, 3650),
    }
}

fn sma_dist(rows: &[HistoricalPriceRow], n: usize) -> Option<f64> {
    if rows.len() < n {
        return None;
    }
    let newest = rows.first()?;
    let sma: f64 = rows.iter().take(n).map(|r| r.close).sum::<f64>() / n as f64;
    pct(sma, newest.close)
}

fn rsi14(rows: &[HistoricalPriceRow]) -> Option<f64> {
    if rows.len() < 15 {
        return None;
    }
    // Oldest→newest over the last 15 closes.
    let closes: Vec<f64> = rows.iter().take(15).map(|r| r.close).rev().collect();
    let (mut gain, mut loss) = (0.0_f64, 0.0_f64);
    for w in closes.windows(2) {
        let d = w[1] - w[0];
        if d >= 0.0 {
            gain += d;
        } else {
            loss -= d;
        }
    }
    if loss <= f64::EPSILON {
        return Some(100.0);
    }
    let rs = (gain / 14.0) / (loss / 14.0);
    Some(100.0 - 100.0 / (1.0 + rs))
}

fn w52_dists(rows: &[HistoricalPriceRow]) -> (Option<f64>, Option<f64>) {
    let Some(newest) = rows.first() else {
        return (None, None);
    };
    let year: Vec<&HistoricalPriceRow> = rows.iter().take(252).collect();
    if year.is_empty() {
        return (None, None);
    }
    let hi = year.iter().map(|r| r.high).fold(f64::MIN, f64::max);
    let lo = year.iter().map(|r| r.low).fold(f64::MAX, f64::min);
    (pct(hi, newest.close), pct(lo, newest.close))
}

/// Trailing-twelve-month sum over quarterly values (newest-first input).
fn ttm(values: &[f64]) -> Option<f64> {
    (values.len() >= 4).then(|| values.iter().take(4).sum())
}

/// Compound annual growth between the TTM ending now and the TTM ending
/// `years` earlier (quarterly rows newest-first).
fn cagr_over_quarters(values: &[f64], years: usize) -> Option<f64> {
    let recent = ttm(values)?;
    let past_slice = values.get(years * 4..)?;
    let past = ttm(past_slice)?;
    (past > 0.0 && recent > 0.0).then(|| ((recent / past).powf(1.0 / years as f64) - 1.0) * 100.0)
}

/// Build the consolidated snapshot from stored research tables. `fund` is the
/// fundamentals row when cached; `shortable` comes from broker asset metadata
/// when the caller has it (Alpaca assets), `None` otherwise.
pub fn build_finviz_snapshot(
    conn: &Connection,
    symbol: &str,
    fund: Option<&Fundamentals>,
    shortable: Option<bool>,
) -> FinvizSnapshot {
    let sym = symbol.trim().to_uppercase();
    let mut out = FinvizSnapshot {
        symbol: sym.clone(),
        shortable,
        ..Default::default()
    };

    // Price-derived surfaces.
    if let Ok(Some(rows)) = super::get_historical_price(conn, &sym) {
        out.perf = perf_windows(&rows);
        out.sma20_dist_pct = sma_dist(&rows, 20);
        out.sma50_dist_pct = sma_dist(&rows, 50);
        out.sma200_dist_pct = sma_dist(&rows, 200);
        out.rsi14 = rsi14(&rows);
        let (hi, lo) = w52_dists(&rows);
        out.w52_high_dist_pct = hi;
        out.w52_low_dist_pct = lo;
    }

    // Statement-derived surfaces.
    if let Ok(Some(fin)) = super::get_financials(conn, &sym) {
        let q_rev: Vec<f64> = fin.income_quarterly.iter().map(|i| i.revenue).collect();
        let q_ni: Vec<f64> = fin.income_quarterly.iter().map(|i| i.net_income).collect();
        if q_rev.len() >= 8 {
            let recent = ttm(&q_rev);
            let prior = ttm(&q_rev[4..].to_vec());
            if let (Some(r), Some(p)) = (recent, prior) {
                out.sales_yoy_ttm = pct(p, r);
            }
        }
        if q_rev.len() >= 2 {
            out.sales_qoq = pct(q_rev[1], q_rev[0]);
        }
        if q_ni.len() >= 8 {
            let recent = ttm(&q_ni);
            let prior = ttm(&q_ni[4..].to_vec());
            if let (Some(r), Some(p)) = (recent, prior) {
                if p > 0.0 {
                    out.eps_yoy_ttm = pct(p, r);
                }
            }
        }
        if q_ni.len() >= 2 && q_ni[1] > 0.0 {
            out.eps_qoq = pct(q_ni[1], q_ni[0]);
        }
        out.sales_growth_3y = cagr_over_quarters(&q_rev, 3);
        out.sales_growth_5y = cagr_over_quarters(&q_rev, 5);

        if let Some(bs) = fin.balance_quarterly.first() {
            if bs.total_current_liabilities > 0.0 {
                out.current_ratio = Some(bs.total_current_assets / bs.total_current_liabilities);
                out.quick_ratio =
                    Some((bs.total_current_assets - bs.inventory) / bs.total_current_liabilities);
            }
            if let Some(f) = fund {
                if let Some(shares) = f.shares_outstanding.filter(|s| *s > 0.0) {
                    out.book_per_share = Some(bs.total_equity / shares);
                    out.cash_per_share =
                        Some((bs.cash_and_equiv + bs.short_term_investments) / shares);
                }
            }
            // ROIC = TTM NOPAT / invested capital (equity + total debt).
            let invested = bs.total_equity + bs.total_debt;
            if invested > 0.0 {
                let ttm_op: f64 = fin
                    .income_quarterly
                    .iter()
                    .take(4)
                    .map(|i| i.operating_income)
                    .sum();
                let (ttm_tax, ttm_pretax): (f64, f64) = fin
                    .income_quarterly
                    .iter()
                    .take(4)
                    .map(|i| (i.income_tax_expense, i.income_before_tax))
                    .fold((0.0, 0.0), |acc, (t, p)| (acc.0 + t, acc.1 + p));
                let tax_rate = if ttm_pretax.abs() > f64::EPSILON {
                    (ttm_tax / ttm_pretax).clamp(0.0, 0.6)
                } else {
                    0.21
                };
                out.roic = Some(ttm_op * (1.0 - tax_rate) / invested * 100.0);
            }
        }
        let q_fcf: Vec<f64> = fin
            .cashflow_quarterly
            .iter()
            .map(|c| c.free_cash_flow)
            .collect();
        let q_div: Vec<f64> = fin
            .cashflow_quarterly
            .iter()
            .map(|c| c.dividends_paid.abs())
            .collect();
        if let Some(f) = fund {
            if let (Some(mcap), Some(fcf)) = (f.market_cap, ttm(&q_fcf)) {
                if fcf > 0.0 {
                    out.price_to_fcf = Some(mcap / fcf);
                }
            }
            if let (Some(ev), Some(rev)) = (f.enterprise_value, ttm(&q_rev)) {
                if rev > 0.0 {
                    out.ev_to_sales = Some(ev / rev);
                }
            }
            if let (Some(div), Some(ni)) = (ttm(&q_div), ttm(&q_ni)) {
                if ni > 0.0 && div > 0.0 {
                    out.payout_ratio = Some(div / ni * 100.0);
                }
            }
        }
    }

    if let Some(f) = fund {
        if let (Some(mcap), Some(cash)) = (f.market_cap, f.cash_and_equivalents) {
            if cash > 0.0 {
                out.price_to_cash = Some(mcap / cash);
            }
        }
    }

    // Profile extras: employees from the stored company profile; optionable
    // is inferred from a cached options chain for the symbol.
    if let Ok(Some(profile)) = super::get_profile(conn, &sym) {
        if profile.employees > 0.0 {
            out.employees = Some(profile.employees);
        }
    }
    out.optionable = Some(super::has_cached_options_chain(conn, &sym));

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(date: &str, close: f64) -> HistoricalPriceRow {
        HistoricalPriceRow {
            date: date.into(),
            open: close,
            high: close * 1.02,
            low: close * 0.98,
            close,
            adj_close: close,
            volume: 1e6,
            change: 0.0,
            change_pct: 0.0,
        }
    }

    #[test]
    fn perf_windows_pick_calendar_anchors() {
        // Newest-first: 2026-06-30 @ 120, one week earlier @ 100, YTD base @ 80.
        let rows = vec![
            row("2026-06-30", 120.0),
            row("2026-06-22", 100.0),
            row("2026-05-29", 96.0),
            row("2025-12-31", 80.0),
            row("2025-06-27", 60.0),
        ];
        let perf = perf_windows(&rows);
        assert!((perf.week.unwrap() - 20.0).abs() < 1e-9);
        assert!((perf.month.unwrap() - 25.0).abs() < 1e-9);
        assert!((perf.ytd.unwrap() - 50.0).abs() < 1e-9);
        assert!((perf.year.unwrap() - 100.0).abs() < 1e-9);
        assert!(perf.ten_year.is_none()); // not enough depth — no guessing
    }

    #[test]
    fn ttm_and_cagr_derive_growth() {
        // Quarterly revenue newest-first: TTM now = 400, TTM 1y ago = 200.
        let rev = vec![100.0, 100.0, 100.0, 100.0, 50.0, 50.0, 50.0, 50.0];
        assert_eq!(ttm(&rev), Some(400.0));
        // 1-year CAGR over 4 quarters back = +100%.
        assert!((cagr_over_quarters(&rev, 1).unwrap() - 100.0).abs() < 1e-9);
        assert!(cagr_over_quarters(&rev, 3).is_none());
    }

    #[test]
    fn rsi_needs_depth_and_bounds() {
        let rows: Vec<HistoricalPriceRow> = (0..20)
            .map(|i| row(&format!("2026-06-{:02}", 20 - i % 19), 100.0 + i as f64))
            .collect();
        let v = rsi14(&rows).unwrap();
        assert!((0.0..=100.0).contains(&v));
        assert!(rsi14(&rows[..5]).is_none());
    }
}
