// Segment of the research snapshot renderers — see render.rs (this file is
// order-preserving; segments are mechanical splits, not semantic groups).
#[allow(unused_imports)]
use crate::theme::{AXIS_TEXT, BTN_GREEN_TEXT, BTN_RED_TEXT, DOWN, UP};
#[allow(unused_imports)]
use typhoon_engine::core::research::*;

pub fn render_avgprice_snapshot(ui: &mut egui::Ui, snap: &AvgpriceSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.avgprice_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥1 bar.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.avgprice_label.as_str() {
            "ABOVE_CLOSE" => UP,
            "BELOW_CLOSE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — avgprice {:.4} — close {:.4} — Δ {:+.3}% — as of {}",
                snap.symbol,
                snap.avgprice_label,
                snap.avgprice,
                snap.close,
                snap.delta_pct,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("avgprice_summary")
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
                ui.label(egui::RichText::new("AVGPRICE").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.avgprice))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("AVGPRICE prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.avgprice_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Open").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.open))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("High").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.high))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Low").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.low))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Δ% vs close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}%", snap.delta_pct))
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

pub fn render_medprice_snapshot(ui: &mut egui::Ui, snap: &MedpriceSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.medprice_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥1 bar.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.medprice_label.as_str() {
            "ABOVE_MID" => UP,
            "BELOW_MID" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — medprice {:.4} — close {:.4} — Δ {:+.3}% — as of {}",
                snap.symbol,
                snap.medprice_label,
                snap.medprice,
                snap.close,
                snap.delta_pct,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("medprice_summary")
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
                ui.label(egui::RichText::new("MEDPRICE").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.medprice))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("MEDPRICE prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.medprice_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("High").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.high))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Low").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.low))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Δ% vs close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}%", snap.delta_pct))
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

pub fn render_typprice_snapshot(ui: &mut egui::Ui, snap: &TypPriceSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.typprice_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥1 bar.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.typprice_label.as_str() {
            "ABOVE_CLOSE" => UP,
            "BELOW_CLOSE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — typprice {:.4} — close {:.4} — Δ {:+.3}% — as of {}",
                snap.symbol,
                snap.typprice_label,
                snap.typprice,
                snap.close,
                snap.delta_pct,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("typprice_summary")
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
                ui.label(egui::RichText::new("TYPPRICE").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.typprice))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("TYPPRICE prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.typprice_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("High").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.high))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Low").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.low))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Δ% vs close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}%", snap.delta_pct))
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

pub fn render_wclprice_snapshot(ui: &mut egui::Ui, snap: &WclPriceSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.wclprice_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥1 bar.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.wclprice_label.as_str() {
            "ABOVE_CLOSE" => UP,
            "BELOW_CLOSE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — wclprice {:.4} — close {:.4} — Δ {:+.3}% — as of {}",
                snap.symbol,
                snap.wclprice_label,
                snap.wclprice,
                snap.close,
                snap.delta_pct,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("wclprice_summary")
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
                ui.label(egui::RichText::new("WCLPRICE").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.wclprice))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("WCLPRICE prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.wclprice_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("High").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.high))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Low").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.low))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Δ% vs close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}%", snap.delta_pct))
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

pub fn render_variance_snapshot(ui: &mut egui::Ui, snap: &VarianceSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.variance_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥5 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.variance_label.as_str() {
            "HIGH_VOL" | "ELEVATED" => DOWN,
            "LOW_VOL" => UP,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — variance {:.6} — stddev {:.4} — CV {:.3}% — close {:.4} — as of {}",
                snap.symbol,
                snap.variance_label,
                snap.variance,
                snap.stddev,
                snap.cv,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("variance_summary")
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
                ui.label(egui::RichText::new("Mean").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.mean))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Variance").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.variance))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Variance prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.variance_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Stddev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.stddev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("CV %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}%", snap.cv))
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

pub fn render_accbands_snapshot(ui: &mut egui::Ui, snap: &AccbandsSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.accbands_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥21 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.accbands_label.as_str() {
            "BREAKOUT_UP" | "UPPER" => UP,
            "BREAKOUT_DOWN" | "LOWER" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(egui::RichText::new(format!("{} — {} — upper {:.4} — mid {:.4} — lower {:.4} — width {:.4} — pos {:.3} — close {:.4} — as of {}",
            snap.symbol, snap.accbands_label, snap.acc_upper, snap.acc_middle, snap.acc_lower, snap.width, snap.position, snap.last_close, snap.as_of)).strong().color(color));
        ui.separator();
        egui::Grid::new("accbands_summary")
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
                ui.label(egui::RichText::new("Upper band").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.acc_upper))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Middle band").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.acc_middle))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Lower band").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.acc_lower))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Width").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.width))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Position in band").small().strong());
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

