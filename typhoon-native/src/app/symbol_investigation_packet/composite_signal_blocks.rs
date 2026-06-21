use super::*;

impl TyphooNApp {
    pub(super) fn write_composite_signal_blocks(&self, p: &mut String, sym_upper: &str) {
        use std::fmt::Write as _;
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use typhoon_engine::core::research as rx;

                // ── blocks ──
                if let Ok(Some(gw)) = rx::get_growm(&conn, &sym_upper) {
                    if gw.inputs_available > 0 {
                        let _ = writeln!(p, "### GARP Composite — GROWM (as of {})", gw.as_of);
                        let _ = writeln!(
                            p,
                            "- {} · composite {:.1}/100 · {}/3 inputs",
                            gw.garp_label, gw.composite_score, gw.inputs_available
                        );
                        let _ = writeln!(
                            p,
                            "- Momentum: {} ({:.1}) · Earnings: {} ({:.1}) · Dividend CAGR 3y: {:.2}% ({})",
                            gw.momentum_regime,
                            gw.momentum_score,
                            gw.earnings_label,
                            gw.earnings_momentum_score,
                            gw.dividend_cagr_3y_pct,
                            gw.dividend_trend
                        );
                        if !gw.components.is_empty() {
                            let _ = writeln!(p, "- Components:");
                            for c in gw.components.iter().take(5) {
                                let _ = writeln!(
                                    p,
                                    "  - {}: {} · score {:.1} · weight {:.0}% · contrib {:.1}",
                                    c.name, c.value, c.score, c.weight, c.contribution
                                );
                            }
                        }
                        if !gw.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", gw.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(fl)) = rx::get_flow(&conn, &sym_upper) {
                    if fl.insider_trade_count > 0 || fl.institutional_holders_tracked > 0 {
                        let _ = writeln!(
                            p,
                            "### Smart-Money Flow — FLOW ({}d, as of {})",
                            fl.window_days, fl.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- {} · composite {:.1}/100 · insider {:.1} · institutional {:.1}",
                            fl.flow_label,
                            fl.composite_score,
                            fl.insider_score,
                            fl.institutional_score
                        );
                        let _ = writeln!(
                            p,
                            "- Insiders: {} trades / {} unique · buy ${:.0} · sell ${:.0} · net ${:+.0}",
                            fl.insider_trade_count,
                            fl.unique_insiders,
                            fl.insider_buy_value_usd,
                            fl.insider_sell_value_usd,
                            fl.insider_net_value_usd
                        );
                        let _ = writeln!(
                            p,
                            "- Institutional: {} buyers / {} sellers / {} tracked · net ratio {:+.2} · share delta {:+.0}",
                            fl.institutional_buyers,
                            fl.institutional_sellers,
                            fl.institutional_holders_tracked,
                            fl.institutional_net_ratio,
                            fl.institutional_share_delta
                        );
                        if !fl.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", fl.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(rg)) = rx::get_regime(&conn, &sym_upper) {
                    if rg.inputs_available > 0 {
                        let _ = writeln!(p, "### Market Regime — REGIME (as of {})", rg.as_of);
                        let _ = writeln!(
                            p,
                            "- {} · composite {:.1}/100 · {}/3 inputs",
                            rg.regime_label, rg.composite_score, rg.inputs_available
                        );
                        let _ = writeln!(
                            p,
                            "- Realized vol: {:.2}% ({}) · ADX: {:.1} ({}) · 1Y return {:+.2}% · Sharpe {:.2}",
                            rg.realized_vol_pct,
                            rg.vol_source,
                            rg.adx_value,
                            rg.trend_summary,
                            rg.return_1y_pct,
                            rg.sharpe_ratio
                        );
                        let _ = writeln!(
                            p,
                            "- Sub-scores: trend {:.1} · volatility {:.1} · return {:.1}",
                            rg.trend_strength_score, rg.volatility_score, rg.return_score
                        );
                        if !rg.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", rg.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(rv)) = rx::get_relvol(&conn, &sym_upper) {
                    if rv.activity_label != "INSUFFICIENT_DATA" && !rv.activity_label.is_empty() {
                        let _ = writeln!(p, "### Relative Volume — RELVOL (as of {})", rv.as_of);
                        let _ = writeln!(
                            p,
                            "- {} · {} · rel-vol 5d/20d/60d {:.2}×/{:.2}×/{:.2}×",
                            rv.activity_label,
                            rv.direction_label,
                            rv.rel_volume_5d,
                            rv.rel_volume_20d,
                            rv.rel_volume_60d
                        );
                        let _ = writeln!(
                            p,
                            "- Current {:.0} · avg 5d/20d/60d {:.0}/{:.0}/{:.0}",
                            rv.current_volume,
                            rv.avg_volume_5d,
                            rv.avg_volume_20d,
                            rv.avg_volume_60d
                        );
                        let _ = writeln!(
                            p,
                            "- 5d-vs-20d trend {:+.2}% · 60d percentile {:.0}",
                            rv.volume_trend_5d_pct, rv.volume_percentile_60d
                        );
                        if !rv.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", rv.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(mg)) = rx::get_margins(&conn, &sym_upper) {
                    if mg.periods_used > 0 {
                        let _ = writeln!(
                            p,
                            "### Margin Trajectory — MARGINS ({} basis, as of {})",
                            mg.basis, mg.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Overall: {} · quality {} · latest period {}",
                            mg.overall_trend_label, mg.quality_label, mg.latest_period
                        );
                        let _ = writeln!(
                            p,
                            "- Gross: {:.2}% (prior {:.2}%, Δ{:+.2}pp, {}) · Op: {:.2}% (prior {:.2}%, Δ{:+.2}pp, {}) · Net: {:.2}% (prior {:.2}%, Δ{:+.2}pp, {})",
                            mg.latest_gross_margin_pct,
                            mg.prior_gross_margin_pct,
                            mg.gross_margin_change_pct,
                            mg.gross_trend_label,
                            mg.latest_operating_margin_pct,
                            mg.prior_operating_margin_pct,
                            mg.operating_margin_change_pct,
                            mg.operating_trend_label,
                            mg.latest_net_margin_pct,
                            mg.prior_net_margin_pct,
                            mg.net_margin_change_pct,
                            mg.net_trend_label
                        );
                        let _ = writeln!(
                            p,
                            "- Avg gross/op/net {:.2}%/{:.2}%/{:.2}% · periods used {}",
                            mg.avg_gross_margin_pct,
                            mg.avg_operating_margin_pct,
                            mg.avg_net_margin_pct,
                            mg.periods_used
                        );
                        if !mg.periods.is_empty() {
                            let _ = writeln!(p, "- Per-period (gross/op/net %):");
                            for row in mg.periods.iter().take(6) {
                                let _ = writeln!(
                                    p,
                                    "  - {}: {:.2} / {:.2} / {:.2}",
                                    row.period,
                                    row.gross_margin_pct,
                                    row.operating_margin_pct,
                                    row.net_margin_pct
                                );
                            }
                        }
                        if !mg.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", mg.note);
                        }
                        let _ = writeln!(p);
                    }
                }
            }
        }
    }
}
