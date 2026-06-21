use super::*;

impl TyphooNApp {
    pub(super) fn write_symbol_price_transform_indicator_sections(
        &self,
        p: &mut String,
        sym_upper: &str,
    ) {
        use std::fmt::Write as _;
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use typhoon_engine::core::research as rx;

                self.write_price_transform_adaptive_osc(p, sym_upper);

                self.write_price_transform_volatility_force(p, sym_upper);

                self.write_price_transform_linear_hilbert(p, sym_upper);

                // ── Research section ──
                if let Ok(Some(lr)) = rx::get_linearreg(&conn, &sym_upper) {
                    if lr.linearreg_label != "INSUFFICIENT_DATA" && !lr.linearreg_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Linear Regression — LINEARREG ({}, as of {})",
                            lr.linearreg_label, lr.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · fitted {:.4} (prev {:.4}) · residual {:+.4} · residual_pct {:+.3}% · close {:.4}",
                            lr.bars_used,
                            lr.length,
                            lr.fitted,
                            lr.fitted_prev,
                            lr.residual,
                            lr.residual_pct,
                            lr.last_close
                        );
                        if !lr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", lr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(la)) = rx::get_linearreg_angle(&conn, &sym_upper) {
                    if la.angle_label != "INSUFFICIENT_DATA" && !la.angle_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Linear Regression Angle — LINEARREG_ANGLE ({}, as of {})",
                            la.angle_label, la.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · slope {:+.6} · angle {:+.3}° (prev {:+.3}°) · close {:.4}",
                            la.bars_used,
                            la.length,
                            la.slope,
                            la.angle_deg,
                            la.angle_deg_prev,
                            la.last_close
                        );
                        if !la.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", la.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(dp)) = rx::get_ht_dcphase(&conn, &sym_upper) {
                    if dp.phase_label != "INSUFFICIENT_DATA" && !dp.phase_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Hilbert Dominant Cycle Phase — HT_DCPHASE ({}, as of {})",
                            dp.phase_label, dp.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · phase {:.2}° (prev {:.2}°) · delta {:+.2}° · period {:.2} · close {:.4}",
                            dp.bars_used,
                            dp.phase_deg,
                            dp.phase_deg_prev,
                            dp.phase_delta,
                            dp.period,
                            dp.last_close
                        );
                        if !dp.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", dp.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(hs)) = rx::get_ht_sine(&conn, &sym_upper) {
                    if hs.sine_label != "INSUFFICIENT_DATA" && !hs.sine_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Hilbert Sine Wave — HT_SINE ({}, as of {})",
                            hs.sine_label, hs.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · sine {:+.3} (prev {:+.3}) · leadsine {:+.3} (prev {:+.3}) · crossover {} · period {:.2} · close {:.4}",
                            hs.bars_used,
                            hs.sine,
                            hs.sine_prev,
                            hs.leadsine,
                            hs.leadsine_prev,
                            hs.crossover,
                            hs.period,
                            hs.last_close
                        );
                        if !hs.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", hs.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(hp)) = rx::get_ht_phasor(&conn, &sym_upper) {
                    if hp.phasor_label != "INSUFFICIENT_DATA" && !hp.phasor_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Hilbert Phasor — HT_PHASOR ({}, as of {})",
                            hp.phasor_label, hp.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · I {:+.4} (prev {:+.4}) · Q {:+.4} (prev {:+.4}) · magnitude {:.4} · phase {:+.2}° · close {:.4}",
                            hp.bars_used,
                            hp.i_comp,
                            hp.i_prev,
                            hp.q_comp,
                            hp.q_prev,
                            hp.magnitude,
                            hp.phase_deg,
                            hp.last_close
                        );
                        if !hp.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", hp.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(mp)) = rx::get_midprice(&conn, &sym_upper) {
                    if mp.midprice_label != "INSUFFICIENT_DATA" && !mp.midprice_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Range Midpoint — MIDPRICE ({}, as of {})",
                            mp.midprice_label, mp.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · midprice {:.4} (prev {:.4}) · HHV {:.4} · LLV {:.4} · position {:.3} · close {:.4}",
                            mp.bars_used,
                            mp.length,
                            mp.midprice,
                            mp.midprice_prev,
                            mp.hhv,
                            mp.llv,
                            mp.position,
                            mp.last_close
                        );
                        if !mp.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", mp.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ap)) = rx::get_apo(&conn, &sym_upper) {
                    if ap.apo_label != "INSUFFICIENT_DATA" && !ap.apo_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Absolute Price Oscillator — APO ({}, as of {})",
                            ap.apo_label, ap.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · fast {} · slow {} · APO {:+.4} (prev {:+.4}) · fast_EMA {:.4} · slow_EMA {:.4} · close {:.4}",
                            ap.bars_used,
                            ap.fast_period,
                            ap.slow_period,
                            ap.apo,
                            ap.apo_prev,
                            ap.fast_ema,
                            ap.slow_ema,
                            ap.last_close
                        );
                        if !ap.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ap.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(mm)) = rx::get_mom(&conn, &sym_upper) {
                    if mm.mom_label != "INSUFFICIENT_DATA" && !mm.mom_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Raw Momentum — MOM ({}, as of {})",
                            mm.mom_label, mm.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · MOM {:+.4} (prev {:+.4}) · MOM% {:+.3} · close {:.4}",
                            mm.bars_used, mm.period, mm.mom, mm.mom_prev, mm.mom_pct, mm.last_close
                        );
                        if !mm.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", mm.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(sx)) = rx::get_sarext(&conn, &sym_upper) {
                    if sx.sarext_label != "INSUFFICIENT_DATA" && !sx.sarext_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Extended Parabolic SAR — SAREXT ({}, as of {})",
                            sx.sarext_label, sx.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · AF long init/step/max {:.3}/{:.3}/{:.3} · AF short init/step/max {:.3}/{:.3}/{:.3} · SAR {:.4} · EP {:.4} · AF {:.3} · trend {} · in-trend {} · distance {:+.3}% · close {:.4}",
                            sx.bars_used,
                            sx.af_init_long,
                            sx.af_step_long,
                            sx.af_max_long,
                            sx.af_init_short,
                            sx.af_step_short,
                            sx.af_max_short,
                            sx.sar_value,
                            sx.extreme_point,
                            sx.acceleration_factor,
                            if sx.trend_is_up { "UP" } else { "DOWN" },
                            sx.bars_in_trend,
                            sx.distance_pct,
                            sx.last_close
                        );
                        if !sx.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", sx.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ar)) = rx::get_adxr(&conn, &sym_upper) {
                    if ar.adxr_label != "INSUFFICIENT_DATA" && !ar.adxr_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### ADX Rating — ADXR ({}, as of {})",
                            ar.adxr_label, ar.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · ADX now {:.3} · ADX prior {:.3} · ADXR {:.3} (prev {:.3}) · close {:.4}",
                            ar.bars_used,
                            ar.period,
                            ar.adx_now,
                            ar.adx_prior,
                            ar.adxr,
                            ar.adxr_prev,
                            ar.last_close
                        );
                        if !ar.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ar.note);
                        }
                        let _ = writeln!(p);
                    }
                }
            }
        }
    }
}
