//! Pure snapshot-display renderers for the research floating windows (ADR-125
//! Phase 1 step 3): the per-window display bodies, free functions over
//! `(&mut egui::Ui, &Snapshot)` with no `TyphooNApp` access — the egui analog of
//! the symbol-investigation packet's formatter layer. Crate-movable: the future
//! `typhoon-research-ui` crate may depend on egui directly.
use crate::app::common::{AXIS_TEXT, DOWN, UP};
// Glob: these renderers cover ~80 distinct research snapshot DTOs; listing each is
// noise. Unused names from a glob do not warn.
use typhoon_engine::core::research::*;

pub(super) fn render_avgprice_snapshot(ui: &mut egui::Ui, snap: &AvgpriceSnapshot) {
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

pub(super) fn render_medprice_snapshot(ui: &mut egui::Ui, snap: &MedpriceSnapshot) {
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

pub(super) fn render_typprice_snapshot(ui: &mut egui::Ui, snap: &TypPriceSnapshot) {
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

pub(super) fn render_wclprice_snapshot(ui: &mut egui::Ui, snap: &WclPriceSnapshot) {
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

pub(super) fn render_variance_snapshot(ui: &mut egui::Ui, snap: &VarianceSnapshot) {
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

pub(super) fn render_accbands_snapshot(ui: &mut egui::Ui, snap: &AccbandsSnapshot) {
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

pub(super) fn render_adx_snapshot(ui: &mut egui::Ui, snap: &AdxSnapshot) {
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

pub(super) fn render_adxr_snapshot(ui: &mut egui::Ui, snap: &AdxrSnapshot) {
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

pub(super) fn render_apo_snapshot(ui: &mut egui::Ui, snap: &ApoSnapshot) {
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

pub(super) fn render_aroon_snapshot(ui: &mut egui::Ui, snap: &AroonSnapshot) {
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

pub(super) fn render_aroonosc_snapshot(ui: &mut egui::Ui, snap: &AroonoscSnapshot) {
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

pub(super) fn render_cci_snapshot(ui: &mut egui::Ui, snap: &CciSnapshot) {
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

pub(super) fn render_cdl_belt_hold_snapshot(ui: &mut egui::Ui, snap: &CdlBeltHoldSnapshot) {
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

pub(super) fn render_cdl_high_wave_snapshot(ui: &mut egui::Ui, snap: &CdlHighWaveSnapshot) {
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

pub(super) fn render_cdl_long_line_snapshot(ui: &mut egui::Ui, snap: &CdlLongLineSnapshot) {
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

pub(super) fn render_cdl_short_line_snapshot(ui: &mut egui::Ui, snap: &CdlShortLineSnapshot) {
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

pub(super) fn render_chaikosc_snapshot(ui: &mut egui::Ui, snap: &ChaikoscSnapshot) {
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

pub(super) fn render_chop_snapshot(ui: &mut egui::Ui, snap: &ChopSnapshot) {
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

pub(super) fn render_cmf_snapshot(ui: &mut egui::Ui, snap: &CmfSnapshot) {
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

pub(super) fn render_dema_snapshot(ui: &mut egui::Ui, snap: &DemaSnapshot) {
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

pub(super) fn render_donchian_snapshot(ui: &mut egui::Ui, snap: &DonchianSnapshot) {
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

pub(super) fn render_dpo_snapshot(ui: &mut egui::Ui, snap: &DpoSnapshot) {
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

pub(super) fn render_dx_snapshot(ui: &mut egui::Ui, snap: &DxSnapshot) {
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

pub(super) fn render_fisher_snapshot(ui: &mut egui::Ui, snap: &FisherSnapshot) {
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

pub(super) fn render_force_index_snapshot(ui: &mut egui::Ui, snap: &ForceIndexSnapshot) {
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

pub(super) fn render_frama_snapshot(ui: &mut egui::Ui, snap: &FramaSnapshot) {
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

pub(super) fn render_heikin_snapshot(ui: &mut egui::Ui, snap: &HeikinSnapshot) {
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

pub(super) fn render_hma_snapshot(ui: &mut egui::Ui, snap: &HmaSnapshot) {
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

pub(super) fn render_ht_dcperiod_snapshot(ui: &mut egui::Ui, snap: &HtDcperiodSnapshot) {
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

pub(super) fn render_ht_dcphase_snapshot(ui: &mut egui::Ui, snap: &HtDcphaseSnapshot) {
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

pub(super) fn render_ht_phasor_snapshot(ui: &mut egui::Ui, snap: &HtPhasorSnapshot) {
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

pub(super) fn render_ht_sine_snapshot(ui: &mut egui::Ui, snap: &HtSineSnapshot) {
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

pub(super) fn render_ht_trendline_snapshot(ui: &mut egui::Ui, snap: &HtTrendlineSnapshot) {
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

pub(super) fn render_ht_trendmode_snapshot(ui: &mut egui::Ui, snap: &HtTrendmodeSnapshot) {
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

pub(super) fn render_ibs_snapshot(ui: &mut egui::Ui, snap: &IbsSnapshot) {
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

pub(super) fn render_ichimoku_snapshot(ui: &mut egui::Ui, snap: &IchimokuSnapshot) {
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

pub(super) fn render_kama_snapshot(ui: &mut egui::Ui, snap: &KamaSnapshot) {
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

pub(super) fn render_keltner_snapshot(ui: &mut egui::Ui, snap: &KeltnerSnapshot) {
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

pub(super) fn render_klinger_snapshot(ui: &mut egui::Ui, snap: &KlingerSnapshot) {
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

pub(super) fn render_kst_snapshot(ui: &mut egui::Ui, snap: &KstSnapshot) {
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

pub(super) fn render_laguerre_rsi_snapshot(ui: &mut egui::Ui, snap: &LaguerreRsiSnapshot) {
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

pub(super) fn render_linearreg_angle_snapshot(ui: &mut egui::Ui, snap: &LinearregAngleSnapshot) {
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

pub(super) fn render_linearreg_slope_snapshot(ui: &mut egui::Ui, snap: &LinearregSlopeSnapshot) {
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

pub(super) fn render_linearreg_snapshot(ui: &mut egui::Ui, snap: &LinearregSnapshot) {
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

pub(super) fn render_linreg_snapshot(ui: &mut egui::Ui, snap: &LinregSnapshot) {
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

pub(super) fn render_macdext_snapshot(ui: &mut egui::Ui, snap: &MacdextSnapshot) {
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

pub(super) fn render_macdfix_snapshot(ui: &mut egui::Ui, snap: &MacdfixSnapshot) {
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

pub(super) fn render_mass_index_snapshot(ui: &mut egui::Ui, snap: &MassIndexSnapshot) {
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

pub(super) fn render_mass_snapshot(ui: &mut egui::Ui, snap: &MassSnapshot) {
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

pub(super) fn render_mavp_snapshot(ui: &mut egui::Ui, snap: &MavpSnapshot) {
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

pub(super) fn render_mesa_sine_snapshot(ui: &mut egui::Ui, snap: &MesaSineSnapshot) {
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

pub(super) fn render_mfi_snapshot(ui: &mut egui::Ui, snap: &MfiSnapshot) {
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

pub(super) fn render_midpoint_snapshot(ui: &mut egui::Ui, snap: &MidpointSnapshot) {
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

pub(super) fn render_midprice_snapshot(ui: &mut egui::Ui, snap: &MidpriceSnapshot) {
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

pub(super) fn render_minmaxindex_snapshot(ui: &mut egui::Ui, snap: &MinMaxIndexSnapshot) {
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

pub(super) fn render_minus_di_snapshot(ui: &mut egui::Ui, snap: &MinusDiSnapshot) {
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

pub(super) fn render_minus_dm_snapshot(ui: &mut egui::Ui, snap: &MinusDmSnapshot) {
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

pub(super) fn render_mom_snapshot(ui: &mut egui::Ui, snap: &MomSnapshot) {
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

pub(super) fn render_natr_snapshot(ui: &mut egui::Ui, snap: &NatrSnapshot) {
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

pub(super) fn render_obv_snapshot(ui: &mut egui::Ui, snap: &ObvSnapshot) {
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

pub(super) fn render_pgo_snapshot(ui: &mut egui::Ui, snap: &PgoSnapshot) {
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

pub(super) fn render_pivots_snapshot(ui: &mut egui::Ui, snap: &PivotsSnapshot) {
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

pub(super) fn render_plus_di_snapshot(ui: &mut egui::Ui, snap: &PlusDiSnapshot) {
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

pub(super) fn render_plus_dm_snapshot(ui: &mut egui::Ui, snap: &PlusDmSnapshot) {
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

pub(super) fn render_ppo_snapshot(ui: &mut egui::Ui, snap: &PpoSnapshot) {
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

pub(super) fn render_psar_snapshot(ui: &mut egui::Ui, snap: &PsarSnapshot) {
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

pub(super) fn render_rainbow_snapshot(ui: &mut egui::Ui, snap: &RainbowSnapshot) {
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

pub(super) fn render_sarext_snapshot(ui: &mut egui::Ui, snap: &SarextSnapshot) {
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

pub(super) fn render_squeeze_snapshot(ui: &mut egui::Ui, snap: &SqueezeSnapshot) {
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

pub(super) fn render_stochf_snapshot(ui: &mut egui::Ui, snap: &StochfSnapshot) {
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

pub(super) fn render_stochrsi_snapshot(ui: &mut egui::Ui, snap: &StochRsiSnapshot) {
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

pub(super) fn render_supertrend_snapshot(ui: &mut egui::Ui, snap: &SupertrendSnapshot) {
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

pub(super) fn render_tema_snapshot(ui: &mut egui::Ui, snap: &TemaSnapshot) {
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

pub(super) fn render_trange_snapshot(ui: &mut egui::Ui, snap: &TrangeSnapshot) {
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

pub(super) fn render_trix_snapshot(ui: &mut egui::Ui, snap: &TrixSnapshot) {
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

pub(super) fn render_ttm_squeeze_snapshot(ui: &mut egui::Ui, snap: &TtmSqueezeSnapshot) {
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

pub(super) fn render_ultosc_snapshot(ui: &mut egui::Ui, snap: &UltoscSnapshot) {
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

pub(super) fn render_vortex_snapshot(ui: &mut egui::Ui, snap: &VortexSnapshot) {
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

pub(super) fn render_willr_snapshot(ui: &mut egui::Ui, snap: &WillrSnapshot) {
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

pub(super) fn render_wma_snapshot(ui: &mut egui::Ui, snap: &WmaSnapshot) {
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

pub(super) fn render_zigzag_snapshot(ui: &mut egui::Ui, snap: &ZigzagSnapshot) {
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
