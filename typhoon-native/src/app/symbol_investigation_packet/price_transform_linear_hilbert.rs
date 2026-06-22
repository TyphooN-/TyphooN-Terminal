use super::context::SymbolResearchContext;
use std::fmt::Write as _;
use typhoon_engine::core::research as rx;

pub(super) fn write_price_transform_linear_hilbert(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    // ── Research section ──
    if let Ok(Some(ls)) = rx::get_linearreg_slope(ctx.conn, &sym_upper) {
        if ls.slope_label != "INSUFFICIENT_DATA" && !ls.slope_label.is_empty() {
            let _ = writeln!(
                p,
                "### Linear Regression Slope — LINEARREG_SLOPE ({}, as of {})",
                ls.slope_label, ls.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · length {} · slope {:+.6} (prev {:+.6}) · slope_pct {:+.3}% · close {:.4}",
                ls.bars_used,
                ls.length,
                ls.slope,
                ls.slope_prev,
                ls.slope_pct,
                ls.last_close
            );
            if !ls.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", ls.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(dc)) = rx::get_ht_dcperiod(ctx.conn, &sym_upper) {
        if dc.period_label != "INSUFFICIENT_DATA" && !dc.period_label.is_empty() {
            let _ = writeln!(
                p,
                "### Hilbert Dominant Cycle Period — HT_DCPERIOD ({}, as of {})",
                dc.period_label, dc.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · period {:.2} (prev {:.2}) · min(64) {:.2} · max(64) {:.2} · close {:.4}",
                dc.bars_used,
                dc.period,
                dc.period_prev,
                dc.period_min_64,
                dc.period_max_64,
                dc.last_close
            );
            if !dc.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", dc.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(tm)) = rx::get_ht_trendmode(ctx.conn, &sym_upper) {
        if tm.mode_label != "INSUFFICIENT_DATA" && !tm.mode_label.is_empty() {
            let _ = writeln!(
                p,
                "### Hilbert Trend vs Cycle Mode — HT_TRENDMODE ({}, as of {})",
                tm.mode_label, tm.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · trendmode {} (prev {}) · lock_in_bars {} · period {:.2} · close {:.4}",
                tm.bars_used,
                tm.trendmode,
                tm.trendmode_prev,
                tm.lock_in_bars,
                tm.period,
                tm.last_close
            );
            if !tm.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", tm.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(ab)) = rx::get_accbands(ctx.conn, &sym_upper) {
        if ab.accbands_label != "INSUFFICIENT_DATA" && !ab.accbands_label.is_empty() {
            let _ = writeln!(
                p,
                "### Acceleration Bands — ACCBANDS ({}, as of {})",
                ab.accbands_label, ab.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · length {} · upper {:.4} · middle {:.4} · lower {:.4} · width {:.4} · pos {:.3} · close {:.4}",
                ab.bars_used,
                ab.length,
                ab.acc_upper,
                ab.acc_middle,
                ab.acc_lower,
                ab.width,
                ab.position,
                ab.last_close
            );
            if !ab.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", ab.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(sf)) = rx::get_stochf(ctx.conn, &sym_upper) {
        if sf.stochf_label != "INSUFFICIENT_DATA" && !sf.stochf_label.is_empty() {
            let _ = writeln!(
                p,
                "### Fast Stochastic — STOCHF ({}, as of {})",
                sf.stochf_label, sf.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · length {} · d_period {} · fastK {:.2} (prev {:.2}) · fastD {:.2} (prev {:.2}) · close {:.4}",
                sf.bars_used,
                sf.length,
                sf.d_period,
                sf.fastk,
                sf.fastk_prev,
                sf.fastd,
                sf.fastd_prev,
                sf.last_close
            );
            if !sf.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", sf.note);
            }
            let _ = writeln!(p);
        }
    }
}