pub fn render_adx_snapshot(ui: &mut egui::Ui, snap: &AdxSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.adx_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥29 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.adx_label.as_str() {
            "STRONG_TREND" | "TREND" => UP,
            "NO_TREND" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — ADX {:.2} — +DI {:.2} — −DI {:.2} — as of {}",
                snap.symbol, snap.adx_label, snap.adx, snap.plus_di, snap.minus_di, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("adx_summary")
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
                ui.label(egui::RichText::new("+DI").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.plus_di))
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
                ui.label(egui::RichText::new("DX").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.dx))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("ADX").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.adx))
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
    }
}

pub fn render_adxr_snapshot(ui: &mut egui::Ui, snap: &AdxrSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.adxr_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥43 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.adxr_label.as_str() {
            "STRONG_TREND" | "TREND" => UP,
            "NO_TREND" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — ADXR {:.3} — ADX now {:.3} — ADX prior {:.3} — close {:.4} — as of {}",
                snap.symbol,
                snap.adxr_label,
                snap.adxr,
                snap.adx_now,
                snap.adx_prior,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("adxr_summary")
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
                ui.label(egui::RichText::new("ADX now").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.adx_now))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("ADX prior").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.adx_prior))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("ADXR").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.adxr))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("ADXR prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.adxr_prev))
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

pub fn render_apo_snapshot(ui: &mut egui::Ui, snap: &ApoSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.apo_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥27 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.apo_label.as_str() {
            "STRONG_BULL" | "BULL" => UP,
            "STRONG_BEAR" | "BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — APO {:+.4} — fast_EMA {:.4} — slow_EMA {:.4} — close {:.4} — as of {}",
                snap.symbol,
                snap.apo_label,
                snap.apo,
                snap.fast_ema,
                snap.slow_ema,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("apo_summary")
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
                ui.label(egui::RichText::new("Fast period").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.fast_period))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Slow period").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.slow_period))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("APO").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.apo))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("APO prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.apo_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Fast EMA").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.fast_ema))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Slow EMA").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.slow_ema))
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

pub fn render_aroon_snapshot(ui: &mut egui::Ui, snap: &AroonSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.aroon_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥26 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.aroon_label.as_str() {
            "STRONG_UP" | "WEAK_UP" => UP,
            "STRONG_DOWN" | "WEAK_DOWN" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — osc {:+.1} — up {:.1} — down {:.1} — as of {}",
                snap.symbol,
                snap.aroon_label,
                snap.aroon_oscillator,
                snap.aroon_up,
                snap.aroon_down,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("aroon_summary")
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
                ui.label(egui::RichText::new("Aroon Up").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.aroon_up))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Aroon Down").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.aroon_down))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Oscillator").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}", snap.aroon_oscillator))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Bars since high").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_since_high))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Bars since low").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_since_low))
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

pub fn render_aroonosc_snapshot(ui: &mut egui::Ui, snap: &AroonoscSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.aroonosc_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥16 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.aroonosc_label.as_str() {
            "STRONG_BULL" | "BULL" => UP,
            "BEAR" | "STRONG_BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — osc {:+.1} — up {:.1} / down {:.1} — close {:.4} — as of {}",
                snap.symbol,
                snap.aroonosc_label,
                snap.aroonosc,
                snap.aroon_up,
                snap.aroon_down,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("aroonosc_summary")
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
                ui.label(egui::RichText::new("AROONOSC").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}", snap.aroonosc))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("AROONOSC prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}", snap.aroonosc_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("AROON_UP").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.aroon_up))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("AROON_DOWN").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.aroon_down))
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

pub fn render_cci_snapshot(ui: &mut egui::Ui, snap: &CciSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.cci_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥20 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.cci_label.as_str() {
            "OVERBOUGHT" => DOWN,
            "OVERSOLD" => UP,
            "BULL" => UP,
            "BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — CCI {:+.2} — tp {:.4} — as of {}",
                snap.symbol, snap.cci_label, snap.cci_value, snap.typical_price, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("cci_summary")
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
                ui.label(egui::RichText::new("Typical price").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.typical_price))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("TP SMA").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.tp_sma))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Mean abs dev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.mean_abs_dev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("CCI value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}", snap.cci_value))
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

