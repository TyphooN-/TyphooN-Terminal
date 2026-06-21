use super::*;

impl TyphooNApp {
    pub(super) fn write_rank_drift_accs_vrp(&self, p: &mut String, sym_upper: &str) {
        use std::fmt::Write as _;
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use typhoon_engine::core::research as rx;

                if let Ok(Some(od)) = rx::get_operank_delta(&conn, &sym_upper) {
                    if od.rank_label != "NO_DATA"
                        && od.rank_label != "INSUFFICIENT_DATA"
                        && !od.rank_label.is_empty()
                    {
                        let _ = writeln!(
                            p,
                            "### Operating Margin Trend Rank — OPERANK_DELTA ({} / {}, as of {})",
                            od.operating_trend_label, od.rank_label, od.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- {} basis {} · operating margin {:.2}% · change {:+.2} pts · rank {}/{} · pct {:.0}",
                            od.basis,
                            od.latest_period,
                            od.operating_margin_pct,
                            od.operating_margin_change_pct,
                            od.rank_position,
                            od.peers_considered + 1,
                            od.percentile_rank
                        );
                        let _ = writeln!(
                            p,
                            "- Sector {} median / p25 / p75 change: {:+.2} / {:+.2} / {:+.2} pts",
                            od.sector,
                            od.sector_median_change_pct,
                            od.sector_p25_change_pct,
                            od.sector_p75_change_pct
                        );
                        if !od.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", od.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(da)) = rx::get_divacc(&conn, &sym_upper) {
                    if da.divacc_label != "NO_HISTORY" && !da.divacc_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Dividend Acceleration — DIVACC ({}, as of {})",
                            da.divacc_label, da.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- {} years · latest annual dividend {:.4} ({}) · latest/prior y/y {:+.2}% / {:+.2}%",
                            da.years_covered,
                            da.latest_annual_dividend,
                            da.latest_year,
                            da.latest_yoy_growth_pct,
                            da.prior_yoy_growth_pct
                        );
                        let _ = writeln!(
                            p,
                            "- Acceleration {:+.2} pts · 3y avg growth {:+.2}% vs prior {:+.2}% · consistency {:.0}%",
                            da.acceleration_pct_pts,
                            da.recent_3y_avg_growth_pct,
                            da.prior_3y_avg_growth_pct,
                            da.consistency_score_pct
                        );
                        if !da.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", da.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ea)) = rx::get_epsacc(&conn, &sym_upper) {
                    if ea.epsacc_label != "INSUFFICIENT_DATA" && !ea.epsacc_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### EPS Acceleration — EPSACC ({}, as of {})",
                            ea.epsacc_label, ea.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Latest {} EPS {:.3} vs {:.3} y/y · latest/prior y/y {:+.2}% / {:+.2}%",
                            ea.latest_period,
                            ea.latest_eps,
                            ea.prior_year_eps,
                            ea.latest_yoy_growth_pct,
                            ea.prior_yoy_growth_pct
                        );
                        let _ = writeln!(
                            p,
                            "- Acceleration {:+.2} pts · recent 2q avg {:+.2}% vs prior {:+.2}% · positive y/y quarters {}",
                            ea.acceleration_pct_pts,
                            ea.recent_2q_avg_yoy_growth_pct,
                            ea.prior_2q_avg_yoy_growth_pct,
                            ea.positive_yoy_quarters
                        );
                        if !ea.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ea.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(vrp)) = rx::get_vrp(&conn, &sym_upper) {
                    if vrp.premium_label != "INSUFFICIENT_DATA" && !vrp.premium_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Vol Risk Premium — VRP ({} / {}, as of {})",
                            vrp.premium_label, vrp.rv_cone_label, vrp.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- ATM IV {:.1}% (rank {:.0} / pct {:.0}) · RV20 / RV60 / RV252 {:.1}% / {:.1}% / {:.1}%",
                            vrp.current_atm_iv_pct,
                            vrp.iv_rank,
                            vrp.iv_percentile,
                            vrp.rv20_pct,
                            vrp.rv60_pct,
                            vrp.rv252_pct
                        );
                        let _ = writeln!(
                            p,
                            "- IV-RV20 {:+.1} pts ({:.2}x) · IV-RV252 {:+.1} pts ({:.2}x) · RV20 cone pct {:.0}",
                            vrp.iv_minus_rv20_pct,
                            vrp.iv_to_rv20_ratio,
                            vrp.iv_minus_rv252_pct,
                            vrp.iv_to_rv252_ratio,
                            vrp.rv20_percentile
                        );
                        if !vrp.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", vrp.note);
                        }
                        let _ = writeln!(p);
                    }
                }
            }
        }
    }
}
