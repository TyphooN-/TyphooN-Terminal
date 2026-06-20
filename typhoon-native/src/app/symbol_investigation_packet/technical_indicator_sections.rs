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
                if let Ok(Some(ik)) = rx::get_ichimoku(&conn, &sym_upper) {
                    if ik.ichimoku_label != "INSUFFICIENT_DATA" && !ik.ichimoku_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Ichimoku Cloud — ICHIMOKU ({}, as of {})",
                            ik.ichimoku_label, ik.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · tenkan={:.4} · kijun={:.4} · senkou A={:.4} · senkou B={:.4} · chikou={:.4}",
                            ik.bars_used,
                            ik.tenkan_sen,
                            ik.kijun_sen,
                            ik.senkou_span_a,
                            ik.senkou_span_b,
                            ik.chikou_span
                        );
                        let _ = writeln!(
                            p,
                            "- Cloud top={:.4} · bottom={:.4} · close={:.4} · close vs cloud={:+.2}%",
                            ik.cloud_top, ik.cloud_bottom, ik.last_close, ik.close_vs_cloud_pct
                        );
                        if !ik.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ik.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(st)) = rx::get_supertrend(&conn, &sym_upper) {
                    if st.supertrend_label != "INSUFFICIENT_DATA" && !st.supertrend_label.is_empty()
                    {
                        let _ = writeln!(
                            p,
                            "### Supertrend ATR Stop — SUPERTREND ({}, as of {})",
                            st.supertrend_label, st.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · mult {:.1} · ATR={:.4} · upper={:.4} · lower={:.4}",
                            st.bars_used,
                            st.period,
                            st.multiplier,
                            st.atr,
                            st.upper_band,
                            st.lower_band
                        );
                        let _ = writeln!(
                            p,
                            "- Active ST={:.4} · trend={} · close={:.4} · dist={:+.2}% · bars in trend={}",
                            st.supertrend_value,
                            if st.trend_is_up { "UP" } else { "DOWN" },
                            st.last_close,
                            st.distance_pct,
                            st.bars_in_trend
                        );
                        if !st.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", st.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(kc)) = rx::get_keltner(&conn, &sym_upper) {
                    if kc.keltner_label != "INSUFFICIENT_DATA" && !kc.keltner_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Keltner Channels — KELTNER ({}, as of {})",
                            kc.keltner_label, kc.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · EMA{} / ATR{} · mult {:.1} · EMA={:.4} · ATR={:.4}",
                            kc.bars_used,
                            kc.ema_period,
                            kc.atr_period,
                            kc.multiplier,
                            kc.ema_value,
                            kc.atr
                        );
                        let _ = writeln!(
                            p,
                            "- Upper={:.4} · lower={:.4} · width={:.4} · width %={:.2} · close={:.4} · pos={:.1}% · TTM squeeze={}",
                            kc.upper_channel,
                            kc.lower_channel,
                            kc.channel_width,
                            kc.width_pct_of_mid,
                            kc.last_close,
                            kc.channel_position_pct,
                            kc.ttm_squeeze_on
                        );
                        if !kc.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", kc.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(fs)) = rx::get_fisher(&conn, &sym_upper) {
                    if fs.fisher_label != "INSUFFICIENT_DATA" && !fs.fisher_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Fisher Transform — FISHER ({}, as of {})",
                            fs.fisher_label, fs.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · fisher={:+.3} · signal={:+.3} · peak |f| 10={:.3} · ±2 cross last 3={} · close={:.4}",
                            fs.bars_used,
                            fs.period,
                            fs.fisher_value,
                            fs.fisher_signal,
                            fs.peak_abs_10,
                            fs.extreme_2_cross,
                            fs.last_close
                        );
                        if !fs.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", fs.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ar)) = rx::get_aroon(&conn, &sym_upper) {
                    if ar.aroon_label != "INSUFFICIENT_DATA" && !ar.aroon_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Aroon — AROON ({}, as of {})",
                            ar.aroon_label, ar.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · up={:.2} · down={:.2} · osc={:+.2} · bars since high={} · bars since low={} · close={:.4}",
                            ar.bars_used,
                            ar.period,
                            ar.aroon_up,
                            ar.aroon_down,
                            ar.aroon_oscillator,
                            ar.bars_since_high,
                            ar.bars_since_low,
                            ar.last_close
                        );
                        if !ar.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ar.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── Research section ──
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
