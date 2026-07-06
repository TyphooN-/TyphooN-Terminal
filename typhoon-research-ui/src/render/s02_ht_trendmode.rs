// Segment of the research snapshot renderers — see render.rs (this file is
// order-preserving; segments are mechanical splits, not semantic groups).
#[allow(unused_imports)]
use crate::theme::{AXIS_TEXT, BTN_GREEN_TEXT, BTN_RED_TEXT, DOWN, UP};
#[allow(unused_imports)]
use typhoon_engine::core::research::*;

pub fn render_ht_trendmode_snapshot(ui: &mut egui::Ui, snap: &HtTrendmodeSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.mode_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥64 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.mode_label.as_str() {
            "TREND" => UP,
            "CYCLE" => AXIS_TEXT,
            _ => AXIS_TEXT,
        };
        ui.label(egui::RichText::new(format!("{} — {} — trendmode {} (prev {}) — lock_in_bars {} — period {:.2} — close {:.4} — as of {}",
            snap.symbol, snap.mode_label, snap.trendmode, snap.trendmode_prev, snap.lock_in_bars, snap.period, snap.last_close, snap.as_of)).strong().color(color));
        ui.separator();
        egui::Grid::new("ht_trendmode_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Trendmode").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.trendmode))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Trendmode prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.trendmode_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Lock-in bars").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.lock_in_bars))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Period").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.period))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.note.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
        }
    }
}

pub fn render_ibs_snapshot(ui: &mut egui::Ui, snap: &IbsSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.ibs_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥15 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.ibs_label.as_str() {
            "OVERBOUGHT" => DOWN,
            "OVERSOLD" => UP,
            "BULL" => UP,
            "BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — raw {:.4} · smoothed {:.4} — close {:.4} — as of {}",
                snap.symbol,
                snap.ibs_label,
                snap.ibs_raw,
                snap.ibs_smoothed,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("ibs_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Length").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.length))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("IBS (raw)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.ibs_raw))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("IBS (smoothed)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.ibs_smoothed))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("IBS prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.ibs_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last high").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_high))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last low").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_low))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.note.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
        }
    }
}

