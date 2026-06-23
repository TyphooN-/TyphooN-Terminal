use super::context::SymbolResearchContext;
use std::fmt::Write as _;
use typhoon_engine::core::research as rx;

pub fn write_price_transform_adaptive_osc(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    // ── WMA / RAINBOW / MESA_SINE / FRAMA / IBS ──
    if let Ok(Some(wm)) = rx::get_wma(ctx.conn, &sym_upper) {
        if wm.wma_label != "INSUFFICIENT_DATA" && !wm.wma_label.is_empty() {
            let _ = writeln!(
                p,
                "### Weighted Moving Average — WMA ({}, as of {})",
                wm.wma_label, wm.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · length {} · WMA {:.4} (prev {:.4}) · SMA {:.4} · spread {:+.4} ({:+.3}%) · close {:.4}",
                wm.bars_used,
                wm.length,
                wm.wma_value,
                wm.wma_prev,
                wm.sma_value,
                wm.spread,
                wm.spread_pct * 100.0,
                wm.last_close
            );
            if !wm.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", wm.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(rb)) = rx::get_rainbow(ctx.conn, &sym_upper) {
        if rb.rainbow_label != "INSUFFICIENT_DATA" && !rb.rainbow_label.is_empty() {
            let _ = writeln!(
                p,
                "### Rainbow MA Oscillator — RAINBOW ({}, as of {})",
                rb.rainbow_label, rb.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · levels {} · highest {:.4} · lowest {:.4} · width {:.4} ({:.3}%) · center {:.4} · r1 {:.4} · r5 {:.4} · r10 {:.4} · close {:.4}",
                rb.bars_used,
                rb.levels,
                rb.highest_level,
                rb.lowest_level,
                rb.rainbow_width,
                rb.rainbow_width_pct * 100.0,
                rb.center_value,
                rb.r1,
                rb.r5,
                rb.r10,
                rb.last_close
            );
            if !rb.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", rb.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(ms)) = rx::get_mesa_sine(ctx.conn, &sym_upper) {
        if ms.mesa_label != "INSUFFICIENT_DATA" && !ms.mesa_label.is_empty() {
            let _ = writeln!(
                p,
                "### MESA Sine Wave — MESA_SINE ({}, as of {})",
                ms.mesa_label, ms.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · period {:.2} · phase {:+.4} rad · sine {:+.4} (prev {:+.4}) · lead_sine {:+.4} (prev {:+.4}) · close {:.4}",
                ms.bars_used,
                ms.period,
                ms.phase_rad,
                ms.sine_value,
                ms.sine_prev,
                ms.lead_sine,
                ms.lead_prev,
                ms.last_close
            );
            if !ms.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", ms.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(fm)) = rx::get_frama(ctx.conn, &sym_upper) {
        if fm.frama_label != "INSUFFICIENT_DATA" && !fm.frama_label.is_empty() {
            let _ = writeln!(
                p,
                "### Fractal Adaptive Moving Average — FRAMA ({}, as of {})",
                fm.frama_label, fm.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · length {} · D {:.4} · α {:.4} · FRAMA {:.4} (prev {:.4}) · spread {:+.4} · close {:.4}",
                fm.bars_used,
                fm.length,
                fm.fractal_dim,
                fm.alpha,
                fm.frama_value,
                fm.frama_prev,
                fm.spread,
                fm.last_close
            );
            if !fm.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", fm.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(ib)) = rx::get_ibs(ctx.conn, &sym_upper) {
        if ib.ibs_label != "INSUFFICIENT_DATA" && !ib.ibs_label.is_empty() {
            let _ = writeln!(
                p,
                "### Internal Bar Strength — IBS ({}, as of {})",
                ib.ibs_label, ib.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · length {} · IBS raw {:.4} · smoothed {:.4} (prev {:.4}) · bar H {:.4} L {:.4} C {:.4}",
                ib.bars_used,
                ib.length,
                ib.ibs_raw,
                ib.ibs_smoothed,
                ib.ibs_prev,
                ib.last_high,
                ib.last_low,
                ib.last_close
            );
            if !ib.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", ib.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(lr)) = rx::get_laguerre_rsi(ctx.conn, &sym_upper) {
        if lr.lrsi_label != "INSUFFICIENT_DATA" && !lr.lrsi_label.is_empty() {
            let _ = writeln!(
                p,
                "### Laguerre RSI — LAGUERRE_RSI ({}, as of {})",
                lr.lrsi_label, lr.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · γ {:.2} · L0 {:.6} L1 {:.6} L2 {:.6} L3 {:.6} · LRSI {:.4} (prev {:.4}) · close {:.4}",
                lr.bars_used,
                lr.gamma,
                lr.l0,
                lr.l1,
                lr.l2,
                lr.l3,
                lr.laguerre_rsi,
                lr.laguerre_rsi_prev,
                lr.last_close
            );
            if !lr.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", lr.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(zz)) = rx::get_zigzag(ctx.conn, &sym_upper) {
        if zz.zigzag_label != "INSUFFICIENT_DATA" && !zz.zigzag_label.is_empty() {
            let _ = writeln!(
                p,
                "### ZigZag Pattern — ZIGZAG ({}, as of {})",
                zz.zigzag_label, zz.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · threshold {:.2}% · leg {} · last high {:.4} ({} bars ago) · last low {:.4} ({} bars ago) · reversal at {:.4} · close {:.4}",
                zz.bars_used,
                zz.threshold_pct,
                zz.current_leg,
                zz.last_high_value,
                zz.last_high_bars_ago,
                zz.last_low_value,
                zz.last_low_bars_ago,
                zz.reversal_level,
                zz.last_close
            );
            if !zz.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", zz.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(pg)) = rx::get_pgo(ctx.conn, &sym_upper) {
        if pg.pgo_label != "INSUFFICIENT_DATA" && !pg.pgo_label.is_empty() {
            let _ = writeln!(
                p,
                "### Pretty Good Oscillator — PGO ({}, as of {})",
                pg.pgo_label, pg.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · length {} · SMA {:.4} · ATR {:.4} · PGO {:.4} (prev {:.4}) · close {:.4}",
                pg.bars_used,
                pg.length,
                pg.sma_value,
                pg.atr_value,
                pg.pgo_value,
                pg.pgo_prev,
                pg.last_close
            );
            if !pg.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", pg.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(ht)) = rx::get_ht_trendline(ctx.conn, &sym_upper) {
        if ht.ht_label != "INSUFFICIENT_DATA" && !ht.ht_label.is_empty() {
            let _ = writeln!(
                p,
                "### Hilbert Instantaneous Trendline — HT_TRENDLINE ({}, as of {})",
                ht.ht_label, ht.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · detected period {:.2} · trendline {:.4} (prev {:.4}) · spread {:.4} ({:+.3}%) · close {:.4}",
                ht.bars_used,
                ht.period,
                ht.trendline_value,
                ht.trendline_prev,
                ht.spread,
                ht.spread_pct * 100.0,
                ht.last_close
            );
            if !ht.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", ht.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(mp)) = rx::get_midpoint(ctx.conn, &sym_upper) {
        if mp.midpoint_label != "INSUFFICIENT_DATA" && !mp.midpoint_label.is_empty() {
            let _ = writeln!(
                p,
                "### Midpoint of N — MIDPOINT ({}, as of {})",
                mp.midpoint_label, mp.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · length {} · HHV {:.4} · LLV {:.4} · midpoint {:.4} (prev {:.4}) · close position {:.4} · close {:.4}",
                mp.bars_used,
                mp.length,
                mp.hhv,
                mp.llv,
                mp.midpoint,
                mp.midpoint_prev,
                mp.close_position,
                mp.last_close
            );
            if !mp.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", mp.note);
            }
            let _ = writeln!(p);
        }
    }
}
