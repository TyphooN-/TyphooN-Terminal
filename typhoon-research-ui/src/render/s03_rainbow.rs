// Segment of the research snapshot renderers — see render.rs (this file is
// order-preserving; segments are mechanical splits, not semantic groups).
#[allow(unused_imports)]
use crate::theme::{AXIS_TEXT, BTN_GREEN_TEXT, BTN_RED_TEXT, DOWN, UP};
#[allow(unused_imports)]
use typhoon_engine::core::research::*;

pub fn render_rainbow_snapshot(ui: &mut egui::Ui, snap: &RainbowSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.rainbow_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥22 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.rainbow_label.as_str() {
            "STRONG_TREND" => UP,
            "CONSOLIDATING" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — width {:.4} ({:.3}%) · center {:.4} — close {:.4} — as of {}",
                snap.symbol,
                snap.rainbow_label,
                snap.rainbow_width,
                snap.rainbow_width_pct * 100.0,
                snap.center_value,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("rainbow_summary")
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
                ui.label(egui::RichText::new("Levels").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.levels))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Highest level").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.highest_level))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Lowest level").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.lowest_level))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Rainbow width").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.rainbow_width))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Width %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}%", snap.rainbow_width_pct * 100.0))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Center (mean of levels)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.center_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("r1 / r5 / r10").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{:.4} / {:.4} / {:.4}",
                        snap.r1, snap.r5, snap.r10
                    ))
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

pub fn render_sarext_snapshot(ui: &mut egui::Ui, snap: &SarextSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.sarext_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥4 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.sarext_label.as_str() {
            "STRONG_UP" | "UP" => UP,
            "STRONG_DOWN" | "DOWN" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(egui::RichText::new(format!("{} — {} — SAR {:.4} — EP {:.4} — AF {:.3} — trend {} — in-trend {} — distance {:+.3}% — close {:.4} — as of {}",
            snap.symbol, snap.sarext_label, snap.sar_value, snap.extreme_point, snap.acceleration_factor,
            if snap.trend_is_up { "UP" } else { "DOWN" }, snap.bars_in_trend, snap.distance_pct, snap.last_close, snap.as_of)).strong().color(color));
        ui.separator();
        egui::Grid::new("sarext_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("AF long init").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.af_init_long))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("AF long step").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.af_step_long))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("AF long max").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.af_max_long))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("AF short init").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.af_init_short))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("AF short step").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.af_step_short))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("AF short max").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.af_max_short))
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
                ui.label(egui::RichText::new("AF current").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.acceleration_factor))
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
                ui.label(egui::RichText::new("Distance %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}", snap.distance_pct))
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

pub fn render_squeeze_snapshot(ui: &mut egui::Ui, snap: &SqueezeSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.squeeze_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new(
                "No data — need ≥3 of 5 axes (short interest, IV, relvol, HP bars).",
            )
            .color(AXIS_TEXT)
            .small(),
        );
    } else {
        let color = match snap.squeeze_label.as_str() {
            "NO_SQUEEZE" | "WATCH" => UP,
            "STRONG" | "EXTREME" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — composite {:.1}/100 — as of {}",
                snap.symbol, snap.squeeze_label, snap.composite_score, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("squeeze_summary")
            .striped(true)
            .num_columns(3)
            .min_col_width(120.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Axis").small().strong());
                ui.label(egui::RichText::new("Raw").small().strong());
                ui.label(egui::RichText::new("Score 0..100").small().strong());
                ui.end_row();
                ui.label(egui::RichText::new("Short % float").small());
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.short_percent_of_float))
                        .small()
                        .monospace(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.0}", snap.short_float_score))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Days to cover").small());
                ui.label(
                    egui::RichText::new(format!("{:.2}d", snap.days_to_cover))
                        .small()
                        .monospace(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.0}", snap.days_to_cover_score))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("20d momentum").small());
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.momentum_20d_pct))
                        .small()
                        .monospace(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.0}", snap.momentum_score))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("RelVol 20d").small());
                ui.label(
                    egui::RichText::new(format!("{:.2}×", snap.relvol_20d))
                        .small()
                        .monospace(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.0}", snap.relvol_score))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("IV rank").small());
                ui.label(
                    egui::RichText::new(format!("{:.1}", snap.iv_rank))
                        .small()
                        .monospace(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.0}", snap.iv_rank_score))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Axes present").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}/5", snap.inputs_present))
                        .small()
                        .monospace(),
                );
                ui.label("");
                ui.end_row();
            });
    }
}

