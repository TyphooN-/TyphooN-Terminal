use super::context::SymbolResearchContext;
use std::fmt::Write as _;
use typhoon_engine::core::research as rx;

pub fn write_price_behavior_vol_estimators(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    if let Ok(Some(pk)) = rx::get_parkinson(ctx.conn, &sym_upper) {
        if pk.vol_label != "INSUFFICIENT_DATA" && !pk.vol_label.is_empty() {
            let _ = writeln!(
                p,
                "### Parkinson H-L Volatility — PARKINSON ({}, as of {})",
                pk.vol_label, pk.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · daily σ {:.3}% · annualized σ {:.2}% · mean ln(H/L) {:.5}",
                pk.bars_used, pk.daily_vol_pct, pk.annualized_vol_pct, pk.mean_hl_log_ratio
            );
            if !pk.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", pk.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(gk)) = rx::get_gkvol(ctx.conn, &sym_upper) {
        if gk.vol_label != "INSUFFICIENT_DATA" && !gk.vol_label.is_empty() {
            let _ = writeln!(
                p,
                "### Garman-Klass OHLC Volatility — GKVOL ({}, as of {})",
                gk.vol_label, gk.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · daily σ {:.3}% · annualized σ {:.2}%",
                gk.bars_used, gk.daily_vol_pct, gk.annualized_vol_pct
            );
            let _ = writeln!(
                p,
                "- Range component {:.6} · C/O component {:.6}",
                gk.range_component, gk.co_component
            );
            if !gk.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", gk.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(rv)) = rx::get_rsvol(ctx.conn, &sym_upper) {
        if rv.vol_label != "INSUFFICIENT_DATA" && !rv.vol_label.is_empty() {
            let _ = writeln!(
                p,
                "### Rogers-Satchell Drift-Free Volatility — RSVOL ({}, as of {})",
                rv.vol_label, rv.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · daily σ {:.3}% · annualized σ {:.2}% · unbiased under drift",
                rv.bars_used, rv.daily_vol_pct, rv.annualized_vol_pct
            );
            if !rv.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", rv.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(cv)) = rx::get_cvar(ctx.conn, &sym_upper) {
        if cv.cvar_label != "INSUFFICIENT_DATA" && !cv.cvar_label.is_empty() {
            let _ = writeln!(
                p,
                "### Conditional VaR / Expected Shortfall — CVAR ({}, as of {})",
                cv.cvar_label, cv.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · VaR(5%) {:+.3}% · ES(5%) {:+.3}% · tail days 5% {}",
                cv.bars_used, cv.var_5pct_ret_pct, cv.cvar_5pct_ret_pct, cv.tail_days_5pct
            );
            let _ = writeln!(
                p,
                "- VaR(1%) {:+.3}% · ES(1%) {:+.3}% · tail days 1% {}",
                cv.var_1pct_ret_pct, cv.cvar_1pct_ret_pct, cv.tail_days_1pct
            );
            if !cv.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", cv.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(dw)) = rx::get_doweffect(ctx.conn, &sym_upper) {
        if dw.dow_label != "INSUFFICIENT_DATA" && !dw.dow_label.is_empty() {
            const DOWS: [&str; 5] = ["Mon", "Tue", "Wed", "Thu", "Fri"];
            let _ = writeln!(
                p,
                "### Day-of-Week Seasonality — DOWEFFECT ({}, as of {})",
                dw.dow_label, dw.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · weeks covered {}",
                dw.bars_used, dw.weeks_covered
            );
            let _ = writeln!(
                p,
                "- Best day: **{}** ({:.1}% hit, {:+.3}% mean)",
                DOWS[dw.best_dow_idx], dw.best_dow_hit_pct, dw.dow_mean_ret_pct[dw.best_dow_idx]
            );
            let _ = writeln!(
                p,
                "- Worst day: **{}** ({:.1}% hit, {:+.3}% mean)",
                DOWS[dw.worst_dow_idx], dw.worst_dow_hit_pct, dw.dow_mean_ret_pct[dw.worst_dow_idx]
            );
            let cells: Vec<String> = (0..5)
                .map(|i| {
                    format!(
                        "{} {:.0}%/{:+.2}%",
                        DOWS[i], dw.dow_hit_pct[i], dw.dow_mean_ret_pct[i]
                    )
                })
                .collect();
            let _ = writeln!(p, "- Weekday O→C hit %/mean: {}", cells.join(" · "));
            if !dw.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", dw.note);
            }
            let _ = writeln!(p);
        }
    }

    // ── Research section ──
}
