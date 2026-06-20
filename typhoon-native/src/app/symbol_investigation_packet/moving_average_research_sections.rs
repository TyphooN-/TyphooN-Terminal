use super::*;

impl TyphooNApp {
    pub(super) fn write_symbol_moving_average_research_sections(
        &self,
        p: &mut String,
        sym_upper: &str,
    ) {
        use std::fmt::Write as _;
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use typhoon_engine::core::research as rx;

                // ── research emitters ──
                if let Ok(Some(dm)) = rx::get_dema(&conn, &sym_upper) {
                    if dm.dema_label != "INSUFFICIENT_DATA" && !dm.dema_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Double EMA — DEMA ({}, as of {})",
                            dm.dema_label, dm.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · DEMA {:.4} · close {:.4} · dev {:+.2}%",
                            dm.bars_used, dm.length, dm.dema_value, dm.last_close, dm.deviation_pct
                        );
                        if !dm.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", dm.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(tm)) = rx::get_tema(&conn, &sym_upper) {
                    if tm.tema_label != "INSUFFICIENT_DATA" && !tm.tema_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Triple EMA — TEMA ({}, as of {})",
                            tm.tema_label, tm.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · TEMA {:.4} · close {:.4} · dev {:+.2}%",
                            tm.bars_used, tm.length, tm.tema_value, tm.last_close, tm.deviation_pct
                        );
                        if !tm.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", tm.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(lr)) = rx::get_linreg(&conn, &sym_upper) {
                    if lr.linreg_label != "INSUFFICIENT_DATA" && !lr.linreg_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Linear Regression Channel — LINREG ({}, as of {})",
                            lr.linreg_label, lr.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · slope {:.5} · intercept {:.4} · R² {:.3} · σ {:.4} · fit {:.4} · ±2σ [{:.4}, {:.4}] · close {:.4}",
                            lr.bars_used,
                            lr.length,
                            lr.slope,
                            lr.intercept,
                            lr.r_squared,
                            lr.sigma,
                            lr.fit_value,
                            lr.channel_lower,
                            lr.channel_upper,
                            lr.last_close
                        );
                        if !lr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", lr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(pv)) = rx::get_pivots(&conn, &sym_upper) {
                    if pv.pivots_label != "INSUFFICIENT_DATA" && !pv.pivots_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Floor-Trader Pivots — PIVOTS ({}, as of {})",
                            pv.pivots_label, pv.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · PP {:.4} · R1 {:.4} · R2 {:.4} · S1 {:.4} · S2 {:.4} · prior OHLC [{:.4}/{:.4}/{:.4}] · close {:.4}",
                            pv.bars_used,
                            pv.pp,
                            pv.r1,
                            pv.r2,
                            pv.s1,
                            pv.s2,
                            pv.prior_high,
                            pv.prior_low,
                            pv.prior_close,
                            pv.last_close
                        );
                        if !pv.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", pv.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(hk)) = rx::get_heikin(&conn, &sym_upper) {
                    if hk.heikin_label != "INSUFFICIENT_DATA" && !hk.heikin_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Heikin-Ashi Candle — HEIKIN ({}, as of {})",
                            hk.heikin_label, hk.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · HA_O {:.4} · HA_H {:.4} · HA_L {:.4} · HA_C {:.4} · body {:.4} · wicks [u {:.4} / l {:.4}] · run {}",
                            hk.bars_used,
                            hk.ha_open,
                            hk.ha_high,
                            hk.ha_low,
                            hk.ha_close,
                            hk.body_abs,
                            hk.upper_wick,
                            hk.lower_wick,
                            hk.consecutive_same_color
                        );
                        if !hk.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", hk.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── research emitters ──
                if let Ok(Some(al)) = rx::get_alma(&conn, &sym_upper) {
                    if al.alma_label != "INSUFFICIENT_DATA" && !al.alma_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Arnaud Legoux MA — ALMA ({}, as of {})",
                            al.alma_label, al.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · offset {:.2} · sigma {:.1} · ALMA {:.4} · close {:.4} · dev {:+.2}%",
                            al.bars_used,
                            al.length,
                            al.offset,
                            al.sigma,
                            al.alma_value,
                            al.last_close,
                            al.deviation_pct
                        );
                        if !al.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", al.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(zl)) = rx::get_zlema(&conn, &sym_upper) {
                    if zl.zlema_label != "INSUFFICIENT_DATA" && !zl.zlema_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Zero-Lag EMA — ZLEMA ({}, as of {})",
                            zl.zlema_label, zl.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · lag {} · ZLEMA {:.4} · close {:.4} · dev {:+.2}%",
                            zl.bars_used,
                            zl.length,
                            zl.lag_shift,
                            zl.zlema_value,
                            zl.last_close,
                            zl.deviation_pct
                        );
                        if !zl.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", zl.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(er)) = rx::get_elderray(&conn, &sym_upper) {
                    if er.elder_label != "INSUFFICIENT_DATA" && !er.elder_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Elder Ray Bull/Bear Power — ELDERRAY ({}, as of {})",
                            er.elder_label, er.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · EMA{} {:.4} · Bull {:+.4} (prev {:+.4}) · Bear {:+.4} (prev {:+.4}) · close {:.4}",
                            er.bars_used,
                            er.ema_length,
                            er.ema13,
                            er.bull_power,
                            er.bull_power_prev,
                            er.bear_power,
                            er.bear_power_prev,
                            er.last_close
                        );
                        if !er.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", er.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ts)) = rx::get_tsf(&conn, &sym_upper) {
                    if ts.tsf_label != "INSUFFICIENT_DATA" && !ts.tsf_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Time Series Forecast — TSF ({}, as of {})",
                            ts.tsf_label, ts.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · slope {:+.5} · intercept {:.4} · forecast(t+1) {:.4} · close {:.4} · dev {:+.2}% · R² {:.3}",
                            ts.bars_used,
                            ts.length,
                            ts.slope,
                            ts.intercept,
                            ts.forecast_value,
                            ts.last_close,
                            ts.forecast_deviation_pct,
                            ts.r_squared
                        );
                        if !ts.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ts.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(rv)) = rx::get_rvi(&conn, &sym_upper) {
                    if rv.rvi_label != "INSUFFICIENT_DATA" && !rv.rvi_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Relative Vigor Index — RVI ({}, as of {})",
                            rv.rvi_label, rv.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · RVI {:+.4} (prev {:+.4}) · signal {:+.4} (prev {:+.4}) · close {:.4}",
                            rv.bars_used,
                            rv.length,
                            rv.rvi_value,
                            rv.rvi_prev,
                            rv.signal_value,
                            rv.signal_prev,
                            rv.last_close
                        );
                        if !rv.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", rv.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(tm)) = rx::get_trima(&conn, &sym_upper) {
                    if tm.trima_label != "INSUFFICIENT_DATA" && !tm.trima_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Triangular MA — TRIMA ({}, as of {})",
                            tm.trima_label, tm.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · TRIMA {:.4} (prev {:.4}) · deviation {:+.2}% · close {:.4}",
                            tm.bars_used,
                            tm.length,
                            tm.trima_value,
                            tm.trima_prev,
                            tm.deviation_pct,
                            tm.last_close
                        );
                        if !tm.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", tm.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(t3)) = rx::get_t3(&conn, &sym_upper) {
                    if t3.t3_label != "INSUFFICIENT_DATA" && !t3.t3_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Tillson T3 — T3 ({}, as of {})",
                            t3.t3_label, t3.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · v {:.2} · T3 {:.4} (prev {:.4}) · deviation {:+.2}% · close {:.4}",
                            t3.bars_used,
                            t3.length,
                            t3.v_factor,
                            t3.t3_value,
                            t3.t3_prev,
                            t3.deviation_pct,
                            t3.last_close
                        );
                        if !t3.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", t3.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(vd)) = rx::get_vidya(&conn, &sym_upper) {
                    if vd.vidya_label != "INSUFFICIENT_DATA" && !vd.vidya_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Chande VIDYA — VIDYA ({}, as of {})",
                            vd.vidya_label, vd.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · CMO length {} · VIDYA {:.4} (prev {:.4}) · α {:.4} · |CMO| {:.2} · deviation {:+.2}% · close {:.4}",
                            vd.bars_used,
                            vd.length,
                            vd.cmo_length,
                            vd.vidya_value,
                            vd.vidya_prev,
                            vd.current_alpha,
                            vd.cmo_magnitude,
                            vd.deviation_pct,
                            vd.last_close
                        );
                        if !vd.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", vd.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(sm)) = rx::get_smi(&conn, &sym_upper) {
                    if sm.smi_label != "INSUFFICIENT_DATA" && !sm.smi_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Stochastic Momentum Index — SMI ({}, as of {})",
                            sm.smi_label, sm.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · smooth {} · signal {} · SMI {:+.2} (prev {:+.2}) · signal {:+.2} (prev {:+.2}) · close {:.4}",
                            sm.bars_used,
                            sm.length,
                            sm.smooth_length,
                            sm.signal_length,
                            sm.smi_value,
                            sm.smi_prev,
                            sm.signal_value,
                            sm.signal_prev,
                            sm.last_close
                        );
                        if !sm.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", sm.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(pv)) = rx::get_pvt(&conn, &sym_upper) {
                    if pv.pvt_label != "INSUFFICIENT_DATA" && !pv.pvt_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Price Volume Trend — PVT ({}, as of {})",
                            pv.pvt_label, pv.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · PVT {:.2} (prev {:.2}) · PVT EMA20 {:.2} · 20-bar slope {:+.2} · close {:.4}",
                            pv.bars_used,
                            pv.pvt_value,
                            pv.pvt_prev,
                            pv.pvt_ema,
                            pv.pvt_slope,
                            pv.last_close
                        );
                        if !pv.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", pv.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ac)) = rx::get_ac(&conn, &sym_upper) {
                    if ac.ac_label != "INSUFFICIENT_DATA" && !ac.ac_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Accelerator Oscillator — AC ({}, as of {})",
                            ac.ac_label, ac.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · AC {:+.4} (prev {:+.4}) · AO {:+.4} · AO SMA5 {:+.4} · close {:.4}",
                            ac.bars_used,
                            ac.ac_value,
                            ac.ac_prev,
                            ac.ao_value,
                            ac.ao_sma5,
                            ac.last_close
                        );
                        if !ac.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ac.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(cv)) = rx::get_chvol(&conn, &sym_upper) {
                    if cv.chvol_label != "INSUFFICIENT_DATA" && !cv.chvol_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Chaikin Volatility — CHVOL ({}, as of {})",
                            cv.chvol_label, cv.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · EMA length {} · ROC length {} · CHVOL {:+.2}% (prev {:+.2}%) · EMA(H−L) {:.4} · close {:.4}",
                            cv.bars_used,
                            cv.ema_length,
                            cv.roc_length,
                            cv.chvol_value,
                            cv.chvol_prev,
                            cv.ema_range,
                            cv.last_close
                        );
                        if !cv.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", cv.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(bw)) = rx::get_bbwidth(&conn, &sym_upper) {
                    if bw.bbw_label != "INSUFFICIENT_DATA" && !bw.bbw_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Bollinger Bandwidth — BBWIDTH ({}, as of {})",
                            bw.bbw_label, bw.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · ±{:.1}σ · BBW {:.4} (prev {:.4}) · 125-bar pct {:.1} · upper {:.4} · mid {:.4} · lower {:.4} · close {:.4}",
                            bw.bars_used,
                            bw.length,
                            bw.num_stdev,
                            bw.bbw_value,
                            bw.bbw_prev,
                            bw.bbw_percentile,
                            bw.upper,
                            bw.middle,
                            bw.lower,
                            bw.last_close
                        );
                        if !bw.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", bw.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ei)) = rx::get_elderimp(&conn, &sym_upper) {
                    if ei.impulse_label != "INSUFFICIENT_DATA" && !ei.impulse_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Elder Impulse System — ELDERIMP ({}, as of {})",
                            ei.impulse_label, ei.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · EMA length {} · EMA {:.4} (slope {:+.4}) · MACD hist {:+.4} (prev {:+.4}, slope {:+.4}) · close {:.4}",
                            ei.bars_used,
                            ei.ema_length,
                            ei.ema_value,
                            ei.ema_slope,
                            ei.macd_hist,
                            ei.macd_hist_prev,
                            ei.macd_hist_slope,
                            ei.last_close
                        );
                        if !ei.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ei.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(rm)) = rx::get_rmi(&conn, &sym_upper) {
                    if rm.rmi_label != "INSUFFICIENT_DATA" && !rm.rmi_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Relative Momentum Index — RMI ({}, as of {})",
                            rm.rmi_label, rm.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · momentum {} · RMI {:.2} (prev {:.2}) · close {:.4}",
                            rm.bars_used,
                            rm.length,
                            rm.momentum_length,
                            rm.rmi_value,
                            rm.rmi_prev,
                            rm.last_close
                        );
                        if !rm.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", rm.note);
                        }
                        let _ = writeln!(p);
                    }
                }
            }
        }
    }
}