pub fn render_stochf_snapshot(ui: &mut egui::Ui, snap: &StochfSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.stochf_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥17 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.stochf_label.as_str() {
            "OVERBOUGHT" | "BULL" => UP,
            "OVERSOLD" | "BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(egui::RichText::new(format!("{} — {} — fastK {:.2} (prev {:.2}) — fastD {:.2} (prev {:.2}) — close {:.4} — as of {}",
            snap.symbol, snap.stochf_label, snap.fastk, snap.fastk_prev, snap.fastd, snap.fastd_prev, snap.last_close, snap.as_of)).strong().color(color));
        ui.separator();
        egui::Grid::new("stochf_summary")
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
                ui.label(egui::RichText::new("D period").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.d_period))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("FastK").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.fastk))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("FastK prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.fastk_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("FastD").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.fastd))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("FastD prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.fastd_prev))
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

pub fn render_stochrsi_snapshot(ui: &mut egui::Ui, snap: &StochRsiSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.stochrsi_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥36 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.stochrsi_label.as_str() {
            "OVERBOUGHT" | "BULL" => UP,
            "OVERSOLD" | "BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — %K {:.2} — %D {:.2} — RSI {:.2} — as of {}",
                snap.symbol,
                snap.stochrsi_label,
                snap.k_value,
                snap.d_value,
                snap.rsi_value,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("stochrsi_summary")
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
                    egui::RichText::new("Periods (RSI/stoch/%K/%D)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {} / {} / {}",
                        snap.rsi_period, snap.stoch_period, snap.k_period, snap.d_period
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("RSI value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.rsi_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("RSI min").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.rsi_min))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("RSI max").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.rsi_max))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("StochRSI raw").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.stoch_rsi_raw))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("%K").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.k_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("%D").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.d_value))
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

pub fn render_supertrend_snapshot(ui: &mut egui::Ui, snap: &SupertrendSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.supertrend_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥12 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.supertrend_label.as_str() {
            "STRONG_UP" | "UP" => UP,
            "STRONG_DOWN" | "DOWN" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — ST {:.4} — dist {:+.2}% — bars {} — as of {}",
                snap.symbol,
                snap.supertrend_label,
                snap.supertrend_value,
                snap.distance_pct,
                snap.bars_in_trend,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("st_summary")
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
                ui.label(egui::RichText::new("Period / multiplier").small().strong());
                ui.label(
                    egui::RichText::new(format!("{} / {:.1}", snap.period, snap.multiplier))
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
                ui.label(egui::RichText::new("Upper band").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.upper_band))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Lower band").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.lower_band))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Active ST").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.supertrend_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Trend direction").small().strong());
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

pub fn render_tema_snapshot(ui: &mut egui::Ui, snap: &TemaSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.tema_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥63 bars with OHLC.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.tema_label.as_str() {
            "STRONG_BULL" | "BULL" => UP,
            "STRONG_BEAR" | "BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — TEMA {:.4} — close {:.4} — dev {:+.2}% — as of {}",
                snap.symbol,
                snap.tema_label,
                snap.tema_value,
                snap.last_close,
                snap.deviation_pct,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("tema_summary")
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
                ui.label(egui::RichText::new("TEMA").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.tema_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("TEMA prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.tema_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Deviation %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.deviation_pct))
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

pub fn render_trange_snapshot(ui: &mut egui::Ui, snap: &TrangeSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.trange_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥21 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.trange_label.as_str() {
            "EXPANSION" => DOWN,
            "CONTRACTION" => AXIS_TEXT,
            _ => AXIS_TEXT,
        };
        ui.label(egui::RichText::new(format!("{} — {} — TR {:.4} (prev {:.4}) — mean(20) {:.4} — ratio {:.3} — close {:.4} — as of {}",
            snap.symbol, snap.trange_label, snap.trange_value, snap.trange_prev, snap.mean_trange_20, snap.trange_ratio, snap.last_close, snap.as_of)).strong().color(color));
        ui.separator();
        egui::Grid::new("trange_summary")
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
                ui.label(egui::RichText::new("TR value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.trange_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("TR prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.trange_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Mean TR(20)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.mean_trange_20))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("TR ratio").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.trange_ratio))
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
                ui.label(egui::RichText::new("Prev close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.prev_close))
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

pub fn render_trix_snapshot(ui: &mut egui::Ui, snap: &TrixSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.trix_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥55 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.trix_label.as_str() {
            "STRONG_BULL" | "BULL" => UP,
            "STRONG_BEAR" | "BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — TRIX {:+.4} — signal {:+.4} — hist {:+.4} — as of {}",
                snap.symbol,
                snap.trix_label,
                snap.trix_value,
                snap.signal_value,
                snap.histogram,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("trix_summary")
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
                ui.label(egui::RichText::new("Signal period").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.signal_period))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("TRIX %Δ").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.trix_value))
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
                ui.label(egui::RichText::new("EMA³ level").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.ema3_value))
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

pub fn render_ttm_squeeze_snapshot(ui: &mut egui::Ui, snap: &TtmSqueezeSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.squeeze_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥21 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.squeeze_label.as_str() {
            "FIRE_UP" => UP,
            "FIRE_DOWN" => DOWN,
            "SQUEEZE_ON" => AXIS_TEXT,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — momentum {:+.4} (prev {:+.4}) — squeeze_on {} — close {:.4} — as of {}",
                snap.symbol,
                snap.squeeze_label,
                snap.momentum,
                snap.momentum_prev,
                snap.squeeze_on,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("ttm_squeeze_summary")
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
                ui.label(egui::RichText::new("BB upper").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.bb_upper))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("BB lower").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.bb_lower))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("KC upper").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.kc_upper))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("KC lower").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.kc_lower))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Squeeze ON").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.squeeze_on))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Momentum").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.momentum))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Momentum prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.momentum_prev))
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

