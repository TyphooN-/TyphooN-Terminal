//! Pure section/table formatters for the symbol investigation packet.
//!
//! Free functions over engine DTOs — no `TyphooNApp` access — so they stay
//! crate-movable for the future `typhoon-research-ui` crate (ADR-125, Phase 1
//! step 2). The pattern: a `write_*_sections` method gathers the data from app
//! state and hands the resolved DTO to one of these formatters; the formatter
//! itself never touches app state, so its output depends only on its inputs.

use std::fmt::Write as _;
use typhoon_engine::core::fundamentals::{Fundamentals, format_large_number};

/// Write the symbol-investigation **overview** block for a resolved
/// [`Fundamentals`] record: the company header line, an optional (length-bounded)
/// description, and the "Valuation & Risk" metric table. Pure — identical
/// fundamentals produce identical markdown.
pub(super) fn write_fundamentals_overview(p: &mut String, f: &Fundamentals) {
    let _ = writeln!(
        p,
        "**{}** — {} / {}",
        if f.company_name.is_empty() {
            "(unnamed)"
        } else {
            f.company_name.as_str()
        },
        if f.sector.is_empty() {
            "Unknown"
        } else {
            f.sector.as_str()
        },
        if f.industry.is_empty() {
            "Unknown"
        } else {
            f.industry.as_str()
        }
    );
    if !f.description.is_empty() {
        // Trim long descriptions to keep the prompt bounded.
        let d = if f.description.len() > 800 {
            &f.description[..800]
        } else {
            f.description.as_str()
        };
        let _ = writeln!(p, "{d}");
    }
    let _ = writeln!(p);
    let _ = writeln!(p, "### Valuation & Risk");
    let fmt_money = format_large_number;
    let fmt_opt = |v: Option<f64>| {
        v.map(|x| format!("{:.2}", x))
            .unwrap_or_else(|| "—".to_string())
    };
    let fmt_money_opt = |v: Option<f64>| v.map(fmt_money).unwrap_or_else(|| "—".to_string());
    let _ = writeln!(p, "| Metric | Value |");
    let _ = writeln!(p, "|---|---|");
    let _ = writeln!(p, "| Market Cap | {} |", fmt_money_opt(f.market_cap));
    let _ = writeln!(
        p,
        "| Enterprise Value | {} |",
        fmt_money_opt(f.enterprise_value)
    );
    let _ = writeln!(p, "| MCap/EV % | {} |", fmt_opt(f.mcap_ev_ratio));
    let _ = writeln!(p, "| Total Debt | {} |", fmt_money_opt(f.total_debt));
    let _ = writeln!(
        p,
        "| Cash & Equivalents | {} |",
        fmt_money_opt(f.cash_and_equivalents)
    );
    let _ = writeln!(p, "| Stock Price | {} |", fmt_opt(f.stock_price));
    let _ = writeln!(p, "| P/E (trailing) | {} |", fmt_opt(f.pe_ratio));
    let _ = writeln!(p, "| Forward P/E | {} |", fmt_opt(f.forward_pe));
    let _ = writeln!(p, "| PEG | {} |", fmt_opt(f.peg_ratio));
    let _ = writeln!(p, "| P/B | {} |", fmt_opt(f.price_to_book));
    let _ = writeln!(p, "| P/S | {} |", fmt_opt(f.price_to_sales));
    let _ = writeln!(p, "| EV/EBITDA | {} |", fmt_opt(f.ev_to_ebitda));
    let _ = writeln!(p, "| Profit Margin | {} |", fmt_opt(f.profit_margin));
    let _ = writeln!(p, "| Operating Margin | {} |", fmt_opt(f.operating_margin));
    let _ = writeln!(p, "| ROE | {} |", fmt_opt(f.roe));
    let _ = writeln!(p, "| ROA | {} |", fmt_opt(f.roa));
    let _ = writeln!(p, "| Beta | {} |", fmt_opt(f.beta));
    let _ = writeln!(p, "| Short Ratio | {} |", fmt_opt(f.short_ratio));
    let _ = writeln!(
        p,
        "| Short % of Float | {} |",
        fmt_opt(f.short_percent_of_float)
    );
    let _ = writeln!(p, "| Dividend Yield | {} |", fmt_opt(f.dividend_yield));
    let _ = writeln!(
        p,
        "| Next Earnings | {} |",
        f.next_earnings_date.clone().unwrap_or_else(|| "—".into())
    );
    let _ = writeln!(p);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overview_emits_header_and_valuation_table() {
        let f = Fundamentals {
            company_name: "Acme Corp".to_string(),
            sector: "Technology".to_string(),
            industry: "Software".to_string(),
            market_cap: Some(1_500_000_000.0),
            pe_ratio: Some(12.5),
            ..Default::default()
        };
        let mut out = String::new();
        write_fundamentals_overview(&mut out, &f);
        assert!(out.contains("**Acme Corp** — Technology / Software"));
        assert!(out.contains("### Valuation & Risk"));
        assert!(out.contains("| P/E (trailing) | 12.50 |"));
        // Absent optionals render as the em-dash placeholder.
        assert!(out.contains("| ROE | — |"));
    }

    #[test]
    fn overview_uses_placeholders_for_unnamed_fields() {
        let f = Fundamentals::default();
        let mut out = String::new();
        write_fundamentals_overview(&mut out, &f);
        assert!(out.contains("**(unnamed)** — Unknown / Unknown"));
    }
}
