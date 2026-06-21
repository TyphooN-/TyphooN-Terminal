use super::*;

impl TyphooNApp {
    pub(super) fn write_price_behavior_seasonality_vol(&self, p: &mut String, sym_upper: &str) {
        use std::fmt::Write as _;
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use typhoon_engine::core::research as rx;

                if let Ok(Some(om)) = rx::get_omega(&conn, &sym_upper) {
                    if om.omega_label != "INSUFFICIENT_DATA" && !om.omega_label.is_empty() {
                        let omega_disp = if om.omega_ratio.is_finite() {
                            format!("{:.3}", om.omega_ratio)
                        } else {
                            "∞ (no loss days)".to_string()
                        };
                        let _ = writeln!(
                            p,
                            "### Omega Ratio (τ=0) — OMEGA ({}, as of {})",
                            om.omega_label, om.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · gains Σ {:.4} · losses Σ {:.4}",
                            om.bars_used, om.gains_sum, om.losses_sum
                        );
                        let _ = writeln!(
                            p,
                            "- Gain days {} · loss days {} · win rate {:.1}%",
                            om.gain_days, om.loss_days, om.win_rate_pct
                        );
                        let _ = writeln!(p, "- Omega ratio {}", omega_disp);
                        if !om.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", om.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(df)) = rx::get_dfa(&conn, &sym_upper) {
                    if df.dfa_label != "INSUFFICIENT_DATA" && !df.dfa_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Detrended Fluctuation Analysis — DFA ({}, as of {})",
                            df.dfa_label, df.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · α {:.4} · scales {} · log-log R² {:.3}",
                            df.bars_used, df.alpha, df.num_scales, df.r_squared
                        );
                        if !df.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", df.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(bk)) = rx::get_burke(&conn, &sym_upper) {
                    if bk.burke_label != "INSUFFICIENT_DATA" && !bk.burke_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Burke Ratio — BURKE ({}, as of {})",
                            bk.burke_label, bk.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · annualized return {:+.2}% · Burke ratio {:+.3}",
                            bk.bars_used, bk.annualized_return_pct, bk.burke_ratio
                        );
                        let _ = writeln!(
                            p,
                            "- Drawdown events {} · Σdd² {:.3} · worst event {:.2}%",
                            bk.dd_event_count, bk.sum_sq_drawdowns, bk.worst_event_dd_pct
                        );
                        if !bk.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", bk.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ms)) = rx::get_monthseas(&conn, &sym_upper) {
                    if ms.season_label != "INSUFFICIENT_DATA" && !ms.season_label.is_empty() {
                        const MONTHS: [&str; 12] = [
                            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct",
                            "Nov", "Dec",
                        ];
                        let _ = writeln!(
                            p,
                            "### Monthly Seasonality — MONTHSEAS ({}, as of {})",
                            ms.season_label, ms.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Years covered {} · bars {}",
                            ms.years_covered, ms.bars_used
                        );
                        let _ = writeln!(
                            p,
                            "- Best month: **{}** ({:.1}% hit, {:+.3}% mean)",
                            MONTHS[ms.best_month_idx],
                            ms.best_month_hit_pct,
                            ms.month_mean_ret_pct[ms.best_month_idx]
                        );
                        let _ = writeln!(
                            p,
                            "- Worst month: **{}** ({:.1}% hit, {:+.3}% mean)",
                            MONTHS[ms.worst_month_idx],
                            ms.worst_month_hit_pct,
                            ms.month_mean_ret_pct[ms.worst_month_idx]
                        );
                        let cells: Vec<String> = (0..12)
                            .map(|i| {
                                format!(
                                    "{} {:.0}%/{:+.2}%",
                                    MONTHS[i], ms.month_hit_pct[i], ms.month_mean_ret_pct[i]
                                )
                            })
                            .collect();
                        let _ = writeln!(p, "- Monthly hit %/mean: {}", cells.join(" · "));
                        if !ms.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ms.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(rs)) = rx::get_rollsprd(&conn, &sym_upper) {
                    if rs.roll_label != "INSUFFICIENT_DATA" && !rs.roll_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Roll Implicit Bid-Ask Spread — ROLLSPRD ({}, as of {})",
                            rs.roll_label, rs.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · first-lag cov {:+.6} · mean price {:.4}",
                            rs.bars_used, rs.first_lag_cov, rs.mean_price
                        );
                        if rs.roll_label != "INVALID_POSITIVE_COV" {
                            let _ = writeln!(
                                p,
                                "- Implicit spread {:.4} ({:.2} bps)",
                                rs.implicit_spread, rs.implicit_spread_bps
                            );
                        } else {
                            let _ = writeln!(
                                p,
                                "- Spread undefined: first-lag cov non-negative (trending series)."
                            );
                        }
                        if !rs.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", rs.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── Research section ──
            }
        }
    }
}
