use super::*;

impl TyphooNApp {
    pub(super) fn write_symbol_investigation_overview_sections(
        &self,
        p: &mut String,
        sym_upper: &str,
    ) {
        use std::fmt::Write as _;
        // User's open positions in this symbol — emit before fundamentals
        // so the AI treats the user's exposure as primary context when
        // answering questions like "what do you think about my position?".
        let pos_section = self.user_position_section(&sym_upper);
        if !pos_section.is_empty() {
            let _ = write!(p, "{pos_section}");
        }

        // Fundamentals row
        let fund = self
            .bg
            .all_fundamentals
            .iter()
            .find(|f| f.symbol.eq_ignore_ascii_case(&sym_upper));
        if let Some(f) = fund {
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
                // Trim long descriptions to keep the prompt bounded
                let d = if f.description.len() > 800 {
                    &f.description[..800]
                } else {
                    f.description.as_str()
                };
                let _ = writeln!(p, "{d}");
            }
            let _ = writeln!(p);
            let _ = writeln!(p, "### Valuation & Risk");
            let fmt_money = typhoon_engine::core::fundamentals::format_large_number;
            let fmt_opt = |v: Option<f64>| {
                v.map(|x| format!("{:.2}", x))
                    .unwrap_or_else(|| "—".to_string())
            };
            let fmt_money_opt =
                |v: Option<f64>| v.map(fmt_money).unwrap_or_else(|| "—".to_string());
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
        } else {
            let _ = writeln!(
                p,
                "_No fundamentals on file for this symbol. Run EVSCRAPE to populate._"
            );
            let _ = writeln!(p);
        }
    }
}
