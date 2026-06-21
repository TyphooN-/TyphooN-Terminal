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

                if let Ok(Some(pp)) = rx::get_ppo(&conn, &sym_upper) {
                    if pp.ppo_label != "INSUFFICIENT_DATA" && !pp.ppo_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Percentage Price Oscillator — PPO ({}, as of {})",
                            pp.ppo_label, pp.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · {}/{}/{} · EMA fast {:.4} · EMA slow {:.4} · PPO {:+.4} · signal {:+.4} · hist {:+.4} · close {:.4}",
                            pp.bars_used,
                            pp.fast_period,
                            pp.slow_period,
                            pp.signal_period,
                            pp.ema_fast,
                            pp.ema_slow,
                            pp.ppo_value,
                            pp.signal_value,
                            pp.histogram,
                            pp.last_close
                        );
                        if !pp.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", pp.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(dp)) = rx::get_dpo(&conn, &sym_upper) {
                    if dp.dpo_label != "INSUFFICIENT_DATA" && !dp.dpo_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Detrended Price Oscillator — DPO ({}, as of {})",
                            dp.dpo_label, dp.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · shift {} · SMA {:.4} · DPO {:+.4} ({:+.2}%) · close {:.4}",
                            dp.bars_used,
                            dp.period,
                            dp.shift,
                            dp.sma_value,
                            dp.dpo_value,
                            dp.dpo_pct,
                            dp.last_close
                        );
                        if !dp.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", dp.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(kt)) = rx::get_kst(&conn, &sym_upper) {
                    if kt.kst_label != "INSUFFICIENT_DATA" && !kt.kst_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Know Sure Thing — KST ({}, as of {})",
                            kt.kst_label, kt.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · RCMA1 {:+.4} · RCMA2 {:+.4} · RCMA3 {:+.4} · RCMA4 {:+.4} · KST {:+.4} · signal {:+.4} · hist {:+.4} · close {:.4}",
                            kt.bars_used,
                            kt.rcma1,
                            kt.rcma2,
                            kt.rcma3,
                            kt.rcma4,
                            kt.kst_value,
                            kt.signal_value,
                            kt.histogram,
                            kt.last_close
                        );
                        if !kt.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", kt.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(uo)) = rx::get_ultosc(&conn, &sym_upper) {
                    if uo.ultosc_label != "INSUFFICIENT_DATA" && !uo.ultosc_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Ultimate Oscillator — ULTOSC ({}, as of {})",
                            uo.ultosc_label, uo.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · {}/{}/{} · avg_s {:.4} · avg_m {:.4} · avg_l {:.4} · UO {:.2} · close {:.4}",
                            uo.bars_used,
                            uo.period_short,
                            uo.period_mid,
                            uo.period_long,
                            uo.avg_short,
                            uo.avg_mid,
                            uo.avg_long,
                            uo.ultosc_value,
                            uo.last_close
                        );
                        if !uo.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", uo.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(wr)) = rx::get_willr(&conn, &sym_upper) {
                    if wr.willr_label != "INSUFFICIENT_DATA" && !wr.willr_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Williams %R — WILLR ({}, as of {})",
                            wr.willr_label, wr.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · highest high {:.4} · lowest low {:.4} · %R {:.2} · close {:.4}",
                            wr.bars_used,
                            wr.period,
                            wr.highest_high,
                            wr.lowest_low,
                            wr.willr_value,
                            wr.last_close
                        );
                        if !wr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", wr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── Research section ──
                if let Ok(Some(ms)) = rx::get_mass(&conn, &sym_upper) {
                    if ms.mass_label != "INSUFFICIENT_DATA" && !ms.mass_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Mass Index — MASS ({}, as of {})",
                            ms.mass_label, ms.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · EMA {} · sum {} · single ratio {:.4} · Mass {:.4} · close {:.4}",
                            ms.bars_used,
                            ms.ema_period,
                            ms.sum_period,
                            ms.single_ratio,
                            ms.mass_value,
                            ms.last_close
                        );
                        if !ms.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ms.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(co)) = rx::get_chaikosc(&conn, &sym_upper) {
                    if co.chaikosc_label != "INSUFFICIENT_DATA" && !co.chaikosc_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Chaikin Oscillator — CHAIKOSC ({}, as of {})",
                            co.chaikosc_label, co.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · fast {} · slow {} · A/D {:.2} · EMA(3) A/D {:.2} · EMA(10) A/D {:.2} · osc {:+.2} · close {:.4}",
                            co.bars_used,
                            co.fast_period,
                            co.slow_period,
                            co.ad_last,
                            co.ema_fast_ad,
                            co.ema_slow_ad,
                            co.chaikosc_value,
                            co.last_close
                        );
                        if !co.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", co.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(kv)) = rx::get_klinger(&conn, &sym_upper) {
                    if kv.klinger_label != "INSUFFICIENT_DATA" && !kv.klinger_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Klinger Volume Oscillator — KLINGER ({}, as of {})",
                            kv.klinger_label, kv.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · {}/{}/{} · EMA fast VF {:.2} · EMA slow VF {:.2} · KVO {:+.2} · signal {:+.2} · hist {:+.2} · close {:.4}",
                            kv.bars_used,
                            kv.fast_period,
                            kv.slow_period,
                            kv.signal_period,
                            kv.ema_fast_vf,
                            kv.ema_slow_vf,
                            kv.kvo_value,
                            kv.signal_value,
                            kv.histogram,
                            kv.last_close
                        );
                        if !kv.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", kv.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(sr)) = rx::get_stochrsi(&conn, &sym_upper) {
                    if sr.stochrsi_label != "INSUFFICIENT_DATA" && !sr.stochrsi_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Stochastic RSI — STOCHRSI ({}, as of {})",
                            sr.stochrsi_label, sr.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · RSI {} · stoch {} · %K {} · %D {} · RSI {:.2} (min {:.2}, max {:.2}) · SR {:.4} · %K {:.2} · %D {:.2} · close {:.4}",
                            sr.bars_used,
                            sr.rsi_period,
                            sr.stoch_period,
                            sr.k_period,
                            sr.d_period,
                            sr.rsi_value,
                            sr.rsi_min,
                            sr.rsi_max,
                            sr.stoch_rsi_raw,
                            sr.k_value,
                            sr.d_value,
                            sr.last_close
                        );
                        if !sr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", sr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ao)) = rx::get_awesome(&conn, &sym_upper) {
                    if ao.awesome_label != "INSUFFICIENT_DATA" && !ao.awesome_label.is_empty() {
                        let color_arrow = if ao.ao_color_up { "▲" } else { "▼" };
                        let _ = writeln!(
                            p,
                            "### Awesome Oscillator — AWESOME ({}, as of {})",
                            ao.awesome_label, ao.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · fast {} · slow {} · SMA(5) {:.4} · SMA(34) {:.4} · AO {:+.4} {} · prev {:+.4} · close {:.4}",
                            ao.bars_used,
                            ao.fast_period,
                            ao.slow_period,
                            ao.sma_fast,
                            ao.sma_slow,
                            ao.ao_value,
                            color_arrow,
                            ao.ao_prev,
                            ao.last_close
                        );
                        if !ao.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ao.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── Research section ──
                if let Ok(Some(ef)) = rx::get_efi(&conn, &sym_upper) {
                    if ef.efi_label != "INSUFFICIENT_DATA" && !ef.efi_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Force Index — EFI ({}, as of {})",
                            ef.efi_label, ef.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · EMA {} · raw {:+.2} · EFI {:+.2} · prev {:+.2} · close {:.4}",
                            ef.bars_used,
                            ef.ema_period,
                            ef.raw_efi,
                            ef.efi_value,
                            ef.efi_prev,
                            ef.last_close
                        );
                        if !ef.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ef.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(em)) = rx::get_emv(&conn, &sym_upper) {
                    if em.emv_label != "INSUFFICIENT_DATA" && !em.emv_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Ease of Movement — EMV ({}, as of {})",
                            em.emv_label, em.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · SMA {} · scale {:.0} · raw {:+.4} · EMV {:+.4} · close {:.4}",
                            em.bars_used,
                            em.sma_period,
                            em.volume_scale,
                            em.raw_emv,
                            em.emv_value,
                            em.last_close
                        );
                        if !em.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", em.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(nv)) = rx::get_nvi(&conn, &sym_upper) {
                    if nv.nvi_label != "INSUFFICIENT_DATA" && !nv.nvi_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Negative Volume Index — NVI ({}, as of {})",
                            nv.nvi_label, nv.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · signal EMA {} · NVI {:.2} · signal {:.2} · close {:.4}",
                            nv.bars_used,
                            nv.signal_period,
                            nv.nvi_value,
                            nv.signal_value,
                            nv.last_close
                        );
                        if !nv.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", nv.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(pv)) = rx::get_pvi(&conn, &sym_upper) {
                    if pv.pvi_label != "INSUFFICIENT_DATA" && !pv.pvi_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Positive Volume Index — PVI ({}, as of {})",
                            pv.pvi_label, pv.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · signal EMA {} · PVI {:.2} · signal {:.2} · close {:.4}",
                            pv.bars_used,
                            pv.signal_period,
                            pv.pvi_value,
                            pv.signal_value,
                            pv.last_close
                        );
                        if !pv.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", pv.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(cp)) = rx::get_coppock(&conn, &sym_upper) {
                    if cp.coppock_label != "INSUFFICIENT_DATA" && !cp.coppock_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Coppock Curve — COPPOCK ({}, as of {})",
                            cp.coppock_label, cp.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · WMA {} · ROC fast {} · ROC slow {} · Coppock {:+.4} · prev {:+.4} · close {:.4}",
                            cp.bars_used,
                            cp.wma_period,
                            cp.roc_fast,
                            cp.roc_slow,
                            cp.coppock_value,
                            cp.coppock_prev,
                            cp.last_close
                        );
                        if !cp.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", cp.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(cm)) = rx::get_cmo(&conn, &sym_upper) {
                    if cm.cmo_label != "INSUFFICIENT_DATA" && !cm.cmo_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Chande Momentum Oscillator — CMO ({}, as of {})",
                            cm.cmo_label, cm.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · Σ up {:.4} · Σ dn {:.4} · CMO {:+.2} · close {:.4}",
                            cm.bars_used,
                            cm.period,
                            cm.sum_up,
                            cm.sum_dn,
                            cm.cmo_value,
                            cm.last_close
                        );
                        if !cm.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", cm.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(qs)) = rx::get_qstick(&conn, &sym_upper) {
                    if qs.qstick_label != "INSUFFICIENT_DATA" && !qs.qstick_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Q-Stick — QSTICK ({}, as of {})",
                            qs.qstick_label, qs.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · Q-Stick {:+.4} · prev {:+.4} · close {:.4}",
                            qs.bars_used, qs.period, qs.qstick_value, qs.qstick_prev, qs.last_close
                        );
                        if !qs.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", qs.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ds)) = rx::get_disparity(&conn, &sym_upper) {
                    if ds.disparity_label != "INSUFFICIENT_DATA" && !ds.disparity_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Disparity Index — DISPARITY ({}, as of {})",
                            ds.disparity_label, ds.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · SMA {:.4} · disparity {:+.2}% · close {:.4}",
                            ds.bars_used,
                            ds.period,
                            ds.sma_value,
                            ds.disparity_value,
                            ds.last_close
                        );
                        if !ds.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ds.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(bp)) = rx::get_bop(&conn, &sym_upper) {
                    if bp.bop_label != "INSUFFICIENT_DATA" && !bp.bop_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Balance of Power — BOP ({}, as of {})",
                            bp.bop_label, bp.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · BOP {:+.3} · raw {:+.3} · close {:.4}",
                            bp.bars_used, bp.period, bp.bop_value, bp.raw_bop, bp.last_close
                        );
                        if !bp.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", bp.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(st)) = rx::get_schaff(&conn, &sym_upper) {
                    if st.schaff_label != "INSUFFICIENT_DATA" && !st.schaff_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Schaff Trend Cycle — SCHAFF ({}, as of {})",
                            st.schaff_label, st.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · EMA fast {} · EMA slow {} · cycle {} · STC {:.2} · prev {:.2} · close {:.4}",
                            st.bars_used,
                            st.ema_fast,
                            st.ema_slow,
                            st.cycle,
                            st.stc_value,
                            st.stc_prev,
                            st.last_close
                        );
                        if !st.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", st.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(so)) = rx::get_stoch(&conn, &sym_upper) {
                    if so.stoch_label != "INSUFFICIENT_DATA" && !so.stoch_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Stochastic Oscillator — STOCH ({}, as of {})",
                            so.stoch_label, so.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · %K period {} · %D period {} · smoothing {} · %K {:.2} · %D {:.2} · close {:.4}",
                            so.bars_used,
                            so.k_period,
                            so.d_period,
                            so.smoothing,
                            so.percent_k,
                            so.percent_d,
                            so.last_close
                        );
                        if !so.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", so.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(mc)) = rx::get_macd(&conn, &sym_upper) {
                    if mc.macd_label != "INSUFFICIENT_DATA" && !mc.macd_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### MACD — Appel ({}, as of {})",
                            mc.macd_label, mc.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · fast {} · slow {} · signal {} · MACD {:+.4} · signal {:+.4} · hist {:+.4} · prev hist {:+.4} · close {:.4}",
                            mc.bars_used,
                            mc.fast_period,
                            mc.slow_period,
                            mc.signal_period,
                            mc.macd_value,
                            mc.signal_value,
                            mc.histogram,
                            mc.histogram_prev,
                            mc.last_close
                        );
                        if !mc.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", mc.note);
                        }
                        let _ = writeln!(p);
                    }
                }

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