pub fn render_ultosc_snapshot(ui: &mut egui::Ui, snap: &UltoscSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.ultosc_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.ultosc_label.as_str() {
            "OVERBOUGHT" | "BULL" => UP,
            "OVERSOLD" | "BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — UO {:.2} — as of {}",
                snap.symbol, snap.ultosc_label, snap.ultosc_value, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("ultosc_summary")
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
                ui.label(egui::RichText::new("Periods").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {} / {}",
                        snap.period_short, snap.period_mid, snap.period_long
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Avg short (7)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.avg_short))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Avg mid (14)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.avg_mid))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Avg long (28)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.avg_long))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Ultimate Osc").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.ultosc_value))
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

pub fn render_vortex_snapshot(ui: &mut egui::Ui, snap: &VortexSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.vortex_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥15 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.vortex_label.as_str() {
            "BULL_CROSS" | "BULL" => UP,
            "BEAR_CROSS" | "BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — VI+ {:.3} / VI− {:.3} — Δ {:+.3} — as of {}",
                snap.symbol,
                snap.vortex_label,
                snap.vi_plus,
                snap.vi_minus,
                snap.vi_diff,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("vortex_summary")
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
                ui.label(egui::RichText::new("VI+").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.vi_plus))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("VI−").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.vi_minus))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("VI diff").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.vi_diff))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Σ TR").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.sum_tr))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Σ VM+").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.sum_vm_plus))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Σ VM−").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.sum_vm_minus))
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