pub fn render_ichimoku_snapshot(ui: &mut egui::Ui, snap: &IchimokuSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.ichimoku_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥78 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.ichimoku_label.as_str() {
            "STRONG_BULL" | "BULL" => UP,
            "STRONG_BEAR" | "BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — close vs cloud {:+.2}% — as of {}",
                snap.symbol, snap.ichimoku_label, snap.close_vs_cloud_pct, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("ichi_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(180.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Tenkan (9)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.tenkan_sen))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Kijun (26)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.kijun_sen))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Senkou A").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.senkou_span_a))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Senkou B (52)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.senkou_span_b))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Cloud top").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.cloud_top))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Cloud bottom").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.cloud_bottom))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Chikou").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.chikou_span))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub fn render_kama_snapshot(ui: &mut egui::Ui, snap: &KamaSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.kama_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥25 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.kama_label.as_str() {
            "STRONG_TREND" | "MODERATE_TREND" => UP,
            "CHOPPY" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — ER {:.3} — slope {:+.2}% — as of {}",
                snap.symbol,
                snap.kama_label,
                snap.efficiency_ratio,
                snap.kama_slope_pct,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("kama_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(180.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Period").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.period))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Efficiency ratio").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.efficiency_ratio))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("KAMA value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.kama_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("KAMA 5-bar slope %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}", snap.kama_slope_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub fn render_keltner_snapshot(ui: &mut egui::Ui, snap: &KeltnerSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.keltner_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥22 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.keltner_label.as_str() {
            "BREAKOUT_UP" => UP,
            "BREAKOUT_DOWN" => DOWN,
            _ => AXIS_TEXT,
        };
        let ttm = if snap.ttm_squeeze_on {
            " • TTM SQUEEZE ON"
        } else {
            ""
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — pos {:.1}%{} — as of {}",
                snap.symbol, snap.keltner_label, snap.channel_position_pct, ttm, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("kelt_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("EMA / ATR periods").small().strong());
                ui.label(
                    egui::RichText::new(format!("{} / {}", snap.ema_period, snap.atr_period))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Multiplier").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}", snap.multiplier))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("EMA midline").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.ema_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("ATR").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.atr))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Upper").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.upper_channel))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Lower").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.lower_channel))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Width").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.channel_width))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Width % of mid").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.width_pct_of_mid))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("TTM squeeze").small().strong());
                ui.label(
                    egui::RichText::new(if snap.ttm_squeeze_on { "YES" } else { "no" })
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub fn render_klinger_snapshot(ui: &mut egui::Ui, snap: &KlingerSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.klinger_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥71 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.klinger_label.as_str() {
            "STRONG_BULL" | "BULL" => UP,
            "STRONG_BEAR" | "BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — KVO {:+.1} — signal {:+.1} — hist {:+.1} — as of {}",
                snap.symbol,
                snap.klinger_label,
                snap.kvo_value,
                snap.signal_value,
                snap.histogram,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("klinger_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Fast / slow / signal").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {} / {}",
                        snap.fast_period, snap.slow_period, snap.signal_period
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("EMA fast VF").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.ema_fast_vf))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("EMA slow VF").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.ema_slow_vf))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("KVO").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}", snap.kvo_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Signal (EMA-13)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}", snap.signal_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Histogram").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}", snap.histogram))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub fn render_kst_snapshot(ui: &mut egui::Ui, snap: &KstSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.kst_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥56 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.kst_label.as_str() {
            "STRONG_BULL" | "BULL" => UP,
            "STRONG_BEAR" | "BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — KST {:+.3} — signal {:+.3} — hist {:+.3} — as of {}",
                snap.symbol,
                snap.kst_label,
                snap.kst_value,
                snap.signal_value,
                snap.histogram,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("kst_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("RCMA1 (ROC10/SMA10)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.rcma1))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("RCMA2 (ROC15/SMA10)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.rcma2))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("RCMA3 (ROC20/SMA10)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.rcma3))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("RCMA4 (ROC30/SMA15)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.rcma4))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("KST").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.kst_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Signal (SMA-9)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.signal_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Histogram").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.histogram))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub fn render_laguerre_rsi_snapshot(ui: &mut egui::Ui, snap: &LaguerreRsiSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.lrsi_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥20 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.lrsi_label.as_str() {
            "OVERBOUGHT" => DOWN,
            "OVERSOLD" => UP,
            "BULL" => UP,
            "BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — LRSI {:.4} (prev {:.4}) — close {:.4} — as of {}",
                snap.symbol,
                snap.lrsi_label,
                snap.laguerre_rsi,
                snap.laguerre_rsi_prev,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("laguerre_rsi_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("γ (gamma)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.gamma))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("L0").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.l0))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("L1").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.l1))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("L2").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.l2))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("L3").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.l3))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Laguerre RSI").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.laguerre_rsi))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Laguerre RSI prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.laguerre_rsi_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.note.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
        }
    }
}

pub fn render_linearreg_angle_snapshot(ui: &mut egui::Ui, snap: &LinearregAngleSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.angle_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥15 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.angle_label.as_str() {
            "STRONG_UP" | "UP" => UP,
            "STRONG_DOWN" | "DOWN" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — slope {:+.6} — angle {:+.3}° (prev {:+.3}°) — close {:.4} — as of {}",
                snap.symbol,
                snap.angle_label,
                snap.slope,
                snap.angle_deg,
                snap.angle_deg_prev,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("linearreg_angle_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Length").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.length))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Slope").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.6}", snap.slope))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Angle (deg)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}°", snap.angle_deg))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Angle prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}°", snap.angle_deg_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.note.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
        }
    }
}

pub fn render_linearreg_slope_snapshot(ui: &mut egui::Ui, snap: &LinearregSlopeSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.slope_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥15 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.slope_label.as_str() {
            "STRONG_UP" | "UP" => UP,
            "STRONG_DOWN" | "DOWN" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — slope {:+.6} (prev {:+.6}) — slope_pct {:+.3}% — close {:.4} — as of {}",
                snap.symbol,
                snap.slope_label,
                snap.slope,
                snap.slope_prev,
                snap.slope_pct,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("linearreg_slope_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Length").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.length))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Slope").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.6}", snap.slope))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Slope prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.6}", snap.slope_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Slope % of close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}%", snap.slope_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.note.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
        }
    }
}

