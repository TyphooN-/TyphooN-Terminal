use super::*;

impl TyphooNApp {
    pub(super) fn write_symbol_sector_peer_comparison(
        &self,
        p: &mut String,
        sym_upper: &str,
        fund: Option<&typhoon_engine::core::fundamentals::Fundamentals>,
    ) {
        use std::fmt::Write as _;
        // Sector peer comparison (median of sector peers for the same fields)
        if let Some(f) = fund {
            if !f.sector.is_empty() {
                let peers: Vec<_> = self
                    .bg
                    .all_fundamentals
                    .iter()
                    .filter(|o| {
                        o.sector.eq_ignore_ascii_case(&f.sector)
                            && !o.symbol.eq_ignore_ascii_case(&sym_upper)
                    })
                    .collect();
                if peers.len() >= 3 {
                    let median = |mut v: Vec<f64>| -> Option<f64> {
                        if v.is_empty() {
                            return None;
                        }
                        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                        Some(v[v.len() / 2])
                    };
                    let collect = |getter: fn(
                        &typhoon_engine::core::fundamentals::Fundamentals,
                    ) -> Option<f64>|
                     -> Vec<f64> {
                        peers.iter().filter_map(|p| getter(p)).collect()
                    };
                    let fmt_o = |v: Option<f64>| {
                        v.map(|x| format!("{:.2}", x)).unwrap_or_else(|| "—".into())
                    };
                    let _ = writeln!(
                        p,
                        "### Sector Peer Comparison ({} — {} peers)",
                        f.sector,
                        peers.len()
                    );
                    let _ = writeln!(p, "| Metric | This Symbol | Sector Median |");
                    let _ = writeln!(p, "|---|---|---|");
                    let _ = writeln!(
                        p,
                        "| P/E | {} | {} |",
                        fmt_o(f.pe_ratio),
                        fmt_o(median(collect(|x| x.pe_ratio)))
                    );
                    let _ = writeln!(
                        p,
                        "| Forward P/E | {} | {} |",
                        fmt_o(f.forward_pe),
                        fmt_o(median(collect(|x| x.forward_pe)))
                    );
                    let _ = writeln!(
                        p,
                        "| P/B | {} | {} |",
                        fmt_o(f.price_to_book),
                        fmt_o(median(collect(|x| x.price_to_book)))
                    );
                    let _ = writeln!(
                        p,
                        "| P/S | {} | {} |",
                        fmt_o(f.price_to_sales),
                        fmt_o(median(collect(|x| x.price_to_sales)))
                    );
                    let _ = writeln!(
                        p,
                        "| EV/EBITDA | {} | {} |",
                        fmt_o(f.ev_to_ebitda),
                        fmt_o(median(collect(|x| x.ev_to_ebitda)))
                    );
                    let _ = writeln!(
                        p,
                        "| Profit Margin | {} | {} |",
                        fmt_o(f.profit_margin),
                        fmt_o(median(collect(|x| x.profit_margin)))
                    );
                    let _ = writeln!(
                        p,
                        "| ROE | {} | {} |",
                        fmt_o(f.roe),
                        fmt_o(median(collect(|x| x.roe)))
                    );
                    let _ = writeln!(
                        p,
                        "| Beta | {} | {} |",
                        fmt_o(f.beta),
                        fmt_o(median(collect(|x| x.beta)))
                    );
                    let _ = writeln!(
                        p,
                        "| Short % Float | {} | {} |",
                        fmt_o(f.short_percent_of_float),
                        fmt_o(median(collect(|x| x.short_percent_of_float)))
                    );
                    let _ = writeln!(
                        p,
                        "| Div Yield | {} | {} |",
                        fmt_o(f.dividend_yield),
                        fmt_o(median(collect(|x| x.dividend_yield)))
                    );
                    let _ = writeln!(p);
                }
            }
        }
    }
}
