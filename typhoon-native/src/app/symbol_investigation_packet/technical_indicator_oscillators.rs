use super::*;

impl TyphooNApp {
    pub(super) fn write_technical_indicator_oscillators(&self, p: &mut String, sym_upper: &str) {
        use std::fmt::Write as _;
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use typhoon_engine::core::research as rx;

                if let Ok(Some(ax)) = rx::get_adx(&conn, &sym_upper) {
                    if ax.adx_label != "INSUFFICIENT_DATA" && !ax.adx_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Directional Movement — ADX ({}, as of {})",
                            ax.adx_label, ax.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · +DI={:.2} · -DI={:.2} · ADX={:.2} · DX={:.2} · ATR={:.4} · close={:.4}",
                            ax.bars_used,
                            ax.period,
                            ax.plus_di,
                            ax.minus_di,
                            ax.adx,
                            ax.dx,
                            ax.atr,
                            ax.last_close
                        );
                        if !ax.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ax.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(cc)) = rx::get_cci(&conn, &sym_upper) {
                    if cc.cci_label != "INSUFFICIENT_DATA" && !cc.cci_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Commodity Channel Index — CCI ({}, as of {})",
                            cc.cci_label, cc.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · TP={:.4} · SMA(TP)={:.4} · MAD={:.4} · CCI={:+.2} · close={:.4}",
                            cc.bars_used,
                            cc.period,
                            cc.typical_price,
                            cc.tp_sma,
                            cc.mean_abs_dev,
                            cc.cci_value,
                            cc.last_close
                        );
                        if !cc.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", cc.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(cm)) = rx::get_cmf(&conn, &sym_upper) {
                    if cm.cmf_label != "INSUFFICIENT_DATA" && !cm.cmf_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Chaikin Money Flow — CMF ({}, as of {})",
                            cm.cmf_label, cm.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · CMF={:+.4} · ΣMFV={:.2} · Σvol={:.2} · close={:.4}",
                            cm.bars_used,
                            cm.period,
                            cm.cmf_value,
                            cm.money_flow_volume_sum,
                            cm.volume_sum,
                            cm.last_close
                        );
                        if !cm.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", cm.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(mf)) = rx::get_mfi(&conn, &sym_upper) {
                    if mf.mfi_label != "INSUFFICIENT_DATA" && !mf.mfi_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Money Flow Index — MFI ({}, as of {})",
                            mf.mfi_label, mf.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · MFI={:.2} · +MF={:.2} · -MF={:.2} · ratio={:.3} · close={:.4}",
                            mf.bars_used,
                            mf.period,
                            mf.mfi_value,
                            mf.positive_mf_sum,
                            mf.negative_mf_sum,
                            mf.money_flow_ratio,
                            mf.last_close
                        );
                        if !mf.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", mf.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ps)) = rx::get_psar(&conn, &sym_upper) {
                    if ps.psar_label != "INSUFFICIENT_DATA" && !ps.psar_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Parabolic SAR — PSAR ({}, as of {})",
                            ps.psar_label, ps.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · AF={:.2}/{:.2}/{:.2} · SAR={:.4} · EP={:.4} · cur AF={:.3} · trend={} · bars in trend={} · dist={:+.2}% · close={:.4}",
                            ps.bars_used,
                            ps.af_start,
                            ps.af_step,
                            ps.af_max,
                            ps.sar_value,
                            ps.extreme_point,
                            ps.acceleration_factor,
                            if ps.trend_is_up { "UP" } else { "DOWN" },
                            ps.bars_in_trend,
                            ps.distance_pct,
                            ps.last_close
                        );
                        if !ps.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ps.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── Research section ──
            }
        }
    }
}
