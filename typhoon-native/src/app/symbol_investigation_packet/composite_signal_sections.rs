use super::*;

impl TyphooNApp {
    pub(super) fn write_symbol_composite_signal_sections(&self, p: &mut String, sym_upper: &str) {
        use std::fmt::Write as _;
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use typhoon_engine::core::research as rx;

                self.write_composite_signal_early(p, sym_upper);

                self.write_composite_signal_blocks(p, sym_upper);

                // ── Research section ──
                if let Ok(Some(v)) = rx::get_val(&conn, &sym_upper) {
                    if v.value_label != "NO_DATA" && !v.value_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Value-Factor Composite — VAL ({}, as of {})",
                            v.value_label, v.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Sector: {} · peers considered: {} · composite {:.1} · inputs {}",
                            if v.sector.is_empty() {
                                "?".to_string()
                            } else {
                                v.sector.clone()
                            },
                            v.peers_considered,
                            v.composite_score,
                            v.inputs_available
                        );
                        let _ = writeln!(
                            p,
                            "- P/E {:.2} vs sector median {:.2} · Forward P/E {:.2} vs {:.2} · P/B {:.2} vs {:.2}",
                            v.pe_ratio,
                            v.pe_sector_median,
                            v.forward_pe,
                            v.forward_pe_sector_median,
                            v.price_to_book,
                            v.price_to_book_sector_median
                        );
                        let _ = writeln!(
                            p,
                            "- P/S {:.2} vs {:.2} · EV/EBITDA {:.2} vs {:.2} · FCF yield {:.2}% vs {:.2}%",
                            v.price_to_sales,
                            v.price_to_sales_sector_median,
                            v.ev_to_ebitda,
                            v.ev_to_ebitda_sector_median,
                            v.fcf_yield_pct,
                            v.fcf_yield_sector_median_pct
                        );
                        if !v.components.is_empty() {
                            let _ = writeln!(p, "- Components:");
                            for c in &v.components {
                                let _ = writeln!(
                                    p,
                                    "  - {} ({}): score {:.1}, weight {:.0}%",
                                    c.name, c.value, c.score, c.weight
                                );
                            }
                        }
                        if !v.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", v.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(q)) = rx::get_qual(&conn, &sym_upper) {
                    if q.quality_label != "NO_DATA" && !q.quality_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Quality-Factor Composite — QUAL ({}, as of {})",
                            q.quality_label, q.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Composite {:.1} · inputs {}/4",
                            q.composite_score, q.inputs_available
                        );
                        let _ = writeln!(
                            p,
                            "- Piotroski F {}/9 ({}) · Op margin {:.2}% ({}) · Cash conversion {:.0}% ({}) · Leverage {} (D/EBITDA {:.2})",
                            q.piotroski_score,
                            q.piotroski_label,
                            q.operating_margin_pct,
                            q.margin_trend_label,
                            q.cash_conversion_pct,
                            q.accruals_trend_label,
                            q.leverage_summary,
                            q.debt_to_ebitda
                        );
                        if !q.components.is_empty() {
                            let _ = writeln!(p, "- Components:");
                            for c in &q.components {
                                let _ = writeln!(
                                    p,
                                    "  - {} ({}): score {:.1}, weight {:.0}%",
                                    c.name, c.value, c.score, c.weight
                                );
                            }
                        }
                        if !q.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", q.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(r)) = rx::get_risk(&conn, &sym_upper) {
                    if r.risk_label != "NO_DATA" && !r.risk_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Risk-Factor Composite — RISK ({}, as of {})",
                            r.risk_label, r.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Composite {:.1} (higher = riskier) · inputs {}/5",
                            r.composite_score, r.inputs_available
                        );
                        let _ = writeln!(
                            p,
                            "- Realized vol {:.1}% · Beta 1Y {:.2} · Liquidity {} · Short % float {:.1}% · DTC {:.1}",
                            r.realized_vol_pct,
                            r.beta_1y,
                            r.liquidity_tier,
                            r.short_percent_of_float,
                            r.days_to_cover
                        );
                        let _ = writeln!(p, "- Altman Z {:.2} ({})", r.altman_z, r.altman_zone);
                        if !r.components.is_empty() {
                            let _ = writeln!(p, "- Components:");
                            for c in &r.components {
                                let _ = writeln!(
                                    p,
                                    "  - {} ({}): score {:.1}, weight {:.0}%",
                                    c.name, c.value, c.score, c.weight
                                );
                            }
                        }
                        if !r.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", r.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ins)) = rx::get_insstrk(&conn, &sym_upper) {
                    if ins.streak_label != "NONE" && !ins.streak_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Insider Streak Detector — INSSTRK ({}, window {}d, as of {})",
                            ins.streak_label, ins.window_days, ins.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Unique insiders: {} · buy streaks {} · sell streaks {} · longest buy {} · longest sell {}",
                            ins.unique_insiders,
                            ins.buy_streak_count,
                            ins.sell_streak_count,
                            ins.longest_buy_streak,
                            ins.longest_sell_streak
                        );
                        let _ = writeln!(
                            p,
                            "- Net buy ${:.0} · Net sell ${:.0}",
                            ins.net_buy_value_usd, ins.net_sell_value_usd
                        );
                        if !ins.rows.is_empty() {
                            let _ = writeln!(p, "- Top streaks:");
                            for row in ins.rows.iter().take(6) {
                                let _ = writeln!(
                                    p,
                                    "  - {} ({}) — {} events, net ${:.0} ({}..{})",
                                    row.insider_name,
                                    row.streak_direction,
                                    row.consecutive_events,
                                    row.net_value_usd,
                                    row.first_date,
                                    row.latest_date
                                );
                            }
                        }
                        if !ins.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ins.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(cv)) = rx::get_covg(&conn, &sym_upper) {
                    if cv.coverage_label != "NONE" && !cv.coverage_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Analyst Coverage — COVG ({}, as of {})",
                            cv.coverage_label, cv.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Analysts: {} · Target ${:.2} (${:.2}..${:.2}) · Composite {:.1} · inputs {}/3",
                            cv.num_analysts,
                            cv.target_mean,
                            cv.target_low,
                            cv.target_high,
                            cv.composite_score,
                            cv.inputs_available
                        );
                        let _ = writeln!(
                            p,
                            "- Consensus SB/B/H/S/SS {}/{}/{}/{}/{} · Bull ratio {:.0}%",
                            cv.consensus_strong_buy,
                            cv.consensus_buy,
                            cv.consensus_hold,
                            cv.consensus_sell,
                            cv.consensus_strong_sell,
                            cv.consensus_bull_ratio * 100.0
                        );
                        let _ = writeln!(
                            p,
                            "- 90d: Upgrades {} · Downgrades {} · Net {:+} · Churn {}",
                            cv.upgrades_90d, cv.downgrades_90d, cv.net_90d, cv.churn_90d
                        );
                        let _ = writeln!(
                            p,
                            "- Breadth score {:.0} · Consensus score {:.0} · Churn score {:.0}",
                            cv.breadth_score, cv.consensus_score, cv.churn_score
                        );
                        if !cv.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", cv.note);
                        }
                        let _ = writeln!(p);
                    }
                }
            }
        }
    }
}
