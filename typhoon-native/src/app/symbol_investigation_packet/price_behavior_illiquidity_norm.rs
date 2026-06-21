use super::*;

impl TyphooNApp {
    pub(super) fn write_price_behavior_illiquidity_norm(&self, p: &mut String, sym_upper: &str) {
        use std::fmt::Write as _;
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use typhoon_engine::core::research as rx;

                if let Ok(Some(vr)) = rx::get_varratio(&conn, &sym_upper) {
                    if vr.rw_label != "INSUFFICIENT_DATA" && !vr.rw_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Lo-MacKinlay Variance Ratio — VARRATIO ({}, as of {})",
                            vr.rw_label, vr.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · VR(2) {:.3} · VR(5) {:.3} · VR(10) {:.3} · VR(20) {:.3}",
                            vr.bars_used, vr.vr_2, vr.vr_5, vr.vr_10, vr.vr_20
                        );
                        let _ =
                            writeln!(p, "- z(2) {:+.2} · z(5) {:+.2}", vr.z_stat_2, vr.z_stat_5);
                        if !vr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", vr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(am)) = rx::get_amihud(&conn, &sym_upper) {
                    if am.illiq_label != "INSUFFICIENT_DATA" && !am.illiq_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Amihud Illiquidity — AMIHUD ({}, as of {})",
                            am.illiq_label, am.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · mean ILLIQ {:.4} · median {:.4} · 90th pctile {:.4}",
                            am.bars_used, am.mean_illiq, am.median_illiq, am.illiq_90th
                        );
                        let _ = writeln!(p, "- Avg daily $ volume ${:.0}", am.avg_dollar_volume);
                        if !am.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", am.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(jb)) = rx::get_jbnorm(&conn, &sym_upper) {
                    if jb.normal_label != "INSUFFICIENT_DATA" && !jb.normal_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Jarque-Bera Normality Test — JBNORM ({}, as of {})",
                            jb.normal_label, jb.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · skewness {:+.3} · excess kurtosis {:.3}",
                            jb.bars_used, jb.skewness, jb.excess_kurtosis
                        );
                        let _ = writeln!(
                            p,
                            "- JB statistic {:.2} · p-value {:.6}",
                            jb.jb_statistic, jb.jb_pvalue
                        );
                        if !jb.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", jb.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── Research section ──
            }
        }
    }
}
