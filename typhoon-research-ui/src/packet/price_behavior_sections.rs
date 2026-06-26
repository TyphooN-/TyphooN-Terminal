use super::context::SymbolResearchContext;
use std::fmt::Write as _;
use typhoon_engine::core::research as rx;

pub fn write_symbol_price_behavior_sections(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    super::price_behavior_distribution::write_price_behavior_distribution(ctx, p, sym_upper);

    super::price_behavior_local::write_price_behavior_local(ctx, p, sym_upper);

    super::price_behavior_ratios::write_price_behavior_ratios(ctx, p, sym_upper);

    super::price_behavior_risk_metrics::write_price_behavior_risk_metrics(ctx, p, sym_upper);

    super::price_behavior_illiquidity_norm::write_price_behavior_illiquidity_norm(
        ctx, p, sym_upper,
    );

    super::price_behavior_seasonality_vol::write_price_behavior_seasonality_vol(ctx, p, sym_upper);

    super::price_behavior_vol_estimators::write_price_behavior_vol_estimators(ctx, p, sym_upper);

    super::price_behavior_tests_ratios::write_price_behavior_tests_ratios(ctx, p, sym_upper);

    super::price_behavior_stat_tests::write_price_behavior_stat_tests(ctx, p, sym_upper);

    if let Ok(Some(bp)) = rx::get_bipower(ctx.conn, &sym_upper) {
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

    if let Ok(Some(dd)) = rx::get_dddur(ctx.conn, &sym_upper) {
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
    if let Ok(Some(ht)) = rx::get_hilltail(ctx.conn, &sym_upper) {
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

    if let Ok(Some(al)) = rx::get_archlm(ctx.conn, &sym_upper) {
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

    if let Ok(Some(pr)) = rx::get_painratio(ctx.conn, &sym_upper) {
        if pr.pain_label != "INSUFFICIENT_DATA" && !pr.pain_label.is_empty() {
            let _ = writeln!(
                p,
                "### Pain Index / Pain Ratio — PAINRATIO ({}, as of {})",
                pr.pain_label, pr.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · pain index (mean |dd|) {:.3}% · annualized return {:+.3}% · pain ratio {:+.3}",
                pr.bars_used, pr.pain_index_pct, pr.annualized_return_pct, pr.pain_ratio
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

    if let Ok(Some(cs)) = rx::get_cusum(ctx.conn, &sym_upper) {
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

    if let Ok(Some(cf)) = rx::get_cfvar(ctx.conn, &sym_upper) {
        if cf.cfvar_label != "INSUFFICIENT_DATA" && !cf.cfvar_label.is_empty() {
            let _ = writeln!(
                p,
                "### Cornish-Fisher Modified VaR — CFVAR ({}, as of {})",
                cf.cfvar_label, cf.as_of
            );
            let _ = writeln!(
                p,
                "- Returns {} · μ {:+.4}% · σ {:.4}% · skew γ₃ {:+.3} · excess kurt γ₄ {:+.3}",
                cf.bars_used, cf.mean_ret_pct, cf.sigma_ret_pct, cf.skewness, cf.excess_kurtosis
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
