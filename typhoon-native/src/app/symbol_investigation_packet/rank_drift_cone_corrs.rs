use super::*;

impl TyphooNApp {
    pub(super) fn write_rank_drift_cone_corrs(&self, p: &mut String, sym_upper: &str) {
        use std::fmt::Write as _;
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use typhoon_engine::core::research as rx;

                if let Ok(Some(rvc)) = rx::get_rvcone(&conn, &sym_upper) {
                    if rvc.cone_label != "INSUFFICIENT_DATA" && !rvc.cone_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Realized Volatility Cone — RVCONE ({}, as of {})",
                            rvc.cone_label, rvc.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- 20d / 60d / 120d / 252d RV: {:.1}% / {:.1}% / {:.1}% / {:.1}%",
                            rvc.rv20_pct, rvc.rv60_pct, rvc.rv120_pct, rvc.rv252_pct
                        );
                        let _ = writeln!(
                            p,
                            "- 20d rolling min / median / max: {:.1}% / {:.1}% / {:.1}% · latest 20d pct {:.0}",
                            rvc.rv20_min_pct,
                            rvc.rv20_median_pct,
                            rvc.rv20_max_pct,
                            rvc.rv20_percentile
                        );
                        if !rvc.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", rvc.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(cpb)) = rx::get_calpb(&conn, &sym_upper) {
                    if cpb.momentum_label != "INSUFFICIENT_DATA" && !cpb.momentum_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Calendar Period Breakdown — CALPB ({} / {} {}, as of {})",
                            cpb.momentum_label, cpb.current_year, cpb.current_quarter, cpb.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- MTD {:+.2}% · QTD {:+.2}% · YTD {:+.2}%",
                            cpb.mtd_pct, cpb.qtd_pct, cpb.ytd_pct
                        );
                        let _ = writeln!(
                            p,
                            "- Prior quarter {:+.2}% · prior year {:+.2}%",
                            cpb.prior_quarter_pct, cpb.prior_year_pct
                        );
                        if !cpb.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", cpb.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(cs)) = rx::get_corrstk(&conn, &sym_upper) {
                    if cs.correlation_label != "INSUFFICIENT_DATA"
                        && !cs.correlation_label.is_empty()
                    {
                        let _ = writeln!(
                            p,
                            "### Benchmark Correlation — CORRSTK ({}, as of {})",
                            cs.correlation_label, cs.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Dominant {} · SPY 20/60/252 {:.2} / {:.2} / {:.2}",
                            cs.dominant_benchmark,
                            cs.corr_spy_20d,
                            cs.corr_spy_60d,
                            cs.corr_spy_252d
                        );
                        let _ = writeln!(
                            p,
                            "- Sector 20/60/252 {:.2} / {:.2} / {:.2} · SPY β {:.2} · sector β {:.2}",
                            cs.corr_sector_20d,
                            cs.corr_sector_60d,
                            cs.corr_sector_252d,
                            cs.beta_spy_252d,
                            cs.beta_sector_252d
                        );
                        if !cs.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", cs.note);
                        }
                        let _ = writeln!(p);
                    }
                }
            }
        }
    }
}
