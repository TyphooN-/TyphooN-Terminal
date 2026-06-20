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

                // ── Research section ──
                if let Ok(Some(ap)) = rx::get_avgprice(&conn, &sym_upper) {
                    if ap.avgprice_label != "INSUFFICIENT_DATA" && !ap.avgprice_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### OHLC Average — AVGPRICE ({}, as of {})",
                            ap.avgprice_label, ap.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · avgprice {:.4} (prev {:.4}) · O {:.4} · H {:.4} · L {:.4} · C {:.4} · Δ {:+.3}%",
                            ap.bars_used,
                            ap.avgprice,
                            ap.avgprice_prev,
                            ap.open,
                            ap.high,
                            ap.low,
                            ap.close,
                            ap.delta_pct
                        );
                        if !ap.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ap.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(mp)) = rx::get_medprice(&conn, &sym_upper) {
                    if mp.medprice_label != "INSUFFICIENT_DATA" && !mp.medprice_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Range Median — MEDPRICE ({}, as of {})",
                            mp.medprice_label, mp.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · medprice {:.4} (prev {:.4}) · H {:.4} · L {:.4} · C {:.4} · Δ {:+.3}%",
                            mp.bars_used,
                            mp.medprice,
                            mp.medprice_prev,
                            mp.high,
                            mp.low,
                            mp.close,
                            mp.delta_pct
                        );
                        if !mp.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", mp.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(tp)) = rx::get_typprice(&conn, &sym_upper) {
                    if tp.typprice_label != "INSUFFICIENT_DATA" && !tp.typprice_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Typical Price — TYPPRICE ({}, as of {})",
                            tp.typprice_label, tp.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · typprice {:.4} (prev {:.4}) · H {:.4} · L {:.4} · C {:.4} · Δ {:+.3}%",
                            tp.bars_used,
                            tp.typprice,
                            tp.typprice_prev,
                            tp.high,
                            tp.low,
                            tp.close,
                            tp.delta_pct
                        );
                        if !tp.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", tp.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(wp)) = rx::get_wclprice(&conn, &sym_upper) {
                    if wp.wclprice_label != "INSUFFICIENT_DATA" && !wp.wclprice_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Weighted Close — WCLPRICE ({}, as of {})",
                            wp.wclprice_label, wp.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · wclprice {:.4} (prev {:.4}) · H {:.4} · L {:.4} · C {:.4} · Δ {:+.3}%",
                            wp.bars_used,
                            wp.wclprice,
                            wp.wclprice_prev,
                            wp.high,
                            wp.low,
                            wp.close,
                            wp.delta_pct
                        );
                        if !wp.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", wp.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(vr)) = rx::get_variance(&conn, &sym_upper) {
                    if vr.variance_label != "INSUFFICIENT_DATA" && !vr.variance_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Close Variance — VARIANCE ({}, as of {})",
                            vr.variance_label, vr.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · mean {:.4} · variance {:.6} (prev {:.6}) · stddev {:.4} · CV {:.3}% · close {:.4}",
                            vr.bars_used,
                            vr.period,
                            vr.mean,
                            vr.variance,
                            vr.variance_prev,
                            vr.stddev,
                            vr.cv,
                            vr.last_close
                        );
                        if !vr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", vr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── (DMI family) ──
                if let Ok(Some(pd)) = rx::get_plus_di(&conn, &sym_upper) {
                    if pd.plus_di_label != "INSUFFICIENT_DATA" && !pd.plus_di_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Positive Directional Indicator — PLUS_DI ({}, as of {})",
                            pd.plus_di_label, pd.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · +DI {:.3} (prev {:.3}) · -DI {:.3} · ATR {:.4} · close {:.4}",
                            pd.bars_used,
                            pd.period,
                            pd.plus_di,
                            pd.plus_di_prev,
                            pd.minus_di,
                            pd.atr,
                            pd.last_close
                        );
                        if !pd.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", pd.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(md)) = rx::get_minus_di(&conn, &sym_upper) {
                    if md.minus_di_label != "INSUFFICIENT_DATA" && !md.minus_di_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Negative Directional Indicator — MINUS_DI ({}, as of {})",
                            md.minus_di_label, md.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · -DI {:.3} (prev {:.3}) · +DI {:.3} · ATR {:.4} · close {:.4}",
                            md.bars_used,
                            md.period,
                            md.minus_di,
                            md.minus_di_prev,
                            md.plus_di,
                            md.atr,
                            md.last_close
                        );
                        if !md.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", md.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(pm)) = rx::get_plus_dm(&conn, &sym_upper) {
                    if pm.plus_dm_label != "INSUFFICIENT_DATA" && !pm.plus_dm_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Positive Directional Movement — PLUS_DM ({}, as of {})",
                            pm.plus_dm_label, pm.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · +DM raw {:.4} · +DM smoothed {:.4} (prev {:.4}) · up {:+.4} · dn {:+.4} · close {:.4}",
                            pm.bars_used,
                            pm.period,
                            pm.plus_dm_raw,
                            pm.plus_dm_smoothed,
                            pm.plus_dm_smoothed_prev,
                            pm.up_move,
                            pm.down_move,
                            pm.last_close
                        );
                        if !pm.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", pm.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(mm)) = rx::get_minus_dm(&conn, &sym_upper) {
                    if mm.minus_dm_label != "INSUFFICIENT_DATA" && !mm.minus_dm_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Negative Directional Movement — MINUS_DM ({}, as of {})",
                            mm.minus_dm_label, mm.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · -DM raw {:.4} · -DM smoothed {:.4} (prev {:.4}) · up {:+.4} · dn {:+.4} · close {:.4}",
                            mm.bars_used,
                            mm.period,
                            mm.minus_dm_raw,
                            mm.minus_dm_smoothed,
                            mm.minus_dm_smoothed_prev,
                            mm.up_move,
                            mm.down_move,
                            mm.last_close
                        );
                        if !mm.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", mm.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(dxr)) = rx::get_dx(&conn, &sym_upper) {
                    if dxr.dx_label != "INSUFFICIENT_DATA" && !dxr.dx_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Directional Movement Index — DX ({}, as of {})",
                            dxr.dx_label, dxr.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · period {} · DX {:.3} (prev {:.3}) · +DI {:.3} · -DI {:.3} · close {:.4}",
                            dxr.bars_used,
                            dxr.period,
                            dxr.dx,
                            dxr.dx_prev,
                            dxr.plus_di,
                            dxr.minus_di,
                            dxr.last_close
                        );
                        if !dxr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", dxr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

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