pub fn render_cdl_belt_hold_snapshot(ui: &mut egui::Ui, snap: &CdlBeltHoldSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.cdl_belt_hold_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥2 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.cdl_belt_hold_label.as_str() {
            "BULLISH_PATTERN" => UP,
            "BEARISH_PATTERN" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(egui::RichText::new(format!("{} — {} — value {} — body {:.1}% — opening_shadow {:.1}% — closing_shadow {:.1}% — close {:.4} — as of {}",
            snap.symbol, snap.cdl_belt_hold_label, snap.pattern_value, snap.body_pct_range, snap.opening_shadow_pct, snap.closing_shadow_pct, snap.last_close, snap.as_of)).strong().color(color));
        ui.separator();
        egui::Grid::new("cdl_belt_hold_summary")
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
                ui.label(egui::RichText::new("Pattern value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.pattern_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Prev value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.pattern_value_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Body % range").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.body_pct_range))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Opening shadow %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.opening_shadow_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Closing shadow %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.closing_shadow_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last bar match").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.last_bar_match))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Days since pattern").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.days_since_pattern))
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

pub fn render_cdl_high_wave_snapshot(ui: &mut egui::Ui, snap: &CdlHighWaveSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.cdl_high_wave_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥2 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.cdl_high_wave_label.as_str() {
            "GREEN_BODY_PATTERN" => UP,
            "RED_BODY_PATTERN" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(egui::RichText::new(format!("{} — {} — value {} — body {:.1}% — upper {:.1}% — lower {:.1}% — close {:.4} — as of {}",
            snap.symbol, snap.cdl_high_wave_label, snap.pattern_value, snap.body_pct_range, snap.upper_shadow_pct, snap.lower_shadow_pct, snap.last_close, snap.as_of)).strong().color(color));
        ui.separator();
        egui::Grid::new("cdl_high_wave_summary")
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
                ui.label(egui::RichText::new("Pattern value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.pattern_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Prev value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.pattern_value_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Body % range").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.body_pct_range))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Upper shadow %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.upper_shadow_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Lower shadow %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.lower_shadow_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last bar match").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.last_bar_match))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Days since pattern").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.days_since_pattern))
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

pub fn render_cdl_long_line_snapshot(ui: &mut egui::Ui, snap: &CdlLongLineSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.cdl_long_line_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥2 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.cdl_long_line_label.as_str() {
            "GREEN_BODY_PATTERN" => UP,
            "RED_BODY_PATTERN" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(egui::RichText::new(format!("{} — {} — value {} — body {:.1}% — upper {:.1}% — lower {:.1}% — close {:.4} — as of {}",
            snap.symbol, snap.cdl_long_line_label, snap.pattern_value, snap.body_pct_range, snap.upper_shadow_pct, snap.lower_shadow_pct, snap.last_close, snap.as_of)).strong().color(color));
        ui.separator();
        egui::Grid::new("cdl_long_line_summary")
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
                ui.label(egui::RichText::new("Pattern value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.pattern_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Prev value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.pattern_value_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Body % range").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.body_pct_range))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Upper shadow %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.upper_shadow_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Lower shadow %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.lower_shadow_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last bar match").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.last_bar_match))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Days since pattern").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.days_since_pattern))
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

pub fn render_cdl_short_line_snapshot(ui: &mut egui::Ui, snap: &CdlShortLineSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.cdl_short_line_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥2 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.cdl_short_line_label.as_str() {
            "GREEN_BODY_PATTERN" => UP,
            "RED_BODY_PATTERN" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(egui::RichText::new(format!("{} — {} — value {} — body {:.1}% — upper {:.1}% — lower {:.1}% — close {:.4} — as of {}",
            snap.symbol, snap.cdl_short_line_label, snap.pattern_value, snap.body_pct_range, snap.upper_shadow_pct, snap.lower_shadow_pct, snap.last_close, snap.as_of)).strong().color(color));
        ui.separator();
        egui::Grid::new("cdl_short_line_summary")
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
                ui.label(egui::RichText::new("Pattern value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.pattern_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Prev value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.pattern_value_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Body % range").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.body_pct_range))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Upper shadow %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.upper_shadow_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Lower shadow %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.lower_shadow_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last bar match").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.last_bar_match))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Days since pattern").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.days_since_pattern))
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

