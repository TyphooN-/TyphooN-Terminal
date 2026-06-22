use super::context::SymbolResearchContext;
use std::fmt::Write as _;
use typhoon_engine::core::research as rx;

pub(super) fn write_technical_indicator_cloud_trend(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    if let Ok(Some(ik)) = rx::get_ichimoku(ctx.conn, &sym_upper) {
        if ik.ichimoku_label != "INSUFFICIENT_DATA" && !ik.ichimoku_label.is_empty() {
            let _ = writeln!(
                p,
                "### Ichimoku Cloud — ICHIMOKU ({}, as of {})",
                ik.ichimoku_label, ik.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · tenkan={:.4} · kijun={:.4} · senkou A={:.4} · senkou B={:.4} · chikou={:.4}",
                ik.bars_used,
                ik.tenkan_sen,
                ik.kijun_sen,
                ik.senkou_span_a,
                ik.senkou_span_b,
                ik.chikou_span
            );
            let _ = writeln!(
                p,
                "- Cloud top={:.4} · bottom={:.4} · close={:.4} · close vs cloud={:+.2}%",
                ik.cloud_top, ik.cloud_bottom, ik.last_close, ik.close_vs_cloud_pct
            );
            if !ik.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", ik.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(st)) = rx::get_supertrend(ctx.conn, &sym_upper) {
        if st.supertrend_label != "INSUFFICIENT_DATA" && !st.supertrend_label.is_empty() {
            let _ = writeln!(
                p,
                "### Supertrend ATR Stop — SUPERTREND ({}, as of {})",
                st.supertrend_label, st.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · period {} · mult {:.1} · ATR={:.4} · upper={:.4} · lower={:.4}",
                st.bars_used, st.period, st.multiplier, st.atr, st.upper_band, st.lower_band
            );
            let _ = writeln!(
                p,
                "- Active ST={:.4} · trend={} · close={:.4} · dist={:+.2}% · bars in trend={}",
                st.supertrend_value,
                if st.trend_is_up { "UP" } else { "DOWN" },
                st.last_close,
                st.distance_pct,
                st.bars_in_trend
            );
            if !st.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", st.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(kc)) = rx::get_keltner(ctx.conn, &sym_upper) {
        if kc.keltner_label != "INSUFFICIENT_DATA" && !kc.keltner_label.is_empty() {
            let _ = writeln!(
                p,
                "### Keltner Channels — KELTNER ({}, as of {})",
                kc.keltner_label, kc.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · EMA{} / ATR{} · mult {:.1} · EMA={:.4} · ATR={:.4}",
                kc.bars_used, kc.ema_period, kc.atr_period, kc.multiplier, kc.ema_value, kc.atr
            );
            let _ = writeln!(
                p,
                "- Upper={:.4} · lower={:.4} · width={:.4} · width %={:.2} · close={:.4} · pos={:.1}% · TTM squeeze={}",
                kc.upper_channel,
                kc.lower_channel,
                kc.channel_width,
                kc.width_pct_of_mid,
                kc.last_close,
                kc.channel_position_pct,
                kc.ttm_squeeze_on
            );
            if !kc.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", kc.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(fs)) = rx::get_fisher(ctx.conn, &sym_upper) {
        if fs.fisher_label != "INSUFFICIENT_DATA" && !fs.fisher_label.is_empty() {
            let _ = writeln!(
                p,
                "### Fisher Transform — FISHER ({}, as of {})",
                fs.fisher_label, fs.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · period {} · fisher={:+.3} · signal={:+.3} · peak |f| 10={:.3} · ±2 cross last 3={} · close={:.4}",
                fs.bars_used,
                fs.period,
                fs.fisher_value,
                fs.fisher_signal,
                fs.peak_abs_10,
                fs.extreme_2_cross,
                fs.last_close
            );
            if !fs.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", fs.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(ar)) = rx::get_aroon(ctx.conn, &sym_upper) {
        if ar.aroon_label != "INSUFFICIENT_DATA" && !ar.aroon_label.is_empty() {
            let _ = writeln!(
                p,
                "### Aroon — AROON ({}, as of {})",
                ar.aroon_label, ar.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · period {} · up={:.2} · down={:.2} · osc={:+.2} · bars since high={} · bars since low={} · close={:.4}",
                ar.bars_used,
                ar.period,
                ar.aroon_up,
                ar.aroon_down,
                ar.aroon_oscillator,
                ar.bars_since_high,
                ar.bars_since_low,
                ar.last_close
            );
            if !ar.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", ar.note);
            }
            let _ = writeln!(p);
        }
    }

    // ── Research section ──
}
