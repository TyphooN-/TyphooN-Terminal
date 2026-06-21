use super::*;

impl TyphooNApp {
    pub(super) fn write_symbol_price_behavior_sections(&self, p: &mut String, sym_upper: &str) {
        use std::fmt::Write as _;
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use typhoon_engine::core::research as rx;

                self.write_price_behavior_distribution(p, sym_upper);

                self.write_price_behavior_local(p, sym_upper);

                self.write_price_behavior_ratios(p, sym_upper);

                self.write_price_behavior_risk_metrics(p, sym_upper);

                self.write_price_behavior_illiquidity_norm(p, sym_upper);

                self.write_price_behavior_seasonality_vol(p, sym_upper);

                self.write_price_behavior_vol_estimators(p, sym_upper);

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