pub fn render_chaikosc_snapshot(ui: &mut egui::Ui, snap: &ChaikoscSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.chaikosc_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥12 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.chaikosc_label.as_str() {
            "STRONG_ACCUM" | "ACCUM" => UP,
            "STRONG_DIST" | "DIST" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — CO {:+.1} — as of {}",
                snap.symbol, snap.chaikosc_label, snap.chaikosc_value, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("chaikosc_summary")
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
                ui.label(egui::RichText::new("Fast / slow").small().strong());
                ui.label(
                    egui::RichText::new(format!("{} / {}", snap.fast_period, snap.slow_period))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("A/D last").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.ad_last))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("EMA(3) A/D").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.ema_fast_ad))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("EMA(10) A/D").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.ema_slow_ad))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Oscillator").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}", snap.chaikosc_value))
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

pub fn render_chop_snapshot(ui: &mut egui::Ui, snap: &ChopSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.chop_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥15 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.chop_label.as_str() {
            "TRENDING" => UP,
            "CHOP" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — CI {:.2} — range {:.4} — as of {}",
                snap.symbol, snap.chop_label, snap.chop_value, snap.range_span, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("chop_summary")
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
                ui.label(egui::RichText::new("CI value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.chop_value))
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
                ui.label(egui::RichText::new("Range high").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.range_high))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Range low").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.range_low))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Range span").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.range_span))
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

pub fn render_cmf_snapshot(ui: &mut egui::Ui, snap: &CmfSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.cmf_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥20 bars with volume.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.cmf_label.as_str() {
            "STRONG_ACCUM" | "ACCUM" => UP,
            "STRONG_DIST" | "DIST" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — CMF {:+.3} — vol {:.0} — as of {}",
                snap.symbol, snap.cmf_label, snap.cmf_value, snap.volume_sum, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("cmf_summary")
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
                ui.label(egui::RichText::new("CMF value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.cmf_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Σ MFV").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.0}", snap.money_flow_volume_sum))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Σ volume").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.0}", snap.volume_sum))
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