pub fn render_willr_snapshot(ui: &mut egui::Ui, snap: &WillrSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.willr_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥15 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.willr_label.as_str() {
            "OVERBOUGHT" | "BULL" => UP,
            "OVERSOLD" | "BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — %R {:.2} — as of {}",
                snap.symbol, snap.willr_label, snap.willr_value, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("willr_summary")
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
                ui.label(egui::RichText::new("Highest high").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.highest_high))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Lowest low").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.lowest_low))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("%R").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.willr_value))
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

pub fn render_wma_snapshot(ui: &mut egui::Ui, snap: &WmaSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.wma_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥21 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.wma_label.as_str() {
            "BULL" | "WEAK_BULL" => UP,
            "BEAR" | "WEAK_BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(egui::RichText::new(format!("{} — {} — WMA {:.4} · SMA {:.4} · spread {:+.4} ({:+.3}%) — close {:.4} — as of {}",
            snap.symbol, snap.wma_label, snap.wma_value, snap.sma_value, snap.spread, snap.spread_pct * 100.0,
            snap.last_close, snap.as_of)).strong().color(color));
        ui.separator();
        egui::Grid::new("wma_summary")
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
                ui.label(egui::RichText::new("WMA value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.wma_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("WMA prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.wma_prev))
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
                ui.label(egui::RichText::new("Spread (close − WMA)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.spread))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Spread %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}%", snap.spread_pct * 100.0))
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

pub fn render_zigzag_snapshot(ui: &mut egui::Ui, snap: &ZigzagSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.zigzag_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥10 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.zigzag_label.as_str() {
            "UP_LEG" => UP,
            "DOWN_LEG" => DOWN,
            "AT_REVERSAL" => AXIS_TEXT,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — leg {} — reversal {:.4} — close {:.4} — as of {}",
                snap.symbol,
                snap.zigzag_label,
                snap.current_leg,
                snap.reversal_level,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("zigzag_summary")
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
                ui.label(egui::RichText::new("Threshold %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.threshold_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Current leg").small().strong());
                ui.label(egui::RichText::new(&snap.current_leg).small().monospace());
                ui.end_row();
                ui.label(egui::RichText::new("Last high value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_high_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last high bars ago").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.last_high_bars_ago))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last low value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.last_low_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last low bars ago").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.last_low_bars_ago))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Reversal level").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.reversal_level))
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

pub fn render_acrl_snapshot(ui: &mut egui::Ui, snap: &AccrualsSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.periods.is_empty() {
        ui.label(
            egui::RichText::new(
                "No data — run FA (Financials) for this symbol, then click Compute.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        ui.label(
            egui::RichText::new(format!(
            "{} — {} — TTM NI ${:.0}M · TTM FCF ${:.0}M · cash conv {:.1}% · avg {:.1}% — as of {}",
            snap.symbol, snap.trend_label,
            snap.ttm_net_income / 1e6, snap.ttm_free_cash_flow / 1e6,
            snap.ttm_cash_conversion_pct, snap.avg_cash_conversion_pct, snap.as_of,
        ))
            .strong()
            .color(AXIS_TEXT),
        );
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("acrl_grid")
                .striped(true)
                .num_columns(6)
                .min_col_width(72.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Period")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Date")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(egui::RichText::new("NI").color(AXIS_TEXT).small().strong());
                    ui.label(egui::RichText::new("FCF").color(AXIS_TEXT).small().strong());
                    ui.label(
                        egui::RichText::new("Cash Conv %")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Quality")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.end_row();
                    for p in &snap.periods {
                        let color = match p.quality_label.as_str() {
                            "HIGH" => UP,
                            "LOW" | "NEGATIVE_NI" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(&p.period).small().monospace());
                        ui.label(egui::RichText::new(&p.date).small().monospace());
                        ui.label(
                            egui::RichText::new(format!("{:.0}M", p.net_income / 1e6))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.0}M", p.free_cash_flow / 1e6))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.1}%", p.cash_conversion_pct))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(&p.quality_label)
                                .color(color)
                                .small()
                                .monospace()
                                .strong(),
                        );
                        ui.end_row();
                    }
                });
        });
    }
}

