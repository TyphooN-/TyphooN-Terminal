use super::*;

impl TyphooNApp {
    pub(super) fn write_price_behavior_ratios(&self, p: &mut String, sym_upper: &str) {
        use std::fmt::Write as _;
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use typhoon_engine::core::research as rx;

                if let Ok(Some(sr)) = rx::get_sharpr(&conn, &sym_upper) {
                    if sr.sharpe_label != "INSUFFICIENT_DATA" && !sr.sharpe_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Sharpe Ratio (rf=0) — SHARPR ({}, as of {})",
                            sr.sharpe_label, sr.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · mean r {:.6} · stdev r {:.6}",
                            sr.bars_used, sr.mean_log_return, sr.stdev_log_return
                        );
                        let _ = writeln!(
                            p,
                            "- Sharpe {:.3} (ann {:.3}) · mean ann {:.4} · stdev ann {:.4}",
                            sr.sharpe_ratio,
                            sr.sharpe_ratio_ann,
                            sr.mean_return_ann,
                            sr.stdev_return_ann
                        );
                        if !sr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", sr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(er)) = rx::get_effratio(&conn, &sym_upper) {
                    if er.efficiency_label != "INSUFFICIENT_DATA" && !er.efficiency_label.is_empty()
                    {
                        let _ = writeln!(
                            p,
                            "### Kaufman Efficiency Ratio — EFFRATIO ({}, as of {})",
                            er.efficiency_label, er.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · start {:.4} · end {:.4} · net {:+.4} ({:+.2}%)",
                            er.bars_used,
                            er.start_close,
                            er.end_close,
                            er.net_change,
                            er.net_change_pct
                        );
                        let _ = writeln!(
                            p,
                            "- Σ |Δclose| {:.4} · ER {:.3} · signed {:+.3}",
                            er.sum_abs_changes, er.efficiency_ratio, er.signed_efficiency
                        );
                        if !er.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", er.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(wb)) = rx::get_wickbias(&conn, &sym_upper) {
                    if wb.bias_label != "INSUFFICIENT_DATA" && !wb.bias_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Wick Bias — WICKBIAS ({}, as of {})",
                            wb.bias_label, wb.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · avg upper {:.3} · avg lower {:.3} · body {:.3}",
                            wb.bars_used, wb.avg_upper_wick, wb.avg_lower_wick, wb.avg_body_share
                        );
                        let _ = writeln!(
                            p,
                            "- Median upper {:.3} · median lower {:.3} · bias score {:+.4}",
                            wb.median_upper_wick, wb.median_lower_wick, wb.wick_bias_score
                        );
                        if !wb.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", wb.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(vv)) = rx::get_volofvol(&conn, &sym_upper) {
                    if vv.cv_label != "INSUFFICIENT_DATA" && !vv.cv_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Vol-of-Vol (stdev of rolling 20d RV) — VOLOFVOL ({}, as of {})",
                            vv.cv_label, vv.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- RV points {} · mean RV20 {:.5} · stdev RV20 {:.5} · CV {:.3}",
                            vv.bars_used, vv.mean_rv20, vv.stdev_rv20, vv.cv_rv20
                        );
                        let _ = writeln!(
                            p,
                            "- Min RV20 {:.5} · max {:.5} · latest {:.5}",
                            vv.min_rv20, vv.max_rv20, vv.latest_rv20
                        );
                        if !vv.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", vv.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── Research section ──
            }
        }
    }
}