pub fn render_dema_snapshot(ui: &mut egui::Ui, snap: &DemaSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.dema_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥42 bars with OHLC.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.dema_label.as_str() {
            "STRONG_BULL" | "BULL" => UP,
            "STRONG_BEAR" | "BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — DEMA {:.4} — close {:.4} — dev {:+.2}% — as of {}",
                snap.symbol,
                snap.dema_label,
                snap.dema_value,
                snap.last_close,
                snap.deviation_pct,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("dema_summary")
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
                ui.label(egui::RichText::new("DEMA").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.dema_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("DEMA prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.dema_prev))
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

pub fn render_donchian_snapshot(ui: &mut egui::Ui, snap: &DonchianSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.donchian_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥21 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.donchian_label.as_str() {
            "BREAKOUT_UP" => UP,
            "BREAKOUT_DOWN" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — pos {:.1}% — as of {}",
                snap.symbol, snap.donchian_label, snap.channel_position_pct, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("donchian_summary")
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
                ui.label(egui::RichText::new("Upper channel").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.upper_channel))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Mid channel").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.mid_channel))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Lower channel").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.lower_channel))
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
                ui.label(egui::RichText::new("Channel position %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}", snap.channel_position_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Breakout upper").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.breakout_upper))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Breakout lower").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.breakout_lower))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub fn render_dpo_snapshot(ui: &mut egui::Ui, snap: &DpoSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.dpo_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥32 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.dpo_label.as_str() {
            "PEAK_HIGH" | "BULL" => UP,
            "PEAK_LOW" | "BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — DPO {:+.4} ({:+.2}%) — as of {}",
                snap.symbol, snap.dpo_label, snap.dpo_value, snap.dpo_pct, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("dpo_summary")
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
                ui.label(egui::RichText::new("Period / shift").small().strong());
                ui.label(
                    egui::RichText::new(format!("{} / {}", snap.period, snap.shift))
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
                ui.label(egui::RichText::new("DPO").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.dpo_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("DPO %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}%", snap.dpo_pct))
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

pub fn render_dx_snapshot(ui: &mut egui::Ui, snap: &DxSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.dx_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥16 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.dx_label.as_str() {
            "STRONG_DIR" | "DIR" => {
                if snap.plus_di >= snap.minus_di {
                    UP
                } else {
                    DOWN
                }
            }
            "NO_DIR" => AXIS_TEXT,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — DX {:.3} — +DI {:.3} — -DI {:.3} — close {:.4} — as of {}",
                snap.symbol,
                snap.dx_label,
                snap.dx,
                snap.plus_di,
                snap.minus_di,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("dx_summary")
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
                ui.label(egui::RichText::new("DX").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.dx))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("DX prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.dx_prev))
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
                ui.label(egui::RichText::new("−DI").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.minus_di))
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

pub fn render_fisher_snapshot(ui: &mut egui::Ui, snap: &FisherSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.fisher_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥22 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.fisher_label.as_str() {
            "STRONG_POS" | "POS" => UP,
            "STRONG_NEG" | "NEG" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — fisher {:+.3} — signal {:+.3} — as of {}",
                snap.symbol, snap.fisher_label, snap.fisher_value, snap.fisher_signal, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("fisher_summary")
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
                ui.label(egui::RichText::new("Fisher value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}", snap.fisher_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Signal (prev)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}", snap.fisher_signal))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Peak |fisher| 10").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.peak_abs_10))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("±2 cross last 3").small().strong());
                ui.label(
                    egui::RichText::new(if snap.extreme_2_cross { "YES" } else { "no" })
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

pub fn render_force_index_snapshot(ui: &mut egui::Ui, snap: &ForceIndexSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.force_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥15 bars with volume.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.force_label.as_str() {
            "STRONG_BULL" | "BULL" => UP,
            "STRONG_BEAR" | "BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — EMA {:.2} (prev {:.2}) — raw {:.2} — close {:.4} — as of {}",
                snap.symbol,
                snap.force_label,
                snap.force_ema,
                snap.force_ema_prev,
                snap.force_raw,
                snap.last_close,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("force_index_summary")
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
                ui.label(egui::RichText::new("Raw force").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.force_raw))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Force EMA").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.force_ema))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Force EMA prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.force_ema_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Last volume").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.0}", snap.last_volume))
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

pub fn render_frama_snapshot(ui: &mut egui::Ui, snap: &FramaSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.frama_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥32 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.frama_label.as_str() {
            "STRONG_TREND" => UP,
            "CHOP" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(egui::RichText::new(format!("{} — {} — D {:.4} · α {:.4} · FRAMA {:.4} · spread {:+.4} — close {:.4} — as of {}",
            snap.symbol, snap.frama_label, snap.fractal_dim, snap.alpha, snap.frama_value, snap.spread,
            snap.last_close, snap.as_of)).strong().color(color));
        ui.separator();
        egui::Grid::new("frama_summary")
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
                ui.label(egui::RichText::new("Fractal dim D").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.fractal_dim))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Alpha α").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.alpha))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("FRAMA value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.frama_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("FRAMA prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.frama_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Spread (close − FRAMA)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.spread))
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

pub fn render_heikin_snapshot(ui: &mut egui::Ui, snap: &HeikinSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.heikin_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥2 bars with OHLC.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.heikin_label.as_str() {
            "STRONG_BULL_RUN" | "BULL" => UP,
            "STRONG_BEAR_RUN" | "BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — HA_C {:.4} — run {} — as of {}",
                snap.symbol,
                snap.heikin_label,
                snap.ha_close,
                snap.consecutive_same_color,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("heikin_summary")
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
                ui.label(egui::RichText::new("HA open").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.ha_open))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("HA high").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.ha_high))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("HA low").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.ha_low))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("HA close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.ha_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Body |HA_C − HA_O|").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.body_abs))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Upper wick").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.upper_wick))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Lower wick").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.lower_wick))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Consecutive run").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.consecutive_same_color))
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

pub fn render_hma_snapshot(ui: &mut egui::Ui, snap: &HmaSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.hma_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥29 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.hma_label.as_str() {
            "STRONG_UP" | "UP" => UP,
            "STRONG_DOWN" | "DOWN" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — HMA {:.4} — slope {:+.2}% — vs close {:+.2}% — as of {}",
                snap.symbol,
                snap.hma_label,
                snap.hma_value,
                snap.hma_slope_pct,
                snap.hma_vs_close_pct,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("hma_summary")
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
                ui.label(egui::RichText::new("Period / half / √").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {} / {}",
                        snap.period, snap.half_period, snap.sqrt_period
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("HMA value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.hma_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("5-bar slope %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}%", snap.hma_slope_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Close vs HMA %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}%", snap.hma_vs_close_pct))
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

pub fn render_ht_dcperiod_snapshot(ui: &mut egui::Ui, snap: &HtDcperiodSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.period_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥64 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.period_label.as_str() {
            "VERY_SHORT" | "SHORT" => DOWN,
            "LONG" | "VERY_LONG" => UP,
            _ => AXIS_TEXT,
        };
        ui.label(egui::RichText::new(format!("{} — {} — period {:.2} (prev {:.2}) — range [{:.2} .. {:.2}] — close {:.4} — as of {}",
            snap.symbol, snap.period_label, snap.period, snap.period_prev, snap.period_min_64, snap.period_max_64, snap.last_close, snap.as_of)).strong().color(color));
        ui.separator();
        egui::Grid::new("ht_dcperiod_summary")
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
                    egui::RichText::new(format!("{:.2}", snap.period))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Period prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.period_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Period min (64)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.period_min_64))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Period max (64)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.period_max_64))
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

