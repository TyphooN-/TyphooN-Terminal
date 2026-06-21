use super::*;

impl TyphooNApp {
    pub(super) fn write_price_transform_volatility_force(&self, p: &mut String, sym_upper: &str) {
        use std::fmt::Write as _;
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use typhoon_engine::core::research as rx;

                // ── MASSINDEX / NATR / TTM_SQUEEZE / FORCE_INDEX / TRANGE ──
                if let Ok(Some(mi)) = rx::get_mass_index(&conn, &sym_upper) {
                    if mi.mass_label != "INSUFFICIENT_DATA" && !mi.mass_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Dorsey Mass Index — MASSINDEX ({}, as of {})",
                            mi.mass_label, mi.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · EMA len {} · sum len {} · EMA(H-L) {:.4} · EMA-of-EMA {:.4} · ratio {:.4} · MI {:.2} (prev {:.2}) · close {:.4}",
                            mi.bars_used,
                            mi.ema_len,
                            mi.sum_len,
                            mi.ema_range,
                            mi.ema_ema_range,
                            mi.ratio,
                            mi.mass_index,
                            mi.mass_index_prev,
                            mi.last_close
                        );
                        if !mi.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", mi.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(na)) = rx::get_natr(&conn, &sym_upper) {
                    if na.natr_label != "INSUFFICIENT_DATA" && !na.natr_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Normalized ATR — NATR ({}, as of {})",
                            na.natr_label, na.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · ATR {:.4} · NATR {:.4}% (prev {:.4}%) · close {:.4}",
                            na.bars_used,
                            na.length,
                            na.atr_value,
                            na.natr_value,
                            na.natr_prev,
                            na.last_close
                        );
                        if !na.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", na.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(tq)) = rx::get_ttm_squeeze(&conn, &sym_upper) {
                    if tq.squeeze_label != "INSUFFICIENT_DATA" && !tq.squeeze_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### TTM Squeeze — TTM_SQUEEZE ({}, as of {})",
                            tq.squeeze_label, tq.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · BB [{:.4} .. {:.4}] · KC [{:.4} .. {:.4}] · squeeze_on {} · momentum {:+.4} (prev {:+.4}) · close {:.4}",
                            tq.bars_used,
                            tq.length,
                            tq.bb_lower,
                            tq.bb_upper,
                            tq.kc_lower,
                            tq.kc_upper,
                            tq.squeeze_on,
                            tq.momentum,
                            tq.momentum_prev,
                            tq.last_close
                        );
                        if !tq.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", tq.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(fi)) = rx::get_force_index(&conn, &sym_upper) {
                    if fi.force_label != "INSUFFICIENT_DATA" && !fi.force_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Elder Force Index — FORCE_INDEX ({}, as of {})",
                            fi.force_label, fi.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · raw {:.2} · EMA {:.2} (prev {:.2}) · volume {:.0} · close {:.4}",
                            fi.bars_used,
                            fi.length,
                            fi.force_raw,
                            fi.force_ema,
                            fi.force_ema_prev,
                            fi.last_volume,
                            fi.last_close
                        );
                        if !fi.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", fi.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(tr)) = rx::get_trange(&conn, &sym_upper) {
                    if tr.trange_label != "INSUFFICIENT_DATA" && !tr.trange_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### True Range (raw) — TRANGE ({}, as of {})",
                            tr.trange_label, tr.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · TR {:.4} (prev {:.4}) · mean(20) {:.4} · ratio {:.3} · H {:.4} · L {:.4} · prev close {:.4} · close {:.4}",
                            tr.bars_used,
                            tr.trange_value,
                            tr.trange_prev,
                            tr.mean_trange_20,
                            tr.trange_ratio,
                            tr.last_high,
                            tr.last_low,
                            tr.prev_close,
                            tr.last_close
                        );
                        if !tr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", tr.note);
                        }
                        let _ = writeln!(p);
                    }
                }
            }
        }
    }
}
