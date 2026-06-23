use super::context::SymbolResearchContext;
use std::fmt::Write as _;
use typhoon_engine::core::research as rx;

pub fn write_price_behavior_stat_tests(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    if let Ok(Some(rt)) = rx::get_runstest(ctx.conn, &sym_upper) {
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

    if let Ok(Some(zr)) = rx::get_zeroret(ctx.conn, &sym_upper) {
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
    if let Ok(Some(ps)) = rx::get_psr(ctx.conn, &sym_upper) {
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

    if let Ok(Some(ad)) = rx::get_adf(ctx.conn, &sym_upper) {
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

    if let Ok(Some(mk)) = rx::get_mnkendall(ctx.conn, &sym_upper) {
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
}