pub fn render_adf_snapshot(ui: &mut egui::Ui, snap: &DickeyFullerSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.adf_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 bars with positive closes.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.adf_label.as_str() {
            "STATIONARY" => UP,
            "NON_STATIONARY" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — t {:+.3} — β {:+.4} — crit5% {:+.2} — reject {} — as of {}",
                snap.symbol,
                snap.adf_label,
                snap.t_statistic,
                snap.beta,
                snap.crit_5pct,
                snap.reject_unit_root,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("adf_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("β (slope)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.6}", snap.beta))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("SE(β)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.se_beta))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("t-statistic").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.t_statistic))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Crit 1%").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}", snap.crit_1pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Crit 5%").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}", snap.crit_5pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Crit 10%").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}", snap.crit_10pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Reject unit root").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.reject_unit_root))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub fn render_adtest_snapshot(ui: &mut egui::Ui, snap: &AdtestSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.adtest_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.adtest_label.as_str() {
            "NORMAL" => UP,
            "STRONG_NON_NORMAL" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — A²_adj {:.4} — as of {}",
                snap.symbol, snap.adtest_label, snap.ad_adjusted, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("adtest_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(180.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Returns used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("A²").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.ad_statistic))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("A² adjusted").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.ad_adjusted))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("p-value (approx)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.p_value_approx))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Critical 10%").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{:.3}  (reject {})",
                        snap.critical_10pct, snap.reject_10pct
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Critical 5%").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{:.3}  (reject {})",
                        snap.critical_5pct, snap.reject_5pct
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Critical 1%").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{:.3}  (reject {})",
                        snap.critical_1pct, snap.reject_1pct
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
            });
    }
}

pub fn render_altz_snapshot(ui: &mut egui::Ui, snap: &AltmanZSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.components.is_empty() {
        ui.label(
            egui::RichText::new(
                "No data — run FA (Financials) + Fundamentals, then click Compute.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.zone.as_str() {
            "SAFE" => UP,
            "GRAY" => AXIS_TEXT,
            "DISTRESS" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — Z = {:.2} — {} — as of {}",
                snap.symbol, snap.z_score, snap.zone, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.label(egui::RichText::new(format!(
            "WC ${:.0}M · RE ${:.0}M · EBIT ${:.0}M · MVE ${:.0}M · Sales ${:.0}M · TA ${:.0}M · TL ${:.0}M",
            snap.working_capital / 1e6, snap.retained_earnings / 1e6, snap.ebit / 1e6,
            snap.market_value_equity / 1e6, snap.sales / 1e6,
            snap.total_assets / 1e6, snap.total_liabilities / 1e6,
        )).small().color(AXIS_TEXT));
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("altz_grid")
                .striped(true)
                .num_columns(5)
                .min_col_width(80.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Component")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Ratio")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Coeff")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Contribution")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Note")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.end_row();
                    for c in &snap.components {
                        ui.label(egui::RichText::new(&c.name).small().monospace().strong());
                        ui.label(
                            egui::RichText::new(format!("{:.3}", c.ratio))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.1}", c.coefficient))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.3}", c.contribution))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(&c.note)
                                .color(AXIS_TEXT)
                                .small()
                                .monospace(),
                        );
                        ui.end_row();
                    }
                });
        });
    }
}

