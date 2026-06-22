use super::context::SymbolResearchContext;
use std::fmt::Write as _;
use typhoon_engine::core::research as rx;

pub(super) fn write_price_behavior_distribution(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    // ── HP return-distribution + behavior stats ──
    if let Ok(Some(rsk)) = rx::get_retskew(ctx.conn, &sym_upper) {
        if rsk.skew_label != "INSUFFICIENT_DATA" && !rsk.skew_label.is_empty() {
            let _ = writeln!(
                p,
                "### Return Distribution Skewness — RETSKEW ({}, as of {})",
                rsk.skew_label, rsk.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · mean log ret {:.4} · stdev {:.4} · skewness {:+.3}",
                rsk.bars_used, rsk.mean_log_return, rsk.stdev_log_return, rsk.skewness
            );
            let _ = writeln!(
                p,
                "- Positive-day share {:.1}% · largest up {:+.2}% · largest down {:+.2}%",
                rsk.positive_return_pct, rsk.largest_up_pct, rsk.largest_down_pct
            );
            if !rsk.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", rsk.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(rkt)) = rx::get_retkurt(ctx.conn, &sym_upper) {
        if rkt.kurt_label != "INSUFFICIENT_DATA" && !rkt.kurt_label.is_empty() {
            let _ = writeln!(
                p,
                "### Return Distribution Excess Kurtosis — RETKURT ({}, as of {})",
                rkt.kurt_label, rkt.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · stdev {:.4} · excess kurtosis {:+.3}",
                rkt.bars_used, rkt.stdev_log_return, rkt.excess_kurtosis
            );
            let _ = writeln!(
                p,
                "- |z|>2 count {} ({:.1}%) · |z|>3 count {}",
                rkt.outlier_2sigma_count, rkt.outlier_2sigma_pct, rkt.outlier_3sigma_count
            );
            if !rkt.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", rkt.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(tlr)) = rx::get_tailr(ctx.conn, &sym_upper) {
        if tlr.bias_label != "INSUFFICIENT_DATA" && !tlr.bias_label.is_empty() {
            let _ = writeln!(
                p,
                "### Tail Ratio — TAILR ({}, as of {})",
                tlr.bias_label, tlr.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · P95 {:+.2}% · P05 {:+.2}% · tail ratio {:.2}",
                tlr.bars_used, tlr.pct_95_return, tlr.pct_05_return, tlr.tail_ratio
            );
            let _ = writeln!(
                p,
                "- P99 {:+.2}% · P01 {:+.2}% · 99/01 ratio {:.2}",
                tlr.pct_99_return, tlr.pct_01_return, tlr.tail_ratio_99_01
            );
            if !tlr.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", tlr.note);
            }
            let _ = writeln!(p);
        }
    }
}