pub fn render_ht_dcphase_snapshot(ui: &mut egui::Ui, snap: &HtDcphaseSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.phase_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥64 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.phase_label.as_str() {
            "CYCLE_BOTTOM" => UP,
            "CYCLE_TOP" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(egui::RichText::new(format!("{} — {} — phase {:.2}° (prev {:.2}°) — Δ {:+.2}° — period {:.2} — close {:.4} — as of {}",
            snap.symbol, snap.phase_label, snap.phase_deg, snap.phase_deg_prev, snap.phase_delta, snap.period, snap.last_close, snap.as_of)).strong().color(color));
        ui.separator();
        egui::Grid::new("ht_dcphase_summary")
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
                ui.label(egui::RichText::new("Phase (deg)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}°", snap.phase_deg))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Phase prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}°", snap.phase_deg_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Phase Δ").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}°", snap.phase_delta))
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

pub fn render_ht_phasor_snapshot(ui: &mut egui::Ui, snap: &HtPhasorSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.phasor_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥64 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.phasor_label.as_str() {
            "STRONG_CYCLE" => UP,
            "WEAK_CYCLE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(egui::RichText::new(format!("{} — {} — I {:+.4} — Q {:+.4} — magnitude {:.4} — phase {:+.2}° — close {:.4} — as of {}",
            snap.symbol, snap.phasor_label, snap.i_comp, snap.q_comp, snap.magnitude, snap.phase_deg, snap.last_close, snap.as_of)).strong().color(color));
        ui.separator();
        egui::Grid::new("ht_phasor_summary")
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
                ui.label(egui::RichText::new("I (in-phase)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.i_comp))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("I prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.i_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Q (quadrature)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.q_comp))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Q prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.q_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Magnitude").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.magnitude))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Phase (deg)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}°", snap.phase_deg))
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

pub fn render_ht_sine_snapshot(ui: &mut egui::Ui, snap: &HtSineSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.sine_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥64 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.sine_label.as_str() {
            "CYCLE_TURN_UP" | "BULL" => UP,
            "CYCLE_TURN_DOWN" | "BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(egui::RichText::new(format!("{} — {} — sine {:+.3} (prev {:+.3}) — leadsine {:+.3} (prev {:+.3}) — crossover {} — period {:.2} — close {:.4} — as of {}",
            snap.symbol, snap.sine_label, snap.sine, snap.sine_prev, snap.leadsine, snap.leadsine_prev, snap.crossover, snap.period, snap.last_close, snap.as_of)).strong().color(color));
        ui.separator();
        egui::Grid::new("ht_sine_summary")
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
                ui.label(egui::RichText::new("Sine").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}", snap.sine))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Sine prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}", snap.sine_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Leadsine").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}", snap.leadsine))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Leadsine prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}", snap.leadsine_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Crossover").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.crossover))
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

pub fn render_ht_trendline_snapshot(ui: &mut egui::Ui, snap: &HtTrendlineSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.ht_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥64 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.ht_label.as_str() {
            "BULL" | "WEAK_BULL" => UP,
            "BEAR" | "WEAK_BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(egui::RichText::new(format!("{} — {} — period {:.2} — trendline {:.4} (prev {:.4}) — spread {:+.4} ({:+.3}%) — close {:.4} — as of {}",
            snap.symbol, snap.ht_label, snap.period,
            snap.trendline_value, snap.trendline_prev,
            snap.spread, snap.spread_pct * 100.0, snap.last_close, snap.as_of)).strong().color(color));
        ui.separator();
        egui::Grid::new("ht_trendline_summary")
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
                ui.label(egui::RichText::new("Detected period").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.period))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Trendline").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.trendline_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Trendline prev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.trendline_prev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Spread (close − trendline)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.spread))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Spread %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}%", snap.spread_pct * 100.0))
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
