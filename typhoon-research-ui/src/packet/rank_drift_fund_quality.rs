use super::context::SymbolResearchContext;
use std::fmt::Write as _;
use typhoon_engine::core::research as rx;

pub fn write_rank_drift_fund_quality(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    if let Ok(Some(pr)) = rx::get_peadrank(ctx.conn, &sym_upper) {
        if pr.rank_label != "NO_DATA"
            && pr.rank_label != "INSUFFICIENT_DATA"
            && !pr.rank_label.is_empty()
        {
            let _ = writeln!(
                p,
                "### PEAD Rank — PEADRANK ({}, as of {})",
                pr.rank_label, pr.as_of
            );
            let _ = writeln!(
                p,
                "- Avg 5d drift {:+.2}% · rank {}/{} · pct {:.0}",
                pr.avg_drift_5d_pct,
                pr.rank_position,
                pr.peers_considered + 1,
                pr.percentile_rank
            );
            let _ = writeln!(
                p,
                "- Sector {} median / p25 / p75: {:+.2}% / {:+.2}% / {:+.2}%",
                pr.sector,
                pr.sector_median_drift_5d_pct,
                pr.sector_p25_drift_5d_pct,
                pr.sector_p75_drift_5d_pct
            );
            if !pr.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", pr.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(fq)) = rx::get_fqm(ctx.conn, &sym_upper) {
        if fq.operator_label != "NO_DATA" && !fq.operator_label.is_empty() {
            let _ = writeln!(
                p,
                "### Fundamental Quality Meter — FQM ({}, as of {})",
                fq.operator_label, fq.as_of
            );
            let _ = writeln!(
                p,
                "- Composite {:.1}/100 · {} inputs · Piotroski {:.0} ({}) · Op margin {:.2}% ({}) · Cash conv {:.2}% ({})",
                fq.composite_score,
                fq.inputs_available,
                fq.piotroski_score,
                fq.piotroski_label,
                fq.operating_margin_pct,
                fq.margin_trend_label,
                fq.cash_conversion_pct,
                fq.accruals_trend_label
            );
            if !fq.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", fq.note);
            }
            let _ = writeln!(p);
        }
    }
}
