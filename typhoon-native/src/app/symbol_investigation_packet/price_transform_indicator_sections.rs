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

                // ── Round 60: WMA / RAINBOW / MESA_SINE / FRAMA / IBS ──
                if let Ok(Some(wm)) = rx::get_wma(&conn, &sym_upper) {
                    if wm.wma_label != "INSUFFICIENT_DATA" && !wm.wma_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Weighted Moving Average — WMA ({}, as of {})",
                            wm.wma_label, wm.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · WMA {:.4} (prev {:.4}) · SMA {:.4} · spread {:+.4} ({:+.3}%) · close {:.4}",
                            wm.bars_used,
                            wm.length,
                            wm.wma_value,
                            wm.wma_prev,
                            wm.sma_value,
                            wm.spread,
                            wm.spread_pct * 100.0,
                            wm.last_close
                        );
                        if !wm.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", wm.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(rb)) = rx::get_rainbow(&conn, &sym_upper) {
                    if rb.rainbow_label != "INSUFFICIENT_DATA" && !rb.rainbow_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Rainbow MA Oscillator — RAINBOW ({}, as of {})",
                            rb.rainbow_label, rb.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · levels {} · highest {:.4} · lowest {:.4} · width {:.4} ({:.3}%) · center {:.4} · r1 {:.4} · r5 {:.4} · r10 {:.4} · close {:.4}",
                            rb.bars_used,
                            rb.levels,
                            rb.highest_level,
                            rb.lowest_level,
                            rb.rainbow_width,
                            rb.rainbow_width_pct * 100.0,
                            rb.center_value,
                            rb.r1,
                            rb.r5,
                            rb.r10,
                            rb.last_close
                        );
                        if !rb.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", rb.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ms)) = rx::get_mesa_sine(&conn, &sym_upper) {
                    if ms.mesa_label != "INSUFFICIENT_DATA" && !ms.mesa_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### MESA Sine Wave — MESA_SINE ({}, as of {})",
                            ms.mesa_label, ms.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {:.2} · phase {:+.4} rad · sine {:+.4} (prev {:+.4}) · lead_sine {:+.4} (prev {:+.4}) · close {:.4}",
                            ms.bars_used,
                            ms.period,
                            ms.phase_rad,
                            ms.sine_value,
                            ms.sine_prev,
                            ms.lead_sine,
                            ms.lead_prev,
                            ms.last_close
                        );
                        if !ms.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ms.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(fm)) = rx::get_frama(&conn, &sym_upper) {
                    if fm.frama_label != "INSUFFICIENT_DATA" && !fm.frama_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Fractal Adaptive Moving Average — FRAMA ({}, as of {})",
                            fm.frama_label, fm.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · D {:.4} · α {:.4} · FRAMA {:.4} (prev {:.4}) · spread {:+.4} · close {:.4}",
                            fm.bars_used,
                            fm.length,
                            fm.fractal_dim,
                            fm.alpha,
                            fm.frama_value,
                            fm.frama_prev,
                            fm.spread,
                            fm.last_close
                        );
                        if !fm.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", fm.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ib)) = rx::get_ibs(&conn, &sym_upper) {
                    if ib.ibs_label != "INSUFFICIENT_DATA" && !ib.ibs_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Internal Bar Strength — IBS ({}, as of {})",
                            ib.ibs_label, ib.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · IBS raw {:.4} · smoothed {:.4} (prev {:.4}) · bar H {:.4} L {:.4} C {:.4}",
                            ib.bars_used,
                            ib.length,
                            ib.ibs_raw,
                            ib.ibs_smoothed,
                            ib.ibs_prev,
                            ib.last_high,
                            ib.last_low,
                            ib.last_close
                        );
                        if !ib.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ib.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(lr)) = rx::get_laguerre_rsi(&conn, &sym_upper) {
                    if lr.lrsi_label != "INSUFFICIENT_DATA" && !lr.lrsi_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Laguerre RSI — LAGUERRE_RSI ({}, as of {})",
                            lr.lrsi_label, lr.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · γ {:.2} · L0 {:.6} L1 {:.6} L2 {:.6} L3 {:.6} · LRSI {:.4} (prev {:.4}) · close {:.4}",
                            lr.bars_used,
                            lr.gamma,
                            lr.l0,
                            lr.l1,
                            lr.l2,
                            lr.l3,
                            lr.laguerre_rsi,
                            lr.laguerre_rsi_prev,
                            lr.last_close
                        );
                        if !lr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", lr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(zz)) = rx::get_zigzag(&conn, &sym_upper) {
                    if zz.zigzag_label != "INSUFFICIENT_DATA" && !zz.zigzag_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### ZigZag Pattern — ZIGZAG ({}, as of {})",
                            zz.zigzag_label, zz.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · threshold {:.2}% · leg {} · last high {:.4} ({} bars ago) · last low {:.4} ({} bars ago) · reversal at {:.4} · close {:.4}",
                            zz.bars_used,
                            zz.threshold_pct,
                            zz.current_leg,
                            zz.last_high_value,
                            zz.last_high_bars_ago,
                            zz.last_low_value,
                            zz.last_low_bars_ago,
                            zz.reversal_level,
                            zz.last_close
                        );
                        if !zz.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", zz.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(pg)) = rx::get_pgo(&conn, &sym_upper) {
                    if pg.pgo_label != "INSUFFICIENT_DATA" && !pg.pgo_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Pretty Good Oscillator — PGO ({}, as of {})",
                            pg.pgo_label, pg.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · SMA {:.4} · ATR {:.4} · PGO {:.4} (prev {:.4}) · close {:.4}",
                            pg.bars_used,
                            pg.length,
                            pg.sma_value,
                            pg.atr_value,
                            pg.pgo_value,
                            pg.pgo_prev,
                            pg.last_close
                        );
                        if !pg.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", pg.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ht)) = rx::get_ht_trendline(&conn, &sym_upper) {
                    if ht.ht_label != "INSUFFICIENT_DATA" && !ht.ht_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Hilbert Instantaneous Trendline — HT_TRENDLINE ({}, as of {})",
                            ht.ht_label, ht.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · detected period {:.2} · trendline {:.4} (prev {:.4}) · spread {:.4} ({:+.3}%) · close {:.4}",
                            ht.bars_used,
                            ht.period,
                            ht.trendline_value,
                            ht.trendline_prev,
                            ht.spread,
                            ht.spread_pct * 100.0,
                            ht.last_close
                        );
                        if !ht.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ht.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(mp)) = rx::get_midpoint(&conn, &sym_upper) {
                    if mp.midpoint_label != "INSUFFICIENT_DATA" && !mp.midpoint_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Midpoint of N — MIDPOINT ({}, as of {})",
                            mp.midpoint_label, mp.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · HHV {:.4} · LLV {:.4} · midpoint {:.4} (prev {:.4}) · close position {:.4} · close {:.4}",
                            mp.bars_used,
                            mp.length,
                            mp.hhv,
                            mp.llv,
                            mp.midpoint,
                            mp.midpoint_prev,
                            mp.close_position,
                            mp.last_close
                        );
                        if !mp.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", mp.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── Round 62: MASSINDEX / NATR / TTM_SQUEEZE / FORCE_INDEX / TRANGE ──
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

                // ── Round 63 packet emitters ──
                if let Ok(Some(ls)) = rx::get_linearreg_slope(&conn, &sym_upper) {
                    if ls.slope_label != "INSUFFICIENT_DATA" && !ls.slope_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Linear Regression Slope — LINEARREG_SLOPE ({}, as of {})",
                            ls.slope_label, ls.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · slope {:+.6} (prev {:+.6}) · slope_pct {:+.3}% · close {:.4}",
                            ls.bars_used,
                            ls.length,
                            ls.slope,
                            ls.slope_prev,
                            ls.slope_pct,
                            ls.last_close
                        );
                        if !ls.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ls.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(dc)) = rx::get_ht_dcperiod(&conn, &sym_upper) {
                    if dc.period_label != "INSUFFICIENT_DATA" && !dc.period_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Hilbert Dominant Cycle Period — HT_DCPERIOD ({}, as of {})",
                            dc.period_label, dc.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {:.2} (prev {:.2}) · min(64) {:.2} · max(64) {:.2} · close {:.4}",
                            dc.bars_used,
                            dc.period,
                            dc.period_prev,
                            dc.period_min_64,
                            dc.period_max_64,
                            dc.last_close
                        );
                        if !dc.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", dc.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(tm)) = rx::get_ht_trendmode(&conn, &sym_upper) {
                    if tm.mode_label != "INSUFFICIENT_DATA" && !tm.mode_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Hilbert Trend vs Cycle Mode — HT_TRENDMODE ({}, as of {})",
                            tm.mode_label, tm.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · trendmode {} (prev {}) · lock_in_bars {} · period {:.2} · close {:.4}",
                            tm.bars_used,
                            tm.trendmode,
                            tm.trendmode_prev,
                            tm.lock_in_bars,
                            tm.period,
                            tm.last_close
                        );
                        if !tm.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", tm.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ab)) = rx::get_accbands(&conn, &sym_upper) {
                    if ab.accbands_label != "INSUFFICIENT_DATA" && !ab.accbands_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Acceleration Bands — ACCBANDS ({}, as of {})",
                            ab.accbands_label, ab.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · upper {:.4} · middle {:.4} · lower {:.4} · width {:.4} · pos {:.3} · close {:.4}",
                            ab.bars_used,
                            ab.length,
                            ab.acc_upper,
                            ab.acc_middle,
                            ab.acc_lower,
                            ab.width,
                            ab.position,
                            ab.last_close
                        );
                        if !ab.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ab.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(sf)) = rx::get_stochf(&conn, &sym_upper) {
                    if sf.stochf_label != "INSUFFICIENT_DATA" && !sf.stochf_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Fast Stochastic — STOCHF ({}, as of {})",
                            sf.stochf_label, sf.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · d_period {} · fastK {:.2} (prev {:.2}) · fastD {:.2} (prev {:.2}) · close {:.4}",
                            sf.bars_used,
                            sf.length,
                            sf.d_period,
                            sf.fastk,
                            sf.fastk_prev,
                            sf.fastd,
                            sf.fastd_prev,
                            sf.last_close
                        );
                        if !sf.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", sf.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── Round 64 packet emitters ──
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
