use super::context::SymbolResearchContext;
use std::fmt::Write as _;
use typhoon_engine::core::research as rx;

pub fn write_symbol_distribution_risk_sections(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    // ── Research section ──
    if let Ok(Some(en)) = rx::get_entropy(ctx.conn, &sym_upper) {
        if en.entropy_label != "INSUFFICIENT_DATA" && !en.entropy_label.is_empty() {
            let _ = writeln!(
                p,
                "### Shannon Entropy of Returns — ENTROPY ({}, as of {})",
                en.entropy_label, en.as_of
            );
            let _ = writeln!(
                p,
                "- Returns {} · {} bins · H {:.3} bits · H_max {:.3} bits · normalised {:.3}",
                en.bars_used,
                en.num_bins,
                en.entropy_bits,
                en.max_entropy_bits,
                en.normalised_entropy
            );
            if !en.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", en.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(rv)) = rx::get_rachev(ctx.conn, &sym_upper) {
        if rv.rachev_label != "INSUFFICIENT_DATA" && !rv.rachev_label.is_empty() {
            let _ = writeln!(
                p,
                "### Rachev Ratio — RACHEV ({}, as of {})",
                rv.rachev_label, rv.as_of
            );
            let _ = writeln!(
                p,
                "- Returns {} · ES right 5% {:+.3}% · ES left 5% {:+.3}% · Rachev 5% {:.3}",
                rv.bars_used, rv.es_right_5pct, rv.es_left_5pct, rv.rachev_5pct
            );
            let _ = writeln!(
                p,
                "- ES right 1% {:+.3}% · ES left 1% {:+.3}% · Rachev 1% {:.3}",
                rv.es_right_1pct, rv.es_left_1pct, rv.rachev_1pct
            );
            if !rv.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", rv.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(gp)) = rx::get_gpr(ctx.conn, &sym_upper) {
        if gp.gpr_label != "INSUFFICIENT_DATA" && !gp.gpr_label.is_empty() {
            let _ = writeln!(
                p,
                "### Gain-to-Pain Ratio — GPR ({}, as of {})",
                gp.gpr_label, gp.as_of
            );
            let _ = writeln!(
                p,
                "- Returns {} · Σ all {:+.3}% · Σ gains {:.3}% · Σ |losses| {:.3}%",
                gp.bars_used, gp.sum_all_returns_pct, gp.sum_gains_pct, gp.sum_losses_pct
            );
            let _ = writeln!(
                p,
                "- GPR {:+.3} · Profit Factor {:.3} · wins {} · losses {}",
                gp.gain_to_pain, gp.profit_factor, gp.win_count, gp.loss_count
            );
            if !gp.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", gp.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(pc)) = rx::get_pacf(ctx.conn, &sym_upper) {
        if pc.pacf_label != "INSUFFICIENT_DATA" && !pc.pacf_label.is_empty() {
            let _ = writeln!(
                p,
                "### Partial Autocorrelation — PACF ({}, as of {})",
                pc.pacf_label, pc.as_of
            );
            let _ = writeln!(
                p,
                "- Returns {} · crit 95% ±{:.4} · sig lags {} · max |PACF| {:.4} at lag {}",
                pc.bars_used,
                pc.bartlett_crit_95,
                pc.significant_lags,
                pc.max_abs_pacf,
                pc.max_abs_lag
            );
            let _ = writeln!(
                p,
                "- PACF lag 1..5: {:.4} / {:.4} / {:.4} / {:.4} / {:.4}",
                pc.pacf_lag1, pc.pacf_lag2, pc.pacf_lag3, pc.pacf_lag4, pc.pacf_lag5
            );
            if !pc.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", pc.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(ap)) = rx::get_apen(ctx.conn, &sym_upper) {
        if ap.apen_label != "INSUFFICIENT_DATA" && !ap.apen_label.is_empty() {
            let _ = writeln!(
                p,
                "### Approximate Entropy — APEN ({}, as of {})",
                ap.apen_label, ap.as_of
            );
            let _ = writeln!(
                p,
                "- Returns {} · m={} · r={:.6} · Φ^m {:.4} · Φ^{{m+1}} {:.4} · ApEn {:.4}",
                ap.bars_used, ap.embed_dim, ap.tolerance, ap.phi_m, ap.phi_m1, ap.apen
            );
            if !ap.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", ap.note);
            }
            let _ = writeln!(p);
        }
    }

    // ── Research section ──
    if let Ok(Some(up)) = rx::get_upr(ctx.conn, &sym_upper) {
        if up.upr_label != "INSUFFICIENT_DATA" && !up.upr_label.is_empty() {
            let _ = writeln!(
                p,
                "### Upside Potential Ratio — UPR ({}, as of {})",
                up.upr_label, up.as_of
            );
            let _ = writeln!(
                p,
                "- Returns {} · UPM₁ {:.6} · LPM₂ {:.8} · downside dev {:.6} · UPR {:.4}",
                up.bars_used, up.upm1, up.lpm2, up.downside_dev, up.upr
            );
            if !up.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", up.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(le)) = rx::get_levereff(ctx.conn, &sym_upper) {
        if le.lever_label != "INSUFFICIENT_DATA" && !le.lever_label.is_empty() {
            let _ = writeln!(
                p,
                "### Leverage Effect — LEVEREFF ({}, as of {})",
                le.lever_label, le.as_of
            );
            let _ = writeln!(
                p,
                "- Returns {} · corr(rₜ,rₜ₊₁²) {:+.4} · mean |r| after neg {:.3}% · after pos {:.3}% · asym ratio {:.3}",
                le.bars_used,
                le.corr_r_nextsq,
                le.mean_vol_after_neg,
                le.mean_vol_after_pos,
                le.asym_ratio
            );
            if !le.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", le.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(dd)) = rx::get_drawdar(ctx.conn, &sym_upper) {
        if dd.drawdar_label != "INSUFFICIENT_DATA" && !dd.drawdar_label.is_empty() {
            let _ = writeln!(
                p,
                "### Drawdown-at-Risk — DRAWDAR ({}, as of {})",
                dd.drawdar_label, dd.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · DaR(5%) {:.2}% · CDaR(5%) {:.2}% · DaR(1%) {:.2}% · CDaR(1%) {:.2}%",
                dd.bars_used, dd.dar_5pct, dd.cdar_5pct, dd.dar_1pct, dd.cdar_1pct
            );
            let _ = writeln!(
                p,
                "- Max dd {:.2}% · mean dd {:.2}%",
                dd.max_dd_pct, dd.mean_dd_pct
            );
            if !dd.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", dd.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(vh)) = rx::get_varhalf(ctx.conn, &sym_upper) {
        if vh.varhalf_label != "INSUFFICIENT_DATA" && !vh.varhalf_label.is_empty() {
            let _ = writeln!(
                p,
                "### Volatility Half-Life — VARHALF ({}, as of {})",
                vh.varhalf_label, vh.as_of
            );
            let _ = writeln!(
                p,
                "- Returns {} · vol obs {} · AR(1) β {:.4} · α {:.6} · R² {:.4} · half-life {:.1} days",
                vh.bars_used,
                vh.vol_obs,
                vh.ar1_beta,
                vh.ar1_alpha,
                vh.ar1_r2,
                vh.half_life_days
            );
            if !vh.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", vh.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(gi)) = rx::get_gini(ctx.conn, &sym_upper) {
        if gi.gini_label != "INSUFFICIENT_DATA" && !gi.gini_label.is_empty() {
            let _ = writeln!(
                p,
                "### Gini Coefficient of |Returns| — GINI ({}, as of {})",
                gi.gini_label, gi.as_of
            );
            let _ = writeln!(
                p,
                "- Returns {} · Gini {:.4} · mean |r| {:.4}% · median |r| {:.4}%",
                gi.bars_used, gi.gini_coeff, gi.mean_abs_return_pct, gi.median_abs_return_pct
            );
            if !gi.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", gi.note);
            }
            let _ = writeln!(p);
        }
    }

    // ── Research section ──
    if let Ok(Some(se)) = rx::get_sampen(ctx.conn, &sym_upper) {
        if se.sampen_label != "INSUFFICIENT_DATA"
            && se.sampen_label != "UNDEFINED"
            && !se.sampen_label.is_empty()
        {
            let _ = writeln!(
                p,
                "### Sample Entropy — SAMPEN ({}, as of {})",
                se.sampen_label, se.as_of
            );
            let _ = writeln!(
                p,
                "- Returns {} · m={} · r={:.6} · A={} · B={} · SampEn {:.4}",
                se.bars_used, se.embed_dim, se.tolerance, se.a_count, se.b_count, se.sampen
            );
            if !se.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", se.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(pe)) = rx::get_permen(ctx.conn, &sym_upper) {
        if pe.permen_label != "INSUFFICIENT_DATA" && !pe.permen_label.is_empty() {
            let _ = writeln!(
                p,
                "### Permutation Entropy — PERMEN ({}, as of {})",
                pe.permen_label, pe.as_of
            );
            let _ = writeln!(
                p,
                "- Returns {} · m={} · patterns {}/{} · H_raw {:.4} bits · H_norm {:.4}",
                pe.bars_used,
                pe.embed_dim,
                pe.patterns_observed,
                pe.patterns_possible,
                pe.permen_raw,
                pe.permen_normalised
            );
            if !pe.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", pe.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(rf)) = rx::get_recfact(ctx.conn, &sym_upper) {
        if rf.recfact_label != "INSUFFICIENT_DATA" && !rf.recfact_label.is_empty() {
            let _ = writeln!(
                p,
                "### Recovery Factor — RECFACT ({}, as of {})",
                rf.recfact_label, rf.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · cum return {:.2}% · max dd {:.2}% · recovery factor {:.4}",
                rf.bars_used, rf.cum_return_pct, rf.max_drawdown_pct, rf.recovery_factor
            );
            if !rf.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", rf.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(kp)) = rx::get_kpss(ctx.conn, &sym_upper) {
        if kp.kpss_label != "INSUFFICIENT_DATA" && !kp.kpss_label.is_empty() {
            let _ = writeln!(
                p,
                "### KPSS Stationarity Test — KPSS ({}, as of {})",
                kp.kpss_label, kp.as_of
            );
            let _ = writeln!(
                p,
                "- Returns {} · η_μ {:.4} · lag ℓ={} · crit 10%={:.3} 5%={:.3} 1%={:.3} · reject_stationary {}",
                kp.bars_used,
                kp.kpss_stat,
                kp.lag_truncation,
                kp.crit_10,
                kp.crit_5,
                kp.crit_1,
                kp.reject_stationary
            );
            if !kp.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", kp.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(sp)) = rx::get_specent(ctx.conn, &sym_upper) {
        if sp.specent_label != "INSUFFICIENT_DATA" && !sp.specent_label.is_empty() {
            let _ = writeln!(
                p,
                "### Spectral Entropy — SPECENT ({}, as of {})",
                sp.specent_label, sp.as_of
            );
            let _ = writeln!(
                p,
                "- Returns {} · freqs {} · H_raw {:.4} · H_norm {:.4} · peak idx {} · peak share {:.4}",
                sp.bars_used,
                sp.num_freqs,
                sp.spectral_entropy_raw,
                sp.spectral_entropy_norm,
                sp.peak_freq_idx,
                sp.peak_power_share
            );
            if !sp.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", sp.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(rv)) = rx::get_robvol(ctx.conn, &sym_upper) {
        if rv.robvol_label != "INSUFFICIENT_DATA" && !rv.robvol_label.is_empty() {
            let _ = writeln!(
                p,
                "### Robust Volatility — ROBVOL ({}, as of {})",
                rv.robvol_label, rv.as_of
            );
            let _ = writeln!(
                p,
                "- Returns {} · classical σ {:.4} · MAD σ {:.4} · IQR σ {:.4} · MAD ratio {:.3} · IQR ratio {:.3}",
                rv.bars_used,
                rv.classical_sigma,
                rv.mad_sigma,
                rv.iqr_sigma,
                rv.mad_ratio,
                rv.iqr_ratio
            );
            if !rv.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", rv.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(re)) = rx::get_renyient(ctx.conn, &sym_upper) {
        if re.renyient_label != "INSUFFICIENT_DATA" && !re.renyient_label.is_empty() {
            let _ = writeln!(
                p,
                "### Rényi Entropy (α=2) — RENYIENT ({}, as of {})",
                re.renyient_label, re.as_of
            );
            let _ = writeln!(
                p,
                "- Returns {} · bins {} · H₂ {:.4} · H_norm {:.4} · collision_prob {:.4}",
                re.bars_used, re.num_bins, re.renyi_raw, re.renyi_normalised, re.collision_prob
            );
            if !re.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", re.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(rq)) = rx::get_retquant(ctx.conn, &sym_upper) {
        if rq.retquant_label != "INSUFFICIENT_DATA" && !rq.retquant_label.is_empty() {
            let _ = writeln!(
                p,
                "### Return Quantile Profile — RETQUANT ({}, as of {})",
                rq.retquant_label, rq.as_of
            );
            let _ = writeln!(
                p,
                "- Returns {} · P1 {:.3}% · P5 {:.3}% · P10 {:.3}% · P25 {:.3}% · P50 {:.3}% · P75 {:.3}% · P90 {:.3}% · P95 {:.3}% · P99 {:.3}% · IQR {:.3}% · tail_asymm {:.3}",
                rq.bars_used,
                rq.p01_pct,
                rq.p05_pct,
                rq.p10_pct,
                rq.p25_pct,
                rq.p50_pct,
                rq.p75_pct,
                rq.p90_pct,
                rq.p95_pct,
                rq.p99_pct,
                rq.iqr_pct,
                rq.tail_asymmetry
            );
            if !rq.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", rq.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(ms)) = rx::get_msent(ctx.conn, &sym_upper) {
        if ms.msent_label != "INSUFFICIENT_DATA" && !ms.msent_label.is_empty() {
            let _ = writeln!(
                p,
                "### Multiscale Entropy — MSENT ({}, as of {})",
                ms.msent_label, ms.as_of
            );
            let _ = writeln!(
                p,
                "- Returns {} · m={} · r={:.5} · τ1={:.3} · τ2={:.3} · τ3={:.3} · τ4={:.3} · τ5={:.3} · CI {:.3}",
                ms.bars_used,
                ms.embed_dim,
                ms.tolerance,
                ms.sampen_scale1,
                ms.sampen_scale2,
                ms.sampen_scale3,
                ms.sampen_scale4,
                ms.sampen_scale5,
                ms.msent_complexity_index
            );
            if !ms.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", ms.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(ew)) = rx::get_ewmavol(ctx.conn, &sym_upper) {
        if ew.ewmavol_label != "INSUFFICIENT_DATA" && !ew.ewmavol_label.is_empty() {
            let _ = writeln!(
                p,
                "### EWMA Volatility — EWMAVOL ({}, as of {})",
                ew.ewmavol_label, ew.as_of
            );
            let _ = writeln!(
                p,
                "- Returns {} · λ={:.3} · σ_daily {:.5} · σ_annual {:.4} · classical σ_annual {:.4} · ratio {:.3}",
                ew.bars_used,
                ew.lambda,
                ew.ewma_sigma_daily,
                ew.ewma_sigma_annual,
                ew.classical_sigma_annual,
                ew.ewma_to_classical
            );
            if !ew.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", ew.note);
            }
            let _ = writeln!(p);
        }
    }

    // ── Research section ──
    if let Ok(Some(ks)) = rx::get_ksnorm(ctx.conn, &sym_upper) {
        if ks.ksnorm_label != "INSUFFICIENT_DATA" && !ks.ksnorm_label.is_empty() {
            let _ = writeln!(
                p,
                "### KS Normality Test — KSNORM ({}, as of {})",
                ks.ksnorm_label, ks.as_of
            );
            let _ = writeln!(
                p,
                "- Returns {} · D={:.4} · crit 10%/5%/1% {:.4}/{:.4}/{:.4} · reject 10%/5%/1% {}/{}/{} · μ={:.6} σ={:.5}",
                ks.bars_used,
                ks.ks_statistic,
                ks.critical_10pct,
                ks.critical_5pct,
                ks.critical_1pct,
                ks.reject_10pct,
                ks.reject_5pct,
                ks.reject_1pct,
                ks.mean,
                ks.sigma
            );
            if !ks.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", ks.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(ad)) = rx::get_adtest(ctx.conn, &sym_upper) {
        if ad.adtest_label != "INSUFFICIENT_DATA" && !ad.adtest_label.is_empty() {
            let _ = writeln!(
                p,
                "### Anderson-Darling Test — ADTEST ({}, as of {})",
                ad.adtest_label, ad.as_of
            );
            let _ = writeln!(
                p,
                "- Returns {} · A²={:.4} · A²_adj={:.4} · p≈{:.4} · crit 10%/5%/1% {:.3}/{:.3}/{:.3} · reject 10%/5%/1% {}/{}/{}",
                ad.bars_used,
                ad.ad_statistic,
                ad.ad_adjusted,
                ad.p_value_approx,
                ad.critical_10pct,
                ad.critical_5pct,
                ad.critical_1pct,
                ad.reject_10pct,
                ad.reject_5pct,
                ad.reject_1pct
            );
            if !ad.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", ad.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(lm)) = rx::get_lmom(ctx.conn, &sym_upper) {
        if lm.lmom_label != "INSUFFICIENT_DATA" && !lm.lmom_label.is_empty() {
            let _ = writeln!(
                p,
                "### L-Moments (Hosking 1990) — LMOM ({}, as of {})",
                lm.lmom_label, lm.as_of
            );
            let _ = writeln!(
                p,
                "- Returns {} · L1={:.6} · L2={:.6} · L3={:.6} · L4={:.6} · τ3={:+.4} · τ4={:+.4}",
                lm.bars_used, lm.l1_mean, lm.l2_scale, lm.l3, lm.l4, lm.tau3_skew, lm.tau4_kurt
            );
            if !lm.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", lm.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(ky)) = rx::get_kylelam(ctx.conn, &sym_upper) {
        if ky.kylelam_label != "INSUFFICIENT_DATA" && !ky.kylelam_label.is_empty() {
            let _ = writeln!(
                p,
                "### Kyle's Price Impact λ — KYLELAM ({}, as of {})",
                ky.kylelam_label, ky.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · λ={:.3e} · mean|Δp|={:.5} · mean V={:.1} · ρ={:+.4} · R²={:.4}",
                ky.bars_used,
                ky.kyle_lambda,
                ky.mean_abs_dp,
                ky.mean_volume,
                ky.correlation,
                ky.r_squared
            );
            if !ky.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", ky.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(po)) = rx::get_peakover(ctx.conn, &sym_upper) {
        if po.peakover_label != "INSUFFICIENT_DATA" && !po.peakover_label.is_empty() {
            let _ = writeln!(
                p,
                "### Peaks-Over-Threshold — PEAKOVER ({}, as of {})",
                po.peakover_label, po.as_of
            );
            let _ = writeln!(
                p,
                "- Returns {} · P95={:.4}% · P99={:.4}% · count>P95={} · count>P99={} · mean excess 95/99 {:.4}%/{:.4}% · max excess 95/99 {:.4}%/{:.4}%",
                po.bars_used,
                po.threshold_p95 * 100.0,
                po.threshold_p99 * 100.0,
                po.count_p95,
                po.count_p99,
                po.mean_excess_p95 * 100.0,
                po.mean_excess_p99 * 100.0,
                po.max_excess_p95 * 100.0,
                po.max_excess_p99 * 100.0
            );
            if !po.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", po.note);
            }
            let _ = writeln!(p);
        }
    }
}
