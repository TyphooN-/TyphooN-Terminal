use super::context::SymbolResearchContext;
use std::fmt::Write as _;

use typhoon_engine::core::fundamentals::Fundamentals;

fn opt_pct(v: Option<f64>) -> String {
    v.map(|x| format!("{x:+.2}%")).unwrap_or_else(|| "—".into())
}

fn opt_x(v: Option<f64>) -> String {
    v.map(|x| format!("{x:.2}")).unwrap_or_else(|| "—".into())
}

/// ADR-116 `### Finviz-Style Snapshot`: the consolidated derivation table —
/// perf windows, derived valuation ratios, growth, technicals, and profile
/// extras — computed from data already cached (never per-frame, ADR-098).
pub fn write_symbol_finviz_snapshot_section(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
    fund: Option<&Fundamentals>,
) {
    let snap = typhoon_engine::core::research::build_finviz_snapshot(
        ctx.conn, sym_upper, fund, /* shortable from asset metadata */ None,
    );
    // Skip entirely when nothing derived (no price history, no statements).
    let has_any = snap.perf.week.is_some()
        || snap.perf.year.is_some()
        || snap.sales_yoy_ttm.is_some()
        || snap.current_ratio.is_some()
        || snap.price_to_fcf.is_some();
    if !has_any {
        return;
    }

    let _ = writeln!(p, "### Finviz-Style Snapshot ({sym_upper})");
    let _ = writeln!(
        p,
        "| Perf W | Perf M | Perf Q | Perf ½Y | Perf YTD | Perf Y | Perf 3Y | Perf 5Y | Perf 10Y |"
    );
    let _ = writeln!(p, "|---|---|---|---|---|---|---|---|---|");
    let _ = writeln!(
        p,
        "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
        opt_pct(snap.perf.week),
        opt_pct(snap.perf.month),
        opt_pct(snap.perf.quarter),
        opt_pct(snap.perf.half_year),
        opt_pct(snap.perf.ytd),
        opt_pct(snap.perf.year),
        opt_pct(snap.perf.three_year),
        opt_pct(snap.perf.five_year),
        opt_pct(snap.perf.ten_year),
    );
    let _ = writeln!(p);
    let _ = writeln!(
        p,
        "| P/C | P/FCF | EV/Sales | ROIC | Book/sh | Cash/sh | Current R | Quick R | Payout |"
    );
    let _ = writeln!(p, "|---|---|---|---|---|---|---|---|---|");
    let _ = writeln!(
        p,
        "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
        opt_x(snap.price_to_cash),
        opt_x(snap.price_to_fcf),
        opt_x(snap.ev_to_sales),
        opt_pct(snap.roic),
        snap.book_per_share
            .map(|v| format!("${v:.2}"))
            .unwrap_or_else(|| "—".into()),
        snap.cash_per_share
            .map(|v| format!("${v:.2}"))
            .unwrap_or_else(|| "—".into()),
        opt_x(snap.current_ratio),
        opt_x(snap.quick_ratio),
        opt_pct(snap.payout_ratio),
    );
    let _ = writeln!(p);
    let _ = writeln!(
        p,
        "| Sales Y/Y TTM | Sales Q/Q | EPS Y/Y TTM | EPS Q/Q | Sales 3Y CAGR | Sales 5Y CAGR |"
    );
    let _ = writeln!(p, "|---|---|---|---|---|---|");
    let _ = writeln!(
        p,
        "| {} | {} | {} | {} | {} | {} |",
        opt_pct(snap.sales_yoy_ttm),
        opt_pct(snap.sales_qoq),
        opt_pct(snap.eps_yoy_ttm),
        opt_pct(snap.eps_qoq),
        opt_pct(snap.sales_growth_3y),
        opt_pct(snap.sales_growth_5y),
    );
    let _ = writeln!(p);
    let _ = writeln!(
        p,
        "| vs SMA20 | vs SMA50 | vs SMA200 | RSI14 | vs 52W High | vs 52W Low | Employees | Optionable | Shortable |"
    );
    let _ = writeln!(p, "|---|---|---|---|---|---|---|---|---|");
    let _ = writeln!(
        p,
        "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
        opt_pct(snap.sma20_dist_pct),
        opt_pct(snap.sma50_dist_pct),
        opt_pct(snap.sma200_dist_pct),
        opt_x(snap.rsi14),
        opt_pct(snap.w52_high_dist_pct),
        opt_pct(snap.w52_low_dist_pct),
        snap.employees
            .map(|e| format!("{e:.0}"))
            .unwrap_or_else(|| "—".into()),
        snap.optionable
            .map(|b| if b { "Yes" } else { "n/a" })
            .unwrap_or("—"),
        snap.shortable
            .map(|b| if b { "Yes" } else { "No" })
            .unwrap_or("—"),
    );
    let _ = writeln!(p);
}
