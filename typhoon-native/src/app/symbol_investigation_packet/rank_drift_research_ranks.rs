use super::context::SymbolResearchContext;
use std::fmt::Write as _;
use typhoon_engine::core::research as rx;

pub(super) fn write_rank_drift_research_ranks(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    if let Ok(Some(rv)) = rx::get_revrank(ctx.conn, &sym_upper) {
        if rv.relative_label != "NO_DATA"
            && rv.relative_label != "INSUFFICIENT_DATA"
            && !rv.relative_label.is_empty()
        {
            let _ = writeln!(
                p,
                "### Relative Revenue Growth — REVRANK ({}, as of {})",
                rv.relative_label, rv.as_of
            );
            let _ = writeln!(
                p,
                "- 3y CAGR {:+.2}% ({} yrs) · gap {:+.2}pp vs sector {} median {:+.2}%",
                rv.symbol_cagr_pct,
                rv.years_used,
                rv.gap_to_median_pp,
                rv.sector,
                rv.sector_median_cagr_pct
            );
            let _ = writeln!(
                p,
                "- Latest ${:.2}B → earliest ${:.2}B · p25 / p75: {:+.2}% / {:+.2}%",
                rv.latest_revenue / 1e9,
                rv.earliest_revenue / 1e9,
                rv.sector_p25_cagr_pct,
                rv.sector_p75_cagr_pct
            );
            if !rv.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", rv.note);
            }
            let _ = writeln!(p);
        }
    }

    // ── Research section ──
    if let Ok(Some(lv)) = rx::get_levrank(ctx.conn, &sym_upper) {
        if lv.rank_label != "NO_DATA"
            && lv.rank_label != "INSUFFICIENT_DATA"
            && !lv.rank_label.is_empty()
        {
            let _ = writeln!(
                p,
                "### Leverage Rank — LEVRANK ({}, as of {})",
                lv.rank_label, lv.as_of
            );
            if lv.rank_label == "NEGATIVE_EQUITY" {
                let _ = writeln!(
                    p,
                    "- Negative/zero equity: total_debt ${:.2}B · total_equity ${:.2}B",
                    lv.total_debt / 1e9,
                    lv.total_equity / 1e9
                );
            } else {
                let _ = writeln!(
                    p,
                    "- D/E {:.2} · rank {}/{} · pct {:.0} (higher = safer)",
                    lv.debt_to_equity,
                    lv.rank_position,
                    lv.peers_considered + 1,
                    lv.percentile_rank
                );
                let _ = writeln!(
                    p,
                    "- Sector {} median / p25 / p75 D/E: {:.2} / {:.2} / {:.2}",
                    lv.sector, lv.sector_median_d2e, lv.sector_p25_d2e, lv.sector_p75_d2e
                );
            }
            if !lv.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", lv.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(op)) = rx::get_operank(ctx.conn, &sym_upper) {
        if op.rank_label != "NO_DATA"
            && op.rank_label != "INSUFFICIENT_DATA"
            && !op.rank_label.is_empty()
        {
            let _ = writeln!(
                p,
                "### Operating Quality Rank — OPERANK ({}, as of {})",
                op.rank_label, op.as_of
            );
            let _ = writeln!(
                p,
                "- Op margin {:.2}% ({}) · rank {}/{} · pct {:.0}",
                op.operating_margin_pct,
                op.margin_trend_label,
                op.rank_position,
                op.peers_considered + 1,
                op.percentile_rank
            );
            let _ = writeln!(
                p,
                "- Sector {} median / p25 / p75 op margin: {:.2}% / {:.2}% / {:.2}%",
                op.sector,
                op.sector_median_margin_pct,
                op.sector_p25_margin_pct,
                op.sector_p75_margin_pct
            );
            if !op.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", op.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(fqr)) = rx::get_fqmrank(ctx.conn, &sym_upper) {
        if fqr.rank_label != "NO_DATA"
            && fqr.rank_label != "INSUFFICIENT_DATA"
            && !fqr.rank_label.is_empty()
        {
            let _ = writeln!(
                p,
                "### Fundamental Quality Rank — FQMRANK ({} / {}, as of {})",
                fqr.operator_label, fqr.rank_label, fqr.as_of
            );
            let _ = writeln!(
                p,
                "- Composite {:.1}/100 · rank {}/{} · pct {:.0}",
                fqr.composite_score,
                fqr.rank_position,
                fqr.peers_considered + 1,
                fqr.percentile_rank
            );
            let _ = writeln!(
                p,
                "- Sector {} median / p25 / p75: {:.1} / {:.1} / {:.1}",
                fqr.sector, fqr.sector_median_score, fqr.sector_p25, fqr.sector_p75
            );
            if !fqr.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", fqr.note);
            }
            let _ = writeln!(p);
        }
    }
}
