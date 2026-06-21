use super::context::SymbolResearchContext;
use std::fmt::Write as _;
use typhoon_engine::core::research as rx;

pub(super) fn write_symbol_ownership_price_history_sections(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    // insider flow / holders / float / HP / EPS

    // INS — insider Form-4 flow (last ~10 filings + net summary)
    if let Ok(Some(ins)) = rx::get_insider_trades(ctx.conn, &sym_upper) {
        if !ins.is_empty() {
            let mut bought = 0.0_f64;
            let mut sold = 0.0_f64;
            for t in ins.iter() {
                match t.acquisition_disposition.as_str() {
                    "A" => bought += t.value_usd,
                    "D" => sold += t.value_usd,
                    _ => {}
                }
            }
            let net = bought - sold;
            let _ = writeln!(p, "### Insider Flow (Form 4)");
            let _ = writeln!(
                p,
                "- Aggregate — buys ${:.2}M · sells ${:.2}M · net ${:+.2}M across {} filings",
                bought / 1e6,
                sold / 1e6,
                net / 1e6,
                ins.len()
            );
            for t in ins.iter().take(8) {
                let tag = if t.acquisition_disposition == "A" {
                    "BUY "
                } else if t.acquisition_disposition == "D" {
                    "SELL"
                } else {
                    "    "
                };
                let _ = writeln!(
                    p,
                    "- {} {} · {} · {:.0} sh @ ${:.2} = ${:.1}k · {}",
                    t.filing_date,
                    tag,
                    t.reporting_name,
                    t.shares,
                    t.price,
                    t.value_usd / 1e3,
                    t.transaction_type
                );
            }
            let _ = writeln!(p);
        }
    }

    // HDS — top institutional holders
    if let Ok(Some(holders)) = rx::get_institutional_holders(ctx.conn, &sym_upper) {
        if !holders.is_empty() {
            let total_shares: f64 = holders.iter().map(|h| h.shares).sum();
            let _ = writeln!(p, "### Institutional Holders (13F)");
            let _ = writeln!(
                p,
                "- {} holders, {:.0}M total shares reported",
                holders.len(),
                total_shares / 1e6
            );
            for h in holders.iter().take(6) {
                let chg = if h.change > 0.0 {
                    format!("+{:.2}M", h.change / 1e6)
                } else if h.change < 0.0 {
                    format!("{:.2}M", h.change / 1e6)
                } else {
                    "flat".to_string()
                };
                let _ = writeln!(
                    p,
                    "- {} · {:.2}M sh · QoQ Δ {} · reported {}",
                    h.holder,
                    h.shares / 1e6,
                    chg,
                    h.date_reported
                );
            }
            let _ = writeln!(p);
        }
    }

    // FLOAT — shares float snapshot
    if let Ok(Some(sf)) = rx::get_shares_float(ctx.conn, &sym_upper) {
        if sf.outstanding_shares > 0.0 {
            let _ = writeln!(p, "### Shares Float ({})", sf.date);
            let _ = writeln!(
                p,
                "- Outstanding {:.2}M · Float {:.2}M · Free-float {:.1}% · source: {}",
                sf.outstanding_shares / 1e6,
                sf.float_shares / 1e6,
                sf.free_float_pct,
                sf.source
            );
            let _ = writeln!(p);
        }
    }

    // HP — most recent daily bars (concise, last 10)
    if let Ok(Some(hp)) = rx::get_historical_price(ctx.conn, &sym_upper) {
        if !hp.is_empty() {
            let _ = writeln!(p, "### Recent Price History");
            let _ = writeln!(p, "| Date | Open | High | Low | Close | Volume | Chg % |");
            let _ = writeln!(p, "|---|---|---|---|---|---|---|");
            for r in hp.iter().take(10) {
                let _ = writeln!(
                    p,
                    "| {} | {:.2} | {:.2} | {:.2} | {:.2} | {:.1}M | {:+.2}% |",
                    r.date,
                    r.open,
                    r.high,
                    r.low,
                    r.close,
                    r.volume / 1e6,
                    r.change_pct
                );
            }
            let _ = writeln!(p);
        }
    }

    // EPS — earnings surprise history (last 8 quarters)
    if let Ok(Some(eps)) = rx::get_earnings_surprises(ctx.conn, &sym_upper) {
        if !eps.is_empty() {
            let beats = eps.iter().filter(|s| s.surprise > 0.0).count();
            let misses = eps.iter().filter(|s| s.surprise < 0.0).count();
            let avg8: f64 = eps.iter().take(8).map(|s| s.surprise_pct).sum::<f64>()
                / eps.iter().take(8).count().max(1) as f64;
            let _ = writeln!(p, "### EPS Surprise History");
            let _ = writeln!(
                p,
                "- {} quarters tracked · {} beats · {} misses · 8Q avg surprise {:+.2}%",
                eps.len(),
                beats,
                misses,
                avg8
            );
            for s in eps.iter().take(8) {
                let _ = writeln!(
                    p,
                    "- {} · actual ${:.2} · est ${:.2} · {:+.2} ({:+.2}%)",
                    s.date, s.eps_actual, s.eps_estimate, s.surprise, s.surprise_pct
                );
            }
            let _ = writeln!(p);
        }
    }
}