pub fn render_linearreg_snapshot(ui: &mut egui::Ui, snap: &LinearregSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.linearreg_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥15 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.linearreg_label.as_str() {
            "ABOVE_TREND" => UP,
            "BELOW_TREND" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(egui::RichText::new(format!("{} — {} — fitted {:.4} (prev {:.4}) — residual {:+.4} ({:+.3}%) — close {:.4} — as of {}",
            snap.symbol, snap.linearreg_label, snap.fitted, snap.fitted_prev, snap.residual, snap.residual_pct, snap.last_close, snap.as_of)).strong().color(color));
        ui.separator();
        egui::Grid::new("linearreg_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Length").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.length))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Fitted").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.fitted))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Fitted prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.fitted_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Residual").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.residual))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Residual %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}%", snap.residual_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.note.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
        }
    }
}

pub fn render_linreg_snapshot(ui: &mut egui::Ui, snap: &LinregSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.linreg_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥20 bars with OHLC.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.linreg_label.as_str() {
            "STRONG_UP_TREND" | "UP_TREND" => UP,
            "STRONG_DOWN_TREND" | "DOWN_TREND" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — slope {:+.5} — R² {:.3} — close {:.4} — as of {}",
                snap.symbol,
                snap.linreg_label,
                snap.slope,
                snap.r_squared,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("linreg_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Length").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.length))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Slope").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.5}", snap.slope))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Intercept").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.intercept))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("R²").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.r_squared))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("σ (residual)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.sigma))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Fit value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.fit_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Channel upper (+2σ)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.channel_upper))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Channel lower (−2σ)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.channel_lower))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub fn render_macdext_snapshot(ui: &mut egui::Ui, snap: &MacdextSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.macdext_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥37 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.macdext_label.as_str() {
            "STRONG_BULL" | "BULL" => UP,
            "BEAR" | "STRONG_BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — macd {:+.4} — sig {:+.4} — hist {:+.4} — close {:.4} — as of {}",
                snap.symbol,
                snap.macdext_label,
                snap.macd,
                snap.signal,
                snap.hist,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("macdext_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("MA type").small().strong());
                ui.label(egui::RichText::new(&snap.ma_type).small().monospace());
                ui.end_row();
                ui.label(egui::RichText::new("Fast / slow / signal").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{}/{}/{}",
                        snap.fast_period, snap.slow_period, snap.signal_period
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("MACD").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.6}", snap.macd))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Signal").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.6}", snap.signal))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Histogram").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.6}", snap.hist))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Hist prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.6}", snap.hist_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.note.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
        }
    }
}

pub fn render_macdfix_snapshot(ui: &mut egui::Ui, snap: &MacdfixSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.macdfix_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥37 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.macdfix_label.as_str() {
            "STRONG_BULL" | "BULL" => UP,
            "BEAR" | "STRONG_BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — macd {:+.4} — sig {:+.4} — hist {:+.4} — close {:.4} — as of {}",
                snap.symbol,
                snap.macdfix_label,
                snap.macd,
                snap.signal,
                snap.hist,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("macdfix_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Fast / slow (fixed)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}/{}", snap.fast_period, snap.slow_period))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Signal period").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.signal_period))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("MACD").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.6}", snap.macd))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Signal").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.6}", snap.signal))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Histogram").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.6}", snap.hist))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Hist prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.6}", snap.hist_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.note.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
        }
    }
}

pub fn render_mass_index_snapshot(ui: &mut egui::Ui, snap: &MassIndexSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.mass_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥35 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.mass_label.as_str() {
            "REVERSAL_BULGE" => DOWN,
            "ELEVATED" => UP,
            "COMPRESSED" => AXIS_TEXT,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — MI {:.3} (prev {:.3}) — ratio {:.4} — close {:.4} — as of {}",
                snap.symbol,
                snap.mass_label,
                snap.mass_index,
                snap.mass_index_prev,
                snap.ratio,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("mass_index_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("EMA length").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.ema_len))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Sum length").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.sum_len))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("EMA(H-L)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.ema_range))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("EMA-of-EMA(H-L)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.ema_ema_range))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Ratio").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.ratio))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Mass Index").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.mass_index))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Mass Index prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.mass_index_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.note.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
        }
    }
}

