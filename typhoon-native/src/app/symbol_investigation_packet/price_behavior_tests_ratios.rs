use super::context::SymbolResearchContext;
use std::fmt::Write as _;
use typhoon_engine::core::research as rx;

pub(super) fn write_price_behavior_tests_ratios(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    if let Ok(Some(st)) = rx::get_sterling(ctx.conn, &sym_upper) {
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
            let _ = writeln!(p, "- Distinct dd events in window: {}", st.dd_event_count);
            if !st.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", st.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(kf)) = rx::get_kellyf(ctx.conn, &sym_upper) {
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
                kf.win_rate, kf.loss_rate, kf.win_loss_ratio, kf.avg_win_pct, kf.avg_loss_pct
            );
            if !kf.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", kf.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(lb)) = rx::get_ljungb(ctx.conn, &sym_upper) {
        if lb.ljungb_label != "INSUFFICIENT_DATA" && !lb.ljungb_label.is_empty() {
            let _ = writeln!(
                p,
                "### Ljung-Box Joint Autocorrelation — LJUNGB ({}, as of {})",
                lb.ljungb_label, lb.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · lag h={} · Q {:.3} · p {:.4} · reject white noise: {}",
                lb.bars_used, lb.lag_h, lb.q_statistic, lb.p_value, lb.reject_white_noise
            );
            if !lb.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", lb.note);
            }
            let _ = writeln!(p);
        }
    }
}
