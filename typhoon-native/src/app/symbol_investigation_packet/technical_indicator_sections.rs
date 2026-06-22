use super::context::SymbolResearchContext;
use std::fmt::Write as _;
use typhoon_engine::core::research as rx;

pub(super) fn write_symbol_technical_indicator_sections(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    super::technical_indicator_squeeze_breakouts::write_technical_indicator_squeeze_breakouts(
        ctx, p, sym_upper,
    );

    super::technical_indicator_cloud_trend::write_technical_indicator_cloud_trend(
        ctx, p, sym_upper,
    );

    super::technical_indicator_oscillators::write_technical_indicator_oscillators(
        ctx, p, sym_upper,
    );

    super::technical_indicator_volume_trend::write_technical_indicator_volume_trend(
        ctx, p, sym_upper,
    );

    super::technical_indicator_final_osc::write_technical_indicator_final_osc(ctx, p, sym_upper);

    if let Ok(Some(vw)) = rx::get_vwap(ctx.conn, &sym_upper) {
        if vw.vwap_label != "INSUFFICIENT_DATA" && !vw.vwap_label.is_empty() {
            let _ = writeln!(
                p,
                "### Volume-Weighted Average Price — VWAP ({}, as of {})",
                vw.vwap_label, vw.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · window {} · VWAP {:.4} · deviation {:+.2}% · close {:.4}",
                vw.bars_used, vw.window, vw.vwap_value, vw.deviation_pct, vw.last_close
            );
            if !vw.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", vw.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(mg)) = rx::get_mcgd(ctx.conn, &sym_upper) {
        if mg.mcgd_label != "INSUFFICIENT_DATA" && !mg.mcgd_label.is_empty() {
            let _ = writeln!(
                p,
                "### McGinley Dynamic — MCGD ({}, as of {})",
                mg.mcgd_label, mg.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · length {} · MCGD {:.4} · prev {:.4} · deviation {:+.2}% · close {:.4}",
                mg.bars_used,
                mg.length,
                mg.mcgd_value,
                mg.mcgd_prev,
                mg.deviation_pct,
                mg.last_close
            );
            if !mg.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", mg.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(rw)) = rx::get_rwi(ctx.conn, &sym_upper) {
        if rw.rwi_label != "INSUFFICIENT_DATA" && !rw.rwi_label.is_empty() {
            let _ = writeln!(
                p,
                "### Random Walk Index — RWI ({}, as of {})",
                rw.rwi_label, rw.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · length {} · RWI high {:.3} · RWI low {:.3} · close {:.4}",
                rw.bars_used, rw.length, rw.rwi_high, rw.rwi_low, rw.last_close
            );
            if !rw.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", rw.note);
            }
            let _ = writeln!(p);
        }
    }
}
