use super::*;

impl TyphooNApp {
    pub(super) fn write_technical_indicator_volume_trend(&self, p: &mut String, sym_upper: &str) {
        use std::fmt::Write as _;
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use typhoon_engine::core::research as rx;

                if let Ok(Some(vx)) = rx::get_vortex(&conn, &sym_upper) {
                    if vx.vortex_label != "INSUFFICIENT_DATA" && !vx.vortex_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Vortex Indicator — VORTEX ({}, as of {})",
                            vx.vortex_label, vx.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · VI+ {:.4} · VI− {:.4} · Δ {:+.4} · ΣTR {:.4} · ΣVM+ {:.4} · ΣVM− {:.4} · close {:.4}",
                            vx.bars_used,
                            vx.period,
                            vx.vi_plus,
                            vx.vi_minus,
                            vx.vi_diff,
                            vx.sum_tr,
                            vx.sum_vm_plus,
                            vx.sum_vm_minus,
                            vx.last_close
                        );
                        if !vx.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", vx.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ch)) = rx::get_chop(&conn, &sym_upper) {
                    if ch.chop_label != "INSUFFICIENT_DATA" && !ch.chop_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Choppiness Index — CHOP ({}, as of {})",
                            ch.chop_label, ch.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · CI {:.2} · ΣTR {:.4} · range high {:.4} · low {:.4} · span {:.4} · close {:.4}",
                            ch.bars_used,
                            ch.period,
                            ch.chop_value,
                            ch.sum_tr,
                            ch.range_high,
                            ch.range_low,
                            ch.range_span,
                            ch.last_close
                        );
                        if !ch.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ch.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ob)) = rx::get_obv(&conn, &sym_upper) {
                    if ob.obv_label != "INSUFFICIENT_DATA" && !ob.obv_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### On-Balance Volume — OBV ({}, as of {})",
                            ob.obv_label, ob.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · slope window {} · OBV {:.0} · slope {:+.2} · Δ {:+.2}% · 20-bar min {:.0} · max {:.0} · close {:.4}",
                            ob.bars_used,
                            ob.slope_window,
                            ob.obv_value,
                            ob.obv_slope,
                            ob.obv_change_pct,
                            ob.obv_min_20,
                            ob.obv_max_20,
                            ob.last_close
                        );
                        if !ob.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ob.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(tr)) = rx::get_trix(&conn, &sym_upper) {
                    if tr.trix_label != "INSUFFICIENT_DATA" && !tr.trix_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Triple-EMA Oscillator — TRIX ({}, as of {})",
                            tr.trix_label, tr.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · signal {} · TRIX {:+.4} · signal {:+.4} · hist {:+.4} · EMA³ {:.4} · close {:.4}",
                            tr.bars_used,
                            tr.period,
                            tr.signal_period,
                            tr.trix_value,
                            tr.signal_value,
                            tr.histogram,
                            tr.ema3_value,
                            tr.last_close
                        );
                        if !tr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", tr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(hm)) = rx::get_hma(&conn, &sym_upper) {
                    if hm.hma_label != "INSUFFICIENT_DATA" && !hm.hma_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Hull Moving Average — HMA ({}, as of {})",
                            hm.hma_label, hm.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} (half {} / √{}) · HMA {:.4} · 5-bar slope {:+.2}% · close vs HMA {:+.2}% · close {:.4}",
                            hm.bars_used,
                            hm.period,
                            hm.half_period,
                            hm.sqrt_period,
                            hm.hma_value,
                            hm.hma_slope_pct,
                            hm.hma_vs_close_pct,
                            hm.last_close
                        );
                        if !hm.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", hm.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── Research section ──
            }
        }
    }
}
