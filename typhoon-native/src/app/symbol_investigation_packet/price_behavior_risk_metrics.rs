use super::context::SymbolResearchContext;
use std::fmt::Write as _;
use typhoon_engine::core::research as rx;

pub(super) fn write_price_behavior_risk_metrics(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    if let Ok(Some(cm)) = rx::get_calmar(ctx.conn, &sym_upper) {
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

    if let Ok(Some(ul)) = rx::get_ulcer(ctx.conn, &sym_upper) {
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
}
