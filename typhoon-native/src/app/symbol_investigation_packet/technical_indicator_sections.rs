use super::*;

impl TyphooNApp {
    pub(super) fn write_symbol_technical_indicator_sections(
        &self,
        p: &mut String,
        sym_upper: &str,
    ) {
        use std::fmt::Write as _;
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use typhoon_engine::core::research as rx;

                self.write_technical_indicator_squeeze_breakouts(p, sym_upper);

                self.write_technical_indicator_cloud_trend(p, sym_upper);

                self.write_technical_indicator_oscillators(p, sym_upper);

                self.write_technical_indicator_volume_trend(p, sym_upper);

                self.write_technical_indicator_final_osc(p, sym_upper);

                if let Ok(Some(vw)) = rx::get_vwap(&conn, &sym_upper) {
                    if vw.vwap_label != "INSUFFICIENT_DATA" && !vw.vwap_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Volume-Weighted Average Price — VWAP ({}, as of {})",
                            vw.vwap_label, vw.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · window {} · VWAP {:.4} · deviation {:+.2}% · close {:.4}",
                            vw.bars_used, vw.window, vw.vwap_value, vw.deviation_pct, vw.last_close
                        );
                        if !vw.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", vw.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(mg)) = rx::get_mcgd(&conn, &sym_upper) {
                    if mg.mcgd_label != "INSUFFICIENT_DATA" && !mg.mcgd_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### McGinley Dynamic — MCGD ({}, as of {})",
                            mg.mcgd_label, mg.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · MCGD {:.4} · prev {:.4} · deviation {:+.2}% · close {:.4}",
                            mg.bars_used,
                            mg.length,
                            mg.mcgd_value,
                            mg.mcgd_prev,
                            mg.deviation_pct,
                            mg.last_close
                        );
                        if !mg.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", mg.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(rw)) = rx::get_rwi(&conn, &sym_upper) {
                    if rw.rwi_label != "INSUFFICIENT_DATA" && !rw.rwi_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Random Walk Index — RWI ({}, as of {})",
                            rw.rwi_label, rw.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · RWI high {:.3} · RWI low {:.3} · close {:.4}",
                            rw.bars_used, rw.length, rw.rwi_high, rw.rwi_low, rw.last_close
                        );
                        if !rw.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", rw.note);
                        }
                        let _ = writeln!(p);
                    }
                }
            }
        }
    }
}