pub fn render_amihud_snapshot(ui: &mut egui::Ui, snap: &AmihudIlliqSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.illiq_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 bars with volume.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.illiq_label.as_str() {
            "VERY_LIQUID" | "LIQUID" => UP,
            "ILLIQUID" | "VERY_ILLIQUID" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — ILLIQ {:.4} — {} bars — as of {}",
                snap.symbol, snap.illiq_label, snap.mean_illiq, snap.bars_used, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("amihud_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Mean ILLIQ (×1e6)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.mean_illiq))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Median ILLIQ (×1e6)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.median_illiq))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("90th pctile ILLIQ").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.illiq_90th))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Avg daily $ volume").small().strong());
                ui.label(
                    egui::RichText::new(format!("${:.0}", snap.avg_dollar_volume))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub fn render_apen_snapshot(ui: &mut egui::Ui, snap: &ApenSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.apen_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.apen_label.as_str() {
            "REGULAR" => UP,
            "HIGHLY_COMPLEX" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — ApEn {:.4} — as of {}",
                snap.symbol, snap.apen_label, snap.apen, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("apen_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(180.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Returns used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Embedding dim m").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.embed_dim))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Tolerance r").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.tolerance))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Phi^m").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.phi_m))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Phi^(m+1)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.phi_m1))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("ApEn").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.apen))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub fn render_archlm_snapshot(ui: &mut egui::Ui, snap: &ArchLmSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.arch_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥35 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.arch_label.as_str() {
            "NO_ARCH" => UP,
            "STRONG_ARCH" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — LM={:.3} — p={:.4} — as of {}",
                snap.symbol, snap.arch_label, snap.lm_statistic, snap.p_value, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("archlm_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Returns used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Lags q").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.q_lags))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("R²").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.r_squared))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("LM = n·R²").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.lm_statistic))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("p-value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.p_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("crit χ²(5) @5% / @1%").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{:.3} / {:.3}",
                        snap.crit_5pct_chi2, snap.crit_1pct_chi2
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Reject homoskedastic").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.reject_homoskedastic))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub fn render_autocor_snapshot(ui: &mut egui::Ui, snap: &AutocorrelationSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.regime_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — needs ≥30 cached daily bars.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.regime_label.as_str() {
            "MOMENTUM" | "STRONG_MOMENTUM" => UP,
            "MEAN_REVERT" | "STRONG_MEAN_REVERT" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — lag1 {:.3} — {} bars — as of {}",
                snap.symbol, snap.regime_label, snap.lag1_acf, snap.bars_used, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("autocor_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Lag-1 ACF").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.lag1_acf))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Lag-5 ACF").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.lag5_acf))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Lag-10 ACF").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.lag10_acf))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Lag-20 ACF").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.lag20_acf))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Mean log return").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.mean_log_return))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub fn render_automi_snapshot(ui: &mut egui::Ui, snap: &AutomiSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.automi_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥50 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.automi_label.as_str() {
            "INDEPENDENT" | "WEAK" => UP,
            "STRONG" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — MI(1) {:.4} — as of {}",
                snap.symbol, snap.automi_label, snap.mi_lag1, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("automi_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(180.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Returns used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Bins per marginal").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.num_bins))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("MI lag-1 (bits)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.mi_lag1))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("MI lag-5").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.mi_lag5))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("MI lag-10").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.mi_lag10))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("H(X) marginal (bits)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.h_marginal))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("MI(1) / H(X)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.normalized_mi1))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub fn render_bbsqueeze_snapshot(ui: &mut egui::Ui, snap: &BbsqueezeSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.bbsqueeze_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥140 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.bbsqueeze_label.as_str() {
            "TIGHT_SQUEEZE" => DOWN,
            "MODERATE_SQUEEZE" => AXIS_TEXT,
            "EXPANSION" => UP,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — width pct {:.1} — as of {}",
                snap.symbol, snap.bbsqueeze_label, snap.bb_width_percentile, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("bbsq_summary")
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
                ui.label(egui::RichText::new("BB width current").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.5}", snap.bb_width_current))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("BB width min 120").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.5}", snap.bb_width_min_120))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("BB width max 120").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.5}", snap.bb_width_max_120))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Width percentile").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}", snap.bb_width_percentile))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Upper band").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.upper_band))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Mid band").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.mid_band))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Lower band").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.lower_band))
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

