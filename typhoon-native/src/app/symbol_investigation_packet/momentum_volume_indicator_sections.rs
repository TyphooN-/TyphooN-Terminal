use super::*;

impl TyphooNApp {
    pub(super) fn write_symbol_momentum_volume_indicator_sections(
        &self,
        p: &mut String,
        sym_upper: &str,
    ) {
        use std::fmt::Write as _;
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use typhoon_engine::core::research as rx;

                // ── SMMA / ALLIGATOR / CRSI / SEB / IMI ──
                if let Ok(Some(sm)) = rx::get_smma(&conn, &sym_upper) {
                    if sm.smma_label != "INSUFFICIENT_DATA" && !sm.smma_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Wilder Smoothed MA — SMMA ({}, as of {})",
                            sm.smma_label, sm.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · SMMA {:.4} (prev {:.4}) · deviation {:+.2}% · close {:.4}",
                            sm.bars_used,
                            sm.length,
                            sm.smma_value,
                            sm.smma_prev,
                            sm.deviation_pct,
                            sm.last_close
                        );
                        if !sm.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", sm.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(al)) = rx::get_alligator(&conn, &sym_upper) {
                    if al.alligator_label != "INSUFFICIENT_DATA" && !al.alligator_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Bill Williams Alligator — ALLIGATOR ({}, as of {})",
                            al.alligator_label, al.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · jaw {:.4} (prev {:.4}) · teeth {:.4} (prev {:.4}) · lips {:.4} (prev {:.4}) · spread {:.2}% · close {:.4}",
                            al.bars_used,
                            al.jaw,
                            al.jaw_prev,
                            al.teeth,
                            al.teeth_prev,
                            al.lips,
                            al.lips_prev,
                            al.spread_pct,
                            al.last_close
                        );
                        if !al.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", al.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(cr)) = rx::get_crsi(&conn, &sym_upper) {
                    if cr.crsi_label != "INSUFFICIENT_DATA" && !cr.crsi_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Connors RSI — CRSI ({}, as of {})",
                            cr.crsi_label, cr.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · RSI₃ close {:.2} · RSI₂ streak {:.2} · pct-rank ROC {:.2} · CRSI {:.2} (prev {:.2}) · close {:.4}",
                            cr.bars_used,
                            cr.rsi_close,
                            cr.rsi_streak,
                            cr.percent_rank,
                            cr.crsi_value,
                            cr.crsi_prev,
                            cr.last_close
                        );
                        if !cr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", cr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(sb)) = rx::get_seb(&conn, &sym_upper) {
                    if sb.seb_label != "INSUFFICIENT_DATA" && !sb.seb_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Standard Error Bands — SEB ({}, as of {})",
                            sb.seb_label, sb.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · ±{:.1}·SE · upper {:.4} · mid {:.4} · lower {:.4} · bandwidth {:.4} · position {:.1}% · close {:.4}",
                            sb.bars_used,
                            sb.length,
                            sb.num_se,
                            sb.upper,
                            sb.middle,
                            sb.lower,
                            sb.bandwidth,
                            sb.position_pct,
                            sb.last_close
                        );
                        if !sb.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", sb.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(im)) = rx::get_imi(&conn, &sym_upper) {
                    if im.imi_label != "INSUFFICIENT_DATA" && !im.imi_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Intraday Momentum Index — IMI ({}, as of {})",
                            im.imi_label, im.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · ΣUp {:.4} · ΣDown {:.4} · IMI {:.2} (prev {:.2}) · close {:.4}",
                            im.bars_used,
                            im.length,
                            im.sum_gains,
                            im.sum_losses,
                            im.imi_value,
                            im.imi_prev,
                            im.last_close
                        );
                        if !im.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", im.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── GMMA / MAENV / ADL / VHF / VROC ──
                if let Ok(Some(gm)) = rx::get_gmma(&conn, &sym_upper) {
                    if gm.gmma_label != "INSUFFICIENT_DATA" && !gm.gmma_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Guppy Multiple MA — GMMA ({}, as of {})",
                            gm.gmma_label, gm.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · short-avg {:.4} (min {:.4} max {:.4} comp {:.2}%) · long-avg {:.4} (min {:.4} max {:.4} comp {:.2}%) · group-gap {:+.2}% · close {:.4}",
                            gm.bars_used,
                            gm.short_ema_avg,
                            gm.short_min,
                            gm.short_max,
                            gm.short_compression_pct,
                            gm.long_ema_avg,
                            gm.long_min,
                            gm.long_max,
                            gm.long_compression_pct,
                            gm.group_gap_pct,
                            gm.last_close
                        );
                        if !gm.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", gm.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(me)) = rx::get_maenv(&conn, &sym_upper) {
                    if me.maenv_label != "INSUFFICIENT_DATA" && !me.maenv_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Moving Average Envelope — MAENV ({}, as of {})",
                            me.maenv_label, me.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · ±{:.2}% · upper {:.4} · mid {:.4} · lower {:.4} · bandwidth {:.2}% · position {:.1}% · close {:.4}",
                            me.bars_used,
                            me.length,
                            me.pct_band,
                            me.upper,
                            me.middle,
                            me.lower,
                            me.bandwidth_pct,
                            me.position_pct,
                            me.last_close
                        );
                        if !me.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", me.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ad)) = rx::get_adl(&conn, &sym_upper) {
                    if ad.adl_label != "INSUFFICIENT_DATA" && !ad.adl_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Accumulation/Distribution Line — ADL ({}, as of {})",
                            ad.adl_label, ad.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · ADL {:.2} (prev {:.2}) · SMA{} {:.2} · slope/bar {:+.2} · price Δ {:+.2}% · close {:.4}",
                            ad.bars_used,
                            ad.adl_value,
                            ad.adl_prev,
                            ad.adl_sma_length,
                            ad.adl_sma,
                            ad.slope_per_bar,
                            ad.price_delta_pct,
                            ad.last_close
                        );
                        if !ad.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ad.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(vh)) = rx::get_vhf(&conn, &sym_upper) {
                    if vh.vhf_label != "INSUFFICIENT_DATA" && !vh.vhf_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Vertical Horizontal Filter — VHF ({}, as of {})",
                            vh.vhf_label, vh.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · HHV {:.4} · LLV {:.4} · Σ|Δc| {:.4} · VHF {:.4} (prev {:.4}) · close {:.4}",
                            vh.bars_used,
                            vh.length,
                            vh.highest_high,
                            vh.lowest_low,
                            vh.sum_abs_delta,
                            vh.vhf_value,
                            vh.vhf_prev,
                            vh.last_close
                        );
                        if !vh.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", vh.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(vr)) = rx::get_vroc(&conn, &sym_upper) {
                    if vr.vroc_label != "INSUFFICIENT_DATA" && !vr.vroc_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Volume Rate of Change — VROC ({}, as of {})",
                            vr.vroc_label, vr.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · V_now {:.0} · V_then {:.0} · VROC {:+.2}% (prev {:+.2}%) · close {:.4}",
                            vr.bars_used,
                            vr.length,
                            vr.volume_now,
                            vr.volume_then,
                            vr.vroc_value,
                            vr.vroc_prev,
                            vr.last_close
                        );
                        if !vr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", vr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── KDJ / QQE / PMO / CFO / TMF ──
                if let Ok(Some(kj)) = rx::get_kdj(&conn, &sym_upper) {
                    if kj.kdj_label != "INSUFFICIENT_DATA" && !kj.kdj_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### KDJ — Chinese Stochastic Variant ({}, as of {})",
                            kj.kdj_label, kj.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · stoch {} · smooth {} · RSV {:.2} · K {:.2} · D {:.2} · J {:.2} (prev {:.2}) · close {:.4}",
                            kj.bars_used,
                            kj.stoch_length,
                            kj.k_smooth,
                            kj.rsv,
                            kj.k_value,
                            kj.d_value,
                            kj.j_value,
                            kj.j_prev,
                            kj.last_close
                        );
                        if !kj.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", kj.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(qq)) = rx::get_qqe(&conn, &sym_upper) {
                    if qq.qqe_label != "INSUFFICIENT_DATA" && !qq.qqe_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Quantitative Qualitative Estimation — QQE ({}, as of {})",
                            qq.qqe_label, qq.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · RSI{} · smooth{} · factor {:.3} · RSI {:.2} · smoothed {:.2} (prev {:.2}) · ATR_RSI {:.3} · band [{:.2}, {:.2}] · close {:.4}",
                            qq.bars_used,
                            qq.rsi_length,
                            qq.smooth_length,
                            qq.qqe_factor,
                            qq.rsi_value,
                            qq.rsi_smoothed,
                            qq.qqe_prev,
                            qq.fast_atr_rsi_avg,
                            qq.lower_band,
                            qq.upper_band,
                            qq.last_close
                        );
                        if !qq.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", qq.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(pm)) = rx::get_pmo(&conn, &sym_upper) {
                    if pm.pmo_label != "INSUFFICIENT_DATA" && !pm.pmo_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Price Momentum Oscillator — PMO ({}, as of {})",
                            pm.pmo_label, pm.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · smooth1 {} · smooth2 {} · signal {} · PMO {:+.4} (prev {:+.4}) · signal {:+.4} · histogram {:+.4} · close {:.4}",
                            pm.bars_used,
                            pm.smooth1_length,
                            pm.smooth2_length,
                            pm.signal_length,
                            pm.pmo_value,
                            pm.pmo_prev,
                            pm.pmo_signal,
                            pm.histogram,
                            pm.last_close
                        );
                        if !pm.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", pm.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(cf)) = rx::get_cfo(&conn, &sym_upper) {
                    if cf.cfo_label != "INSUFFICIENT_DATA" && !cf.cfo_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Chande Forecast Oscillator — CFO ({}, as of {})",
                            cf.cfo_label, cf.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · slope {:+.6} · intercept {:.4} · forecast {:.4} · CFO {:+.2}% (prev {:+.2}%) · close {:.4}",
                            cf.bars_used,
                            cf.length,
                            cf.slope,
                            cf.intercept,
                            cf.forecast,
                            cf.cfo_value,
                            cf.cfo_prev,
                            cf.last_close
                        );
                        if !cf.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", cf.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(tm)) = rx::get_tmf(&conn, &sym_upper) {
                    if tm.tmf_label != "INSUFFICIENT_DATA" && !tm.tmf_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Twiggs Money Flow — TMF ({}, as of {})",
                            tm.tmf_label, tm.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · EMA money-flow {:.2} · EMA volume {:.2} · TMF {:+.4} (prev {:+.4}) · close {:.4}",
                            tm.bars_used,
                            tm.length,
                            tm.ema_money_flow,
                            tm.ema_volume,
                            tm.tmf_value,
                            tm.tmf_prev,
                            tm.last_close
                        );
                        if !tm.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", tm.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(fr)) = rx::get_fractals(&conn, &sym_upper) {
                    if fr.fractals_label != "INSUFFICIENT_DATA" && !fr.fractals_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Bill Williams Fractals — FRACTALS ({}, as of {})",
                            fr.fractals_label, fr.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · window {} · last up {:.4} ({} bars ago) · last down {:.4} ({} bars ago) · up/down count {}/{} · close {:.4}",
                            fr.bars_used,
                            fr.window,
                            fr.last_up_high,
                            fr.last_up_bars_ago,
                            fr.last_down_low,
                            fr.last_down_bars_ago,
                            fr.up_fractal_count,
                            fr.down_fractal_count,
                            fr.last_close
                        );
                        if !fr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", fr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ir)) = rx::get_ift_rsi(&conn, &sym_upper) {
                    if ir.ift_rsi_label != "INSUFFICIENT_DATA" && !ir.ift_rsi_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Inverse Fisher RSI — IFT_RSI ({}, as of {})",
                            ir.ift_rsi_label, ir.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · RSI length {} · WMA length {} · RSI {:.2} · v {:+.4} · IFT {:+.4} (prev {:+.4}) · close {:.4}",
                            ir.bars_used,
                            ir.rsi_length,
                            ir.wma_length,
                            ir.rsi_value,
                            ir.v_value,
                            ir.ift_value,
                            ir.ift_prev,
                            ir.last_close
                        );
                        if !ir.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ir.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ma)) = rx::get_mama(&conn, &sym_upper) {
                    if ma.mama_label != "INSUFFICIENT_DATA" && !ma.mama_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### MESA Adaptive MA — MAMA ({}, as of {})",
                            ma.mama_label, ma.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · fast_limit {:.2} · slow_limit {:.2} · MAMA {:.4} (prev {:.4}) · FAMA {:.4} (prev {:.4}) · α {:.4} · period {:.2} · close {:.4}",
                            ma.bars_used,
                            ma.fast_limit,
                            ma.slow_limit,
                            ma.mama_value,
                            ma.mama_prev,
                            ma.fama_value,
                            ma.fama_prev,
                            ma.alpha,
                            ma.period,
                            ma.last_close
                        );
                        if !ma.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ma.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(cg)) = rx::get_cog(&conn, &sym_upper) {
                    if cg.cog_label != "INSUFFICIENT_DATA" && !cg.cog_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Ehlers Center of Gravity — COG ({}, as of {})",
                            cg.cog_label, cg.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · COG {:+.4} (prev {:+.4}) · signal {:+.4} · close {:.4}",
                            cg.bars_used,
                            cg.length,
                            cg.cog_value,
                            cg.cog_prev,
                            cg.cog_signal,
                            cg.last_close
                        );
                        if !cg.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", cg.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(dd)) = rx::get_didi(&conn, &sym_upper) {
                    if dd.didi_label != "INSUFFICIENT_DATA" && !dd.didi_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Didi Index — DIDI ({}, as of {})",
                            dd.didi_label, dd.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · short/medium/long {}/{}/{} · short ratio {:+.4} (prev {:+.4}) · long ratio {:+.4} (prev {:+.4}) · close {:.4}",
                            dd.bars_used,
                            dd.short_length,
                            dd.medium_length,
                            dd.long_length,
                            dd.short_ratio,
                            dd.short_prev,
                            dd.long_ratio,
                            dd.long_prev,
                            dd.last_close
                        );
                        if !dd.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", dd.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── DEMARKER / GATOR / BW_MFI / VWMA / STDDEV ──
                if let Ok(Some(dm)) = rx::get_demarker(&conn, &sym_upper) {
                    if dm.demarker_label != "INSUFFICIENT_DATA" && !dm.demarker_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### DeMarker — DEMARKER ({}, as of {})",
                            dm.demarker_label, dm.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · ΣDeMax {:.4} · ΣDeMin {:.4} · DeM {:.4} (prev {:.4}) · close {:.4}",
                            dm.bars_used,
                            dm.length,
                            dm.demax_sum,
                            dm.demin_sum,
                            dm.demarker_value,
                            dm.demarker_prev,
                            dm.last_close
                        );
                        if !dm.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", dm.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(gt)) = rx::get_gator(&conn, &sym_upper) {
                    if gt.gator_label != "INSUFFICIENT_DATA" && !gt.gator_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Gator Oscillator — GATOR ({}, as of {})",
                            gt.gator_label, gt.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · jaw/teeth/lips {}/{}/{} · upper {:+.4} (prev {:+.4}) · lower {:+.4} (prev {:+.4}) · close {:.4}",
                            gt.bars_used,
                            gt.jaw_length,
                            gt.teeth_length,
                            gt.lips_length,
                            gt.upper_bar,
                            gt.upper_prev,
                            gt.lower_bar,
                            gt.lower_prev,
                            gt.last_close
                        );
                        if !gt.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", gt.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(bw)) = rx::get_bw_mfi(&conn, &sym_upper) {
                    if bw.bwmfi_label != "INSUFFICIENT_DATA" && !bw.bwmfi_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Bill Williams Market Facilitation Index — BW_MFI ({}, as of {})",
                            bw.bwmfi_label, bw.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · MFI {:.4} (prev {:.4}) · volume {:.0} (prev {:.0}) · color {} · close {:.4}",
                            bw.bars_used,
                            bw.mfi_value,
                            bw.mfi_prev,
                            bw.volume,
                            bw.volume_prev,
                            bw.bwmfi_color,
                            bw.last_close
                        );
                        if !bw.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", bw.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(vw)) = rx::get_vwma(&conn, &sym_upper) {
                    if vw.vwma_label != "INSUFFICIENT_DATA" && !vw.vwma_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Volume-Weighted Moving Average — VWMA ({}, as of {})",
                            vw.vwma_label, vw.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} · VWMA {:.4} (prev {:.4}) · SMA {:.4} · spread {:+.4} ({:+.3}%) · close {:.4}",
                            vw.bars_used,
                            vw.length,
                            vw.vwma_value,
                            vw.vwma_prev,
                            vw.sma_value,
                            vw.spread,
                            vw.spread_ratio * 100.0,
                            vw.last_close
                        );
                        if !vw.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", vw.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(sd)) = rx::get_stddev(&conn, &sym_upper) {
                    if sd.regime_label != "INSUFFICIENT_DATA" && !sd.regime_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Rolling Standard Deviation — STDDEV ({}, as of {})",
                            sd.regime_label, sd.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · length {} / long {} · mean {:.4} · σ {:.4} · σ_long {:.4} · cv {:.4} · annualized {:.4} · close {:.4}",
                            sd.bars_used,
                            sd.length,
                            sd.long_length,
                            sd.mean,
                            sd.stddev,
                            sd.stddev_long,
                            sd.cv,
                            sd.annualized,
                            sd.last_close
                        );
                        if !sd.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", sd.note);
                        }
                        let _ = writeln!(p);
                    }
                }
            }
        }
    }
}
