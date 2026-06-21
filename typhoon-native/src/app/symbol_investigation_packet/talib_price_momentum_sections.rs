use super::*;

impl TyphooNApp {
    pub(super) fn write_symbol_talib_price_momentum_sections(
        &self,
        p: &mut String,
        sym_upper: &str,
    ) {
        use std::fmt::Write as _;
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use typhoon_engine::core::research as rx;

                self.write_talib_price_ohlc_stats(p, sym_upper);

                self.write_talib_dmi_movement(p, sym_upper);

                // ── Research section ──
                if let Ok(Some(rc)) = rx::get_roc(&conn, &sym_upper) {
                    if rc.roc_label != "INSUFFICIENT_DATA" && !rc.roc_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Rate of Change — ROC ({}, as of {})",
                            rc.roc_label, rc.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · ROC {:+.4} (prev {:+.4}) · close {:.4} · lag {:.4}",
                            rc.bars_used,
                            rc.period,
                            rc.roc,
                            rc.roc_prev,
                            rc.close_now,
                            rc.close_lag
                        );
                        if !rc.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", rc.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(rcp)) = rx::get_rocp(&conn, &sym_upper) {
                    if rcp.rocp_label != "INSUFFICIENT_DATA" && !rcp.rocp_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Rate of Change Percentage — ROCP ({}, as of {})",
                            rcp.rocp_label, rcp.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · ROCP {:+.6} ({:+.4}%) · prev {:+.6} · close {:.4} · lag {:.4}",
                            rcp.bars_used,
                            rcp.period,
                            rcp.rocp,
                            rcp.rocp_pct,
                            rcp.rocp_prev,
                            rcp.close_now,
                            rcp.close_lag
                        );
                        if !rcp.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", rcp.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(rcr)) = rx::get_rocr(&conn, &sym_upper) {
                    if rcr.rocr_label != "INSUFFICIENT_DATA" && !rcr.rocr_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Rate of Change Ratio — ROCR ({}, as of {})",
                            rcr.rocr_label, rcr.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · ROCR {:.6} (prev {:.6}) · close {:.4} · lag {:.4}",
                            rcr.bars_used,
                            rcr.period,
                            rcr.rocr,
                            rcr.rocr_prev,
                            rcr.close_now,
                            rcr.close_lag
                        );
                        if !rcr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", rcr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(rc1)) = rx::get_rocr100(&conn, &sym_upper) {
                    if rc1.rocr100_label != "INSUFFICIENT_DATA" && !rc1.rocr100_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Rate of Change Ratio ×100 — ROCR100 ({}, as of {})",
                            rc1.rocr100_label, rc1.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · ROCR100 {:.4} (prev {:.4}) · close {:.4} · lag {:.4}",
                            rc1.bars_used,
                            rc1.period,
                            rc1.rocr100,
                            rc1.rocr100_prev,
                            rc1.close_now,
                            rc1.close_lag
                        );
                        if !rc1.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", rc1.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(cr)) = rx::get_correl(&conn, &sym_upper) {
                    if cr.correl_label != "INSUFFICIENT_DATA" && !cr.correl_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Lag-1 Autocorrelation — CORREL ({}, as of {})",
                            cr.correl_label, cr.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · ρ {:+.4} (prev {:+.4}) · mean(x) {:.4} · mean(y) {:.4} · σ(x) {:.4} · σ(y) {:.4} · close {:.4}",
                            cr.bars_used,
                            cr.period,
                            cr.correl,
                            cr.correl_prev,
                            cr.mean_x,
                            cr.mean_y,
                            cr.stddev_x,
                            cr.stddev_y,
                            cr.last_close
                        );
                        if !cr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", cr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(mn)) = rx::get_min(&conn, &sym_upper) {
                    if mn.min_label != "INSUFFICIENT_DATA" && !mn.min_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Rolling Minimum — MIN ({}, as of {})",
                            mn.min_label, mn.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · min {:.4} (prev {:.4}) · max_ref {:.4} · close {:.4} · pos {:.2}%",
                            mn.bars_used,
                            mn.period,
                            mn.min_val,
                            mn.min_prev,
                            mn.max_ref,
                            mn.last_close,
                            mn.position_pct
                        );
                        if !mn.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", mn.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(mx)) = rx::get_max(&conn, &sym_upper) {
                    if mx.max_label != "INSUFFICIENT_DATA" && !mx.max_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Rolling Maximum — MAX ({}, as of {})",
                            mx.max_label, mx.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · max {:.4} (prev {:.4}) · min_ref {:.4} · close {:.4} · pos {:.2}%",
                            mx.bars_used,
                            mx.period,
                            mx.max_val,
                            mx.max_prev,
                            mx.min_ref,
                            mx.last_close,
                            mx.position_pct
                        );
                        if !mx.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", mx.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(mm)) = rx::get_minmax(&conn, &sym_upper) {
                    if mm.minmax_label != "INSUFFICIENT_DATA" && !mm.minmax_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Rolling Range — MINMAX ({}, as of {})",
                            mm.minmax_label, mm.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · [{:.4}..{:.4}] · width {:.4} · width% {:.2}% · close {:.4} · pos {:.2}%",
                            mm.bars_used,
                            mm.period,
                            mm.min_val,
                            mm.max_val,
                            mm.range_width,
                            mm.range_pct,
                            mm.last_close,
                            mm.position_pct
                        );
                        if !mm.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", mm.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(mi)) = rx::get_minindex(&conn, &sym_upper) {
                    if mi.min_index_label != "INSUFFICIENT_DATA" && !mi.min_index_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Low Recency — MININDEX ({}, as of {})",
                            mi.min_index_label, mi.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · min {:.4} · {} bars ago (prev {} bars ago) · close {:.4}",
                            mi.bars_used,
                            mi.period,
                            mi.min_val,
                            mi.min_index_bars_ago,
                            mi.min_index_bars_ago_prev,
                            mi.last_close
                        );
                        if !mi.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", mi.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(mxi)) = rx::get_maxindex(&conn, &sym_upper) {
                    if mxi.max_index_label != "INSUFFICIENT_DATA" && !mxi.max_index_label.is_empty()
                    {
                        let _ = writeln!(
                            p,
                            "### High Recency — MAXINDEX ({}, as of {})",
                            mxi.max_index_label, mxi.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · max {:.4} · {} bars ago (prev {} bars ago) · close {:.4}",
                            mxi.bars_used,
                            mxi.period,
                            mxi.max_val,
                            mxi.max_index_bars_ago,
                            mxi.max_index_bars_ago_prev,
                            mxi.last_close
                        );
                        if !mxi.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", mxi.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(bb)) = rx::get_bbands(&conn, &sym_upper) {
                    if bb.bbands_label != "INSUFFICIENT_DATA" && !bb.bbands_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Bollinger Bands — BBANDS ({}, as of {})",
                            bb.bbands_label, bb.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · σ×{:.1} · upper {:.4} · mid {:.4} · lower {:.4} · close {:.4} · %B {:.2} · bw {:.2}%",
                            bb.bars_used,
                            bb.period,
                            bb.num_std,
                            bb.upper,
                            bb.middle,
                            bb.lower,
                            bb.last_close,
                            bb.pct_b,
                            bb.bandwidth
                        );
                        if !bb.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", bb.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ad)) = rx::get_ad(&conn, &sym_upper) {
                    if ad.ad_label != "INSUFFICIENT_DATA" && !ad.ad_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Chaikin A/D Line — AD ({}, as of {})",
                            ad.ad_label, ad.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · AD {:.4} (prev {:.4}, Δ {:+.4}) · slope10 {:+.6} · close {:.4}",
                            ad.bars_used,
                            ad.ad,
                            ad.ad_prev,
                            ad.ad_delta,
                            ad.ad_slope,
                            ad.last_close
                        );
                        if !ad.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ad.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ao)) = rx::get_adosc(&conn, &sym_upper) {
                    if ao.adosc_label != "INSUFFICIENT_DATA" && !ao.adosc_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Chaikin A/D Oscillator — ADOSC ({}, as of {})",
                            ao.adosc_label, ao.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · fast {} · slow {} · ADOSC {:+.4} (prev {:+.4}) · AD ref {:.4} · close {:.4}",
                            ao.bars_used,
                            ao.fast_period,
                            ao.slow_period,
                            ao.adosc,
                            ao.adosc_prev,
                            ao.ad_ref,
                            ao.last_close
                        );
                        if !ao.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ao.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(su)) = rx::get_sum(&conn, &sym_upper) {
                    if su.sum_label != "INSUFFICIENT_DATA" && !su.sum_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Rolling Sum — SUM ({}, as of {})",
                            su.sum_label, su.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · sum {:.4} (prev {:.4}, Δ {:+.4}, {:+.2}%) · close {:.4}",
                            su.bars_used,
                            su.period,
                            su.sum,
                            su.sum_prev,
                            su.sum_delta,
                            su.sum_pct_change,
                            su.last_close
                        );
                        if !su.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", su.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(li)) = rx::get_linreg_intercept(&conn, &sym_upper) {
                    if li.linreg_intercept_label != "INSUFFICIENT_DATA"
                        && !li.linreg_intercept_label.is_empty()
                    {
                        let _ = writeln!(
                            p,
                            "### Linear-Regression Intercept — LINEARREG_INTERCEPT ({}, as of {})",
                            li.linreg_intercept_label, li.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · b {:.6} (prev {:.6}) · m {:+.6} · close {:.4} · drift {:+.4} ({:+.2}%)",
                            li.bars_used,
                            li.period,
                            li.intercept,
                            li.intercept_prev,
                            li.slope,
                            li.last_close,
                            li.drift,
                            li.drift_pct
                        );
                        if !li.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", li.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── emitters ──
                if let Ok(Some(ao)) = rx::get_aroonosc(&conn, &sym_upper) {
                    if ao.aroonosc_label != "INSUFFICIENT_DATA" && !ao.aroonosc_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Aroon Oscillator — AROONOSC ({}, as of {})",
                            ao.aroonosc_label, ao.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · osc {:+.2} (prev {:+.2}) · up {:.2} · down {:.2} · close {:.4}",
                            ao.bars_used,
                            ao.period,
                            ao.aroonosc,
                            ao.aroonosc_prev,
                            ao.aroon_up,
                            ao.aroon_down,
                            ao.last_close
                        );
                        if !ao.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ao.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(mmi)) = rx::get_minmaxindex(&conn, &sym_upper) {
                    if mmi.minmaxindex_label != "INSUFFICIENT_DATA"
                        && !mmi.minmaxindex_label.is_empty()
                    {
                        let _ = writeln!(
                            p,
                            "### Min/Max Index — MINMAXINDEX ({}, as of {})",
                            mmi.minmaxindex_label, mmi.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · low {} ago · high {} ago · age_diff {:+} · order {} · close {:.4}",
                            mmi.bars_used,
                            mmi.period,
                            mmi.min_index_bars_ago,
                            mmi.max_index_bars_ago,
                            mmi.age_diff,
                            mmi.extrema_order,
                            mmi.last_close
                        );
                        if !mmi.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", mmi.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(me)) = rx::get_macdext(&conn, &sym_upper) {
                    if me.macdext_label != "INSUFFICIENT_DATA" && !me.macdext_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### MACD Extended — MACDEXT ({}, as of {})",
                            me.macdext_label, me.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · {}/{}/{} · ma_type {} · macd {:+.6} · signal {:+.6} · hist {:+.6} (prev {:+.6}) · close {:.4}",
                            me.bars_used,
                            me.fast_period,
                            me.slow_period,
                            me.signal_period,
                            me.ma_type,
                            me.macd,
                            me.signal,
                            me.hist,
                            me.hist_prev,
                            me.last_close
                        );
                        if !me.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", me.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(mf)) = rx::get_macdfix(&conn, &sym_upper) {
                    if mf.macdfix_label != "INSUFFICIENT_DATA" && !mf.macdfix_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### MACD Fix — MACDFIX ({}, as of {})",
                            mf.macdfix_label, mf.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · {}/{}/{} · macd {:+.6} · signal {:+.6} · hist {:+.6} (prev {:+.6}) · close {:.4}",
                            mf.bars_used,
                            mf.fast_period,
                            mf.slow_period,
                            mf.signal_period,
                            mf.macd,
                            mf.signal,
                            mf.hist,
                            mf.hist_prev,
                            mf.last_close
                        );
                        if !mf.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", mf.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(mv)) = rx::get_mavp(&conn, &sym_upper) {
                    if mv.mavp_label != "INSUFFICIENT_DATA" && !mv.mavp_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Moving Avg Variable Period — MAVP ({}, as of {})",
                            mv.mavp_label, mv.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · periods {}..{} · last_period {} · mavp {:.6} (prev {:.6}, Δ {:+.6}) · close {:.4}",
                            mv.bars_used,
                            mv.min_period,
                            mv.max_period,
                            mv.last_bar_period,
                            mv.mavp,
                            mv.mavp_prev,
                            mv.mavp_delta,
                            mv.last_close
                        );
                        if !mv.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", mv.note);
                        }
                        let _ = writeln!(p);
                    }
                }
            }
        }
    }
}