pub fn render_bdstest_snapshot(ui: &mut egui::Ui, snap: &BdsTestSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.bds_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥60 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.bds_label.as_str() {
            "IID_CONFIRMED" => UP,
            "WEAK_DEPENDENCE" | "STRONG_DEPENDENCE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — BDS {:+.3} — as of {}",
                snap.symbol, snap.bds_label, snap.bds_stat, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("bds_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(180.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Returns used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Embedding dim m").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.embed_dim))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("ε / σ").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.epsilon_mult))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("BDS statistic").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}", snap.bds_stat))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("p-value (2-sided)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.p_value_two_sided))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Reject iid null").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.reject_null))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub fn render_beta_snapshot(ui: &mut egui::Ui, snap: &BetaSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.windows.is_empty() {
        ui.label(
            egui::RichText::new("No data — click Fetch to pull 5Y history for symbol + SPY.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        ui.label(
            egui::RichText::new(format!(
                "{} vs {} — as of {}",
                snap.symbol, snap.market_ticker, snap.as_of
            ))
            .strong()
            .color(AXIS_TEXT),
        );
        ui.separator();
        egui::Grid::new("beta_grid")
            .striped(true)
            .num_columns(6)
            .min_col_width(70.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Window")
                        .color(AXIS_TEXT)
                        .small()
                        .strong(),
                );
                ui.label(egui::RichText::new("β").color(AXIS_TEXT).small().strong());
                ui.label(
                    egui::RichText::new("α (ann)")
                        .color(AXIS_TEXT)
                        .small()
                        .strong(),
                );
                ui.label(egui::RichText::new("R²").color(AXIS_TEXT).small().strong());
                ui.label(
                    egui::RichText::new("Corr")
                        .color(AXIS_TEXT)
                        .small()
                        .strong(),
                );
                ui.label(egui::RichText::new("N").color(AXIS_TEXT).small().strong());
                ui.end_row();
                for w in &snap.windows {
                    ui.label(
                        egui::RichText::new(&w.window_label)
                            .small()
                            .monospace()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new(format!("{:.3}", w.beta))
                            .small()
                            .monospace(),
                    );
                    ui.label(
                        egui::RichText::new(format!("{:+.2}%", w.alpha_pct))
                            .small()
                            .monospace(),
                    );
                    ui.label(
                        egui::RichText::new(format!("{:.3}", w.r_squared))
                            .small()
                            .monospace(),
                    );
                    ui.label(
                        egui::RichText::new(format!("{:.3}", w.correlation))
                            .small()
                            .monospace(),
                    );
                    ui.label(
                        egui::RichText::new(format!("{}", w.n_observations))
                            .small()
                            .monospace(),
                    );
                    ui.end_row();
                }
            });
        if !snap.note.is_empty() {
            ui.separator();
            ui.label(
                egui::RichText::new(&snap.note)
                    .color(AXIS_TEXT)
                    .small()
                    .italics(),
            );
        }
    }
}