pub fn render_mass_snapshot(ui: &mut egui::Ui, snap: &MassSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.mass_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥45 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.mass_label.as_str() {
            "REVERSAL_BULGE" => DOWN,
            "WATCH" => AXIS_TEXT,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — Mass {:.3} — ratio {:.3} — as of {}",
                snap.symbol, snap.mass_label, snap.mass_value, snap.single_ratio, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("mass_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("EMA / sum").small().strong());
                ui.label(
                    egui::RichText::new(format!("{} / {}", snap.ema_period, snap.sum_period))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Single ratio").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.single_ratio))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Mass value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.mass_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub fn render_mavp_snapshot(ui: &mut egui::Ui, snap: &MavpSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.mavp_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥32 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.mavp_label.as_str() {
            "STRONG_UP" | "UP" => UP,
            "DOWN" | "STRONG_DOWN" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — mavp {:.4} — Δ {:+.4} — last period {} — close {:.4} — as of {}",
                snap.symbol,
                snap.mavp_label,
                snap.mavp,
                snap.mavp_delta,
                snap.last_bar_period,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("mavp_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Period range").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}..{}", snap.min_period, snap.max_period))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last-bar period").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.last_bar_period))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("MAVP").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.mavp))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("MAVP prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.mavp_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Delta").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.6}", snap.mavp_delta))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.note.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
        }
    }
}

pub fn render_mesa_sine_snapshot(ui: &mut egui::Ui, snap: &MesaSineSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.mesa_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥32 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.mesa_label.as_str() {
            "CYCLE_BUY" => UP,
            "CYCLE_SELL" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — sine {:+.4} · lead {:+.4} · phase {:+.4} rad — close {:.4} — as of {}",
                snap.symbol,
                snap.mesa_label,
                snap.sine_value,
                snap.lead_sine,
                snap.phase_rad,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("mesa_sine_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Period (bars)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.period))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Phase (rad)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.phase_rad))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Sine value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.sine_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Sine prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.sine_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Lead sine").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.lead_sine))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Lead prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.lead_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.note.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
        }
    }
}

pub fn render_mfi_snapshot(ui: &mut egui::Ui, snap: &MfiSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.mfi_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥15 bars with volume.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.mfi_label.as_str() {
            "OVERBOUGHT" => DOWN,
            "OVERSOLD" => UP,
            "BULL" => UP,
            "BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — MFI {:.2} — ratio {:.3} — as of {}",
                snap.symbol, snap.mfi_label, snap.mfi_value, snap.money_flow_ratio, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("mfi_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Period").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.period))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("MFI value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.mfi_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("+MF sum").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.0}", snap.positive_mf_sum))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("−MF sum").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.0}", snap.negative_mf_sum))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("MF ratio").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.money_flow_ratio))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub fn render_midpoint_snapshot(ui: &mut egui::Ui, snap: &MidpointSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.midpoint_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥15 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.midpoint_label.as_str() {
            "UPPER" | "NEAR_UPPER" => UP,
            "LOWER" | "NEAR_LOWER" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — midpoint {:.4} (prev {:.4}) — close pos {:.4} — close {:.4} — as of {}",
                snap.symbol,
                snap.midpoint_label,
                snap.midpoint,
                snap.midpoint_prev,
                snap.close_position,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("midpoint_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Length").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.length))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("HHV").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.hhv))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("LLV").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.llv))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Midpoint").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.midpoint))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Midpoint prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.midpoint_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Close position [0-1]").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.close_position))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.note.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
        }
    }
}

pub fn render_midprice_snapshot(ui: &mut egui::Ui, snap: &MidpriceSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.midprice_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥15 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.midprice_label.as_str() {
            "NEAR_HIGH" | "ABOVE_MID" => UP,
            "NEAR_LOW" | "BELOW_MID" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(egui::RichText::new(format!("{} — {} — midprice {:.4} — HHV {:.4} — LLV {:.4} — position {:.3} — close {:.4} — as of {}",
            snap.symbol, snap.midprice_label, snap.midprice, snap.hhv, snap.llv, snap.position, snap.last_close, snap.as_of)).strong().color(color));
        ui.separator();
        egui::Grid::new("midprice_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Length").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.length))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Midprice").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.midprice))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Midprice prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.midprice_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("HHV").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.hhv))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("LLV").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.llv))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Position").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.position))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.note.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
        }
    }
}

