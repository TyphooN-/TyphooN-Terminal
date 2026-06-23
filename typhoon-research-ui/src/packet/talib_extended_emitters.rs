use super::context::SymbolResearchContext;
use std::fmt::Write as _;
use typhoon_engine::core::research as rx;

pub fn write_talib_extended_emitters(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    // ── emitters ──
    if let Ok(Some(ao)) = rx::get_aroonosc(ctx.conn, &sym_upper) {
        if ao.aroonosc_label != "INSUFFICIENT_DATA" && !ao.aroonosc_label.is_empty() {
            let _ = writeln!(
                p,
                "### Aroon Oscillator — AROONOSC ({}, as of {})",
                ao.aroonosc_label, ao.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · period {} · osc {:+.2} (prev {:+.2}) · up {:.2} · down {:.2} · close {:.4}",
                ao.bars_used,
                ao.period,
                ao.aroonosc,
                ao.aroonosc_prev,
                ao.aroon_up,
                ao.aroon_down,
                ao.last_close
            );
            if !ao.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", ao.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(mmi)) = rx::get_minmaxindex(ctx.conn, &sym_upper) {
        if mmi.minmaxindex_label != "INSUFFICIENT_DATA" && !mmi.minmaxindex_label.is_empty() {
            let _ = writeln!(
                p,
                "### Min/Max Index — MINMAXINDEX ({}, as of {})",
                mmi.minmaxindex_label, mmi.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · period {} · low {} ago · high {} ago · age_diff {:+} · order {} · close {:.4}",
                mmi.bars_used,
                mmi.period,
                mmi.min_index_bars_ago,
                mmi.max_index_bars_ago,
                mmi.age_diff,
                mmi.extrema_order,
                mmi.last_close
            );
            if !mmi.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", mmi.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(me)) = rx::get_macdext(ctx.conn, &sym_upper) {
        if me.macdext_label != "INSUFFICIENT_DATA" && !me.macdext_label.is_empty() {
            let _ = writeln!(
                p,
                "### MACD Extended — MACDEXT ({}, as of {})",
                me.macdext_label, me.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · {}/{}/{} · ma_type {} · macd {:+.6} · signal {:+.6} · hist {:+.6} (prev {:+.6}) · close {:.4}",
                me.bars_used,
                me.fast_period,
                me.slow_period,
                me.signal_period,
                me.ma_type,
                me.macd,
                me.signal,
                me.hist,
                me.hist_prev,
                me.last_close
            );
            if !me.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", me.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(mf)) = rx::get_macdfix(ctx.conn, &sym_upper) {
        if mf.macdfix_label != "INSUFFICIENT_DATA" && !mf.macdfix_label.is_empty() {
            let _ = writeln!(
                p,
                "### MACD Fix — MACDFIX ({}, as of {})",
                mf.macdfix_label, mf.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · {}/{}/{} · macd {:+.6} · signal {:+.6} · hist {:+.6} (prev {:+.6}) · close {:.4}",
                mf.bars_used,
                mf.fast_period,
                mf.slow_period,
                mf.signal_period,
                mf.macd,
                mf.signal,
                mf.hist,
                mf.hist_prev,
                mf.last_close
            );
            if !mf.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", mf.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(mv)) = rx::get_mavp(ctx.conn, &sym_upper) {
        if mv.mavp_label != "INSUFFICIENT_DATA" && !mv.mavp_label.is_empty() {
            let _ = writeln!(
                p,
                "### Moving Avg Variable Period — MAVP ({}, as of {})",
                mv.mavp_label, mv.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · periods {}..{} · last_period {} · mavp {:.6} (prev {:.6}, Δ {:+.6}) · close {:.4}",
                mv.bars_used,
                mv.min_period,
                mv.max_period,
                mv.last_bar_period,
                mv.mavp,
                mv.mavp_prev,
                mv.mavp_delta,
                mv.last_close
            );
            if !mv.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", mv.note);
            }
            let _ = writeln!(p);
        }
    }
}