pub fn render_bipower_snapshot(ui: &mut egui::Ui, snap: &BipowerVariationSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.jump_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.jump_label.as_str() {
            "NO_JUMPS" => UP,
            "HEAVY_JUMPS" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — jump {:.1}% — cont vol {:.2}% — RV vol {:.2}% — as of {}",
                snap.symbol,
                snap.jump_label,
                snap.jump_pct,
                snap.continuous_vol_ann_pct,
                snap.realized_vol_ann_pct,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("bipower_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Realized variance (RV)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.realized_var))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Bipower variation (BPV)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.bipower_var))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Continuous vol (ann %)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.continuous_vol_ann_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Realized vol (ann %)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.realized_vol_ann_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Jump ratio").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.jump_ratio))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Jump %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.jump_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub fn render_bnsjump_snapshot(ui: &mut egui::Ui, snap: &BnsjumpSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.bnsjump_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.bnsjump_label.as_str() {
            "NO_JUMP" => UP,
            "STRONG_JUMP" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — z {:+.3} — as of {}",
                snap.symbol, snap.bnsjump_label, snap.jump_z_stat, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("bnsjump_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(180.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Returns used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Realised variance").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3e}", snap.realized_variance))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Bipower variance").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3e}", snap.bipower_variance))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Jump ratio").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.jump_ratio))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Z-statistic").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}", snap.jump_z_stat))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("p-value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.p_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub fn render_break_snapshot(ui: &mut egui::Ui, snap: &BreakoutSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.current_price <= 0.0 {
        ui.label(
            egui::RichText::new(
                "No data — ensure HP bars are cached for this symbol, then click Compute.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.breakout_label.as_str() {
            "NEW_HIGH" => UP,
            "NEW_LOW" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — setup: {} — last: {:.2} — as of {}",
                snap.symbol, snap.breakout_label, snap.setup_label, snap.current_price, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("break_grid")
            .striped(true)
            .num_columns(4)
            .spacing([14.0, 3.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Window").strong().small());
                ui.label(egui::RichText::new("High").strong().small().color(UP));
                ui.label(egui::RichText::new("Low").strong().small().color(DOWN));
                ui.label(egui::RichText::new("Pos in range").strong().small());
                ui.end_row();
                ui.label(egui::RichText::new("20d").monospace().small());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.high_20d))
                        .monospace()
                        .small(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.low_20d))
                        .monospace()
                        .small(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.0}%", snap.position_in_20d_range_pct))
                        .monospace()
                        .small(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("60d").monospace().small());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.high_60d))
                        .monospace()
                        .small(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.low_60d))
                        .monospace()
                        .small(),
                );
                ui.label(egui::RichText::new("").small());
                ui.end_row();
                ui.label(egui::RichText::new("52w").monospace().small());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.high_52w))
                        .monospace()
                        .small(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.low_52w))
                        .monospace()
                        .small(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.0}%", snap.position_in_52w_range_pct))
                        .monospace()
                        .small(),
                );
                ui.end_row();
            });
        ui.separator();
        egui::Grid::new("break_sub")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                let dist_row = |ui: &mut egui::Ui, label: &str, val: f64| {
                    ui.label(egui::RichText::new(label).small().strong());
                    let c = if val >= 0.0 { UP } else { DOWN };
                    ui.label(
                        egui::RichText::new(format!("{:+.2}%", val))
                            .small()
                            .monospace()
                            .color(c),
                    );
                    ui.end_row();
                };
                dist_row(ui, "Distance from 52w high", snap.dist_from_52w_high_pct);
                dist_row(ui, "Distance from 52w low", snap.dist_from_52w_low_pct);
                dist_row(ui, "Distance from 20d high", snap.dist_from_20d_high_pct);
                dist_row(ui, "Distance from 60d high", snap.dist_from_60d_high_pct);
                ui.label(
                    egui::RichText::new("Consolidation (20d range/mean)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.consolidation_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub fn render_breuschpagan_snapshot(ui: &mut egui::Ui, snap: &BreuschPaganSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.bp_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥40 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.bp_label.as_str() {
            "HOMOSKEDASTIC" => UP,
            "MILD_HETERO" | "STRONG_HETERO" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — LM {:.3} — as of {}",
                snap.symbol, snap.bp_label, snap.lm_stat, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("bp_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(180.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Returns used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("LM statistic (n×R²)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.lm_stat))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Aux-regression R²").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.r_squared))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Degrees of freedom").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.df))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("χ²(df) 95% critical").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.critical_95))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Reject homoskedasticity")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{}", snap.reject_null))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub fn render_burgspec_snapshot(ui: &mut egui::Ui, snap: &BurgSpecSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.burgspec_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥32 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.burgspec_label.as_str() {
            "STRONG_AR_CYCLE" | "MODERATE_AR_CYCLE" => UP,
            "WEAK_AR_CYCLE" | "NO_AR_CYCLE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — period {:.1} bars — as of {}",
                snap.symbol, snap.burgspec_label, snap.dominant_period_bars, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("burg_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(180.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Returns used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("AR order p").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.ar_order))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Dominant frequency").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.5}", snap.dominant_freq))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Dominant period (bars)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.1}", snap.dominant_period_bars))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Peak power").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3e}", snap.peak_power))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Mean power").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3e}", snap.mean_power))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Peak / mean").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.peak_to_mean_ratio))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub fn render_burke_snapshot(ui: &mut egui::Ui, snap: &BurkeRatioSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.burke_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.burke_label.as_str() {
            "GOOD" | "EXCELLENT" => UP,
            "POOR" | "VERY_POOR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — Burke {:+.3} — events {} — ann ret {:+.2}% — as of {}",
                snap.symbol,
                snap.burke_label,
                snap.burke_ratio,
                snap.dd_event_count,
                snap.annualized_return_pct,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("burke_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Annualized return").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}%", snap.annualized_return_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Drawdown events").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.dd_event_count))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Σdd²").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.sum_sq_drawdowns))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Worst event dd").small().strong());
                ui.label(
                    egui::RichText::new(format!("-{:.2}%", snap.worst_event_dd_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}