pub fn render_minmaxindex_snapshot(ui: &mut egui::Ui, snap: &MinMaxIndexSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.minmaxindex_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥31 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.minmaxindex_label.as_str() {
            "FRESH_HIGH" => UP,
            "FRESH_LOW" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — low {} ago / high {} ago — order {} — close {:.4} — as of {}",
                snap.symbol,
                snap.minmaxindex_label,
                snap.min_index_bars_ago,
                snap.max_index_bars_ago,
                snap.extrema_order,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("minmaxindex_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Period").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.period))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Min bars ago").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.min_index_bars_ago))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Max bars ago").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.max_index_bars_ago))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Age diff (min−max)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+}", snap.age_diff))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Extrema order").small().strong());
                ui.label(egui::RichText::new(&snap.extrema_order).small().monospace());
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.note.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
        }
    }
}

pub fn render_minus_di_snapshot(ui: &mut egui::Ui, snap: &MinusDiSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.minus_di_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥16 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.minus_di_label.as_str() {
            "BEAR_DOMINANT" | "BEAR_LEAN" => DOWN,
            "BULL_LEAN" => UP,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — -DI {:.3} — +DI {:.3} — ATR {:.4} — close {:.4} — as of {}",
                snap.symbol,
                snap.minus_di_label,
                snap.minus_di,
                snap.plus_di,
                snap.atr,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("minus_di_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Period").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.period))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("−DI").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.minus_di))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("−DI prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.minus_di_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("+DI (ref)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.plus_di))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("ATR").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.atr))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.note.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
        }
    }
}

pub fn render_minus_dm_snapshot(ui: &mut egui::Ui, snap: &MinusDmSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.minus_dm_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥16 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.minus_dm_label.as_str() {
            "BEAR_PRESSURE" | "BEAR_SOFT" => DOWN,
            "BULL_PRESSURE" => UP,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — −DM raw {:.4} — −DM smoothed {:.4} — close {:.4} — as of {}",
                snap.symbol,
                snap.minus_dm_label,
                snap.minus_dm_raw,
                snap.minus_dm_smoothed,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("minus_dm_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Period").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.period))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("−DM raw").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.minus_dm_raw))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("−DM smoothed").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.minus_dm_smoothed))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("−DM smoothed prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.minus_dm_smoothed_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Up-move (H − H_prev)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.up_move))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Down-move (L_prev − L)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.down_move))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.note.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
        }
    }
}

pub fn render_mom_snapshot(ui: &mut egui::Ui, snap: &MomSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.mom_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥12 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.mom_label.as_str() {
            "STRONG_UP" | "UP" => UP,
            "STRONG_DOWN" | "DOWN" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — MOM {:+.4} — MOM% {:+.3} — close {:.4} — as of {}",
                snap.symbol, snap.mom_label, snap.mom, snap.mom_pct, snap.last_close, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("mom_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Period").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.period))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("MOM").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.mom))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("MOM prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.mom_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("MOM %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}", snap.mom_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.note.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
        }
    }
}

pub fn render_natr_snapshot(ui: &mut egui::Ui, snap: &NatrSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.natr_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥15 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.natr_label.as_str() {
            "HIGH_VOL" => DOWN,
            "ELEVATED" => UP,
            "LOW_VOL" => AXIS_TEXT,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — NATR {:.3}% (prev {:.3}%) — ATR {:.4} — close {:.4} — as of {}",
                snap.symbol,
                snap.natr_label,
                snap.natr_value,
                snap.natr_prev,
                snap.atr_value,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("natr_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Length").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.length))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("ATR").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.atr_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("NATR %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.natr_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("NATR prev %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.natr_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.note.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
        }
    }
}

pub fn render_obv_snapshot(ui: &mut egui::Ui, snap: &ObvSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.obv_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥21 bars with volume.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.obv_label.as_str() {
            "STRONG_UP" | "UP" => UP,
            "STRONG_DOWN" | "DOWN" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — OBV {:.0} — Δ {:+.2}% — as of {}",
                snap.symbol, snap.obv_label, snap.obv_value, snap.obv_change_pct, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("obv_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Slope window").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.slope_window))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("OBV value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.0}", snap.obv_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("20-bar slope").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}", snap.obv_slope))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("20-bar change").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.obv_change_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("20-bar min").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.0}", snap.obv_min_20))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("20-bar max").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.0}", snap.obv_max_20))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub fn render_pgo_snapshot(ui: &mut egui::Ui, snap: &PgoSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.pgo_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥16 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.pgo_label.as_str() {
            "STRONG_BULL" | "BULL" => UP,
            "STRONG_BEAR" | "BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — PGO {:.4} (prev {:.4}) — SMA {:.4} · ATR {:.4} — close {:.4} — as of {}",
                snap.symbol,
                snap.pgo_label,
                snap.pgo_value,
                snap.pgo_prev,
                snap.sma_value,
                snap.atr_value,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("pgo_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Length").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.length))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("SMA value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.sma_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("ATR (EMA of TR)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.atr_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("PGO value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.pgo_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("PGO prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.pgo_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.note.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
        }
    }
}

