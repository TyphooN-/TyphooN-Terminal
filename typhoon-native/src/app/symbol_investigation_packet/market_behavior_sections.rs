use super::*;

impl TyphooNApp {
    pub(super) fn write_symbol_market_behavior_sections(&self, p: &mut String, sym_upper: &str) {
        use std::fmt::Write as _;
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use typhoon_engine::core::research as rx;

                // Round 9 — SEAG seasonality (monthly + day-of-week)
                if let Ok(Some(sg)) = rx::get_seasonality(&conn, &sym_upper) {
                    if !sg.months.is_empty() || !sg.dow.is_empty() {
                        let _ = writeln!(p, "### Seasonality (as of {})", sg.as_of);
                        let _ = writeln!(
                            p,
                            "- Years covered: {} · best month {} · worst month {}",
                            sg.years_covered, sg.best_month, sg.worst_month
                        );
                        if !sg.months.is_empty() {
                            let _ = writeln!(p, "| Month | Avg | Median | Stdev | +Years | N |");
                            let _ = writeln!(p, "|---|---|---|---|---|---|");
                            for m in sg.months.iter() {
                                let _ = writeln!(
                                    p,
                                    "| {} | {:+.2}% | {:+.2}% | {:.2}% | {}/{} | {} |",
                                    m.label,
                                    m.avg_return_pct,
                                    m.median_return_pct,
                                    m.stdev_pct,
                                    m.positive_years,
                                    m.total_years,
                                    m.total_years
                                );
                            }
                        }
                        if !sg.dow.is_empty() {
                            let _ = writeln!(p, "| Day | Avg | +Days | N |");
                            let _ = writeln!(p, "|---|---|---|---|");
                            for d in sg.dow.iter() {
                                let _ = writeln!(
                                    p,
                                    "| {} | {:+.3}% | {}/{} | {} |",
                                    d.label,
                                    d.avg_return_pct,
                                    d.positive_days,
                                    d.total_days,
                                    d.total_days
                                );
                            }
                        }
                        if !sg.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", sg.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // Round 9 — COR correlation matrix vs peers
                if let Ok(Some(cm)) = rx::get_correlation(&conn, &sym_upper) {
                    if !cm.cells.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Correlation Matrix (as of {}, window {}d)",
                            cm.as_of, cm.window_days
                        );
                        let _ = writeln!(
                            p,
                            "- Mean peer corr {:.2} · highest {} · lowest {}",
                            cm.mean_correlation, cm.highest_corr_symbol, cm.lowest_corr_symbol
                        );
                        let _ = writeln!(p, "| Peer | ρ | β | N |");
                        let _ = writeln!(p, "|---|---|---|---|");
                        for c in cm.cells.iter().take(10) {
                            let _ = writeln!(
                                p,
                                "| {} | {:+.2} | {:+.2} | {} |",
                                c.peer_symbol, c.correlation, c.beta_vs_peer, c.n_observations
                            );
                        }
                        if !cm.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", cm.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // Round 9 — TRA total return (price + dividends)
                if let Ok(Some(tr)) = rx::get_total_return(&conn, &sym_upper) {
                    if !tr.windows.is_empty() {
                        let _ = writeln!(p, "### Total Return Analysis (as of {})", tr.as_of);
                        let _ = writeln!(
                            p,
                            "- Last ${:.2} · TTM div ${:.4} · TTM yield {:.2}%",
                            tr.last_close, tr.trailing_12m_dividends, tr.trailing_12m_yield_pct
                        );
                        let _ = writeln!(
                            p,
                            "| Window | Price | Div Yield | Total | Annualized | N div |"
                        );
                        let _ = writeln!(p, "|---|---|---|---|---|---|");
                        for w in tr.windows.iter() {
                            let _ = writeln!(
                                p,
                                "| {} | {:+.2}% | {:.2}% | {:+.2}% | {:+.2}% | {} |",
                                w.label,
                                w.price_return_pct,
                                w.dividend_yield_pct,
                                w.total_return_pct,
                                w.annualized_pct,
                                w.n_dividends
                            );
                        }
                        if !tr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", tr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // Round 9 — TECH technical indicators (RSI/MACD/BB/ATR/ADX/Stoch)
                if let Ok(Some(ti)) = rx::get_technicals(&conn, &sym_upper) {
                    if !ti.indicators.is_empty() {
                        let _ = writeln!(p, "### Technical Indicators (as of {})", ti.as_of);
                        let _ = writeln!(
                            p,
                            "- Last ${:.2} · trend: {}",
                            ti.last_close, ti.trend_summary
                        );
                        let _ = writeln!(p, "| Indicator | Value | Signal |");
                        let _ = writeln!(p, "|---|---|---|");
                        for ind in ti.indicators.iter() {
                            let val = if ind.value_secondary != 0.0 || ind.value_tertiary != 0.0 {
                                format!(
                                    "{:.2} / {:.2} / {:.2}",
                                    ind.value, ind.value_secondary, ind.value_tertiary
                                )
                            } else {
                                format!("{:.2}", ind.value)
                            };
                            let _ = writeln!(p, "| {} | {} | {} |", ind.name, val, ind.signal);
                        }
                        if !ti.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ti.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // Round 9 — SKEW volatility skew / smile
                if let Ok(Some(sk)) = rx::get_vol_skew(&conn, &sym_upper) {
                    if !sk.expiries.is_empty() {
                        let _ = writeln!(p, "### Volatility Skew (as of {})", sk.as_of);
                        let _ = writeln!(
                            p,
                            "- Underlying ${:.2} · {} expir{} cached",
                            sk.underlying_price,
                            sk.expiries.len(),
                            if sk.expiries.len() == 1 { "y" } else { "ies" }
                        );
                        if let Some(exp) = sk.expiries.first() {
                            let _ = writeln!(
                                p,
                                "- Nearest expiry {} ({} DTE) · ATM IV {:.1}% · 25Δ P/C skew {:+.2}%",
                                exp.expiration,
                                exp.days_to_expiry,
                                exp.atm_iv_pct,
                                exp.put_call_skew_25d_pct
                            );
                            if !exp.points.is_empty() {
                                let _ = writeln!(
                                    p,
                                    "| Strike | Moneyness | Call IV | Put IV | Combined |"
                                );
                                let _ = writeln!(p, "|---|---|---|---|---|");
                                for pt in exp.points.iter().take(9) {
                                    let fmt_iv = |v: f64| {
                                        if v > 0.0 {
                                            format!("{:.1}%", v)
                                        } else {
                                            "—".to_string()
                                        }
                                    };
                                    let _ = writeln!(
                                        p,
                                        "| ${:.2} | {:+.1}% | {} | {} | {} |",
                                        pt.strike,
                                        pt.moneyness_pct,
                                        fmt_iv(pt.call_iv_pct),
                                        fmt_iv(pt.put_iv_pct),
                                        fmt_iv(pt.combined_iv_pct)
                                    );
                                }
                            }
                            if !exp.term_note.is_empty() {
                                let _ = writeln!(p, "- Term: {}", exp.term_note);
                            }
                        }
                        if !sk.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", sk.note);
                        }
                        let _ = writeln!(p);
                    }
                }
            }
        }
    }
}
