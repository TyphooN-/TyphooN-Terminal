use super::context::SymbolResearchContext;
use std::fmt::Write as _;
use typhoon_engine::core::research as rx;

pub fn write_talib_dmi_movement(ctx: &SymbolResearchContext, p: &mut String, sym_upper: &str) {
    // ── (DMI family) ──
    if let Ok(Some(pd)) = rx::get_plus_di(ctx.conn, &sym_upper) {
        if pd.plus_di_label != "INSUFFICIENT_DATA" && !pd.plus_di_label.is_empty() {
            let _ = writeln!(
                p,
                "### Positive Directional Indicator — PLUS_DI ({}, as of {})",
                pd.plus_di_label, pd.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · period {} · +DI {:.3} (prev {:.3}) · -DI {:.3} · ATR {:.4} · close {:.4}",
                pd.bars_used,
                pd.period,
                pd.plus_di,
                pd.plus_di_prev,
                pd.minus_di,
                pd.atr,
                pd.last_close
            );
            if !pd.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", pd.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(md)) = rx::get_minus_di(ctx.conn, &sym_upper) {
        if md.minus_di_label != "INSUFFICIENT_DATA" && !md.minus_di_label.is_empty() {
            let _ = writeln!(
                p,
                "### Negative Directional Indicator — MINUS_DI ({}, as of {})",
                md.minus_di_label, md.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · period {} · -DI {:.3} (prev {:.3}) · +DI {:.3} · ATR {:.4} · close {:.4}",
                md.bars_used,
                md.period,
                md.minus_di,
                md.minus_di_prev,
                md.plus_di,
                md.atr,
                md.last_close
            );
            if !md.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", md.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(pm)) = rx::get_plus_dm(ctx.conn, &sym_upper) {
        if pm.plus_dm_label != "INSUFFICIENT_DATA" && !pm.plus_dm_label.is_empty() {
            let _ = writeln!(
                p,
                "### Positive Directional Movement — PLUS_DM ({}, as of {})",
                pm.plus_dm_label, pm.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · period {} · +DM raw {:.4} · +DM smoothed {:.4} (prev {:.4}) · up {:+.4} · dn {:+.4} · close {:.4}",
                pm.bars_used,
                pm.period,
                pm.plus_dm_raw,
                pm.plus_dm_smoothed,
                pm.plus_dm_smoothed_prev,
                pm.up_move,
                pm.down_move,
                pm.last_close
            );
            if !pm.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", pm.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(mm)) = rx::get_minus_dm(ctx.conn, &sym_upper) {
        if mm.minus_dm_label != "INSUFFICIENT_DATA" && !mm.minus_dm_label.is_empty() {
            let _ = writeln!(
                p,
                "### Negative Directional Movement — MINUS_DM ({}, as of {})",
                mm.minus_dm_label, mm.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · period {} · -DM raw {:.4} · -DM smoothed {:.4} (prev {:.4}) · up {:+.4} · dn {:+.4} · close {:.4}",
                mm.bars_used,
                mm.period,
                mm.minus_dm_raw,
                mm.minus_dm_smoothed,
                mm.minus_dm_smoothed_prev,
                mm.up_move,
                mm.down_move,
                mm.last_close
            );
            if !mm.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", mm.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(dxr)) = rx::get_dx(ctx.conn, &sym_upper) {
        if dxr.dx_label != "INSUFFICIENT_DATA" && !dxr.dx_label.is_empty() {
            let _ = writeln!(
                p,
                "### Directional Movement Index — DX ({}, as of {})",
                dxr.dx_label, dxr.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · period {} · DX {:.3} (prev {:.3}) · +DI {:.3} · -DI {:.3} · close {:.4}",
                dxr.bars_used,
                dxr.period,
                dxr.dx,
                dxr.dx_prev,
                dxr.plus_di,
                dxr.minus_di,
                dxr.last_close
            );
            if !dxr.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", dxr.note);
            }
            let _ = writeln!(p);
        }
    }
}
