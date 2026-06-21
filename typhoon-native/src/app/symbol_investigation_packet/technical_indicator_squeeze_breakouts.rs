use super::*;

impl TyphooNApp {
    pub(super) fn write_technical_indicator_squeeze_breakouts(
        &self,
        p: &mut String,
        sym_upper: &str,
    ) {
        use std::fmt::Write as _;
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use typhoon_engine::core::research as rx;

                // ── Research section ──
                if let Ok(Some(sq)) = rx::get_squeeze(&conn, &sym_upper) {
                    if sq.squeeze_label != "INSUFFICIENT_DATA" && !sq.squeeze_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Short-Squeeze Composite — SQUEEZE ({}, as of {})",
                            sq.squeeze_label, sq.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Composite={:.1}/100 · axes present={}/5",
                            sq.composite_score, sq.inputs_present
                        );
                        let _ = writeln!(
                            p,
                            "- Short%float={:.2}% (score {:.0}) · DTC={:.2}d (score {:.0}) · 20d mom={:+.2}% (score {:.0}) · RelVol20d={:.2}× (score {:.0}) · IVrank={:.1} (score {:.0})",
                            sq.short_percent_of_float,
                            sq.short_float_score,
                            sq.days_to_cover,
                            sq.days_to_cover_score,
                            sq.momentum_20d_pct,
                            sq.momentum_score,
                            sq.relvol_20d,
                            sq.relvol_score,
                            sq.iv_rank,
                            sq.iv_rank_score
                        );
                        if !sq.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", sq.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(sr)) = rx::get_squeezerank(&conn, &sym_upper) {
                    if sr.squeezerank_label != "INSUFFICIENT_DATA"
                        && !sr.squeezerank_label.is_empty()
                    {
                        let _ = writeln!(
                            p,
                            "### Short-Squeeze Cross-Symbol Rank — SQUEEZERANK ({}, as of {})",
                            sr.squeezerank_label, sr.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Composite={:.1} · rank={}/{} · percentile={:.1}",
                            sr.composite_score, sr.rank, sr.peer_count, sr.percentile
                        );
                        if !sr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", sr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(bb)) = rx::get_bbsqueeze(&conn, &sym_upper) {
                    if bb.bbsqueeze_label != "INSUFFICIENT_DATA" && !bb.bbsqueeze_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Bollinger-Band Squeeze — BBSQUEEZE ({}, as of {})",
                            bb.bbsqueeze_label, bb.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · BB width cur={:.5} · min120={:.5} · max120={:.5} · pct={:.1}",
                            bb.bars_used,
                            bb.period,
                            bb.bb_width_current,
                            bb.bb_width_min_120,
                            bb.bb_width_max_120,
                            bb.bb_width_percentile
                        );
                        let _ = writeln!(
                            p,
                            "- Upper={:.4} · mid={:.4} · lower={:.4} · close={:.4}",
                            bb.upper_band, bb.mid_band, bb.lower_band, bb.last_close
                        );
                        if !bb.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", bb.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(dn)) = rx::get_donchian(&conn, &sym_upper) {
                    if dn.donchian_label != "INSUFFICIENT_DATA" && !dn.donchian_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Donchian Channel Breakout — DONCHIAN ({}, as of {})",
                            dn.donchian_label, dn.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · upper={:.4} · mid={:.4} · lower={:.4} · close={:.4} · pos={:.1}% · up break={} · dn break={}",
                            dn.bars_used,
                            dn.period,
                            dn.upper_channel,
                            dn.mid_channel,
                            dn.lower_channel,
                            dn.last_close,
                            dn.channel_position_pct,
                            dn.breakout_upper,
                            dn.breakout_lower
                        );
                        if !dn.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", dn.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(km)) = rx::get_kama(&conn, &sym_upper) {
                    if km.kama_label != "INSUFFICIENT_DATA" && !km.kama_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Kaufman Adaptive MA — KAMA ({}, as of {})",
                            km.kama_label, km.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · ER={:.3} · KAMA={:.4} · close={:.4} · 5-bar slope={:+.2}%",
                            km.bars_used,
                            km.period,
                            km.efficiency_ratio,
                            km.kama_value,
                            km.last_close,
                            km.kama_slope_pct
                        );
                        if !km.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", km.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── Research section ──
            }
        }
    }
}
