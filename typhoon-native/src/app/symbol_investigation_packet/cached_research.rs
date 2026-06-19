use super::*;

impl TyphooNApp {
    pub(super) fn write_symbol_cached_research_surfaces(&self, p: &mut String, sym_upper: &str) {
        use std::fmt::Write as _;
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use typhoon_engine::core::research as rx;
                let fmt_money = typhoon_engine::core::fundamentals::format_large_number;

                // DVD — dividend history (last 6 rows)
                if let Ok(Some(divs)) = rx::get_dividends(&conn, &sym_upper) {
                    if !divs.is_empty() {
                        let _ = writeln!(p, "### Dividend History ({})", divs.len());
                        let _ = writeln!(p, "| Ex-Date | Pay Date | Amount | Label |");
                        let _ = writeln!(p, "|---|---|---|---|");
                        for d in divs.iter().take(6) {
                            let _ = writeln!(
                                p,
                                "| {} | {} | {:.4} | {} |",
                                d.ex_date, d.pay_date, d.amount, d.label
                            );
                        }
                        let _ = writeln!(p);
                    }
                }

                // EEB — forward earnings estimates (next 4 periods)
                if let Ok(Some(est)) = rx::get_earnings_estimates(&conn, &sym_upper) {
                    if !est.is_empty() {
                        let _ = writeln!(p, "### Forward Earnings Estimates");
                        let _ =
                            writeln!(p, "| Period | EPS Avg | EPS Lo/Hi | Rev Avg | Analysts |");
                        let _ = writeln!(p, "|---|---|---|---|---|");
                        for e in est.iter().take(4) {
                            let _ = writeln!(
                                p,
                                "| {} | {:.2} | {:.2}/{:.2} | {} | eps {} / rev {} |",
                                e.date,
                                e.eps_avg,
                                e.eps_low,
                                e.eps_high,
                                fmt_money(e.revenue_avg),
                                e.num_analysts_eps,
                                e.num_analysts_rev
                            );
                        }
                        let _ = writeln!(p);
                    }
                }

                // UPDG — analyst rating changes (most recent 6)
                if let Ok(Some(rc)) = rx::get_rating_changes(&conn, &sym_upper) {
                    if !rc.is_empty() {
                        let _ = writeln!(p, "### Analyst Rating Changes ({})", rc.len());
                        let _ = writeln!(p, "| Date | Firm | Action | From → To | PT |");
                        let _ = writeln!(p, "|---|---|---|---|---|");
                        for r in rc.iter().take(6) {
                            let pt = if r.price_target > 0.0 {
                                format!("{:.2}", r.price_target)
                            } else {
                                "—".into()
                            };
                            let _ = writeln!(
                                p,
                                "| {} | {} | {} | {} → {} | {} |",
                                r.date, r.firm, r.action, r.from_grade, r.to_grade, pt
                            );
                        }
                        let _ = writeln!(p);
                    }
                }

                // FA — financial statements trend (last 4 annual periods)
                if let Ok(Some(fa)) = rx::get_financials(&conn, &sym_upper) {
                    if !fa.income_annual.is_empty() {
                        let _ = writeln!(p, "### Annual Financial Statements Trend");
                        let _ = writeln!(p, "| FY | Revenue | Gross | Op Inc | Net Inc | EPS |");
                        let _ = writeln!(p, "|---|---|---|---|---|---|");
                        for i in fa.income_annual.iter().take(4) {
                            let _ = writeln!(
                                p,
                                "| {} | {} | {} | {} | {} | {:.2} |",
                                i.date,
                                fmt_money(i.revenue),
                                fmt_money(i.gross_profit),
                                fmt_money(i.operating_income),
                                fmt_money(i.net_income),
                                i.eps
                            );
                        }
                        let _ = writeln!(p);
                    }
                    if !fa.cashflow_annual.is_empty() {
                        let _ = writeln!(p, "### Annual Cash Flow Trend");
                        let _ = writeln!(p, "| FY | CFO | Capex | FCF | Div Paid | Buybacks |");
                        let _ = writeln!(p, "|---|---|---|---|---|---|");
                        for c in fa.cashflow_annual.iter().take(4) {
                            let _ = writeln!(
                                p,
                                "| {} | {} | {} | {} | {} | {} |",
                                c.date,
                                fmt_money(c.cash_from_operations),
                                fmt_money(c.capex),
                                fmt_money(c.free_cash_flow),
                                fmt_money(c.dividends_paid),
                                fmt_money(c.stock_repurchases)
                            );
                        }
                        let _ = writeln!(p);
                    }
                    if !fa.balance_annual.is_empty() {
                        let _ = writeln!(p, "### Annual Balance Sheet Trend");
                        let _ = writeln!(p, "| FY | Assets | Net Debt | Total Equity |");
                        let _ = writeln!(p, "|---|---|---|---|");
                        for b in fa.balance_annual.iter().take(4) {
                            let _ = writeln!(
                                p,
                                "| {} | {} | {} | {} |",
                                b.date,
                                fmt_money(b.total_assets),
                                fmt_money(b.net_debt),
                                fmt_money(b.total_equity)
                            );
                        }
                        let _ = writeln!(p);
                    }
                }

                // MGMT — executive team (top 6)
                if let Ok(Some(execs)) = rx::get_executives(&conn, &sym_upper) {
                    if !execs.is_empty() {
                        let total_comp: f64 = execs.iter().map(|e| e.compensation).sum();
                        let _ = writeln!(
                            p,
                            "### Management ({} listed, total comp {})",
                            execs.len(),
                            fmt_money(total_comp)
                        );
                        let _ = writeln!(p, "| Name | Position | Since | Compensation |");
                        let _ = writeln!(p, "|---|---|---|---|");
                        for e in execs.iter().take(6) {
                            let comp = if e.compensation > 0.0 {
                                fmt_money(e.compensation)
                            } else {
                                "—".into()
                            };
                            let _ = writeln!(
                                p,
                                "| {} | {} | {} | {} |",
                                e.name, e.position, e.since, comp
                            );
                        }
                        let _ = writeln!(p);
                    }
                }

                // SPLT — stock splits (most recent 4)
                if let Ok(Some(splits)) = rx::get_stock_splits(&conn, &sym_upper) {
                    if !splits.is_empty() {
                        let _ = writeln!(p, "### Stock Split History");
                        let _ = writeln!(p, "| Date | Ratio |");
                        let _ = writeln!(p, "|---|---|");
                        for s in splits.iter().take(4) {
                            let _ = writeln!(p, "| {} | {} |", s.date, s.label);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ANR — analyst recommendations + consensus price target
                let pt = rx::get_price_target(&conn, &sym_upper).ok().flatten();
                let recs = rx::get_analyst_recs(&conn, &sym_upper).ok().flatten();
                if pt.is_some() || recs.as_ref().map_or(false, |r| !r.is_empty()) {
                    let _ = writeln!(p, "### Analyst Consensus");
                    if let Some(pt) = pt {
                        let _ = writeln!(
                            p,
                            "- Price target ({} analysts): mean {:.2}, median {:.2}, range {:.2}–{:.2} (as of {})",
                            pt.num_analysts,
                            pt.target_mean,
                            pt.target_median,
                            pt.target_low,
                            pt.target_high,
                            pt.last_updated
                        );
                    }
                    if let Some(r) = recs {
                        if let Some(latest) = r.first() {
                            let total = latest.strong_buy
                                + latest.buy
                                + latest.hold
                                + latest.sell
                                + latest.strong_sell;
                            if total > 0 {
                                let _ = writeln!(
                                    p,
                                    "- Recommendations ({}): SBuy {} / Buy {} / Hold {} / Sell {} / SSell {} (of {} analysts)",
                                    latest.period,
                                    latest.strong_buy,
                                    latest.buy,
                                    latest.hold,
                                    latest.sell,
                                    latest.strong_sell,
                                    total
                                );
                            }
                        }
                    }
                    let _ = writeln!(p);
                }

                // ESG — latest sustainability score
                if let Ok(Some(esg)) = rx::get_esg(&conn, &sym_upper) {
                    if let Some(latest) = esg.first() {
                        let _ = writeln!(p, "### ESG Score ({})", latest.year);
                        let _ = writeln!(
                            p,
                            "- Environmental {:.1} | Social {:.1} | Governance {:.1} | Composite {:.1}",
                            latest.environmental_score,
                            latest.social_score,
                            latest.governance_score,
                            latest.esg_score
                        );
                        let _ = writeln!(p);
                    }
                }
            }
        }
    }
}
