use super::*;

impl TyphooNApp {
    pub(super) fn write_symbol_fractal_tail_stationarity_sections(
        &self,
        p: &mut String,
        sym_upper: &str,
    ) {
        use std::fmt::Write as _;
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use typhoon_engine::core::research as rx;

                // ── Research section ──
                if let Ok(Some(hi)) = rx::get_higuchi(&conn, &sym_upper) {
                    if hi.higuchi_label != "INSUFFICIENT_DATA" && !hi.higuchi_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Higuchi Fractal Dimension — HIGUCHI ({}, as of {})",
                            hi.higuchi_label, hi.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Returns {} · k_max={} · FD={:.4} · R²={:.4} · log-k points={}",
                            hi.bars_used, hi.k_max, hi.fractal_dim, hi.r_squared, hi.log_k_count
                        );
                        if !hi.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", hi.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(pk)) = rx::get_pickands(&conn, &sym_upper) {
                    if pk.pickands_label != "INSUFFICIENT_DATA" && !pk.pickands_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Pickands Tail-Index — PICKANDS ({}, as of {})",
                            pk.pickands_label, pk.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Returns {} · k={} · γ̂={:+.4} · tail α={:.3} · x_k={:.5} · x_2k={:.5} · x_4k={:.5}",
                            pk.bars_used,
                            pk.k_index,
                            pk.gamma_hat,
                            pk.tail_index,
                            pk.x_k,
                            pk.x_2k,
                            pk.x_4k
                        );
                        if !pk.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", pk.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(k3)) = rx::get_kappa3(&conn, &sym_upper) {
                    if k3.kappa3_label != "INSUFFICIENT_DATA" && !k3.kappa3_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Kappa-3 Ratio (Kaplan-Knowles 2004) — KAPPA3 ({}, as of {})",
                            k3.kappa3_label, k3.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Returns {} · MAR={:.4} · excess μ (ann) {:+.4} · LPM3={:.3e} · LPM3^(1/3) (ann) {:.4} · κ3={:+.4} · Sortino ref {:+.4}",
                            k3.bars_used,
                            k3.mar,
                            k3.excess_mean,
                            k3.lpm3,
                            k3.lpm3_root,
                            k3.kappa3,
                            k3.sortino_compare
                        );
                        if !k3.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", k3.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ly)) = rx::get_lyapunov(&conn, &sym_upper) {
                    if ly.lyapunov_label != "INSUFFICIENT_DATA" && !ly.lyapunov_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Largest Lyapunov Exponent (Rosenstein 1993) — LYAPUNOV ({}, as of {})",
                            ly.lyapunov_label, ly.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Returns {} · m={} · τ={} · λ_max={:+.5}/bar · R²={:.4} · steps used {}",
                            ly.bars_used,
                            ly.embed_dim,
                            ly.time_delay,
                            ly.lambda_max,
                            ly.r_squared,
                            ly.steps_used
                        );
                        if !ly.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ly.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ra)) = rx::get_rankac(&conn, &sym_upper) {
                    if ra.rankac_label != "INSUFFICIENT_DATA" && !ra.rankac_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Spearman Rank Autocorrelation — RANKAC ({}, as of {})",
                            ra.rankac_label, ra.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Returns {} · ρ(1)={:+.4} · ρ(5)={:+.4} · ρ(10)={:+.4} · mean|ρ|={:.4} · max|ρ|={:.4}",
                            ra.bars_used,
                            ra.rho_lag1,
                            ra.rho_lag5,
                            ra.rho_lag10,
                            ra.mean_abs_rho,
                            ra.max_abs_rho
                        );
                        if !ra.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ra.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── Research section ──
                if let Ok(Some(bj)) = rx::get_bnsjump(&conn, &sym_upper) {
                    if bj.bnsjump_label != "INSUFFICIENT_DATA" && !bj.bnsjump_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Barndorff-Nielsen-Shephard Jump Test — BNSJUMP ({}, as of {})",
                            bj.bnsjump_label, bj.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Returns {} · RV={:.3e} · BV={:.3e} · jump ratio={:.4} · z={:+.3} · p={:.4}",
                            bj.bars_used,
                            bj.realized_variance,
                            bj.bipower_variance,
                            bj.jump_ratio,
                            bj.jump_z_stat,
                            bj.p_value
                        );
                        if !bj.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", bj.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(pp)) = rx::get_pproot(&conn, &sym_upper) {
                    if pp.pproot_label != "INSUFFICIENT_DATA" && !pp.pproot_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Phillips-Perron Unit-Root — PPROOT ({}, as of {})",
                            pp.pproot_label, pp.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · ρ̂={:.5} · t(ρ)={:+.3} · Z(ρ)={:+.3} · Z(t)={:+.3} · lag q={}",
                            pp.bars_used, pp.rho_hat, pp.t_rho, pp.z_rho, pp.z_t, pp.lag_truncation
                        );
                        if !pp.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", pp.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(mf)) = rx::get_mfdfa(&conn, &sym_upper) {
                    if mf.mfdfa_label != "INSUFFICIENT_DATA" && !mf.mfdfa_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Multifractal DFA — MFDFA ({}, as of {})",
                            mf.mfdfa_label, mf.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Returns {} · h(-2)={:.4} · h(0)={:.4} · h(+2)={:.4} · Δh={:+.4} · scales={}",
                            mf.bars_used,
                            mf.h_q_neg2,
                            mf.h_q_zero,
                            mf.h_q_pos2,
                            mf.delta_h,
                            mf.scales_used
                        );
                        if !mf.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", mf.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(hk)) = rx::get_hillks(&conn, &sym_upper) {
                    if hk.hillks_label != "INSUFFICIENT_DATA" && !hk.hillks_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Hill-Tail KS Goodness-of-Fit — HILLKS ({}, as of {})",
                            hk.hillks_label, hk.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Returns {} · k={} · α̂={:.3} · KS stat={:.4} · KS crit 5%={:.4}",
                            hk.bars_used,
                            hk.k_order,
                            hk.alpha_hat,
                            hk.ks_statistic,
                            hk.ks_critical_5pct
                        );
                        if !hk.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", hk.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(tsi)) = rx::get_tsi(&conn, &sym_upper) {
                    if tsi.tsi_label != "INSUFFICIENT_DATA" && !tsi.tsi_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### True Strength Index (Blau 1991) — TSI ({}, as of {})",
                            tsi.tsi_label, tsi.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · EMA long/short {}/{} · TSI={:+.2} · signal={:+.2} · TSI−signal={:+.2}",
                            tsi.bars_used,
                            tsi.ema_long,
                            tsi.ema_short,
                            tsi.tsi_value,
                            tsi.signal_value,
                            tsi.tsi_minus_signal
                        );
                        if !tsi.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", tsi.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── Research section ──
                if let Ok(Some(g11)) = rx::get_garch11(&conn, &sym_upper) {
                    if g11.garch11_label != "INSUFFICIENT_DATA" && !g11.garch11_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### GARCH(1,1) Fit — GARCH11 ({}, as of {})",
                            g11.garch11_label, g11.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Returns {} · ω={:.3e} · α={:.4} · β={:.4} · α+β={:.4} · uncond var={:.3e} · half-life={:.1} bars · LL={:.2}",
                            g11.bars_used,
                            g11.omega,
                            g11.alpha,
                            g11.beta,
                            g11.persistence,
                            g11.unconditional_var,
                            g11.half_life_bars,
                            g11.log_likelihood
                        );
                        if !g11.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", g11.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(sa)) = rx::get_sadf(&conn, &sym_upper) {
                    if sa.sadf_label != "INSUFFICIENT_DATA" && !sa.sadf_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Sup-ADF Bubble Test (Phillips-Wu-Yu 2011) — SADF ({}, as of {})",
                            sa.sadf_label, sa.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · r0={} · full-ADF t={:+.3} · SADF={:+.3} · argmax end={} · crit5={:.3} · reject null={}",
                            sa.bars_used,
                            sa.min_window,
                            sa.adf_full,
                            sa.sadf_stat,
                            sa.sadf_argmax_end,
                            sa.critical_95,
                            sa.reject_null
                        );
                        if !sa.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", sa.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(cd)) = rx::get_cordim(&conn, &sym_upper) {
                    if cd.cordim_label != "INSUFFICIENT_DATA" && !cd.cordim_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Correlation Dimension (Grassberger-Procaccia) — CORDIM ({}, as of {})",
                            cd.cordim_label, cd.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Returns {} · m={} · radii={} · D2={:.3} · R²={:.3}",
                            cd.bars_used, cd.embed_dim, cd.radii_count, cd.d2, cd.r_squared
                        );
                        if !cd.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", cd.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(sk)) = rx::get_skspec(&conn, &sym_upper) {
                    if sk.skspec_label != "INSUFFICIENT_DATA" && !sk.skspec_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Rolling Skewness Spectrum — SKSPEC ({}, as of {})",
                            sk.skspec_label, sk.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Returns {} · window={} · mean skew={:+.3} · std={:.3} · min={:+.3} · max={:+.3} · range={:.3}",
                            sk.bars_used,
                            sk.window_size,
                            sk.mean_skew,
                            sk.std_skew,
                            sk.min_skew,
                            sk.max_skew,
                            sk.range_skew
                        );
                        if !sk.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", sk.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(am)) = rx::get_automi(&conn, &sym_upper) {
                    if am.automi_label != "INSUFFICIENT_DATA" && !am.automi_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Auto Mutual Information — AUTOMI ({}, as of {})",
                            am.automi_label, am.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Returns {} · bins={} · MI(1)={:.4} · MI(5)={:.4} · MI(10)={:.4} · H(X)={:.3} · MI(1)/H(X)={:.3}",
                            am.bars_used,
                            am.num_bins,
                            am.mi_lag1,
                            am.mi_lag5,
                            am.mi_lag10,
                            am.h_marginal,
                            am.normalized_mi1
                        );
                        if !am.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", am.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── Research section ──
                if let Ok(Some(dw)) = rx::get_durbinwatson(&conn, &sym_upper) {
                    if dw.dw_label != "INSUFFICIENT_DATA" && !dw.dw_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Durbin-Watson Autocorrelation — DURBINWATSON ({}, as of {})",
                            dw.dw_label, dw.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Returns {} · d={:.4} · ρ̂≈{:+.4}",
                            dw.bars_used, dw.dw_stat, dw.rho_estimate
                        );
                        if !dw.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", dw.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(bd)) = rx::get_bdstest(&conn, &sym_upper) {
                    if bd.bds_label != "INSUFFICIENT_DATA" && !bd.bds_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### BDS iid Test (Brock-Dechert-Scheinkman) — BDSTEST ({}, as of {})",
                            bd.bds_label, bd.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Returns {} · m={} · ε/σ={:.2} · BDS={:+.3} · p(2-sided)={:.4} · reject iid={}",
                            bd.bars_used,
                            bd.embed_dim,
                            bd.epsilon_mult,
                            bd.bds_stat,
                            bd.p_value_two_sided,
                            bd.reject_null
                        );
                        if !bd.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", bd.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(bp)) = rx::get_breuschpagan(&conn, &sym_upper) {
                    if bp.bp_label != "INSUFFICIENT_DATA" && !bp.bp_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Breusch-Pagan Heteroskedasticity — BREUSCHPAGAN ({}, as of {})",
                            bp.bp_label, bp.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Returns {} · LM={:.3} · R²={:.4} · df={} · χ²(0.95)={:.3} · reject homo.={}",
                            bp.bars_used,
                            bp.lm_stat,
                            bp.r_squared,
                            bp.df,
                            bp.critical_95,
                            bp.reject_null
                        );
                        if !bp.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", bp.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(tp)) = rx::get_turnpts(&conn, &sym_upper) {
                    if tp.turnpts_label != "INSUFFICIENT_DATA" && !tp.turnpts_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Bartels Turning-Points Test — TURNPTS ({}, as of {})",
                            tp.turnpts_label, tp.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Bars {} · observed={} · expected={:.1} · var={:.2} · z={:+.3} · p(2-sided)={:.4} · reject random={}",
                            tp.bars_used,
                            tp.observed_turnpts,
                            tp.expected_turnpts,
                            tp.variance_turnpts,
                            tp.z_stat,
                            tp.p_value_two_sided,
                            tp.reject_null
                        );
                        if !tp.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", tp.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(pg)) = rx::get_periodogram(&conn, &sym_upper) {
                    if pg.periodogram_label != "INSUFFICIENT_DATA"
                        && !pg.periodogram_label.is_empty()
                    {
                        let _ = writeln!(
                            p,
                            "### Direct-DFT Periodogram — PERIODOGRAM ({}, as of {})",
                            pg.periodogram_label, pg.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Returns {} · freqs={} · dom freq={:.5} · dom period={:.1} bars · dom power={:.3e} · total={:.3e} · ratio={:.3}",
                            pg.bars_used,
                            pg.n_freqs,
                            pg.dominant_freq,
                            pg.dominant_period_bars,
                            pg.dominant_power,
                            pg.total_power,
                            pg.dominant_power_ratio
                        );
                        if !pg.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", pg.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── Research section ──
                if let Ok(Some(ml)) = rx::get_mcleodli(&conn, &sym_upper) {
                    if ml.mcleodli_label != "INSUFFICIENT_DATA" && !ml.mcleodli_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### McLeod-Li Squared-Returns Portmanteau — MCLEODLI ({}, as of {})",
                            ml.mcleodli_label, ml.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Returns {} · h={} · Q={:.3} · χ²95(df={})={:.3} · p={:.4} · reject null={}",
                            ml.bars_used,
                            ml.lag_h,
                            ml.q_stat,
                            ml.df,
                            ml.critical_95,
                            ml.p_value,
                            ml.reject_null
                        );
                        if !ml.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ml.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ou)) = rx::get_oufit(&conn, &sym_upper) {
                    if ou.oufit_label != "INSUFFICIENT_DATA" && !ou.oufit_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Ornstein-Uhlenbeck Mean-Reversion Fit — OUFIT ({}, as of {})",
                            ou.oufit_label, ou.as_of
                        );
                        let hl_s = if ou.half_life_bars.is_finite() {
                            format!("{:.2} bars", ou.half_life_bars)
                        } else {
                            "∞".to_string()
                        };
                        let _ = writeln!(
                            p,
                            "- Bars {} · θ={:.5} · μ={:.4} · σ={:.5} · half-life={} · resid sd={:.5} · R²={:.3}",
                            ou.bars_used,
                            ou.theta,
                            ou.mu,
                            ou.sigma,
                            hl_s,
                            ou.residual_sd,
                            ou.r_squared
                        );
                        if !ou.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ou.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(gp)) = rx::get_gph(&conn, &sym_upper) {
                    if gp.gph_label != "INSUFFICIENT_DATA" && !gp.gph_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### GPH Log-Periodogram Long-Memory — GPH ({}, as of {})",
                            gp.gph_label, gp.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Returns {} · m={} · d̂={:+.3} · stderr={:.3} · t={:+.2} · p={:.4}",
                            gp.bars_used,
                            gp.m_freqs,
                            gp.d_estimate,
                            gp.d_stderr,
                            gp.t_stat,
                            gp.p_value_two_sided
                        );
                        if !gp.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", gp.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(bs)) = rx::get_burgspec(&conn, &sym_upper) {
                    if bs.burgspec_label != "INSUFFICIENT_DATA" && !bs.burgspec_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Burg Maximum-Entropy AR Spectrum — BURGSPEC ({}, as of {})",
                            bs.burgspec_label, bs.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Returns {} · AR order={} · dom freq={:.5} · dom period={:.1} bars · peak={:.3e} · mean={:.3e} · peak/mean={:.2}",
                            bs.bars_used,
                            bs.ar_order,
                            bs.dominant_freq,
                            bs.dominant_period_bars,
                            bs.peak_power,
                            bs.mean_power,
                            bs.peak_to_mean_ratio
                        );
                        if !bs.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", bs.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(kt)) = rx::get_kendalltau(&conn, &sym_upper) {
                    if kt.kendalltau_label != "INSUFFICIENT_DATA" && !kt.kendalltau_label.is_empty()
                    {
                        let _ = writeln!(
                            p,
                            "### Kendall's Tau Lag-1 Rank Autocorrelation — KENDALLTAU ({}, as of {})",
                            kt.kendalltau_label, kt.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Returns {} · pairs={} · C={} · D={} · τ={:+.4} · z={:+.3} · p(2-sided)={:.4}",
                            kt.bars_used,
                            kt.pair_count,
                            kt.concordant,
                            kt.discordant,
                            kt.tau,
                            kt.z_stat,
                            kt.p_value_two_sided
                        );
                        if !kt.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", kt.note);
                        }
                        let _ = writeln!(p);
                    }
                }
            }
        }
    }
}
