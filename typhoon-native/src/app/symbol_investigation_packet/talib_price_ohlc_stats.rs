use super::context::SymbolResearchContext;
use std::fmt::Write as _;
use typhoon_engine::core::research as rx;

pub(super) fn write_talib_price_ohlc_stats(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    // ── Research section ──
    if let Ok(Some(ap)) = rx::get_avgprice(ctx.conn, &sym_upper) {
        if ap.avgprice_label != "INSUFFICIENT_DATA" && !ap.avgprice_label.is_empty() {
            let _ = writeln!(
                p,
                "### OHLC Average — AVGPRICE ({}, as of {})",
                ap.avgprice_label, ap.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · avgprice {:.4} (prev {:.4}) · O {:.4} · H {:.4} · L {:.4} · C {:.4} · Δ {:+.3}%",
                ap.bars_used,
                ap.avgprice,
                ap.avgprice_prev,
                ap.open,
                ap.high,
                ap.low,
                ap.close,
                ap.delta_pct
            );
            if !ap.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", ap.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(mp)) = rx::get_medprice(ctx.conn, &sym_upper) {
        if mp.medprice_label != "INSUFFICIENT_DATA" && !mp.medprice_label.is_empty() {
            let _ = writeln!(
                p,
                "### Range Median — MEDPRICE ({}, as of {})",
                mp.medprice_label, mp.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · medprice {:.4} (prev {:.4}) · H {:.4} · L {:.4} · C {:.4} · Δ {:+.3}%",
                mp.bars_used,
                mp.medprice,
                mp.medprice_prev,
                mp.high,
                mp.low,
                mp.close,
                mp.delta_pct
            );
            if !mp.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", mp.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(tp)) = rx::get_typprice(ctx.conn, &sym_upper) {
        if tp.typprice_label != "INSUFFICIENT_DATA" && !tp.typprice_label.is_empty() {
            let _ = writeln!(
                p,
                "### Typical Price — TYPPRICE ({}, as of {})",
                tp.typprice_label, tp.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · typprice {:.4} (prev {:.4}) · H {:.4} · L {:.4} · C {:.4} · Δ {:+.3}%",
                tp.bars_used,
                tp.typprice,
                tp.typprice_prev,
                tp.high,
                tp.low,
                tp.close,
                tp.delta_pct
            );
            if !tp.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", tp.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(wp)) = rx::get_wclprice(ctx.conn, &sym_upper) {
        if wp.wclprice_label != "INSUFFICIENT_DATA" && !wp.wclprice_label.is_empty() {
            let _ = writeln!(
                p,
                "### Weighted Close — WCLPRICE ({}, as of {})",
                wp.wclprice_label, wp.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · wclprice {:.4} (prev {:.4}) · H {:.4} · L {:.4} · C {:.4} · Δ {:+.3}%",
                wp.bars_used,
                wp.wclprice,
                wp.wclprice_prev,
                wp.high,
                wp.low,
                wp.close,
                wp.delta_pct
            );
            if !wp.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", wp.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(vr)) = rx::get_variance(ctx.conn, &sym_upper) {
        if vr.variance_label != "INSUFFICIENT_DATA" && !vr.variance_label.is_empty() {
            let _ = writeln!(
                p,
                "### Close Variance — VARIANCE ({}, as of {})",
                vr.variance_label, vr.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · period {} · mean {:.4} · variance {:.6} (prev {:.6}) · stddev {:.4} · CV {:.3}% · close {:.4}",
                vr.bars_used,
                vr.period,
                vr.mean,
                vr.variance,
                vr.variance_prev,
                vr.stddev,
                vr.cv,
                vr.last_close
            );
            if !vr.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", vr.note);
            }
            let _ = writeln!(p);
        }
    }
}
