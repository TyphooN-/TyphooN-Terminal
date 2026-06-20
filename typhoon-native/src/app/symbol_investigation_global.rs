use super::*;

impl TyphooNApp {
    pub(super) fn write_symbol_investigation_global_context(&self, p: &mut String) {
        use std::fmt::Write as _;
        // ── global market context (not per-symbol) ─────────
        // WEI / MOV / INDU give the model a snapshot of risk-on/off regime,
        // leadership/laggards, and sector rotation at packet-generation time.
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use typhoon_engine::core::research as rx;

                // WEI — global equity indices
                if let Ok(Some(rows)) = rx::get_world_indices(&conn) {
                    if !rows.is_empty() {
                        let advancing = rows.iter().filter(|r| r.change_pct > 0.0).count();
                        let declining = rows.iter().filter(|r| r.change_pct < 0.0).count();
                        let _ = writeln!(p, "## Global Market Context");
                        let _ = writeln!(p, "### World Equity Indices");
                        let _ = writeln!(
                            p,
                            "- {} indices tracked · {} advancing · {} declining",
                            rows.len(),
                            advancing,
                            declining
                        );
                        let _ = writeln!(p, "| Region | Ticker | Name | Last | Chg % |");
                        let _ = writeln!(p, "|---|---|---|---|---|");
                        for r in rows.iter().take(12) {
                            let _ = writeln!(
                                p,
                                "| {} | {} | {} | {:.2} | {:+.2}% |",
                                r.region, r.ticker, r.display, r.price, r.change_pct
                            );
                        }
                        let _ = writeln!(p);
                    }
                }

                // MOV — market movers (top gainers / losers / actives, concise)
                if let Ok(Some(mov)) = rx::get_market_movers(&conn) {
                    if !mov.gainers.is_empty() || !mov.losers.is_empty() || !mov.actives.is_empty()
                    {
                        let _ = writeln!(p, "### Market Movers (US)");
                        let render =
                            |label: &str, rows: &[typhoon_engine::core::research::MarketMover]| {
                                let mut s = String::new();
                                if !rows.is_empty() {
                                    let _ = writeln!(
                                        s,
                                        "- **{}** — {}",
                                        label,
                                        rows.iter()
                                            .take(6)
                                            .map(|m| format!("{} {:+.2}%", m.symbol, m.change_pct))
                                            .collect::<Vec<_>>()
                                            .join(", ")
                                    );
                                }
                                s
                            };
                        p.push_str(&render("Top Gainers", &mov.gainers));
                        p.push_str(&render("Top Losers", &mov.losers));
                        p.push_str(&render("Most Active", &mov.actives));
                        let _ = writeln!(p);
                    }
                }

                // INDU — sector performance (latest daily snapshot)
                if let Ok(Some(sec)) = rx::get_sector_performance(&conn) {
                    if !sec.is_empty() {
                        let mut sorted = sec.clone();
                        sorted.sort_by(|a, b| {
                            b.change_pct
                                .partial_cmp(&a.change_pct)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                        let up = sorted.iter().filter(|s| s.change_pct > 0.0).count();
                        let down = sorted.iter().filter(|s| s.change_pct < 0.0).count();
                        let _ = writeln!(p, "### Sector Performance");
                        let _ =
                            writeln!(p, "- {} sectors · {} up · {} down", sorted.len(), up, down);
                        for s in sorted.iter() {
                            let _ = writeln!(p, "- {} {:+.2}%", s.sector, s.change_pct);
                        }
                        let _ = writeln!(p);
                    }
                }

                // WCR world currency rates (FX regime)
                if let Ok(Some(rates)) = rx::get_currency_rates(&conn) {
                    if !rates.is_empty() {
                        let up = rates.iter().filter(|r| r.change_pct > 0.0).count();
                        let down = rates.iter().filter(|r| r.change_pct < 0.0).count();
                        let _ = writeln!(p, "### World Currency Rates");
                        let _ = writeln!(
                            p,
                            "- {} pairs · {} strengthening vs quote · {} weakening",
                            rates.len(),
                            up,
                            down
                        );
                        let mut by_region: std::collections::BTreeMap<
                            &str,
                            Vec<&typhoon_engine::core::research::CurrencyRate>,
                        > = std::collections::BTreeMap::new();
                        for r in rates.iter() {
                            by_region.entry(r.region.as_str()).or_default().push(r);
                        }
                        for (region, group) in by_region.iter() {
                            let s: Vec<String> = group
                                .iter()
                                .take(8)
                                .map(|r| {
                                    format!("{} {:.4} ({:+.2}%)", r.display, r.price, r.change_pct)
                                })
                                .collect();
                            let _ = writeln!(p, "- **{}** — {}", region, s.join(", "));
                        }
                        let _ = writeln!(p);
                    }
                }
            }
        }
    }
}
