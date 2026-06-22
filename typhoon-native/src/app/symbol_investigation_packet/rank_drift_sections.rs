use super::context::SymbolResearchContext;
use std::fmt::Write as _;
use typhoon_engine::core::research as rx;

pub(super) fn write_symbol_rank_drift_sections(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    super::rank_drift_core_ranks::write_rank_drift_core_ranks(ctx, p, sym_upper);

    super::rank_drift_growth_drift::write_rank_drift_growth_drift(ctx, p, sym_upper);

    super::rank_drift_fund_quality::write_rank_drift_fund_quality(ctx, p, sym_upper);

    super::rank_drift_research_ranks::write_rank_drift_research_ranks(ctx, p, sym_upper);

    super::rank_drift_liquidity_streaks::write_rank_drift_liquidity_streaks(ctx, p, sym_upper);

    super::rank_drift_div_earn_streaks::write_rank_drift_div_earn_streaks(ctx, p, sym_upper);

    super::rank_drift_yield_short_conc::write_rank_drift_yield_short_conc(ctx, p, sym_upper);

    super::rank_drift_vol_perf::write_rank_drift_vol_perf(ctx, p, sym_upper);

    super::rank_drift_cone_corrs::write_rank_drift_cone_corrs(ctx, p, sym_upper);

    if let Ok(Some(cr)) = rx::get_corrrank(ctx.conn, &sym_upper) {
        if cr.rank_label != "NO_DATA"
            && cr.rank_label != "INSUFFICIENT_DATA"
            && !cr.rank_label.is_empty()
        {
            let _ = writeln!(
                p,
                "### Benchmark Linkage Rank — CORRRANK ({} / {}, as of {})",
                cr.subject_correlation_label, cr.rank_label, cr.as_of
            );
            let _ = writeln!(
                p,
                "- {} {} corr {:.2} (|corr| {:.2}) · β {:.2} · R² {:.2} · rank {}/{} · pct {:.0}",
                cr.benchmark_kind,
                cr.benchmark_name,
                cr.subject_corr_252d,
                cr.subject_abs_corr_252d,
                cr.subject_beta_252d,
                cr.subject_r_squared_252d,
                cr.rank_position,
                cr.peers_considered + 1,
                cr.percentile_rank
            );
            let _ = writeln!(
                p,
                "- Sector {} median / p25 / p75 |corr|: {:.2} / {:.2} / {:.2}",
                cr.sector,
                cr.sector_median_abs_corr_252d,
                cr.sector_p25_abs_corr_252d,
                cr.sector_p75_abs_corr_252d
            );
            if !cr.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", cr.note);
            }
            let _ = writeln!(p);
        }
    }

    super::rank_drift_accs_vrp::write_rank_drift_accs_vrp(ctx, p, sym_upper);
}