pub fn render_pivots_snapshot(ui: &mut egui::Ui, snap: &PivotsSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.pivots_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥2 bars with OHLC.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.pivots_label.as_str() {
            s if s.starts_with("ABOVE_R") || s.starts_with("BETWEEN_R") => UP,
            s if s.starts_with("BELOW_S") || s.starts_with("BETWEEN_S") => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — PP {:.4} — close {:.4} — as of {}",
                snap.symbol, snap.pivots_label, snap.pp, snap.last_close, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("pivots_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("R2").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.r2))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("R1").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.r1))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("PP").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.pp))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("S1").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.s1))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("S2").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.s2))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Prior high").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.prior_high))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Prior low").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.prior_low))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Prior close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.prior_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub fn render_plus_di_snapshot(ui: &mut egui::Ui, snap: &PlusDiSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.plus_di_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥16 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.plus_di_label.as_str() {
            "BULL_DOMINANT" | "BULL_LEAN" => UP,
            "BEAR_LEAN" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — +DI {:.3} — -DI {:.3} — ATR {:.4} — close {:.4} — as of {}",
                snap.symbol,
                snap.plus_di_label,
                snap.plus_di,
                snap.minus_di,
                snap.atr,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("plus_di_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Period").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.period))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("+DI").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.plus_di))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("+DI prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.plus_di_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("−DI (ref)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.minus_di))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("ATR").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.atr))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.note.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
        }
    }
}

pub fn render_plus_dm_snapshot(ui: &mut egui::Ui, snap: &PlusDmSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.plus_dm_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥16 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.plus_dm_label.as_str() {
            "BULL_PRESSURE" | "BULL_SOFT" => UP,
            "BEAR_PRESSURE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — +DM raw {:.4} — +DM smoothed {:.4} — close {:.4} — as of {}",
                snap.symbol,
                snap.plus_dm_label,
                snap.plus_dm_raw,
                snap.plus_dm_smoothed,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("plus_dm_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Period").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.period))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("+DM raw").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.plus_dm_raw))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("+DM smoothed").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.plus_dm_smoothed))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("+DM smoothed prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.plus_dm_smoothed_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Up-move (H − H_prev)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.up_move))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Down-move (L_prev − L)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.down_move))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.note.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
        }
    }
}

pub fn render_ppo_snapshot(ui: &mut egui::Ui, snap: &PpoSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.ppo_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥37 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.ppo_label.as_str() {
            "STRONG_BULL" | "BULL" => UP,
            "STRONG_BEAR" | "BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — PPO {:+.3} — signal {:+.3} — hist {:+.3} — as of {}",
                snap.symbol,
                snap.ppo_label,
                snap.ppo_value,
                snap.signal_value,
                snap.histogram,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("ppo_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Fast / slow / signal").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {} / {}",
                        snap.fast_period, snap.slow_period, snap.signal_period
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("EMA fast").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.ema_fast))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("EMA slow").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.ema_slow))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("PPO").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.ppo_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Signal").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.signal_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Histogram").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.histogram))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub fn render_psar_snapshot(ui: &mut egui::Ui, snap: &PsarSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.psar_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥4 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.psar_label.as_str() {
            "STRONG_UP" | "UP" => UP,
            "STRONG_DOWN" | "DOWN" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — SAR {:.4} — dist {:+.2}% — bars {} — as of {}",
                snap.symbol,
                snap.psar_label,
                snap.sar_value,
                snap.distance_pct,
                snap.bars_in_trend,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("psar_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("AF start / step / max")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.2} / {:.2} / {:.2}",
                        snap.af_start, snap.af_step, snap.af_max
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Current AF").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.acceleration_factor))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("SAR value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.sar_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Extreme point").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.extreme_point))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Trend").small().strong());
                ui.label(
                    egui::RichText::new(if snap.trend_is_up { "UP" } else { "DOWN" })
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Bars in trend").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_in_trend))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}
