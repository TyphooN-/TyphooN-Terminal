use super::*;

impl TyphooNApp {
    pub(super) fn write_symbol_price_behavior_sections(&self, p: &mut String, sym_upper: &str) {
        use std::fmt::Write as _;
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use typhoon_engine::core::research as rx;

                self.write_price_behavior_distribution(p, sym_upper);

                self.write_price_behavior_local(p, sym_upper);

                if let Ok(Some(sr)) = rx::get_sharpr(&conn, &sym_upper) {
                    if sr.sharpe_label != "INSUFFICIENT_DATA" && !sr.sharpe_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Sharpe Ratio (rf=0) — SHARPR ({}, as of {})",
                            sr.sharpe_label, sr.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · mean r {:.6} · stdev r {:.6}",
                            sr.bars_used, sr.mean_log_return, sr.stdev_log_return
                        );
                        let _ = writeln!(
                            p,
                            "- Sharpe {:.3} (ann {:.3}) · mean ann {:.4} · stdev ann {:.4}",
                            sr.sharpe_ratio,
                            sr.sharpe_ratio_ann,
                            sr.mean_return_ann,
                            sr.stdev_return_ann
                        );
                        if !sr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", sr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(er)) = rx::get_effratio(&conn, &sym_upper) {
                    if er.efficiency_label != "INSUFFICIENT_DATA" && !er.efficiency_label.is_empty()
                    {
                        let _ = writeln!(
                            p,
                            "### Kaufman Efficiency Ratio — EFFRATIO ({}, as of {})",
                            er.efficiency_label, er.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · start {:.4} · end {:.4} · net {:+.4} ({:+.2}%)",
                            er.bars_used,
                            er.start_close,
                            er.end_close,
                            er.net_change,
                            er.net_change_pct
                        );
                        let _ = writeln!(
                            p,
                            "- Σ |Δclose| {:.4} · ER {:.3} · signed {:+.3}",
                            er.sum_abs_changes, er.efficiency_ratio, er.signed_efficiency
                        );
                        if !er.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", er.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(wb)) = rx::get_wickbias(&conn, &sym_upper) {
                    if wb.bias_label != "INSUFFICIENT_DATA" && !wb.bias_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Wick Bias — WICKBIAS ({}, as of {})",
                            wb.bias_label, wb.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · avg upper {:.3} · avg lower {:.3} · body {:.3}",
                            wb.bars_used, wb.avg_upper_wick, wb.avg_lower_wick, wb.avg_body_share
                        );
                        let _ = writeln!(
                            p,
                            "- Median upper {:.3} · median lower {:.3} · bias score {:+.4}",
                            wb.median_upper_wick, wb.median_lower_wick, wb.wick_bias_score
                        );
                        if !wb.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", wb.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(vv)) = rx::get_volofvol(&conn, &sym_upper) {
                    if vv.cv_label != "INSUFFICIENT_DATA" && !vv.cv_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Vol-of-Vol (stdev of rolling 20d RV) — VOLOFVOL ({}, as of {})",
                            vv.cv_label, vv.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- RV points {} · mean RV20 {:.5} · stdev RV20 {:.5} · CV {:.3}",
                            vv.bars_used, vv.mean_rv20, vv.stdev_rv20, vv.cv_rv20
                        );
                        let _ = writeln!(
                            p,
                            "- Min RV20 {:.5} · max {:.5} · latest {:.5}",
                            vv.min_rv20, vv.max_rv20, vv.latest_rv20
                        );
                        if !vv.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", vv.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── Research section ──
                if let Ok(Some(cm)) = rx::get_calmar(&conn, &sym_upper) {
                    if cm.calmar_label != "INSUFFICIENT_DATA" && !cm.calmar_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Calmar Ratio — CALMAR ({}, as of {})",
                            cm.calmar_label, cm.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · total return {:+.2}% · annualized {:+.2}%",
                            cm.bars_used, cm.total_return_pct, cm.annualized_return_pct
                        );
                        let _ = writeln!(
                            p,
                            "- Max drawdown {:.2}% · Calmar ratio {:.3}",
                            cm.max_drawdown_pct, cm.calmar_ratio
                        );
                        if !cm.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", cm.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ul)) = rx::get_ulcer(&conn, &sym_upper) {
                    if ul.ulcer_label != "INSUFFICIENT_DATA" && !ul.ulcer_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Ulcer Index + Martin Ratio — ULCER ({}, as of {})",
                            ul.ulcer_label, ul.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · ulcer {:.3} · mean dd {:.2}% · max dd {:.2}%",
                            ul.bars_used, ul.ulcer_index, ul.mean_drawdown_pct, ul.max_drawdown_pct
                        );
                        let _ = writeln!(
                            p,
                            "- In drawdown {:.1}% of bars · ann return {:+.2}% · Martin ratio {:.3}",
                            ul.pct_in_drawdown, ul.annualized_return_pct, ul.martin_ratio
                        );
                        if !ul.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ul.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(vr)) = rx::get_varratio(&conn, &sym_upper) {
                    if vr.rw_label != "INSUFFICIENT_DATA" && !vr.rw_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Lo-MacKinlay Variance Ratio — VARRATIO ({}, as of {})",
                            vr.rw_label, vr.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · VR(2) {:.3} · VR(5) {:.3} · VR(10) {:.3} · VR(20) {:.3}",
                            vr.bars_used, vr.vr_2, vr.vr_5, vr.vr_10, vr.vr_20
                        );
                        let _ =
                            writeln!(p, "- z(2) {:+.2} · z(5) {:+.2}", vr.z_stat_2, vr.z_stat_5);
                        if !vr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", vr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(am)) = rx::get_amihud(&conn, &sym_upper) {
                    if am.illiq_label != "INSUFFICIENT_DATA" && !am.illiq_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Amihud Illiquidity — AMIHUD ({}, as of {})",
                            am.illiq_label, am.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · mean ILLIQ {:.4} · median {:.4} · 90th pctile {:.4}",
                            am.bars_used, am.mean_illiq, am.median_illiq, am.illiq_90th
                        );
                        let _ = writeln!(p, "- Avg daily $ volume ${:.0}", am.avg_dollar_volume);
                        if !am.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", am.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(jb)) = rx::get_jbnorm(&conn, &sym_upper) {
                    if jb.normal_label != "INSUFFICIENT_DATA" && !jb.normal_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Jarque-Bera Normality Test — JBNORM ({}, as of {})",
                            jb.normal_label, jb.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · skewness {:+.3} · excess kurtosis {:.3}",
                            jb.bars_used, jb.skewness, jb.excess_kurtosis
                        );
                        let _ = writeln!(
                            p,
                            "- JB statistic {:.2} · p-value {:.6}",
                            jb.jb_statistic, jb.jb_pvalue
                        );
                        if !jb.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", jb.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── Research section ──
                if let Ok(Some(om)) = rx::get_omega(&conn, &sym_upper) {
                    if om.omega_label != "INSUFFICIENT_DATA" && !om.omega_label.is_empty() {
                        let omega_disp = if om.omega_ratio.is_finite() {
                            format!("{:.3}", om.omega_ratio)
                        } else {
                            "∞ (no loss days)".to_string()
                        };
                        let _ = writeln!(
                            p,
                            "### Omega Ratio (τ=0) — OMEGA ({}, as of {})",
                            om.omega_label, om.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · gains Σ {:.4} · losses Σ {:.4}",
                            om.bars_used, om.gains_sum, om.losses_sum
                        );
                        let _ = writeln!(
                            p,
                            "- Gain days {} · loss days {} · win rate {:.1}%",
                            om.gain_days, om.loss_days, om.win_rate_pct
                        );
                        let _ = writeln!(p, "- Omega ratio {}", omega_disp);
                        if !om.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", om.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(df)) = rx::get_dfa(&conn, &sym_upper) {
                    if df.dfa_label != "INSUFFICIENT_DATA" && !df.dfa_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Detrended Fluctuation Analysis — DFA ({}, as of {})",
                            df.dfa_label, df.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · α {:.4} · scales {} · log-log R² {:.3}",
                            df.bars_used, df.alpha, df.num_scales, df.r_squared
                        );
                        if !df.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", df.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(bk)) = rx::get_burke(&conn, &sym_upper) {
                    if bk.burke_label != "INSUFFICIENT_DATA" && !bk.burke_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Burke Ratio — BURKE ({}, as of {})",
                            bk.burke_label, bk.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · annualized return {:+.2}% · Burke ratio {:+.3}",
                            bk.bars_used, bk.annualized_return_pct, bk.burke_ratio
                        );
                        let _ = writeln!(
                            p,
                            "- Drawdown events {} · Σdd² {:.3} · worst event {:.2}%",
                            bk.dd_event_count, bk.sum_sq_drawdowns, bk.worst_event_dd_pct
                        );
                        if !bk.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", bk.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ms)) = rx::get_monthseas(&conn, &sym_upper) {
                    if ms.season_label != "INSUFFICIENT_DATA" && !ms.season_label.is_empty() {
                        const MONTHS: [&str; 12] = [
                            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct",
                            "Nov", "Dec",
                        ];
                        let _ = writeln!(
                            p,
                            "### Monthly Seasonality — MONTHSEAS ({}, as of {})",
                            ms.season_label, ms.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Years covered {} · bars {}",
                            ms.years_covered, ms.bars_used
                        );
                        let _ = writeln!(
                            p,
                            "- Best month: **{}** ({:.1}% hit, {:+.3}% mean)",
                            MONTHS[ms.best_month_idx],
                            ms.best_month_hit_pct,
                            ms.month_mean_ret_pct[ms.best_month_idx]
                        );
                        let _ = writeln!(
                            p,
                            "- Worst month: **{}** ({:.1}% hit, {:+.3}% mean)",
                            MONTHS[ms.worst_month_idx],
                            ms.worst_month_hit_pct,
                            ms.month_mean_ret_pct[ms.worst_month_idx]
                        );
                        let cells: Vec<String> = (0..12)
                            .map(|i| {
                                format!(
                                    "{} {:.0}%/{:+.2}%",
                                    MONTHS[i], ms.month_hit_pct[i], ms.month_mean_ret_pct[i]
                                )
                            })
                            .collect();
                        let _ = writeln!(p, "- Monthly hit %/mean: {}", cells.join(" · "));
                        if !ms.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ms.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(rs)) = rx::get_rollsprd(&conn, &sym_upper) {
                    if rs.roll_label != "INSUFFICIENT_DATA" && !rs.roll_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Roll Implicit Bid-Ask Spread — ROLLSPRD ({}, as of {})",
                            rs.roll_label, rs.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · first-lag cov {:+.6} · mean price {:.4}",
                            rs.bars_used, rs.first_lag_cov, rs.mean_price
                        );
                        if rs.roll_label != "INVALID_POSITIVE_COV" {
                            let _ = writeln!(
                                p,
                                "- Implicit spread {:.4} ({:.2} bps)",
                                rs.implicit_spread, rs.implicit_spread_bps
                            );
                        } else {
                            let _ = writeln!(
                                p,
                                "- Spread undefined: first-lag cov non-negative (trending series)."
                            );
                        }
                        if !rs.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", rs.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── Research section ──
                if let Ok(Some(pk)) = rx::get_parkinson(&conn, &sym_upper) {
                    if pk.vol_label != "INSUFFICIENT_DATA" && !pk.vol_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Parkinson H-L Volatility — PARKINSON ({}, as of {})",
                            pk.vol_label, pk.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · daily σ {:.3}% · annualized σ {:.2}% · mean ln(H/L) {:.5}",
                            pk.bars_used,
                            pk.daily_vol_pct,
                            pk.annualized_vol_pct,
                            pk.mean_hl_log_ratio
                        );
                        if !pk.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", pk.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(gk)) = rx::get_gkvol(&conn, &sym_upper) {
                    if gk.vol_label != "INSUFFICIENT_DATA" && !gk.vol_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Garman-Klass OHLC Volatility — GKVOL ({}, as of {})",
                            gk.vol_label, gk.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · daily σ {:.3}% · annualized σ {:.2}%",
                            gk.bars_used, gk.daily_vol_pct, gk.annualized_vol_pct
                        );
                        let _ = writeln!(
                            p,
                            "- Range component {:.6} · C/O component {:.6}",
                            gk.range_component, gk.co_component
                        );
                        if !gk.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", gk.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(rv)) = rx::get_rsvol(&conn, &sym_upper) {
                    if rv.vol_label != "INSUFFICIENT_DATA" && !rv.vol_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Rogers-Satchell Drift-Free Volatility — RSVOL ({}, as of {})",
                            rv.vol_label, rv.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · daily σ {:.3}% · annualized σ {:.2}% · unbiased under drift",
                            rv.bars_used, rv.daily_vol_pct, rv.annualized_vol_pct
                        );
                        if !rv.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", rv.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(cv)) = rx::get_cvar(&conn, &sym_upper) {
                    if cv.cvar_label != "INSUFFICIENT_DATA" && !cv.cvar_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Conditional VaR / Expected Shortfall — CVAR ({}, as of {})",
                            cv.cvar_label, cv.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · VaR(5%) {:+.3}% · ES(5%) {:+.3}% · tail days 5% {}",
                            cv.bars_used,
                            cv.var_5pct_ret_pct,
                            cv.cvar_5pct_ret_pct,
                            cv.tail_days_5pct
                        );
                        let _ = writeln!(
                            p,
                            "- VaR(1%) {:+.3}% · ES(1%) {:+.3}% · tail days 1% {}",
                            cv.var_1pct_ret_pct, cv.cvar_1pct_ret_pct, cv.tail_days_1pct
                        );
                        if !cv.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", cv.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(dw)) = rx::get_doweffect(&conn, &sym_upper) {
                    if dw.dow_label != "INSUFFICIENT_DATA" && !dw.dow_label.is_empty() {
                        const DOWS: [&str; 5] = ["Mon", "Tue", "Wed", "Thu", "Fri"];
                        let _ = writeln!(
                            p,
                            "### Day-of-Week Seasonality — DOWEFFECT ({}, as of {})",
                            dw.dow_label, dw.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · weeks covered {}",
                            dw.bars_used, dw.weeks_covered
                        );
                        let _ = writeln!(
                            p,
                            "- Best day: **{}** ({:.1}% hit, {:+.3}% mean)",
                            DOWS[dw.best_dow_idx],
                            dw.best_dow_hit_pct,
                            dw.dow_mean_ret_pct[dw.best_dow_idx]
                        );
                        let _ = writeln!(
                            p,
                            "- Worst day: **{}** ({:.1}% hit, {:+.3}% mean)",
                            DOWS[dw.worst_dow_idx],
                            dw.worst_dow_hit_pct,
                            dw.dow_mean_ret_pct[dw.worst_dow_idx]
                        );
                        let cells: Vec<String> = (0..5)
                            .map(|i| {
                                format!(
                                    "{} {:.0}%/{:+.2}%",
                                    DOWS[i], dw.dow_hit_pct[i], dw.dow_mean_ret_pct[i]
                                )
                            })
                            .collect();
                        let _ = writeln!(p, "- Weekday O→C hit %/mean: {}", cells.join(" · "));
                        if !dw.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", dw.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── Research section ──
                if let Ok(Some(st)) = rx::get_sterling(&conn, &sym_upper) {
                    if st.sterling_label != "INSUFFICIENT_DATA" && !st.sterling_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Sterling Ratio — STERLING ({}, as of {})",
                            st.sterling_label, st.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · annualized return {:+.2}% · mean of worst {} dd {:.2}% · ratio {:.3}",
                            st.bars_used,
                            st.annualized_return_pct,
                            st.worst_n,
                            st.mean_worst_dd_pct,
                            st.sterling_ratio
                        );
                        let _ =
                            writeln!(p, "- Distinct dd events in window: {}", st.dd_event_count);
                        if !st.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", st.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(kf)) = rx::get_kellyf(&conn, &sym_upper) {
                    if kf.kelly_label != "INSUFFICIENT_DATA" && !kf.kelly_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Kelly Fraction — KELLYF ({}, as of {})",
                            kf.kelly_label, kf.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · f* {:.4} · half {:.4}",
                            kf.bars_used, kf.kelly_fraction, kf.half_kelly
                        );
                        let _ = writeln!(
                            p,
                            "- p {:.3} · q {:.3} · b {:.3} · avg win {:.3}% · avg loss {:.3}%",
                            kf.win_rate,
                            kf.loss_rate,
                            kf.win_loss_ratio,
                            kf.avg_win_pct,
                            kf.avg_loss_pct
                        );
                        if !kf.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", kf.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(lb)) = rx::get_ljungb(&conn, &sym_upper) {
                    if lb.ljungb_label != "INSUFFICIENT_DATA" && !lb.ljungb_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Ljung-Box Joint Autocorrelation — LJUNGB ({}, as of {})",
                            lb.ljungb_label, lb.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · lag h={} · Q {:.3} · p {:.4} · reject white noise: {}",
                            lb.bars_used,
                            lb.lag_h,
                            lb.q_statistic,
                            lb.p_value,
                            lb.reject_white_noise
                        );
                        if !lb.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", lb.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(rt)) = rx::get_runstest(&conn, &sym_upper) {
                    if rt.runs_label != "INSUFFICIENT_DATA" && !rt.runs_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Wald-Wolfowitz Runs Test — RUNSTEST ({}, as of {})",
                            rt.runs_label, rt.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Signed days {} ({} pos / {} neg) · runs obs {} · expected {:.1} (σ {:.2})",
                            rt.bars_used,
                            rt.positive_days,
                            rt.negative_days,
                            rt.runs_observed,
                            rt.runs_expected,
                            rt.runs_std
                        );
                        let _ = writeln!(
                            p,
                            "- z {:+.3} · p {:.4} · reject randomness: {}",
                            rt.z_statistic, rt.p_value, rt.reject_randomness
                        );
                        if !rt.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", rt.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(zr)) = rx::get_zeroret(&conn, &sym_upper) {
                    if zr.zero_label != "INSUFFICIENT_DATA" && !zr.zero_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Zero-Return-Day Fraction — ZERORET ({}, as of {})",
                            zr.zero_label, zr.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · zero days {} ({:.2}%) · longest streak {} · ε {:.0e}",
                            zr.bars_used,
                            zr.zero_day_count,
                            zr.zero_day_pct,
                            zr.longest_zero_streak,
                            zr.epsilon
                        );
                        if !zr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", zr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── Research section ──
                if let Ok(Some(ps)) = rx::get_psr(&conn, &sym_upper) {
                    if ps.psr_label != "INSUFFICIENT_DATA" && !ps.psr_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Probabilistic Sharpe Ratio — PSR ({}, as of {})",
                            ps.psr_label, ps.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · annualized Sharpe {:+.3} · PSR(SR*={:.2}) {:.4}",
                            ps.bars_used, ps.sharpe, ps.sr_benchmark, ps.psr
                        );
                        let _ = writeln!(
                            p,
                            "- Skewness γ₃ {:+.3} · kurtosis γ₄ {:.3} (Lopez de Prado 2012)",
                            ps.skewness, ps.kurtosis
                        );
                        if !ps.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ps.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ad)) = rx::get_adf(&conn, &sym_upper) {
                    if ad.adf_label != "INSUFFICIENT_DATA" && !ad.adf_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Dickey-Fuller Unit-Root Test — ADF ({}, as of {})",
                            ad.adf_label, ad.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · β {:+.6} · SE {:.6} · t-stat {:+.3}",
                            ad.bars_used, ad.beta, ad.se_beta, ad.t_statistic
                        );
                        let _ = writeln!(
                            p,
                            "- Crit 1%/5%/10% {:+.2}/{:+.2}/{:+.2} · reject unit root: {}",
                            ad.crit_1pct, ad.crit_5pct, ad.crit_10pct, ad.reject_unit_root
                        );
                        if !ad.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ad.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(mk)) = rx::get_mnkendall(&conn, &sym_upper) {
                    if mk.mk_label != "INSUFFICIENT_DATA" && !mk.mk_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Mann-Kendall Trend Test — MNKENDALL ({}, as of {})",
                            mk.mk_label, mk.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · S {} · Var(S) {:.1} · z {:+.3} · p {:.4}",
                            mk.bars_used, mk.s_statistic, mk.variance, mk.z_statistic, mk.p_value
                        );
                        let _ = writeln!(
                            p,
                            "- Kendall τ {:+.3} · reject no-trend: {}",
                            mk.tau, mk.reject_no_trend
                        );
                        if !mk.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", mk.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(bp)) = rx::get_bipower(&conn, &sym_upper) {
                    if bp.jump_label != "INSUFFICIENT_DATA" && !bp.jump_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Bipower Variation / Jump Ratio — BIPOWER ({}, as of {})",
                            bp.jump_label, bp.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · RV {:.6} · BPV {:.6} · continuous vol {:.2}% · realized vol {:.2}%",
                            bp.bars_used,
                            bp.realized_var,
                            bp.bipower_var,
                            bp.continuous_vol_ann_pct,
                            bp.realized_vol_ann_pct
                        );
                        let _ = writeln!(
                            p,
                            "- Jump ratio {:.3} ({:.1}% of realized variance)",
                            bp.jump_ratio, bp.jump_pct
                        );
                        if !bp.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", bp.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(dd)) = rx::get_dddur(&conn, &sym_upper) {
                    if dd.dddur_label != "INSUFFICIENT_DATA" && !dd.dddur_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Drawdown Duration Statistics — DDDUR ({}, as of {})",
                            dd.dddur_label, dd.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · events {} · max {} · mean {:.2} · median {:.2}",
                            dd.bars_used,
                            dd.dd_event_count,
                            dd.max_dd_duration_bars,
                            dd.mean_dd_duration_bars,
                            dd.median_dd_duration_bars
                        );
                        let _ = writeln!(
                            p,
                            "- Underwater {} bars ({:.1}%) · currently underwater: {} (current dur {} bars)",
                            dd.total_bars_underwater,
                            dd.pct_time_underwater,
                            dd.currently_underwater,
                            dd.current_dd_duration_bars
                        );
                        if !dd.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", dd.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── Research section ──
                if let Ok(Some(ht)) = rx::get_hilltail(&conn, &sym_upper) {
                    if ht.tail_label != "INSUFFICIENT_DATA" && !ht.tail_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Hill Tail-Index Estimator — HILLTAIL ({}, as of {})",
                            ht.tail_label, ht.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Returns {} · k top-order {} · threshold |r|(k+1) {:.6}",
                            ht.bars_used, ht.k_order_stats, ht.threshold_abs
                        );
                        let _ = writeln!(
                            p,
                            "- α(|r|) {:.3} · α(left) {:.3} · α(right) {:.3} (α≤2 ⇒ infinite-variance tails)",
                            ht.hill_alpha_abs, ht.hill_alpha_left, ht.hill_alpha_right
                        );
                        if !ht.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ht.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(al)) = rx::get_archlm(&conn, &sym_upper) {
                    if al.arch_label != "INSUFFICIENT_DATA" && !al.arch_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### ARCH Lagrange-Multiplier Test — ARCHLM ({}, as of {})",
                            al.arch_label, al.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Returns {} · q={} · R² {:.4} · LM=n·R² {:.3} · p-value {:.4}",
                            al.bars_used, al.q_lags, al.r_squared, al.lm_statistic, al.p_value
                        );
                        let _ = writeln!(
                            p,
                            "- Crit χ²(5) 5%/1% {:.3}/{:.3} · reject homoskedastic: {}",
                            al.crit_5pct_chi2, al.crit_1pct_chi2, al.reject_homoskedastic
                        );
                        if !al.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", al.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(pr)) = rx::get_painratio(&conn, &sym_upper) {
                    if pr.pain_label != "INSUFFICIENT_DATA" && !pr.pain_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Pain Index / Pain Ratio — PAINRATIO ({}, as of {})",
                            pr.pain_label, pr.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · pain index (mean |dd|) {:.3}% · annualized return {:+.3}% · pain ratio {:+.3}",
                            pr.bars_used,
                            pr.pain_index_pct,
                            pr.annualized_return_pct,
                            pr.pain_ratio
                        );
                        let _ = writeln!(
                            p,
                            "- Max drawdown {:.2}% (companion magnitude; CALMAR uses this denom, PAIN uses mean|dd|)",
                            pr.max_dd_pct
                        );
                        if !pr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", pr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(cs)) = rx::get_cusum(&conn, &sym_upper) {
                    if cs.cusum_label != "INSUFFICIENT_DATA" && !cs.cusum_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Brown-Durbin-Evans CUSUM Break Test — CUSUM ({}, as of {})",
                            cs.cusum_label, cs.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Returns {} · max|S_t| {:.3} · D=max|S_t|/√n {:.3} · bar at max {} · direction {}",
                            cs.bars_used,
                            cs.max_abs_cusum,
                            cs.test_statistic,
                            cs.max_abs_bar,
                            cs.direction_at_max
                        );
                        let _ = writeln!(
                            p,
                            "- Crit 10%/5%/1% {:.2}/{:.2}/{:.2} · reject stability: {}",
                            cs.crit_10pct, cs.crit_5pct, cs.crit_1pct, cs.reject_stability
                        );
                        if !cs.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", cs.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(cf)) = rx::get_cfvar(&conn, &sym_upper) {
                    if cf.cfvar_label != "INSUFFICIENT_DATA" && !cf.cfvar_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Cornish-Fisher Modified VaR — CFVAR ({}, as of {})",
                            cf.cfvar_label, cf.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Returns {} · μ {:+.4}% · σ {:.4}% · skew γ₃ {:+.3} · excess kurt γ₄ {:+.3}",
                            cf.bars_used,
                            cf.mean_ret_pct,
                            cf.sigma_ret_pct,
                            cf.skewness,
                            cf.excess_kurtosis
                        );
                        let _ = writeln!(
                            p,
                            "- Gauss VaR 5%/1% {:+.3}%/{:+.3}% · CF-VaR 5%/1% {:+.3}%/{:+.3}% · adj 5% {:+.3}pp",
                            cf.gauss_var_5pct_pct,
                            cf.gauss_var_1pct_pct,
                            cf.cf_var_5pct_pct,
                            cf.cf_var_1pct_pct,
                            cf.cf_adjustment_5pct_pct
                        );
                        let _ = writeln!(
                            p,
                            "- Skew term @5% {:+.4} · kurt term @5% {:+.4} (dominance drives the label)",
                            cf.skew_term_5pct, cf.kurt_term_5pct
                        );
                        if !cf.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", cf.note);
                        }
                        let _ = writeln!(p);
                    }
                }
            }
        }
    }
}
