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

pub(super) fn render_acrl_snapshot(ui: &mut egui::Ui, snap: &AccrualsSnapshot) {
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

pub(super) fn render_adf_snapshot(ui: &mut egui::Ui, snap: &DickeyFullerSnapshot) {
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

pub(super) fn render_adtest_snapshot(ui: &mut egui::Ui, snap: &AdtestSnapshot) {
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

pub(super) fn render_altz_snapshot(ui: &mut egui::Ui, snap: &AltmanZSnapshot) {
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

pub(super) fn render_amihud_snapshot(ui: &mut egui::Ui, snap: &AmihudIlliqSnapshot) {
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

pub(super) fn render_apen_snapshot(ui: &mut egui::Ui, snap: &ApenSnapshot) {
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

pub(super) fn render_archlm_snapshot(ui: &mut egui::Ui, snap: &ArchLmSnapshot) {
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

pub(super) fn render_autocor_snapshot(ui: &mut egui::Ui, snap: &AutocorrelationSnapshot) {
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

pub(super) fn render_automi_snapshot(ui: &mut egui::Ui, snap: &AutomiSnapshot) {
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

pub(super) fn render_bbsqueeze_snapshot(ui: &mut egui::Ui, snap: &BbsqueezeSnapshot) {
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

pub(super) fn render_bdstest_snapshot(ui: &mut egui::Ui, snap: &BdsTestSnapshot) {
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

pub(super) fn render_beta_snapshot(ui: &mut egui::Ui, snap: &BetaSnapshot) {
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

pub(super) fn render_bipower_snapshot(ui: &mut egui::Ui, snap: &BipowerVariationSnapshot) {
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

pub(super) fn render_bnsjump_snapshot(ui: &mut egui::Ui, snap: &BnsjumpSnapshot) {
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

pub(super) fn render_break_snapshot(ui: &mut egui::Ui, snap: &BreakoutSnapshot) {
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

pub(super) fn render_breuschpagan_snapshot(ui: &mut egui::Ui, snap: &BreuschPaganSnapshot) {
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

pub(super) fn render_burgspec_snapshot(ui: &mut egui::Ui, snap: &BurgSpecSnapshot) {
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

pub(super) fn render_burke_snapshot(ui: &mut egui::Ui, snap: &BurkeRatioSnapshot) {
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

pub(super) fn render_calmar_snapshot(ui: &mut egui::Ui, snap: &CalmarRatioSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.calmar_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.calmar_label.as_str() {
            "GOOD" | "EXCELLENT" => UP,
            "VERY_POOR" | "POOR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — Calmar {:.2} — {} bars — as of {}",
                snap.symbol, snap.calmar_label, snap.calmar_ratio, snap.bars_used, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("calmar_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Total return").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.total_return_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Annualized return").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.annualized_return_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Max drawdown").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.max_drawdown_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Calmar ratio").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.calmar_ratio))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_ccrl_snapshot(ui: &mut egui::Ui, snap: &CashCycleSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.periods_used == 0 {
        ui.label(
            egui::RichText::new("No data — run FA for this symbol, then click Compute.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.efficiency_label.as_str() {
            "EFFICIENT" => UP,
            "INEFFICIENT" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — trend: {} — latest: {} — CCC {:.1}d — as of {}",
                snap.symbol,
                snap.efficiency_label,
                snap.trend_label,
                snap.latest_period,
                snap.ccc_days,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("ccrl_sub")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("DSO (days sales outstanding)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.1} days", snap.dso_days))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("DIO (days inventory outstanding)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.1} days", snap.dio_days))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("DPO (days payables outstanding)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.1} days", snap.dpo_days))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Prior CCC").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1} days", snap.prior_ccc_days))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("CCC change vs prior").small().strong());
                let cc = if snap.ccc_change_days < 0.0 {
                    UP
                } else if snap.ccc_change_days > 0.0 {
                    DOWN
                } else {
                    AXIS_TEXT
                };
                ui.label(
                    egui::RichText::new(format!("{:+.1} days", snap.ccc_change_days))
                        .small()
                        .monospace()
                        .color(cc),
                );
                ui.end_row();
                ui.label(egui::RichText::new("3y avg CCC").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1} days", snap.ccc_3y_avg_days))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.periods.is_empty() {
            ui.separator();
            ui.label(
                egui::RichText::new("Per-period history")
                    .strong()
                    .small()
                    .color(AXIS_TEXT),
            );
            egui::Grid::new("ccrl_grid")
                .striped(true)
                .num_columns(5)
                .spacing([14.0, 3.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Period").strong().small());
                    ui.label(egui::RichText::new("DSO").strong().small());
                    ui.label(egui::RichText::new("DIO").strong().small());
                    ui.label(egui::RichText::new("DPO").strong().small());
                    ui.label(egui::RichText::new("CCC").strong().small());
                    ui.end_row();
                    for row in &snap.periods {
                        ui.label(egui::RichText::new(&row.period).monospace().small());
                        ui.label(
                            egui::RichText::new(format!("{:.0}", row.dso_days))
                                .monospace()
                                .small(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.0}", row.dio_days))
                                .monospace()
                                .small(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.0}", row.dpo_days))
                                .monospace()
                                .small(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.0}", row.ccc_days))
                                .monospace()
                                .small(),
                        );
                        ui.end_row();
                    }
                });
        }
    }
}

pub(super) fn render_cfvar_snapshot(ui: &mut egui::Ui, snap: &CornishFisherSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.cfvar_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.cfvar_label.as_str() {
            "BENIGN" => UP,
            "EXTREME_DEVIATION" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — CF-VaR(5%)={:.2}% vs Gauss {:.2}% — adj {:+.3}pp — as of {}",
                snap.symbol,
                snap.cfvar_label,
                snap.cf_var_5pct_pct,
                snap.gauss_var_5pct_pct,
                snap.cf_adjustment_5pct_pct,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("cfvar_summary")
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
                ui.label(egui::RichText::new("Mean ret (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.mean_ret_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("σ ret (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.sigma_ret_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Skewness γ₃").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.skewness))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Excess kurtosis γ₄").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.excess_kurtosis))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Gauss VaR 5% (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.gauss_var_5pct_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("CF-VaR 5% (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.cf_var_5pct_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Gauss VaR 1% (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.gauss_var_1pct_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("CF-VaR 1% (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.cf_var_1pct_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Adj 5% (pp)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.cf_adjustment_5pct_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Skew term @ 5%").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.skew_term_5pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Kurt term @ 5%").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.kurt_term_5pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_closeplc_snapshot(ui: &mut egui::Ui, snap: &ClosePlacementSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.placement_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥20 bars with high > low.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.placement_label.as_str() {
            "STRONG_BULL" | "BULL" => UP,
            "STRONG_BEAR" | "BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — avg {:.3} — {} bars — as of {}",
                snap.symbol, snap.placement_label, snap.avg_placement, snap.bars_used, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("closeplc_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Mean / median placement")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.3} / {:.3}",
                        snap.avg_placement, snap.median_placement
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Latest bar placement").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.latest_placement))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("% near high (>0.8)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}%", snap.pct_near_high))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("% near low (<0.2)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}%", snap.pct_near_low))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_cor_snapshot(ui: &mut egui::Ui, snap: &CorrelationMatrix) {
    ui.separator();
    if snap.symbol.is_empty() || snap.cells.is_empty() {
        ui.label(
            egui::RichText::new(
                "No data — run PEERS + HP for the symbol and its peers, then click Compute.",
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
                "{} — {}-day window — avg |ρ| {:.2} — highest {} · lowest {} — as of {}",
                snap.symbol,
                snap.window_days,
                snap.mean_correlation,
                snap.highest_corr_symbol,
                snap.lowest_corr_symbol,
                snap.as_of
            ))
            .strong()
            .color(AXIS_TEXT),
        );
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("cor_grid")
                .striped(true)
                .num_columns(4)
                .min_col_width(100.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Peer")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(egui::RichText::new("ρ").color(AXIS_TEXT).small().strong());
                    ui.label(egui::RichText::new("β").color(AXIS_TEXT).small().strong());
                    ui.label(egui::RichText::new("N").color(AXIS_TEXT).small().strong());
                    ui.end_row();
                    for c in &snap.cells {
                        let color = if c.correlation >= 0.0 { UP } else { DOWN };
                        ui.label(
                            egui::RichText::new(&c.peer_symbol)
                                .small()
                                .monospace()
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:+.3}", c.correlation))
                                .color(color)
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:+.3}", c.beta_vs_peer))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{}", c.n_observations))
                                .small()
                                .monospace(),
                        );
                        ui.end_row();
                    }
                });
        });
    }
}

pub(super) fn render_cordim_snapshot(ui: &mut egui::Ui, snap: &CordimSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.cordim_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥60 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.cordim_label.as_str() {
            "LOW_DIM" => UP,
            "STOCHASTIC" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — D2 {:.3} — as of {}",
                snap.symbol, snap.cordim_label, snap.d2, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("cordim_summary")
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
                ui.label(egui::RichText::new("Radii fitted").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.radii_count))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("D2 (correlation dim)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.d2))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Fit R²").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.r_squared))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_covg_snapshot(ui: &mut egui::Ui, snap: &CoverageSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.coverage_label == "NONE" {
        ui.label(egui::RichText::new("No data — needs ANR (price targets / consensus) and/or UPDG (rating changes) cached.")
            .color(AXIS_TEXT).small());
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.coverage_label.as_str() {
            "EXPANDING" => UP,
            "CONTRACTING" | "THIN" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — composite {:.1} — {} analysts — as of {}",
                snap.symbol,
                snap.coverage_label,
                snap.composite_score,
                snap.num_analysts,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("covg_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Target mean (low / high)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "${:.2} (${:.2} / ${:.2})",
                        snap.target_mean, snap.target_low, snap.target_high
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Consensus SB/B/H/S/SS")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{}/{}/{}/{}/{}",
                        snap.consensus_strong_buy,
                        snap.consensus_buy,
                        snap.consensus_hold,
                        snap.consensus_sell,
                        snap.consensus_strong_sell
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Bullish ratio").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.0}%", snap.consensus_bull_ratio * 100.0))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Upgrades / Downgrades 90d")
                        .small()
                        .strong(),
                );
                let nc = if snap.net_90d > 0 {
                    UP
                } else if snap.net_90d < 0 {
                    DOWN
                } else {
                    AXIS_TEXT
                };
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {} (net {:+})",
                        snap.upgrades_90d, snap.downgrades_90d, snap.net_90d
                    ))
                    .small()
                    .monospace()
                    .color(nc),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Breadth / Consensus / Churn score")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.0} / {:.0} / {:.0}",
                        snap.breadth_score, snap.consensus_score, snap.churn_score
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Inputs used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}/3", snap.inputs_available))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_credit_snapshot(ui: &mut egui::Ui, snap: &CreditSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.inputs_available == 0 {
        ui.label(
            egui::RichText::new(
                "No data — run ALTZ, PTFS, LEV and ACRL for this symbol, then click Compute.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.letter_grade.as_str() {
            "AAA" | "AA" | "A" | "BBB" => UP,
            "CCC" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — {} — composite: {:.1} / 100 — as of {}",
                snap.symbol, snap.letter_grade, snap.credit_label, snap.composite_score, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("credit_sub")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Altman Z").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2} ({})", snap.altman_z, snap.altman_zone))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Piotroski score").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{}/9 ({})",
                        snap.piotroski_score, snap.piotroski_label
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Leverage summary").small().strong());
                ui.label(
                    egui::RichText::new(&snap.leverage_summary)
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Accruals trend").small().strong());
                ui.label(
                    egui::RichText::new(&snap.accruals_trend)
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("TTM cash conversion").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}%", snap.accruals_ttm_cash_conversion_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Inputs available").small().strong());
                ui.label(
                    egui::RichText::new(format!("{} / 4", snap.inputs_available))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.components.is_empty() {
            ui.separator();
            ui.label(
                egui::RichText::new("Component contributions")
                    .strong()
                    .small()
                    .color(AXIS_TEXT),
            );
            egui::Grid::new("credit_grid")
                .striped(true)
                .num_columns(5)
                .spacing([14.0, 3.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Component").strong().small());
                    ui.label(egui::RichText::new("Value").strong().small());
                    ui.label(egui::RichText::new("Score").strong().small());
                    ui.label(egui::RichText::new("Weight").strong().small());
                    ui.label(egui::RichText::new("Contribution").strong().small());
                    ui.end_row();
                    for c in &snap.components {
                        ui.label(egui::RichText::new(&c.name).monospace().small());
                        ui.label(egui::RichText::new(&c.value).monospace().small());
                        ui.label(
                            egui::RichText::new(format!("{:.1}", c.score))
                                .monospace()
                                .small(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.0}%", c.weight))
                                .monospace()
                                .small(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.1}", c.contribution))
                                .monospace()
                                .small(),
                        );
                        ui.end_row();
                    }
                });
        }
    }
}

pub(super) fn render_cusum_snapshot(ui: &mut egui::Ui, snap: &CusumBreakSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.cusum_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.cusum_label.as_str() {
            "STABLE" => UP,
            "BREAK_DETECTED" | "STRONG_BREAK" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — D={:.3} — dir {} — bar {} — as of {}",
                snap.symbol,
                snap.cusum_label,
                snap.test_statistic,
                snap.direction_at_max,
                snap.max_abs_bar,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("cusum_summary")
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
                ui.label(egui::RichText::new("max |S_t|").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.max_abs_cusum))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("D = max|S_t|/√n").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.test_statistic))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("bar at max").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.max_abs_bar))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("direction at max").small().strong());
                ui.label(
                    egui::RichText::new(&snap.direction_at_max)
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("crit 10% / 5% / 1%").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{:.2} / {:.2} / {:.2}",
                        snap.crit_10pct, snap.crit_5pct, snap.crit_1pct
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Reject stability").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.reject_stability))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_cvar_snapshot(ui: &mut egui::Ui, snap: &CVaRSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.cvar_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥100 log returns.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.cvar_label.as_str() {
            "MINIMAL" | "LOW" => UP,
            "HIGH" | "EXTREME" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — ES(5%) {:+.3}% — ES(1%) {:+.3}% — {} bars — as of {}",
                snap.symbol,
                snap.cvar_label,
                snap.cvar_5pct_ret_pct,
                snap.cvar_1pct_ret_pct,
                snap.bars_used,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("cvar_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("VaR (5%) daily return")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:+.3}%", snap.var_5pct_ret_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("CVaR / ES (5%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}%", snap.cvar_5pct_ret_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Tail days (5%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.tail_days_5pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("VaR (1%) daily return")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:+.3}%", snap.var_1pct_ret_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("CVaR / ES (1%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}%", snap.cvar_1pct_ret_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Tail days (1%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.tail_days_1pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_dcf_snapshot(ui: &mut egui::Ui, snap: &DcfSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() {
        ui.label(
            egui::RichText::new(
                "No data — run DES + FA/IS/CF for this symbol first, then click Compute.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
        ui.label(
            egui::RichText::new("Tip: run WACC for this symbol to use it as the discount rate.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = if snap.implied_price > 0.0 { UP } else { DOWN };
        ui.label(
            egui::RichText::new(format!(
                "{} — implied price ${:.2}",
                snap.symbol, snap.implied_price
            ))
            .strong()
            .size(16.0)
            .color(color),
        );
        ui.label(
            egui::RichText::new(format!(
                "{} — as of {} — WACC {:.2}%",
                snap.method, snap.as_of, snap.wacc_pct
            ))
            .color(AXIS_TEXT)
            .small(),
        );
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("dcf_sum_grid")
                .striped(true)
                .num_columns(2)
                .min_col_width(200.0)
                .show(ui, |ui| {
                    let row = |ui: &mut egui::Ui, k: &str, v: String| {
                        ui.label(egui::RichText::new(k).color(AXIS_TEXT).small());
                        ui.label(egui::RichText::new(v).small().monospace().strong());
                        ui.end_row();
                    };
                    row(
                        ui,
                        "Base revenue (TTM)",
                        format!("${:.0}M", snap.base_revenue / 1e6),
                    );
                    row(
                        ui,
                        "Base FCFF (TTM)",
                        format!("${:.0}M", snap.base_fcff / 1e6),
                    );
                    row(ui, "FCFF margin", format!("{:.2}%", snap.fcff_margin_pct));
                    row(ui, "Revenue growth", format!("{:.2}%", snap.growth_pct));
                    row(
                        ui,
                        "Terminal growth",
                        format!("{:.2}%", snap.terminal_growth_pct),
                    );
                    row(
                        ui,
                        "Enterprise value",
                        format!("${:.0}M", snap.enterprise_value / 1e6),
                    );
                    row(
                        ui,
                        "(+) Cash",
                        format!("${:.0}M", snap.cash_and_equivalents / 1e6),
                    );
                    row(ui, "(-) Debt", format!("${:.0}M", snap.total_debt / 1e6));
                    row(
                        ui,
                        "Equity value",
                        format!("${:.0}M", snap.equity_value / 1e6),
                    );
                    row(
                        ui,
                        "Shares outstanding",
                        format!("{:.0}M", snap.shares_outstanding / 1e6),
                    );
                    row(ui, "Implied price", format!("${:.2}", snap.implied_price));
                });
            ui.separator();
            egui::Grid::new("dcf_years_grid")
                .striped(true)
                .num_columns(6)
                .min_col_width(80.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Year")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Revenue")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("EBIT")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("NOPAT")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("FCFF")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("PV FCFF")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.end_row();
                    for y in &snap.years {
                        ui.label(
                            egui::RichText::new(format!("{}", y.year))
                                .small()
                                .monospace()
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new(format!("${:.0}M", y.revenue / 1e6))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("${:.0}M", y.ebit / 1e6))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("${:.0}M", y.nopat / 1e6))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("${:.0}M", y.fcff / 1e6))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("${:.0}M", y.pv_fcff / 1e6))
                                .small()
                                .monospace(),
                        );
                        ui.end_row();
                    }
                });
        });
        if !snap.note.is_empty() {
            ui.separator();
            ui.label(
                egui::RichText::new(&snap.note)
                    .color(DOWN)
                    .small()
                    .italics(),
            );
        }
    }
}

pub(super) fn render_dddur_snapshot(ui: &mut egui::Ui, snap: &DrawdownDurationSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.dddur_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.dddur_label.as_str() {
            "MOSTLY_DRY" => UP,
            "DEEP_WATER" | "PERSISTENT_DD" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — underwater {:.1}% — events {} — max {} bars — as of {}",
                snap.symbol,
                snap.dddur_label,
                snap.pct_time_underwater,
                snap.dd_event_count,
                snap.max_dd_duration_bars,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("dddur_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Drawdown events (closed)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{}", snap.dd_event_count))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Max duration (bars)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.max_dd_duration_bars))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Mean duration").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.mean_dd_duration_bars))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Median duration").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.median_dd_duration_bars))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Total bars underwater")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{}", snap.total_bars_underwater))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("% time underwater").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.pct_time_underwater))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Currently underwater").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.currently_underwater))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Current DD duration").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.current_dd_duration_bars))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_ddm_snapshot(ui: &mut egui::Ui, snap: &DdmSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() {
        ui.label(
            egui::RichText::new("No data — click Compute (needs dividend history cached via DVD).")
                .color(AXIS_TEXT)
                .small(),
        );
        ui.label(
            egui::RichText::new(
                "Tip: run WACC first for this symbol to use Re as required return.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
    } else {
        let color = if snap.implied_price > 0.0 { UP } else { DOWN };
        ui.label(
            egui::RichText::new(format!(
                "{} — implied price ${:.2}",
                snap.symbol, snap.implied_price
            ))
            .strong()
            .size(16.0)
            .color(color),
        );
        ui.label(
            egui::RichText::new(format!("as of {} ({})", snap.as_of, snap.method))
                .color(AXIS_TEXT)
                .small(),
        );
        ui.separator();
        egui::Grid::new("ddm_grid")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                let row = |ui: &mut egui::Ui, k: &str, v: String| {
                    ui.label(egui::RichText::new(k).color(AXIS_TEXT).small());
                    ui.label(egui::RichText::new(v).small().monospace().strong());
                    ui.end_row();
                };
                row(
                    ui,
                    "Trailing annual dividend (D0)",
                    format!("${:.4}", snap.annual_dividend),
                );
                row(
                    ui,
                    "Implied growth (g)",
                    format!("{:.2}%", snap.implied_growth_pct),
                );
                row(
                    ui,
                    "Required return (r)",
                    format!("{:.2}%", snap.required_return_pct),
                );
                row(ui, "Growth source", snap.growth_source.clone());
                row(ui, "Return source", snap.return_source.clone());
                row(
                    ui,
                    "D1 = D0 × (1 + g)",
                    format!(
                        "${:.4}",
                        snap.annual_dividend * (1.0 + snap.implied_growth_pct / 100.0)
                    ),
                );
                row(
                    ui,
                    "Implied price (D1 / (r − g))",
                    format!("${:.2}", snap.implied_price),
                );
            });
        if !snap.note.is_empty() {
            ui.separator();
            ui.label(
                egui::RichText::new(&snap.note)
                    .color(DOWN)
                    .small()
                    .italics(),
            );
        }
    }
}

pub(super) fn render_des_snapshot(ui: &mut egui::Ui, snap: &DailyEventStreakSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.streak_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — needs ≥20 cached daily bars for the subject.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.streak_label.as_str() {
            "STRONG_UPTREND" | "UPTREND_BIAS" => UP,
            "STRONG_DOWNTREND" | "DOWNTREND_BIAS" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — up rate {:.0}% — current {} × {} — as of {}",
                snap.symbol,
                snap.streak_label,
                snap.up_day_rate_pct,
                snap.current_streak_type,
                snap.current_streak_len,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("des_summary")
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
                ui.label(
                    egui::RichText::new("Up / down / flat days")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {} / {}",
                        snap.up_days, snap.down_days, snap.flat_days
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Longest up / down streak")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        snap.longest_up_streak, snap.longest_down_streak
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Avg up / down move %").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{:+.2}% / {:+.2}%",
                        snap.avg_up_move_pct, snap.avg_down_move_pct
                    ))
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

pub(super) fn render_dfa_snapshot(ui: &mut egui::Ui, snap: &DetrendedFluctuationSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.dfa_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥100 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.dfa_label.as_str() {
            "PERSISTENT" | "STRONGLY_PERSISTENT" => UP,
            "ANTI_PERSISTENT" | "MEAN_REVERTING" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — α {:.4} — R² {:.3} — {} scales — {} bars — as of {}",
                snap.symbol,
                snap.dfa_label,
                snap.alpha,
                snap.r_squared,
                snap.num_scales,
                snap.bars_used,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("dfa_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("α (Hurst-like)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.alpha))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Scales sampled").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.num_scales))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Log-log R²").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.r_squared))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_divg_snapshot(ui: &mut egui::Ui, snap: &DivgSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.total_payments == 0 {
        ui.label(
            egui::RichText::new("No data — run DVD for this symbol, then click Compute.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.trend_label.as_str() {
            "GROWING" => UP,
            "CUTTING" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — {} years covered — as of {}",
                snap.symbol, snap.trend_label, snap.years_covered, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("divg_grid")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Latest payment").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "${:.4} on {}",
                        snap.latest_amount, snap.latest_payment_date
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Annualized dividend").small().strong());
                ui.label(
                    egui::RichText::new(format!("${:.2}", snap.annualized_dividend))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("1Y growth").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.cagr_1y_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("3Y CAGR").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.cagr_3y_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("5Y CAGR").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.cagr_5y_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Consecutive growth years")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{}", snap.consecutive_growth_years))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Consistency").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.0}%", snap.consistency_score_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Total payments").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.total_payments))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        ui.separator();
        ui.label(
            egui::RichText::new("Annual buckets")
                .color(AXIS_TEXT)
                .small(),
        );
        egui::Grid::new("divg_years_grid")
            .striped(true)
            .num_columns(4)
            .spacing([18.0, 3.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Year").strong().small());
                ui.label(egui::RichText::new("Total").strong().small());
                ui.label(egui::RichText::new("Payments").strong().small());
                ui.label(egui::RichText::new("YoY%").strong().small());
                ui.end_row();
                for row in &snap.annual_rows {
                    ui.label(
                        egui::RichText::new(format!("{}", row.year))
                            .monospace()
                            .small(),
                    );
                    ui.label(
                        egui::RichText::new(format!("${:.2}", row.total_amount))
                            .monospace()
                            .small(),
                    );
                    ui.label(
                        egui::RichText::new(format!("{}", row.payment_count))
                            .monospace()
                            .small(),
                    );
                    let col = if row.growth_pct > 0.0 {
                        UP
                    } else if row.growth_pct < 0.0 {
                        DOWN
                    } else {
                        AXIS_TEXT
                    };
                    ui.label(
                        egui::RichText::new(format!("{:+.1}%", row.growth_pct))
                            .monospace()
                            .small()
                            .color(col),
                    );
                    ui.end_row();
                }
            });
    }
}

pub(super) fn render_doweffect_snapshot(ui: &mut egui::Ui, snap: &DayOfWeekEffectSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.dow_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥100 bars with ≥10 per weekday.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.dow_label.as_str() {
            "STRONG_EFFECT" | "MILD_EFFECT" => UP,
            "INCONSISTENT" => DOWN,
            _ => AXIS_TEXT,
        };
        const DOWS: [&str; 5] = ["Mon", "Tue", "Wed", "Thu", "Fri"];
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — best {} ({:.0}%) / worst {} ({:.0}%) — {} wks — as of {}",
                snap.symbol,
                snap.dow_label,
                DOWS[snap.best_dow_idx],
                snap.best_dow_hit_pct,
                DOWS[snap.worst_dow_idx],
                snap.worst_dow_hit_pct,
                snap.weeks_covered,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("doweffect_grid")
            .striped(true)
            .num_columns(4)
            .min_col_width(100.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Day").small().strong());
                ui.label(egui::RichText::new("Hit %").small().strong());
                ui.label(egui::RichText::new("Mean ret %").small().strong());
                ui.label(egui::RichText::new("N").small().strong());
                ui.end_row();
                for i in 0..5 {
                    ui.label(egui::RichText::new(DOWS[i]).small());
                    ui.label(
                        egui::RichText::new(format!("{:.1}%", snap.dow_hit_pct[i]))
                            .small()
                            .monospace(),
                    );
                    ui.label(
                        egui::RichText::new(format!("{:+.3}%", snap.dow_mean_ret_pct[i]))
                            .small()
                            .monospace(),
                    );
                    ui.label(
                        egui::RichText::new(format!("{}", snap.dow_sample_count[i]))
                            .small()
                            .monospace(),
                    );
                    ui.end_row();
                }
            });
    }
}

pub(super) fn render_downvol_snapshot(ui: &mut egui::Ui, snap: &DownsideVolSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.sortino_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.sortino_label.as_str() {
            "GOOD" | "EXCELLENT" => UP,
            "POOR" | "VERY_POOR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — Sortino {:.3} (ann {:.3}) — {} bars — as of {}",
                snap.symbol,
                snap.sortino_label,
                snap.sortino_ratio,
                snap.sortino_ratio_ann,
                snap.bars_used,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("downvol_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Mean log return").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.mean_log_return))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Downside deviation").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.downside_dev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Downside deviation (ann)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.downside_dev_ann))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Upside deviation").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.upside_dev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Sortino (raw)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.sortino_ratio))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Sortino (annualized)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.sortino_ratio_ann))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Downside % of total var")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.downside_pct_of_total))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_drawdar_snapshot(ui: &mut egui::Ui, snap: &DrawDaRSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.drawdar_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — needs ≥30 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.drawdar_label.as_str() {
            "LOW_DD_RISK" => UP,
            "SEVERE_DD_RISK" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — DaR(5%) {:.2}% — max dd {:.2}% — as of {}",
                snap.symbol, snap.drawdar_label, snap.dar_5pct, snap.max_dd_pct, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("drawdar_summary")
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
                ui.label(egui::RichText::new("DaR 5% (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.dar_5pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("CDaR 5% (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.cdar_5pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("DaR 1% (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.dar_1pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("CDaR 1% (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.cdar_1pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Max dd (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.max_dd_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Mean dd (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.mean_dd_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_drawup_snapshot(ui: &mut egui::Ui, snap: &DrawupHistorySnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.rally_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥20 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.rally_label.as_str() {
            "STRONG" | "EXPLOSIVE" => UP,
            "MUTED" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — max drawup {:.2}% — {} bars — as of {}",
                snap.symbol, snap.rally_label, snap.max_drawup_pct, snap.bars_used, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("drawup_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Max drawup").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.max_drawup_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Trough → peak dates").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{} → {}",
                        if snap.max_drawup_trough_date.is_empty() {
                            "—"
                        } else {
                            snap.max_drawup_trough_date.as_str()
                        },
                        if snap.max_drawup_peak_date.is_empty() {
                            "—"
                        } else {
                            snap.max_drawup_peak_date.as_str()
                        }
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Longest drawup (days)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{}", snap.longest_drawup_days))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Rallies ≥5% / ≥10%").small().strong());
                ui.label(
                    egui::RichText::new(format!("{} / {}", snap.rallies_5pct, snap.rallies_10pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Current drawup").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.current_drawup_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_durbinwatson_snapshot(ui: &mut egui::Ui, snap: &DurbinWatsonSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.dw_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥40 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.dw_label.as_str() {
            "NO_AUTOCORR" => UP,
            "STRONG_POS" | "STRONG_NEG" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — d {:.4} — as of {}",
                snap.symbol, snap.dw_label, snap.dw_stat, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("dw_summary")
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
                ui.label(egui::RichText::new("DW d-statistic").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.dw_stat))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Implied ρ̂").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.rho_estimate))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_dvdrank_snapshot(ui: &mut egui::Ui, snap: &DividendGrowthRankSnapshot) {
    ui.separator();
    if snap.symbol.is_empty()
        || snap.rank_label == "INSUFFICIENT_DATA"
        || snap.rank_label == "NO_DATA"
    {
        ui.label(
            egui::RichText::new("No data — needs ≥3 sector peers with DIVG snapshots.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.rank_label.as_str() {
            "TOP_DECILE" | "TOP_QUARTILE" | "ABOVE_MEDIAN" => UP,
            "BELOW_MEDIAN" | "BOTTOM_QUARTILE" | "BOTTOM_DECILE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — trend {} — pct {:.0} — rank {}/{} — sector {} — as of {}",
                snap.symbol,
                snap.rank_label,
                snap.trend_label,
                snap.percentile_rank,
                snap.rank_position,
                snap.peers_considered + 1,
                snap.sector,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("dvdrank_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Subject 3y CAGR %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.cagr_3y_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Consecutive growth years")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{}", snap.consecutive_growth_years))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Sector median / p25 / p75 CAGR")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:+.2}% / {:+.2}% / {:+.2}%",
                        snap.sector_median_cagr_pct,
                        snap.sector_p25_cagr_pct,
                        snap.sector_p75_cagr_pct
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Peers considered / with data")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        snap.peers_considered, snap.peers_with_data
                    ))
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

pub(super) fn render_dvdyieldrank_snapshot(ui: &mut egui::Ui, snap: &DividendYieldRankSnapshot) {
    ui.separator();
    if snap.symbol.is_empty()
        || snap.rank_label == "INSUFFICIENT_DATA"
        || snap.rank_label == "NO_DATA"
    {
        ui.label(egui::RichText::new("No data — subject needs a positive dividend yield and ≥3 sector peers also paying dividends.")
            .color(AXIS_TEXT).small());
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.rank_label.as_str() {
            "TOP_DECILE" | "TOP_QUARTILE" | "ABOVE_MEDIAN" => UP,
            "BELOW_MEDIAN" | "BOTTOM_QUARTILE" | "BOTTOM_DECILE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — yield {:.2}% — pct {:.0} — rank {}/{} — sector {} — as of {}",
                snap.symbol,
                snap.rank_label,
                snap.dividend_yield_pct,
                snap.percentile_rank,
                snap.rank_position,
                snap.peers_considered + 1,
                snap.sector,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("dvdyieldrank_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Subject yield %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.dividend_yield_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Sector median / p25 / p75 yield")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.2}% / {:.2}% / {:.2}%",
                        snap.sector_median_yield_pct,
                        snap.sector_p25_yield_pct,
                        snap.sector_p75_yield_pct
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Peers considered / with data")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        snap.peers_considered, snap.peers_with_data
                    ))
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

pub(super) fn render_earm_snapshot(ui: &mut egui::Ui, snap: &EarmSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.quarters_used < 5 {
        ui.label(
            egui::RichText::new("No data — run FA + EPS for this symbol, then click Compute.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.momentum_label.as_str() {
            "ACCELERATING" => UP,
            "DECELERATING" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — composite {:.0}/100 — {}Q used — as of {}",
                snap.symbol,
                snap.momentum_label,
                snap.composite_score,
                snap.quarters_used,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("earm_grid")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Recent revenue growth (4Q avg)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.recent_revenue_growth_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Prior revenue growth (4Q avg)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.prior_revenue_growth_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Revenue acceleration").small().strong());
                let c_rev = if snap.revenue_acceleration_pct > 0.0 {
                    UP
                } else if snap.revenue_acceleration_pct < 0.0 {
                    DOWN
                } else {
                    AXIS_TEXT
                };
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.revenue_acceleration_pct))
                        .small()
                        .monospace()
                        .color(c_rev),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Recent EPS surprise (4Q avg)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.recent_eps_surprise_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Prior EPS surprise (4Q avg)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.prior_eps_surprise_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("EPS surprise acceleration")
                        .small()
                        .strong(),
                );
                let c_eps = if snap.eps_surprise_acceleration_pct > 0.0 {
                    UP
                } else if snap.eps_surprise_acceleration_pct < 0.0 {
                    DOWN
                } else {
                    AXIS_TEXT
                };
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.eps_surprise_acceleration_pct))
                        .small()
                        .monospace()
                        .color(c_eps),
                );
                ui.end_row();
            });
        ui.separator();
        ui.label(
            egui::RichText::new("Quarterly breakdown (newest first)")
                .color(AXIS_TEXT)
                .small(),
        );
        egui::Grid::new("earm_q_grid")
            .striped(true)
            .num_columns(6)
            .spacing([14.0, 3.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Period").strong().small());
                ui.label(egui::RichText::new("Revenue").strong().small());
                ui.label(egui::RichText::new("YoY%").strong().small());
                ui.label(egui::RichText::new("EPS").strong().small());
                ui.label(egui::RichText::new("Est").strong().small());
                ui.label(egui::RichText::new("Surp%").strong().small());
                ui.end_row();
                for q in &snap.quarters {
                    ui.label(egui::RichText::new(&q.period).monospace().small());
                    ui.label(
                        egui::RichText::new(format!("{:.0}M", q.revenue / 1e6))
                            .monospace()
                            .small(),
                    );
                    let c = if q.revenue_yoy_pct > 0.0 {
                        UP
                    } else if q.revenue_yoy_pct < 0.0 {
                        DOWN
                    } else {
                        AXIS_TEXT
                    };
                    ui.label(
                        egui::RichText::new(format!("{:+.1}%", q.revenue_yoy_pct))
                            .monospace()
                            .small()
                            .color(c),
                    );
                    ui.label(
                        egui::RichText::new(format!("{:.2}", q.eps_actual))
                            .monospace()
                            .small(),
                    );
                    ui.label(
                        egui::RichText::new(format!("{:.2}", q.eps_estimate))
                            .monospace()
                            .small(),
                    );
                    let cs = if q.eps_surprise_pct > 0.0 {
                        UP
                    } else if q.eps_surprise_pct < 0.0 {
                        DOWN
                    } else {
                        AXIS_TEXT
                    };
                    ui.label(
                        egui::RichText::new(format!("{:+.1}%", q.eps_surprise_pct))
                            .monospace()
                            .small()
                            .color(cs),
                    );
                    ui.end_row();
                }
            });
    }
}

pub(super) fn render_earmrank_snapshot(ui: &mut egui::Ui, snap: &EarningsMomentumRankSnapshot) {
    ui.separator();
    if snap.symbol.is_empty()
        || snap.rank_label == "INSUFFICIENT_DATA"
        || snap.rank_label == "NO_DATA"
    {
        ui.label(
            egui::RichText::new("No data — needs ≥3 sector peers with EARM snapshots.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.rank_label.as_str() {
            "TOP_DECILE" | "TOP_QUARTILE" | "ABOVE_MEDIAN" => UP,
            "BELOW_MEDIAN" | "BOTTOM_QUARTILE" | "BOTTOM_DECILE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — momentum {} — pct {:.0} — rank {}/{} — sector {} — as of {}",
                snap.symbol,
                snap.rank_label,
                snap.momentum_label,
                snap.percentile_rank,
                snap.rank_position,
                snap.peers_considered + 1,
                snap.sector,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("earmrank_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Subject composite score")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.composite_score))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Sector median / p25 / p75 score")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.2} / {:.2} / {:.2}",
                        snap.sector_median_score, snap.sector_p25, snap.sector_p75
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Peers considered / with data")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        snap.peers_considered, snap.peers_with_data
                    ))
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

pub(super) fn render_effratio_snapshot(ui: &mut egui::Ui, snap: &EfficiencyRatioSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.efficiency_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.efficiency_label.as_str() {
            "TRENDING" | "STRONG_TREND" => UP,
            "CHOP" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — ER {:.3} (signed {:+.3}) — {} bars — as of {}",
                snap.symbol,
                snap.efficiency_label,
                snap.efficiency_ratio,
                snap.signed_efficiency,
                snap.bars_used,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("effratio_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Start close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.start_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("End close").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.end_close))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Net change").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.net_change))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Net change %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.net_change_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Σ |Δclose|").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.sum_abs_changes))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Efficiency ratio").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.efficiency_ratio))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Signed efficiency").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.signed_efficiency))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_entropy_snapshot(ui: &mut egui::Ui, snap: &EntropySnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.entropy_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.entropy_label.as_str() {
            "LOW_ENTROPY" => UP,
            "VERY_HIGH_ENTROPY" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — H={:.3} bits — norm {:.3} — as of {}",
                snap.symbol,
                snap.entropy_label,
                snap.entropy_bits,
                snap.normalised_entropy,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("entropy_summary")
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
                ui.label(egui::RichText::new("Bins").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.num_bins))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Entropy H (bits)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.entropy_bits))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Max entropy (bits)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.max_entropy_bits))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Normalised H/H_max").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.normalised_entropy))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_epsb_snapshot(ui: &mut egui::Ui, snap: &EpsBeatSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.total_reports == 0 {
        ui.label(
            egui::RichText::new(
                "No data — run earnings surprise fetch (ERN) for this symbol, then click Compute.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.bias_label.as_str() {
            "POSITIVE" => UP,
            "NEUTRAL" => AXIS_TEXT,
            "NEGATIVE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} · {} · beat rate {:.0}% · streak {:+} — as of {}",
                snap.symbol,
                snap.bias_label,
                snap.trend_label,
                snap.beat_rate_pct,
                snap.current_streak,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("epsb_grid")
            .striped(true)
            .num_columns(2)
            .min_col_width(160.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Total reports").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.total_reports))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Beats / Misses / Inlines")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {} / {}",
                        snap.beats, snap.misses, snap.inlines
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Longest beat streak").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.longest_beat_streak))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Longest miss streak").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.longest_miss_streak))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Avg surprise %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.avg_surprise_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Median surprise %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.median_surprise_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Recent-4 avg %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.recent_avg_surprise_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Latest report").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{} ({:+.2}%)",
                        snap.latest_date, snap.latest_surprise_pct
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_ewmavol_snapshot(ui: &mut egui::Ui, snap: &EwmaVolSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.ewmavol_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.ewmavol_label.as_str() {
            "NORMAL" => UP,
            "ELEVATED" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — ratio {:.3} — as of {}",
                snap.symbol, snap.ewmavol_label, snap.ewma_to_classical, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("ewmavol_summary")
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
                ui.label(egui::RichText::new("λ (decay)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.lambda))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("EWMA variance").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.ewma_variance))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("EWMA σ daily").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.5}", snap.ewma_sigma_daily))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("EWMA σ annual").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.ewma_sigma_annual))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Classical σ annual").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.classical_sigma_annual))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("EWMA / classical").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.ewma_to_classical))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_fcfy_snapshot(ui: &mut egui::Ui, snap: &FcfYieldSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() {
        ui.label(
            egui::RichText::new(
                "No data — run FA (Financials) and Fundamentals, then click Compute.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.sustainability_label.as_str() {
            "SAFE" => UP,
            "STRETCHED" => AXIS_TEXT,
            "UNSUSTAINABLE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(egui::RichText::new(format!(
            "{} — FCF yield {:.2}% · div yield {:.2}% · payout-from-FCF {:.1}% · 5Y CAGR {:+.1}% — {} — as of {}",
            snap.symbol, snap.ttm_fcf_yield_pct, snap.ttm_dividend_yield_pct,
            snap.ttm_payout_from_fcf_pct, snap.fcf_cagr_5y_pct,
            snap.sustainability_label, snap.as_of,
        )).strong().color(color));
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("fcfy_grid")
                .striped(true)
                .num_columns(6)
                .min_col_width(80.0)
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
                    ui.label(egui::RichText::new("FCF").color(AXIS_TEXT).small().strong());
                    ui.label(
                        egui::RichText::new("Div Paid")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Payout-FCF %")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("FCF Yield %")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.end_row();
                    for p in &snap.periods {
                        ui.label(egui::RichText::new(&p.period).small().monospace());
                        ui.label(egui::RichText::new(&p.date).small().monospace());
                        ui.label(
                            egui::RichText::new(format!("{:.0}M", p.free_cash_flow / 1e6))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.0}M", p.dividends_paid / 1e6))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.1}%", p.payout_from_fcf_pct))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.2}%", p.fcf_yield_pct))
                                .small()
                                .monospace(),
                        );
                        ui.end_row();
                    }
                });
        });
    }
}

pub(super) fn render_figi_snapshot(ui: &mut egui::Ui, snap: &FigiSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.identifiers.is_empty() {
        ui.label(
            egui::RichText::new(
                "No data — click Lookup to query OpenFIGI (free, no auth required).",
            )
            .color(AXIS_TEXT)
            .small(),
        );
    } else {
        ui.label(
            egui::RichText::new(format!(
                "{} — {} identifier(s) — as of {}",
                snap.symbol,
                snap.identifiers.len(),
                snap.as_of
            ))
            .strong()
            .color(AXIS_TEXT),
        );
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, id) in snap.identifiers.iter().enumerate() {
                ui.label(
                    egui::RichText::new(format!("#{}  {}  — {}", i + 1, id.ticker, id.name))
                        .strong()
                        .color(AXIS_TEXT),
                );
                egui::Grid::new(format!("figi_grid_{}", i))
                    .striped(true)
                    .num_columns(2)
                    .min_col_width(160.0)
                    .show(ui, |ui| {
                        let row = |ui: &mut egui::Ui, k: &str, v: &str| {
                            if v.is_empty() {
                                return;
                            }
                            ui.label(egui::RichText::new(k).color(AXIS_TEXT).small());
                            ui.label(egui::RichText::new(v).small().monospace());
                            ui.end_row();
                        };
                        row(ui, "FIGI", &id.figi);
                        row(ui, "Composite FIGI", &id.composite_figi);
                        row(ui, "Share-class FIGI", &id.share_class_figi);
                        row(ui, "Exchange", &id.exch_code);
                        row(ui, "Security type", &id.security_type);
                        row(ui, "Security type 2", &id.security_type_2);
                        row(ui, "Market sector", &id.market_sector);
                        row(ui, "Description", &id.security_description);
                    });
                ui.separator();
            }
        });
    }
}

pub(super) fn render_flow_snapshot(ui: &mut egui::Ui, snap: &FlowSnapshot) {
    ui.separator();
    if snap.symbol.is_empty()
        || (snap.insider_trade_count == 0 && snap.institutional_holders_tracked == 0)
    {
        ui.label(egui::RichText::new("No data — run INS (insider trades) and/or HDS (institutional holders) for this symbol, then click Compute.")
            .color(AXIS_TEXT).small());
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.flow_label.as_str() {
            "STRONG_BUY" | "BUY" => UP,
            "SELL" | "STRONG_SELL" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — composite: {:.1} / 100 — {}d window — as of {}",
                snap.symbol, snap.flow_label, snap.composite_score, snap.window_days, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("flow_sub")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Insider score").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}", snap.insider_score))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Insider buys (USD)").small().strong());
                ui.label(
                    egui::RichText::new(format!("${:.0}", snap.insider_buy_value_usd))
                        .small()
                        .monospace()
                        .color(UP),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Insider sells (USD)").small().strong());
                ui.label(
                    egui::RichText::new(format!("${:.0}", snap.insider_sell_value_usd))
                        .small()
                        .monospace()
                        .color(DOWN),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Insider net (USD)").small().strong());
                let nc = if snap.insider_net_value_usd > 0.0 {
                    UP
                } else if snap.insider_net_value_usd < 0.0 {
                    DOWN
                } else {
                    AXIS_TEXT
                };
                ui.label(
                    egui::RichText::new(format!("${:+.0}", snap.insider_net_value_usd))
                        .small()
                        .monospace()
                        .color(nc),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Insider trades / unique insiders")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        snap.insider_trade_count, snap.unique_insiders
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Institutional score").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}", snap.institutional_score))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Institutional buyers / sellers")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        snap.institutional_buyers, snap.institutional_sellers
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Holders tracked").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.institutional_holders_tracked))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Inst. net ratio").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}", snap.institutional_net_ratio))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Inst. share delta (net)")
                        .small()
                        .strong(),
                );
                let nc2 = if snap.institutional_share_delta > 0.0 {
                    UP
                } else if snap.institutional_share_delta < 0.0 {
                    DOWN
                } else {
                    AXIS_TEXT
                };
                ui.label(
                    egui::RichText::new(format!("{:+.0}", snap.institutional_share_delta))
                        .small()
                        .monospace()
                        .color(nc2),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_fqm_snapshot(ui: &mut egui::Ui, snap: &FundamentalQualityMeterSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.operator_label == "NO_DATA" {
        ui.label(egui::RichText::new("No data — needs at least one of Piotroski / Margins / Accruals cached for this symbol.")
            .color(AXIS_TEXT).small());
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.operator_label.as_str() {
            "ELITE_OPERATOR" | "STRONG_OPERATOR" => UP,
            "WEAK_OPERATOR" | "BROKEN_OPERATOR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — composite {:.1}/100 — {} inputs — as of {}",
                snap.symbol,
                snap.operator_label,
                snap.composite_score,
                snap.inputs_available,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("fqm_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Piotroski (9pt)").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{:.0} — {}",
                        snap.piotroski_score, snap.piotroski_label
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Operating margin (TTM %)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.2}% — {}",
                        snap.operating_margin_pct, snap.margin_trend_label
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Cash conversion (TTM %)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.2}% — {}",
                        snap.cash_conversion_pct, snap.accruals_trend_label
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Components (PTFS / MARGINS / ACRL)")
                        .small()
                        .strong(),
                );
                let find = |key: &str| {
                    snap.components
                        .iter()
                        .find(|c| c.name.eq_ignore_ascii_case(key))
                        .map(|c| c.score)
                        .unwrap_or(0.0)
                };
                ui.label(
                    egui::RichText::new(format!(
                        "{:.1} / {:.1} / {:.1}",
                        find("Piotroski F"),
                        find("Margins"),
                        find("Accruals")
                    ))
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

pub(super) fn render_fqmrank_snapshot(ui: &mut egui::Ui, snap: &FqmRankSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.rank_label == "NO_DATA" {
        ui.label(
            egui::RichText::new(
                "No data — needs a cached FQM snapshot for the subject and ≥3 sector peers.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.rank_label.as_str() {
            "TOP_DECILE" | "TOP_QUARTILE" | "ABOVE_MEDIAN" => UP,
            "BELOW_MEDIAN" | "BOTTOM_QUARTILE" | "BOTTOM_DECILE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — composite {:.1} — {} — pct {:.0} — rank {}/{} — sector {} — as of {}",
                snap.symbol,
                snap.rank_label,
                snap.composite_score,
                snap.operator_label,
                snap.percentile_rank,
                snap.rank_position,
                snap.peers_considered + 1,
                snap.sector,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("fqmrank_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Subject FQM composite")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.1}", snap.composite_score))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Sector median / p25 / p75")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.1} / {:.1} / {:.1}",
                        snap.sector_median_score, snap.sector_p25, snap.sector_p75
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Peers considered / with data")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        snap.peers_considered, snap.peers_with_data
                    ))
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

pub(super) fn render_gapstats_snapshot(ui: &mut egui::Ui, snap: &GapStatsSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.bias_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥20 bars with valid open/close pairs.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.bias_label.as_str() {
            "UP_BIAS" | "SLIGHT_UP" => UP,
            "DOWN_BIAS" | "SLIGHT_DOWN" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — avg gap {:.3}% — {} bars — as of {}",
                snap.symbol, snap.bias_label, snap.avg_gap_pct, snap.bars_used, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("gapstats_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Gap up / down counts").small().strong());
                ui.label(
                    egui::RichText::new(format!("{} / {}", snap.gap_up_count, snap.gap_down_count))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Gap frequency").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}%", snap.gap_frequency_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Avg gap up / down %").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{:.2}% / {:.2}%",
                        snap.avg_gap_up_pct, snap.avg_gap_down_pct
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Largest gap up / down %")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.2}% / {:.2}%",
                        snap.largest_gap_up_pct, snap.largest_gap_down_pct
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Avg all-gap %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}%", snap.avg_gap_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_garch11_snapshot(ui: &mut egui::Ui, snap: &Garch11Snapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.garch11_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥60 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.garch11_label.as_str() {
            "LOW_PERSISTENCE" => UP,
            "NEAR_INTEGRATED" | "HIGH_PERSISTENCE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — α+β {:.4} — as of {}",
                snap.symbol, snap.garch11_label, snap.persistence, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("garch11_summary")
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
                ui.label(egui::RichText::new("ω (baseline)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3e}", snap.omega))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("α (ARCH)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.alpha))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("β (GARCH)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.beta))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Persistence α+β").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.persistence))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Unconditional var").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3e}", snap.unconditional_var))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Half-life (bars)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}", snap.half_life_bars))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Log-likelihood").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.log_likelihood))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_gini_snapshot(ui: &mut egui::Ui, snap: &GiniSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.gini_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.gini_label.as_str() {
            "LOW_CONCENTRATION" => UP,
            "VERY_HIGH_CONCENTRATION" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — Gini {:.4} — as of {}",
                snap.symbol, snap.gini_label, snap.gini_coeff, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("gini_summary")
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
                ui.label(egui::RichText::new("Gini coefficient").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.gini_coeff))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Mean |r| (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.mean_abs_return_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Median |r| (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.median_abs_return_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_gkvol_snapshot(ui: &mut egui::Ui, snap: &GarmanKlassVolSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.vol_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 OHLC bars.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.vol_label.as_str() {
            "VERY_LOW" | "LOW" => UP,
            "HIGH" | "VERY_HIGH" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — {:.2}% annualized ({:.3}% daily) — {} bars — as of {}",
                snap.symbol,
                snap.vol_label,
                snap.annualized_vol_pct,
                snap.daily_vol_pct,
                snap.bars_used,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("gkvol_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Range component 0.5·(ln H/L)²")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.range_component))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("C/O component k·(ln C/O)²")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.co_component))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Daily σ (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.daily_vol_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Annualized σ (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.annualized_vol_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_glasym_snapshot(ui: &mut egui::Ui, snap: &GainLossAsymmetrySnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.asymmetry_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — needs ≥20 cached daily bars.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.asymmetry_label.as_str() {
            "UPSIDE_HEAVY" | "SLIGHT_UPSIDE" => UP,
            "DOWNSIDE_HEAVY" | "SLIGHT_DOWNSIDE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — ratio {:.2} — {} bars — as of {}",
                snap.symbol, snap.asymmetry_label, snap.magnitude_ratio, snap.bars_used, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("glasym_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Avg up-day / down-day magnitude")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.3}% / {:.3}%",
                        snap.avg_up_pct, snap.avg_down_pct
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Median up-day / down-day magnitude")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.3}% / {:.3}%",
                        snap.median_up_pct, snap.median_down_pct
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Magnitude ratio (avg up / avg down)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.magnitude_ratio))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Up / down days count").small().strong());
                ui.label(
                    egui::RichText::new(format!("{} / {}", snap.up_days, snap.down_days))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_gph_snapshot(ui: &mut egui::Ui, snap: &GphSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.gph_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥64 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.gph_label.as_str() {
            "SHORT_MEMORY" => UP,
            "NONSTATIONARY" | "ANTIPERSISTENT" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — d̂ {:+.3} — as of {}",
                snap.symbol, snap.gph_label, snap.d_estimate, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("gph_summary")
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
                ui.label(egui::RichText::new("m (bandwidth)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.m_freqs))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("d̂ estimate").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.d_estimate))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Standard error").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.d_stderr))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("t-stat (H0: d=0)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}", snap.t_stat))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("p (2-sided)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.p_value_two_sided))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_gpr_snapshot(ui: &mut egui::Ui, snap: &GprSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.gpr_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.gpr_label.as_str() {
            "GOOD" | "EXCELLENT" => UP,
            "DEEP_PAIN" | "NEGATIVE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — GPR {:+.3} — PF {:.3} — as of {}",
                snap.symbol, snap.gpr_label, snap.gain_to_pain, snap.profit_factor, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("gpr_summary")
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
                ui.label(egui::RichText::new("Sum all returns (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.sum_all_returns_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Sum gains (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.sum_gains_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Sum |losses| (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.sum_losses_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Gain-to-Pain").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.gain_to_pain))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Profit Factor").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.profit_factor))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Wins / Losses").small().strong());
                ui.label(
                    egui::RichText::new(format!("{} / {}", snap.win_count, snap.loss_count))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_growm_snapshot(ui: &mut egui::Ui, snap: &GrowmSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.inputs_available == 0 {
        ui.label(
            egui::RichText::new(
                "No data — run MOM, EARM and/or DIVG for this symbol first, then click Compute.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.garp_label.as_str() {
            "GARP" | "GROWTH" => UP,
            "SPECULATIVE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — composite: {:.1} / 100 — as of {}",
                snap.symbol, snap.garp_label, snap.composite_score, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("growm_sub")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Momentum regime").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{} ({:.1})",
                        snap.momentum_regime, snap.momentum_score
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Earnings trend").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{} ({:.1})",
                        snap.earnings_label, snap.earnings_momentum_score
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Dividend CAGR 3y").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{:.2}% ({})",
                        snap.dividend_cagr_3y_pct, snap.dividend_trend
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Inputs available").small().strong());
                ui.label(
                    egui::RichText::new(format!("{} / 3", snap.inputs_available))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.components.is_empty() {
            ui.separator();
            ui.label(
                egui::RichText::new("Component contributions")
                    .strong()
                    .small()
                    .color(AXIS_TEXT),
            );
            egui::Grid::new("growm_grid")
                .striped(true)
                .num_columns(5)
                .spacing([14.0, 3.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Component").strong().small());
                    ui.label(egui::RichText::new("Value").strong().small());
                    ui.label(egui::RichText::new("Score").strong().small());
                    ui.label(egui::RichText::new("Weight").strong().small());
                    ui.label(egui::RichText::new("Contribution").strong().small());
                    ui.end_row();
                    for c in &snap.components {
                        ui.label(egui::RichText::new(&c.name).monospace().small());
                        ui.label(egui::RichText::new(&c.value).monospace().small());
                        ui.label(
                            egui::RichText::new(format!("{:.1}", c.score))
                                .monospace()
                                .small(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.0}%", c.weight))
                                .monospace()
                                .small(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.1}", c.contribution))
                                .monospace()
                                .small(),
                        );
                        ui.end_row();
                    }
                });
        }
    }
}

pub(super) fn render_gy_snapshot(ui: &mut egui::Ui, snap: &GapYearlySnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.gap_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — needs ≥20 cached daily bars for the subject.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.gap_label.as_str() {
            "EXPLOSIVE" => DOWN,
            "GAPPY" => UP,
            "SMOOTH" => UP,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — {} bars — {} gaps — as of {}",
                snap.symbol, snap.gap_label, snap.bars_used, snap.gaps_total, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("gy_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Gaps up (≥2% / ≥5% / ≥10%)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {} / {}",
                        snap.gaps_up_2pct, snap.gaps_up_5pct, snap.gaps_up_10pct
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Gaps down (≥2% / ≥5% / ≥10%)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {} / {}",
                        snap.gaps_down_2pct, snap.gaps_down_5pct, snap.gaps_down_10pct
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Largest up gap").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{:+.2}% on {}",
                        snap.largest_up_gap_pct, snap.largest_up_gap_date
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Largest down gap").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{:+.2}% on {}",
                        snap.largest_down_gap_pct, snap.largest_down_gap_date
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Avg |gap %|").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.avg_abs_gap_pct))
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

pub(super) fn render_higuchi_snapshot(ui: &mut egui::Ui, snap: &HiguchiSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.higuchi_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥100 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.higuchi_label.as_str() {
            "SMOOTH" => UP,
            "ROUGH" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — FD {:.4} — as of {}",
                snap.symbol, snap.higuchi_label, snap.fractal_dim, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("higuchi_summary")
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
                ui.label(egui::RichText::new("k_max").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.k_max))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Fractal dim (FD)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.fractal_dim))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("R² (log-k fit)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.r_squared))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("log-k points").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.log_k_count))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_hillks_snapshot(ui: &mut egui::Ui, snap: &HillksSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.hillks_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥50 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.hillks_label.as_str() {
            "GOOD_FIT" | "ACCEPTABLE_FIT" => UP,
            "REJECT" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — D {:.4} — as of {}",
                snap.symbol, snap.hillks_label, snap.ks_statistic, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("hillks_summary")
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
                ui.label(egui::RichText::new("k (tail size)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.k_order))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("α̂ (Hill)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.alpha_hat))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("KS statistic D").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.ks_statistic))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("KS critical 5%").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.ks_critical_5pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_hilltail_snapshot(ui: &mut egui::Ui, snap: &HillTailSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.tail_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥50 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.tail_label.as_str() {
            "GAUSSIAN_LIKE" | "LIGHT_TAIL" => UP,
            "HEAVY_TAIL" | "VERY_HEAVY_TAIL" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — α(|r|)={:.2} — α(left)={:.2} — α(right)={:.2} — as of {}",
                snap.symbol,
                snap.tail_label,
                snap.hill_alpha_abs,
                snap.hill_alpha_left,
                snap.hill_alpha_right,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("hilltail_summary")
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
                ui.label(egui::RichText::new("k order statistics").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.k_order_stats))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Threshold |r|(k+1)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.threshold_abs))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Hill α (|r|)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.hill_alpha_abs))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Hill α (left tail)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.hill_alpha_left))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Hill α (right tail)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.hill_alpha_right))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_hitrate_snapshot(ui: &mut egui::Ui, snap: &HitRateSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.hit_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — needs ≥20 cached daily bars.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.hit_label.as_str() {
            "BULLISH" | "WEAK_BULLISH" => UP,
            "BEARISH" | "WEAK_BEARISH" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — 20d {:.1}% — 60d {:.1}% — {} bars — as of {}",
                snap.symbol,
                snap.hit_label,
                snap.hitrate_20d,
                snap.hitrate_60d,
                snap.bars_used,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("hitrate_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("5d hit rate").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}%", snap.hitrate_5d))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("20d hit rate").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}%", snap.hitrate_20d))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("60d hit rate").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}%", snap.hitrate_60d))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("252d hit rate").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}%", snap.hitrate_252d))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Up / down / flat days")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {} / {}",
                        snap.up_days, snap.down_days, snap.flat_days
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_hra_snapshot(ui: &mut egui::Ui, snap: &HraSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.windows.is_empty() {
        ui.label(
            egui::RichText::new(
                "No data — run HP for this symbol to populate history, then click Compute.",
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
                "{} — last close ${:.2} — as of {}",
                snap.symbol, snap.last_close, snap.as_of
            ))
            .strong()
            .color(AXIS_TEXT),
        );
        ui.separator();
        egui::Grid::new("hra_ratios_grid")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                let row = |ui: &mut egui::Ui, k: &str, v: String| {
                    ui.label(egui::RichText::new(k).color(AXIS_TEXT).small());
                    ui.label(egui::RichText::new(v).small().monospace().strong());
                    ui.end_row();
                };
                row(
                    ui,
                    "Annualized volatility",
                    format!("{:.2}%", snap.volatility_annual_pct),
                );
                row(ui, "Sharpe ratio", format!("{:.3}", snap.sharpe_ratio));
                row(ui, "Sortino ratio", format!("{:.3}", snap.sortino_ratio));
                row(ui, "Calmar ratio", format!("{:.3}", snap.calmar_ratio));
                row(ui, "Max drawdown", format!("{:.2}%", snap.max_drawdown_pct));
                row(ui, "DD peak", snap.drawdown_peak_date.clone());
                row(ui, "DD trough", snap.drawdown_trough_date.clone());
                row(ui, "Risk-free rate", format!("{:.2}%", snap.risk_free_pct));
            });
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("hra_windows_grid")
                .striped(true)
                .num_columns(4)
                .min_col_width(80.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Window")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Return")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("CAGR")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(egui::RichText::new("N").color(AXIS_TEXT).small().strong());
                    ui.end_row();
                    for w in &snap.windows {
                        let c = if w.return_pct >= 0.0 { UP } else { DOWN };
                        ui.label(egui::RichText::new(&w.label).small().monospace().strong());
                        ui.label(
                            egui::RichText::new(format!("{:+.2}%", w.return_pct))
                                .color(c)
                                .small()
                                .monospace(),
                        );
                        let cagr = if w.cagr_pct == 0.0 {
                            "—".to_string()
                        } else {
                            format!("{:+.2}%", w.cagr_pct)
                        };
                        ui.label(egui::RichText::new(cagr).small().monospace());
                        ui.label(
                            egui::RichText::new(format!("{}", w.n_observations))
                                .small()
                                .monospace(),
                        );
                        ui.end_row();
                    }
                });
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

pub(super) fn render_hurst_snapshot(ui: &mut egui::Ui, snap: &HurstSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.memory_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — needs ≥40 cached daily bars.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.memory_label.as_str() {
            "PERSISTENT" | "STRONG_PERSISTENT" => UP,
            "MEAN_REVERT" | "STRONG_MEAN_REVERT" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — H {:.3} — {} bars — as of {}",
                snap.symbol, snap.memory_label, snap.hurst_exponent, snap.bars_used, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("hurst_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Hurst exponent H").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.hurst_exponent))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("R/S scales fit").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.scales_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Min / max scale").small().strong());
                ui.label(
                    egui::RichText::new(format!("{} / {}", snap.min_scale, snap.max_scale))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Memory label").small().strong());
                ui.label(egui::RichText::new(&snap.memory_label).small().monospace());
                ui.end_row();
            });
    }
}

pub(super) fn render_insiderconc_snapshot(ui: &mut egui::Ui, snap: &InsiderConcentrationSnapshot) {
    ui.separator();
    if snap.symbol.is_empty()
        || snap.rank_label == "NO_DATA"
        || snap.rank_label == "INSUFFICIENT_DATA"
    {
        ui.label(
            egui::RichText::new(
                "No data — needs Fundamentals.shares_outstanding and cached INS rows for the subject plus at least 3 same-sector peers. This is estimated from the latest shares_owned_after per reporter, not a direct ownership feed.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.rank_label.as_str() {
            "TOP_DECILE" | "TOP_QUARTILE" | "ABOVE_MEDIAN" => UP,
            "BELOW_MEDIAN" | "BOTTOM_QUARTILE" | "BOTTOM_DECILE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — insider-held {:.2}% — rank {}/{} — sector {} — as of {}",
                snap.symbol,
                snap.rank_label,
                snap.estimated_insider_pct_held,
                snap.rank_position,
                snap.peers_considered + 1,
                snap.sector,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.label(
            egui::RichText::new("Estimated from the latest cached INS holdings per reporter.")
                .small()
                .color(AXIS_TEXT),
        );
        ui.separator();
        egui::Grid::new("insiderconc_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(250.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Estimated insider-held % / shares")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.2}% / {:.0}",
                        snap.estimated_insider_pct_held, snap.total_estimated_insider_shares
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Reporters covered / active holders")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        snap.reporters_covered, snap.reporters_holding_shares
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Shares outstanding / rows used")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.0} / {}",
                        snap.shares_outstanding, snap.trade_rows_used
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Latest holdings date").small().strong());
                ui.label(
                    egui::RichText::new(&snap.latest_holdings_date)
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Largest reporter").small().strong());
                ui.label(
                    egui::RichText::new(if snap.largest_reporter.is_empty() {
                        "-".to_string()
                    } else {
                        format!(
                            "{} ({:.0} shares)",
                            snap.largest_reporter, snap.largest_reporter_shares
                        )
                    })
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Largest reporter % out / weight")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.2}% / {:.1}%",
                        snap.largest_reporter_pct_of_outstanding, snap.largest_reporter_weight_pct
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Sector median / p25 / p75 insider-held")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.2}% / {:.2}% / {:.2}%",
                        snap.sector_median_pct_held,
                        snap.sector_p25_pct_held,
                        snap.sector_p75_pct_held
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Percentile / peers considered")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.0} / {} with {} usable",
                        snap.percentile_rank, snap.peers_considered, snap.peers_with_data
                    ))
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

pub(super) fn render_insstrk_snapshot(ui: &mut egui::Ui, snap: &InsiderStreakSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.streak_label == "NONE" {
        ui.label(
            egui::RichText::new(
                "No insider trades in window — run INS for this symbol, then click Compute.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.streak_label.as_str() {
            "STRONG_ACCUMULATION" | "ACCUMULATION" => UP,
            "STRONG_DISTRIBUTION" | "DISTRIBUTION" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — {} insiders — window {}d — as of {}",
                snap.symbol, snap.streak_label, snap.unique_insiders, snap.window_days, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("insstrk_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Buy streaks / Sell streaks")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        snap.buy_streak_count, snap.sell_streak_count
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Longest buy / sell").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        snap.longest_buy_streak, snap.longest_sell_streak
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Net buy / sell value").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "${:.0} / ${:.0}",
                        snap.net_buy_value_usd, snap.net_sell_value_usd
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
            });
        if !snap.rows.is_empty() {
            ui.separator();
            ui.label(
                egui::RichText::new("Per-insider streaks")
                    .strong()
                    .small()
                    .color(AXIS_TEXT),
            );
            egui::Grid::new("insstrk_rows")
                .striped(true)
                .num_columns(5)
                .min_col_width(130.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Insider").strong().small());
                    ui.label(egui::RichText::new("Dir").strong().small());
                    ui.label(egui::RichText::new("Events").strong().small());
                    ui.label(egui::RichText::new("Net $").strong().small());
                    ui.label(egui::RichText::new("Latest").strong().small());
                    ui.end_row();
                    for r in &snap.rows {
                        let rc = match r.streak_direction.as_str() {
                            "BUY" => UP,
                            "SELL" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(&r.insider_name).small());
                        ui.label(egui::RichText::new(&r.streak_direction).small().color(rc));
                        ui.label(
                            egui::RichText::new(format!("{}", r.consecutive_events))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("${:.0}", r.net_value_usd))
                                .small()
                                .monospace(),
                        );
                        ui.label(egui::RichText::new(&r.latest_date).small().monospace());
                        ui.end_row();
                    }
                });
        }
    }
}

pub(super) fn render_ivol_snapshot(ui: &mut egui::Ui, snap: &IvolSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() {
        ui.label(
            egui::RichText::new("No data — run OMON to pull today's ATM IV, then click Compute.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        ui.label(
            egui::RichText::new(format!(
                "{} — ATM IV {:.1}% — as of {}",
                snap.symbol, snap.current_atm_iv_pct, snap.as_of
            ))
            .strong()
            .color(AXIS_TEXT),
        );
        ui.separator();
        egui::Grid::new("ivol_grid")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                let row = |ui: &mut egui::Ui, k: &str, v: String| {
                    ui.label(egui::RichText::new(k).color(AXIS_TEXT).small());
                    ui.label(egui::RichText::new(v).small().monospace().strong());
                    ui.end_row();
                };
                row(
                    ui,
                    "Current ATM IV",
                    format!("{:.2}%", snap.current_atm_iv_pct),
                );
                row(ui, "52w low", format!("{:.2}%", snap.iv_52w_low_pct));
                row(ui, "52w high", format!("{:.2}%", snap.iv_52w_high_pct));
                row(ui, "IV rank", format!("{:.1}", snap.iv_rank));
                row(ui, "IV percentile", format!("{:.1}", snap.iv_percentile));
                row(ui, "Observations", format!("{}", snap.observation_count));
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

pub(super) fn render_jbnorm_snapshot(ui: &mut egui::Ui, snap: &JarqueBeraSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.normal_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.normal_label.as_str() {
            "NORMAL" => UP,
            "NON_NORMAL" | "STRONGLY_NON_NORMAL" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — JB {:.2} — p {:.4} — {} bars — as of {}",
                snap.symbol,
                snap.normal_label,
                snap.jb_statistic,
                snap.jb_pvalue,
                snap.bars_used,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("jbnorm_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Skewness").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.skewness))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Excess kurtosis").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.excess_kurtosis))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("JB statistic").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.jb_statistic))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("p-value (χ²(2))").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.jb_pvalue))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_kappa3_snapshot(ui: &mut egui::Ui, snap: &Kappa3Snapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.kappa3_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.kappa3_label.as_str() {
            "STRONG" | "POSITIVE" => UP,
            "NEGATIVE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — κ3 {:+.4} — as of {}",
                snap.symbol, snap.kappa3_label, snap.kappa3, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("kappa3_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Returns used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("MAR").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.mar))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Excess μ (annualised)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.excess_mean))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("LPM3").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3e}", snap.lpm3))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("LPM3^(1/3) (ann)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.lpm3_root))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Kappa-3").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.kappa3))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Sortino (reference)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.sortino_compare))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_kellyf_snapshot(ui: &mut egui::Ui, snap: &KellyFractionSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.kelly_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns with wins and losses.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.kelly_label.as_str() {
            "MODERATE" | "AGGRESSIVE" => UP,
            "SKIP" | "ALL_IN" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — f* {:.4} — half {:.4} — p {:.2} — b {:.3} — as of {}",
                snap.symbol,
                snap.kelly_label,
                snap.kelly_fraction,
                snap.half_kelly,
                snap.win_rate,
                snap.win_loss_ratio,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("kellyf_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Kelly fraction (f*)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.kelly_fraction))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Half Kelly").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.half_kelly))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Win rate (p)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.win_rate))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Loss rate (q)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.loss_rate))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Avg win %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}%", snap.avg_win_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Avg loss %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}%", snap.avg_loss_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Win/loss ratio (b)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.win_loss_ratio))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_kendalltau_snapshot(ui: &mut egui::Ui, snap: &KendallTauSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.kendalltau_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.kendalltau_label.as_str() {
            "NO_RANK_AUTO" => UP,
            "STRONG_POS" | "STRONG_NEG" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — τ {:+.4} — as of {}",
                snap.symbol, snap.kendalltau_label, snap.tau, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("ktau_summary")
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
                ui.label(egui::RichText::new("Pair count").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.pair_count))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Concordant").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.concordant))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Discordant").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.discordant))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("τ").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.tau))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("z-stat").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}", snap.z_stat))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("p (2-sided)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.p_value_two_sided))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_kpss_snapshot(ui: &mut egui::Ui, snap: &KpssSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.kpss_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.kpss_label.as_str() {
            "STATIONARY" => UP,
            "NONSTATIONARY" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — η_μ {:.4} — reject {} — as of {}",
                snap.symbol, snap.kpss_label, snap.kpss_stat, snap.reject_stationary, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("kpss_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Returns used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("KPSS stat (η_μ)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.kpss_stat))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Lag truncation (ℓ)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.lag_truncation))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Crit 10%").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.crit_10))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Crit 5%").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.crit_5))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Crit 1%").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.crit_1))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Reject stationary").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.reject_stationary))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_ksnorm_snapshot(ui: &mut egui::Ui, snap: &KsnormSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.ksnorm_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.ksnorm_label.as_str() {
            "NORMAL" => UP,
            "STRONG_NON_NORMAL" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — D {:.4} — as of {}",
                snap.symbol, snap.ksnorm_label, snap.ks_statistic, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("ksnorm_summary")
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
                ui.label(egui::RichText::new("D statistic").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.ks_statistic))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Critical 10%").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{:.4}  (reject {})",
                        snap.critical_10pct, snap.reject_10pct
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Critical 5%").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{:.4}  (reject {})",
                        snap.critical_5pct, snap.reject_5pct
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Critical 1%").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{:.4}  (reject {})",
                        snap.critical_1pct, snap.reject_1pct
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Sample μ").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.mean))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Sample σ").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.5}", snap.sigma))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_kylelam_snapshot(ui: &mut egui::Ui, snap: &KylelamSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.kylelam_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 bars with volume.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.kylelam_label.as_str() {
            "HIGH_IMPACT" => DOWN,
            "LOW_IMPACT" | "NO_SIGNAL" => UP,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — λ {:.3e} — R² {:.4} — as of {}",
                snap.symbol, snap.kylelam_label, snap.kyle_lambda, snap.r_squared, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("kylelam_summary")
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
                ui.label(egui::RichText::new("Kyle λ").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3e}", snap.kyle_lambda))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("mean |Δp|").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.5}", snap.mean_abs_dp))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("mean V").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}", snap.mean_volume))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Correlation ρ").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.correlation))
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
            });
    }
}

pub(super) fn render_lev_snapshot(ui: &mut egui::Ui, snap: &LeverageSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.ratios.is_empty() {
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
                "{} — {} — as of {}",
                snap.symbol, snap.solvency_summary, snap.as_of
            ))
            .strong()
            .color(AXIS_TEXT),
        );
        ui.label(egui::RichText::new(format!(
            "Total Debt ${:.0}M · Net Debt ${:.0}M · EBITDA TTM ${:.0}M · Interest TTM ${:.0}M · Equity ${:.0}M",
            snap.total_debt / 1e6, snap.net_debt / 1e6,
            snap.ebitda_ttm / 1e6, snap.interest_expense_ttm / 1e6, snap.total_equity / 1e6,
        )).small().color(AXIS_TEXT));
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("lev_grid")
                .striped(true)
                .num_columns(5)
                .min_col_width(80.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Ratio")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Value")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Peer Median")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Signal")
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
                    for r in &snap.ratios {
                        let color = match r.signal.as_str() {
                            "HEALTHY" => UP,
                            "ELEVATED" => AXIS_TEXT,
                            "STRETCHED" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(&r.name).small().monospace().strong());
                        ui.label(
                            egui::RichText::new(format!("{:.2}", r.value))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.2}", r.peer_median))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(&r.signal)
                                .color(color)
                                .small()
                                .monospace()
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new(&r.note)
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

pub(super) fn render_levereff_snapshot(ui: &mut egui::Ui, snap: &LeverEffSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.lever_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.lever_label.as_str() {
            "SYMMETRIC" => UP,
            "STRONG_LEVERAGE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — corr {:+.4} — asym {:.3} — as of {}",
                snap.symbol, snap.lever_label, snap.corr_r_nextsq, snap.asym_ratio, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("levereff_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Returns used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("corr(rₜ, rₜ₊₁²)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.corr_r_nextsq))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Mean |r| after neg (%)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.mean_vol_after_neg))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Mean |r| after pos (%)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.mean_vol_after_pos))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Asymmetry ratio").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.asym_ratio))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_levrank_snapshot(ui: &mut egui::Ui, snap: &LeverageRankSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.rank_label == "NO_DATA" {
        ui.label(egui::RichText::new("No data — needs a cached LEV snapshot for the subject and ≥3 sector peers with positive equity.")
            .color(AXIS_TEXT).small());
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else if snap.rank_label == "NEGATIVE_EQUITY" {
        ui.label(
            egui::RichText::new(format!(
                "{} — NEGATIVE_EQUITY — total equity {:.0} (D/E undefined) — as of {}",
                snap.symbol, snap.total_equity, snap.as_of,
            ))
            .strong()
            .color(DOWN),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
        }
    } else {
        let color = match snap.rank_label.as_str() {
            "SAFEST_DECILE" | "SAFEST_QUARTILE" | "ABOVE_MEDIAN_SAFE" => UP,
            "BELOW_MEDIAN_RISKY" | "BOTTOM_QUARTILE_RISKY" | "RISKIEST_DECILE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — pct {:.0} — rank {}/{} — sector {} — as of {}",
                snap.symbol,
                snap.rank_label,
                snap.percentile_rank,
                snap.rank_position,
                snap.peers_considered + 1,
                snap.sector,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("levrank_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Subject D/E").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.debt_to_equity))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Subject total debt / equity")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "${:.2}B / ${:.2}B",
                        snap.total_debt / 1e9,
                        snap.total_equity / 1e9
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Sector median / p25 / p75 D/E")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.2} / {:.2} / {:.2}",
                        snap.sector_median_d2e, snap.sector_p25_d2e, snap.sector_p75_d2e
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Peers considered / with data")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        snap.peers_considered, snap.peers_with_data
                    ))
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

pub(super) fn render_liq_snapshot(ui: &mut egui::Ui, snap: &LiquiditySnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.window_days == 0 {
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
        let color = match snap.liquidity_tier.as_str() {
            "DEEP" | "LIQUID" => UP,
            "THIN" | "ILLIQUID" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — window: {}d — as of {}",
                snap.symbol, snap.liquidity_tier, snap.window_days, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("liq_grid")
            .striped(true)
            .num_columns(2)
            .min_col_width(240.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Avg daily share volume")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:>15.0}", snap.avg_daily_share_volume))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Median daily share volume")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:>15.0}", snap.median_daily_share_volume))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Avg daily dollar volume")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("${:>14.0}", snap.avg_daily_dollar_volume))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Median daily dollar volume")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("${:>14.0}", snap.median_daily_dollar_volume))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Shares outstanding").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:>15.0}", snap.shares_outstanding))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Daily turnover").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}%", snap.daily_turnover_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Amihud illiquidity ×1e6")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.amihud_illiquidity))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Avg true range").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.avg_true_range_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Spread proxy (Corwin-Schultz)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.3}%", snap.spread_proxy_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_liqrank_snapshot(ui: &mut egui::Ui, snap: &LiquidityRankSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.rank_label == "NO_DATA" {
        ui.label(
            egui::RichText::new(
                "No data — needs a cached LIQ snapshot for the subject and ≥3 sector peers.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.rank_label.as_str() {
            "TOP_DECILE" | "TOP_QUARTILE" | "ABOVE_MEDIAN" => UP,
            "BELOW_MEDIAN" | "BOTTOM_QUARTILE" | "BOTTOM_DECILE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — tier {} — pct {:.0} — rank {}/{} — sector {} — as of {}",
                snap.symbol,
                snap.rank_label,
                snap.tier_label,
                snap.percentile_rank,
                snap.rank_position,
                snap.peers_considered + 1,
                snap.sector,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("liqrank_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Subject avg daily $ volume")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("${:.1}M", snap.avg_daily_dollar_volume / 1e6))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Sector median / p25 / p75 ADV$")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "${:.1}M / ${:.1}M / ${:.1}M",
                        snap.sector_median_dollar_volume / 1e6,
                        snap.sector_p25_dollar_volume / 1e6,
                        snap.sector_p75_dollar_volume / 1e6
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Peers considered / with data")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        snap.peers_considered, snap.peers_with_data
                    ))
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

pub(super) fn render_ljungb_snapshot(ui: &mut egui::Ui, snap: &LjungBoxSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.ljungb_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥40 returns (h=10).")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.ljungb_label.as_str() {
            "WHITE_NOISE" => UP,
            "STRONG_DEP" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — Q {:.3} — p {:.4} — h {} — reject {} — {} bars — as of {}",
                snap.symbol,
                snap.ljungb_label,
                snap.q_statistic,
                snap.p_value,
                snap.lag_h,
                snap.reject_white_noise,
                snap.bars_used,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
    }
}

pub(super) fn render_lmom_snapshot(ui: &mut egui::Ui, snap: &LmomSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.lmom_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.lmom_label.as_str() {
            "NEAR_SYMMETRIC" | "LIGHT_TAILS" => UP,
            "HEAVY_LEFT" | "HEAVY_RIGHT" | "HEAVY_TAILS" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — τ3 {:+.4} τ4 {:+.4} — as of {}",
                snap.symbol, snap.lmom_label, snap.tau3_skew, snap.tau4_kurt, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("lmom_summary")
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
                ui.label(egui::RichText::new("L1 (mean)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.l1_mean))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("L2 (scale)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.l2_scale))
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
                ui.label(egui::RichText::new("L4").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.l4))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("τ3 (L-skew)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.tau3_skew))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("τ4 (L-kurt)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.tau4_kurt))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_lyapunov_snapshot(ui: &mut egui::Ui, snap: &LyapunovSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.lyapunov_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥100 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.lyapunov_label.as_str() {
            "STABLE" | "PERIODIC" => UP,
            "CHAOTIC" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — λ {:+.5} — as of {}",
                snap.symbol, snap.lyapunov_label, snap.lambda_max, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("lyapunov_summary")
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
                ui.label(egui::RichText::new("Time delay τ").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.time_delay))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("λ_max (per bar)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.5}", snap.lambda_max))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("R² (fit)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.r_squared))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Steps used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.steps_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_margins_snapshot(ui: &mut egui::Ui, snap: &MarginsSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.periods_used == 0 {
        ui.label(
            egui::RichText::new(
                "No data — run FA (financial statements) for this symbol, then click Compute.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.overall_trend_label.as_str() {
            "EXPANDING" => UP,
            "CONTRACTING" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — overall: {} — quality: {} — basis: {} — latest: {} — as of {}",
                snap.symbol,
                snap.overall_trend_label,
                snap.quality_label,
                snap.basis,
                snap.latest_period,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("margins_sub")
            .striped(true)
            .num_columns(4)
            .min_col_width(140.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Metric").strong().small());
                ui.label(egui::RichText::new("Latest").strong().small());
                ui.label(egui::RichText::new("Prior").strong().small());
                ui.label(egui::RichText::new("Change / Trend").strong().small());
                ui.end_row();
                let cc_g = if snap.gross_margin_change_pct > 0.0 {
                    UP
                } else if snap.gross_margin_change_pct < 0.0 {
                    DOWN
                } else {
                    AXIS_TEXT
                };
                ui.label(egui::RichText::new("Gross margin").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.latest_gross_margin_pct))
                        .small()
                        .monospace(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.prior_gross_margin_pct))
                        .small()
                        .monospace(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:+.2}pp — {}",
                        snap.gross_margin_change_pct, snap.gross_trend_label
                    ))
                    .small()
                    .monospace()
                    .color(cc_g),
                );
                ui.end_row();
                let cc_o = if snap.operating_margin_change_pct > 0.0 {
                    UP
                } else if snap.operating_margin_change_pct < 0.0 {
                    DOWN
                } else {
                    AXIS_TEXT
                };
                ui.label(egui::RichText::new("Operating margin").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.latest_operating_margin_pct))
                        .small()
                        .monospace(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.prior_operating_margin_pct))
                        .small()
                        .monospace(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:+.2}pp — {}",
                        snap.operating_margin_change_pct, snap.operating_trend_label
                    ))
                    .small()
                    .monospace()
                    .color(cc_o),
                );
                ui.end_row();
                let cc_n = if snap.net_margin_change_pct > 0.0 {
                    UP
                } else if snap.net_margin_change_pct < 0.0 {
                    DOWN
                } else {
                    AXIS_TEXT
                };
                ui.label(egui::RichText::new("Net margin").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.latest_net_margin_pct))
                        .small()
                        .monospace(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.prior_net_margin_pct))
                        .small()
                        .monospace(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:+.2}pp — {}",
                        snap.net_margin_change_pct, snap.net_trend_label
                    ))
                    .small()
                    .monospace()
                    .color(cc_n),
                );
                ui.end_row();
            });
        ui.separator();
        egui::Grid::new("margins_avg")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Average gross (periods)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.avg_gross_margin_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Average operating").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.avg_operating_margin_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Average net").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.avg_net_margin_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Periods used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.periods_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.periods.is_empty() {
            ui.separator();
            ui.label(
                egui::RichText::new("Per-period history")
                    .strong()
                    .small()
                    .color(AXIS_TEXT),
            );
            egui::Grid::new("margins_grid")
                .striped(true)
                .num_columns(4)
                .spacing([14.0, 3.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Period").strong().small());
                    ui.label(egui::RichText::new("Gross %").strong().small());
                    ui.label(egui::RichText::new("Op %").strong().small());
                    ui.label(egui::RichText::new("Net %").strong().small());
                    ui.end_row();
                    for row in &snap.periods {
                        ui.label(egui::RichText::new(&row.period).monospace().small());
                        ui.label(
                            egui::RichText::new(format!("{:.2}", row.gross_margin_pct))
                                .monospace()
                                .small(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.2}", row.operating_margin_pct))
                                .monospace()
                                .small(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.2}", row.net_margin_pct))
                                .monospace()
                                .small(),
                        );
                        ui.end_row();
                    }
                });
        }
    }
}

pub(super) fn render_mcleodli_snapshot(ui: &mut egui::Ui, snap: &McLeodLiSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.mcleodli_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.mcleodli_label.as_str() {
            "NO_ARCH" => UP,
            "MILD_ARCH" | "STRONG_ARCH" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — Q {:.3} — as of {}",
                snap.symbol, snap.mcleodli_label, snap.q_stat, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("mcl_summary")
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
                ui.label(egui::RichText::new("Lag h").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.lag_h))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Q statistic").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.q_stat))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Critical χ²(h) 95%").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.critical_95))
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
                ui.label(egui::RichText::new("Reject NO_ARCH").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.reject_null))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_mfdfa_snapshot(ui: &mut egui::Ui, snap: &MfdfaSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.mfdfa_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥120 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.mfdfa_label.as_str() {
            "MONOFRACTAL" | "WEAK_MULTIFRACTAL" => UP,
            "STRONG_MULTIFRACTAL" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — Δh {:+.4} — as of {}",
                snap.symbol, snap.mfdfa_label, snap.delta_h, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("mfdfa_summary")
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
                ui.label(egui::RichText::new("h(q=−2)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.h_q_neg2))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("h(q=0)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.h_q_zero))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("h(q=+2)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.h_q_pos2))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Δh (h(-2)-h(+2))").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.delta_h))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Scales used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.scales_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_mngr_snapshot(ui: &mut egui::Ui, snap: &InsiderActivitySnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.total_trades == 0 {
        ui.label(
            egui::RichText::new("No data — run INS for this symbol, then click Compute.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.bias_label.as_str() {
            "BULLISH" => UP,
            "BEARISH" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — conviction: {} — window: {}d — as of {}",
                snap.symbol, snap.bias_label, snap.conviction_label, snap.window_days, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("mngr_grid")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Total trades").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.total_trades))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Buys / Sells / Other").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {} / {}",
                        snap.buy_count, snap.sell_count, snap.other_count
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Unique insiders").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.unique_insiders))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Gross buy value").small().strong());
                ui.label(
                    egui::RichText::new(format!("${:.0}", snap.gross_buy_value_usd))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Gross sell value").small().strong());
                ui.label(
                    egui::RichText::new(format!("${:.0}", snap.gross_sell_value_usd))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Net value").small().strong());
                ui.label(
                    egui::RichText::new(format!("${:+.0}", snap.net_value_usd))
                        .small()
                        .monospace()
                        .color(color),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Net shares").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.0}", snap.net_shares))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Buy/Sell ratio").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.buy_sell_ratio))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Latest trade").small().strong());
                ui.label(
                    egui::RichText::new(&snap.latest_trade_date)
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_mnkendall_snapshot(ui: &mut egui::Ui, snap: &MannKendallSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.mk_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 bars with positive closes.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.mk_label.as_str() {
            "STRONG_UP" | "UP" => UP,
            "STRONG_DOWN" | "DOWN" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — S {} — z {:+.3} — p {:.4} — τ {:+.3} — as of {}",
                snap.symbol,
                snap.mk_label,
                snap.s_statistic,
                snap.z_statistic,
                snap.p_value,
                snap.tau,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("mnkendall_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("S-statistic").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.s_statistic))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Variance").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}", snap.variance))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("z-statistic").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.z_statistic))
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
                ui.label(egui::RichText::new("Kendall τ").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.tau))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Reject no-trend").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.reject_no_trend))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_momf_snapshot(ui: &mut egui::Ui, snap: &MomentumRankSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.rank_label == "NO_DATA" {
        ui.label(egui::RichText::new("No data — needs a MOMENTUM snapshot on the subject, fundamentals w/ sector, and ≥3 peers in the same sector with MOMENTUM snapshots.")
            .color(AXIS_TEXT).small());
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.rank_label.as_str() {
            "TOP_DECILE" | "TOP_QUARTILE" | "ABOVE_MEDIAN" => UP,
            "BELOW_MEDIAN" | "BOTTOM_QUARTILE" | "BOTTOM_DECILE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — pct {:.0} — rank {}/{} — sector {} — as of {}",
                snap.symbol,
                snap.rank_label,
                snap.percentile_rank,
                snap.rank_position,
                snap.peers_considered + 1,
                snap.sector,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("momf_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Subject momentum composite")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.1}", snap.composite_score))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Sector median / p25 / p75")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.1} / {:.1} / {:.1}",
                        snap.sector_median_score, snap.sector_p25, snap.sector_p75
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Peers considered / with data")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        snap.peers_considered, snap.peers_with_data
                    ))
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

pub(super) fn render_monthseas_snapshot(ui: &mut egui::Ui, snap: &MonthlySeasonalitySnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.season_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥120 bars across ≥1 year.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.season_label.as_str() {
            "STRONG_SEASONAL" | "MILD_SEASONAL" => UP,
            "INCONSISTENT" => DOWN,
            _ => AXIS_TEXT,
        };
        const MONTHS: [&str; 12] = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — {} yrs — best {} ({:.0}%) / worst {} ({:.0}%) — as of {}",
                snap.symbol,
                snap.season_label,
                snap.years_covered,
                MONTHS[snap.best_month_idx],
                snap.best_month_hit_pct,
                MONTHS[snap.worst_month_idx],
                snap.worst_month_hit_pct,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("monthseas_grid")
            .striped(true)
            .num_columns(3)
            .min_col_width(120.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Month").small().strong());
                ui.label(egui::RichText::new("Hit %").small().strong());
                ui.label(egui::RichText::new("Mean ret %").small().strong());
                ui.end_row();
                for m in 0..12 {
                    ui.label(egui::RichText::new(MONTHS[m]).small());
                    ui.label(
                        egui::RichText::new(format!("{:.1}%", snap.month_hit_pct[m]))
                            .small()
                            .monospace(),
                    );
                    ui.label(
                        egui::RichText::new(format!("{:+.3}%", snap.month_mean_ret_pct[m]))
                            .small()
                            .monospace(),
                    );
                    ui.end_row();
                }
            });
    }
}

pub(super) fn render_mrhl_snapshot(ui: &mut egui::Ui, snap: &MeanReversionHalfLifeSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.regime_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.regime_label.as_str() {
            "PERSISTENT" | "STRONG_PERSISTENT" => UP,
            "FAST_REVERT" | "MEAN_REVERTING" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — β {:.3} — half-life {:.2}d — {} bars — as of {}",
                snap.symbol,
                snap.regime_label,
                snap.beta,
                snap.half_life_days,
                snap.bars_used,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("mrhl_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("AR(1) β").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.beta))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("AR(1) α").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.alpha))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Half-life (days)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.half_life_days))
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
            });
    }
}

pub(super) fn render_msent_snapshot(ui: &mut egui::Ui, snap: &MsentSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.msent_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥100 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.msent_label.as_str() {
            "SUSTAINED" => UP,
            "DECAYING" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — CI {:.3} — as of {}",
                snap.symbol, snap.msent_label, snap.msent_complexity_index, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("msent_summary")
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
                ui.label(egui::RichText::new("Embed dim m").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.embed_dim))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Tolerance r").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.5}", snap.tolerance))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("SampEn τ=1").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.sampen_scale1))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("SampEn τ=2").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.sampen_scale2))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("SampEn τ=3").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.sampen_scale3))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("SampEn τ=4").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.sampen_scale4))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("SampEn τ=5").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.sampen_scale5))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Complexity index").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.msent_complexity_index))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_omega_snapshot(ui: &mut egui::Ui, snap: &OmegaRatioSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.omega_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.omega_label.as_str() {
            "GOOD" | "EXCELLENT" => UP,
            "POOR" | "VERY_POOR" => DOWN,
            _ => AXIS_TEXT,
        };
        let omega_disp = if snap.omega_ratio.is_finite() {
            format!("{:.3}", snap.omega_ratio)
        } else {
            "∞".to_string()
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — Ω {} — win {:.1}% — {} bars — as of {}",
                snap.symbol,
                snap.omega_label,
                omega_disp,
                snap.win_rate_pct,
                snap.bars_used,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("omega_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Gains sum (log)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.5}", snap.gains_sum))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Losses sum (log)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.5}", snap.losses_sum))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Gain days").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.gain_days))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Loss days").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.loss_days))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Win rate").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.win_rate_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_omon_snapshot(ui: &mut egui::Ui, snap: &OptionsChainSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.expirations.is_empty() {
        ui.label(
            egui::RichText::new(
                "No data — click Fetch to pull the nearest expiration from Yahoo (no key).",
            )
            .color(AXIS_TEXT)
            .small(),
        );
    } else {
        ui.label(
            egui::RichText::new(format!(
                "{} — underlying ${:.2} — {} expiry — as of {}",
                snap.symbol,
                snap.underlying_price,
                snap.expirations.len(),
                snap.as_of
            ))
            .strong()
            .color(AXIS_TEXT),
        );
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for exp in &snap.expirations {
                ui.label(
                    egui::RichText::new(format!(
                        "Expiry {} ({} days) — {} calls / {} puts",
                        exp.expiration,
                        exp.days_to_expiry,
                        exp.calls.len(),
                        exp.puts.len()
                    ))
                    .strong()
                    .color(AXIS_TEXT),
                );
                egui::Grid::new(format!("omon_calls_{}", exp.expiration))
                    .striped(true)
                    .num_columns(7)
                    .min_col_width(70.0)
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Strike")
                                .color(AXIS_TEXT)
                                .small()
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new("C Last")
                                .color(AXIS_TEXT)
                                .small()
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new("C IV")
                                .color(AXIS_TEXT)
                                .small()
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new("C Vol")
                                .color(AXIS_TEXT)
                                .small()
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new("P Last")
                                .color(AXIS_TEXT)
                                .small()
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new("P IV")
                                .color(AXIS_TEXT)
                                .small()
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new("P Vol")
                                .color(AXIS_TEXT)
                                .small()
                                .strong(),
                        );
                        ui.end_row();
                        let mut strikes: Vec<f64> = exp.calls.iter().map(|c| c.strike).collect();
                        for p in &exp.puts {
                            if !strikes.iter().any(|s| (s - p.strike).abs() < 1e-6) {
                                strikes.push(p.strike);
                            }
                        }
                        strikes
                            .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                        for k in strikes.iter().take(40) {
                            let call = exp.calls.iter().find(|c| (c.strike - k).abs() < 1e-6);
                            let put = exp.puts.iter().find(|p| (p.strike - k).abs() < 1e-6);
                            ui.label(
                                egui::RichText::new(format!("{:.2}", k))
                                    .small()
                                    .monospace()
                                    .strong(),
                            );
                            if let Some(c) = call {
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", c.last_price))
                                        .small()
                                        .monospace(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.1}%",
                                        c.implied_volatility * 100.0
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.0}", c.volume))
                                        .small()
                                        .monospace(),
                                );
                            } else {
                                ui.label("");
                                ui.label("");
                                ui.label("");
                            }
                            if let Some(p) = put {
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", p.last_price))
                                        .small()
                                        .monospace(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.1}%",
                                        p.implied_volatility * 100.0
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.0}", p.volume))
                                        .small()
                                        .monospace(),
                                );
                            } else {
                                ui.label("");
                                ui.label("");
                                ui.label("");
                            }
                            ui.end_row();
                        }
                    });
                ui.separator();
            }
        });
        if !snap.note.is_empty() {
            ui.label(
                egui::RichText::new(&snap.note)
                    .color(AXIS_TEXT)
                    .small()
                    .italics(),
            );
        }
    }
}

pub(super) fn render_operank_snapshot(ui: &mut egui::Ui, snap: &OperatingQualityRankSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.rank_label == "NO_DATA" {
        ui.label(
            egui::RichText::new(
                "No data — needs a cached MARGINS snapshot for the subject and ≥3 sector peers.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.rank_label.as_str() {
            "TOP_DECILE" | "TOP_QUARTILE" | "ABOVE_MEDIAN" => UP,
            "BELOW_MEDIAN" | "BOTTOM_QUARTILE" | "BOTTOM_DECILE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — pct {:.0} — rank {}/{} — sector {} — as of {}",
                snap.symbol,
                snap.rank_label,
                snap.percentile_rank,
                snap.rank_position,
                snap.peers_considered + 1,
                snap.sector,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("operank_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Subject op margin").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{:+.2}% — {}",
                        snap.operating_margin_pct, snap.margin_trend_label
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Sector median / p25 / p75")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:+.2}% / {:+.2}% / {:+.2}%",
                        snap.sector_median_margin_pct,
                        snap.sector_p25_margin_pct,
                        snap.sector_p75_margin_pct
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Peers considered / with data")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        snap.peers_considered, snap.peers_with_data
                    ))
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

pub(super) fn render_oufit_snapshot(ui: &mut egui::Ui, snap: &OuFitSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.oufit_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.oufit_label.as_str() {
            "FAST_REVERT" | "MODERATE_REVERT" => UP,
            "TRENDING" => DOWN,
            _ => AXIS_TEXT,
        };
        let hl_s = if snap.half_life_bars.is_finite() {
            format!("{:.2} bars", snap.half_life_bars)
        } else {
            "∞".to_string()
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — half-life {} — as of {}",
                snap.symbol, snap.oufit_label, hl_s, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("ou_summary")
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
                ui.label(egui::RichText::new("θ (speed)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.5}", snap.theta))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("μ (long-run log-price)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.mu))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("σ (diffusion)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.5}", snap.sigma))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Half-life (bars)").small().strong());
                ui.label(egui::RichText::new(hl_s).small().monospace());
                ui.end_row();
                ui.label(egui::RichText::new("Residual sd").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.5}", snap.residual_sd))
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
            });
    }
}

pub(super) fn render_pacf_snapshot(ui: &mut egui::Ui, snap: &PacfSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.pacf_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.pacf_label.as_str() {
            "NO_STRUCTURE" => UP,
            "STRONG_STRUCTURE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — {} sig lags — max |PACF| {:.4} at lag {} — as of {}",
                snap.symbol,
                snap.pacf_label,
                snap.significant_lags,
                snap.max_abs_pacf,
                snap.max_abs_lag,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("pacf_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Returns used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Bartlett 95% crit").small().strong());
                ui.label(
                    egui::RichText::new(format!("±{:.4}", snap.bartlett_crit_95))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                let pacfs = [
                    snap.pacf_lag1,
                    snap.pacf_lag2,
                    snap.pacf_lag3,
                    snap.pacf_lag4,
                    snap.pacf_lag5,
                ];
                for (i, &v) in pacfs.iter().enumerate() {
                    let sig = v.abs() > snap.bartlett_crit_95;
                    let lbl = format!("PACF lag {}", i + 1);
                    let val_str = format!("{:+.4}{}", v, if sig { " *" } else { "" });
                    ui.label(egui::RichText::new(lbl).small().strong());
                    ui.label(
                        egui::RichText::new(val_str)
                            .small()
                            .monospace()
                            .color(if sig { DOWN } else { AXIS_TEXT }),
                    );
                    ui.end_row();
                }
            });
    }
}

pub(super) fn render_painratio_snapshot(ui: &mut egui::Ui, snap: &PainRatioSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.pain_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.pain_label.as_str() {
            "LOW_PAIN" | "MILD_PAIN" => UP,
            "HIGH_PAIN" | "SEVERE_PAIN" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — pain {:.2}% — ratio {:.3} — ann ret {:.2}% — as of {}",
                snap.symbol,
                snap.pain_label,
                snap.pain_index_pct,
                snap.pain_ratio,
                snap.annualized_return_pct,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("painratio_summary")
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
                ui.label(
                    egui::RichText::new("Pain index (mean |dd|, %)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.pain_index_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Annualized return (%)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.annualized_return_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Pain ratio").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.pain_ratio))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Max drawdown (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.max_dd_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_parkinson_snapshot(ui: &mut egui::Ui, snap: &ParkinsonVolSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.vol_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 bars with H/L.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.vol_label.as_str() {
            "VERY_LOW" | "LOW" => UP,
            "HIGH" | "VERY_HIGH" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — {:.2}% annualized ({:.3}% daily) — {} bars — as of {}",
                snap.symbol,
                snap.vol_label,
                snap.annualized_vol_pct,
                snap.daily_vol_pct,
                snap.bars_used,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("parkinson_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Mean ln(H/L)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.5}", snap.mean_hl_log_ratio))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Daily σ (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.daily_vol_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Annualized σ (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.annualized_vol_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_pead_snapshot(ui: &mut egui::Ui, snap: &PeadSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.drift_direction_label == "INSUFFICIENT_DATA" {
        ui.label(egui::RichText::new("No data — needs ≥3 cached EarningsSurprise rows and historical price bars spanning each event + 10 trading days forward.")
            .color(AXIS_TEXT).small());
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.drift_direction_label.as_str() {
            "DRIFT_UP" => UP,
            "DRIFT_DOWN" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — {} events used — avg 5d {:+.2}% — as of {}",
                snap.symbol,
                snap.drift_direction_label,
                snap.events_used,
                snap.avg_drift_5d_pct,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("pead_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Avg drift 1d / 3d / 5d / 10d")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:+.2}% / {:+.2}% / {:+.2}% / {:+.2}%",
                        snap.avg_drift_1d_pct,
                        snap.avg_drift_3d_pct,
                        snap.avg_drift_5d_pct,
                        snap.avg_drift_10d_pct
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Beat 5d / Miss 5d").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{:+.2}% / {:+.2}%",
                        snap.beat_event_drift_5d_pct, snap.miss_event_drift_5d_pct
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Latest event (date / surprise / 5d drift)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {:+.2}% / {:+.2}%",
                        snap.latest_event_date,
                        snap.latest_event_surprise_pct,
                        snap.latest_event_drift_5d_pct
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Events in cache / used")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{} / {}", snap.num_events, snap.events_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.rows.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new("Per-event detail").strong().small());
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .id_salt("pead_rows")
                .show(ui, |ui| {
                    egui::Grid::new("pead_events")
                        .striped(true)
                        .num_columns(7)
                        .min_col_width(60.0)
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Date").small().strong());
                            ui.label(egui::RichText::new("Class").small().strong());
                            ui.label(egui::RichText::new("Surprise %").small().strong());
                            ui.label(egui::RichText::new("1d").small().strong());
                            ui.label(egui::RichText::new("3d").small().strong());
                            ui.label(egui::RichText::new("5d").small().strong());
                            ui.label(egui::RichText::new("10d").small().strong());
                            ui.end_row();
                            for row in &snap.rows {
                                let rc = match row.classification.as_str() {
                                    "BEAT" => UP,
                                    "MISS" => DOWN,
                                    _ => AXIS_TEXT,
                                };
                                ui.label(egui::RichText::new(&row.event_date).small().monospace());
                                ui.label(
                                    egui::RichText::new(&row.classification)
                                        .small()
                                        .monospace()
                                        .color(rc),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:+.1}%", row.surprise_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}%", row.drift_1d_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}%", row.drift_3d_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}%", row.drift_5d_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}%", row.drift_10d_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            }
                        });
                });
        }
        if !snap.note.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
        }
    }
}

pub(super) fn render_peadrank_snapshot(ui: &mut egui::Ui, snap: &PeadRankSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.rank_label == "NO_DATA" {
        ui.label(egui::RichText::new("No data — needs a PEAD snapshot on the subject (≥3 events used), fundamentals w/ sector, and ≥3 peers in the same sector with qualifying PEAD snapshots.")
            .color(AXIS_TEXT).small());
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.rank_label.as_str() {
            "TOP_DECILE" | "TOP_QUARTILE" | "ABOVE_MEDIAN" => UP,
            "BELOW_MEDIAN" | "BOTTOM_QUARTILE" | "BOTTOM_DECILE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — pct {:.0} — rank {}/{} — sector {} — as of {}",
                snap.symbol,
                snap.rank_label,
                snap.percentile_rank,
                snap.rank_position,
                snap.peers_considered + 1,
                snap.sector,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("peadrank_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Subject avg drift (5d, %)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.avg_drift_5d_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Sector median / p25 / p75 drift")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:+.2}% / {:+.2}% / {:+.2}%",
                        snap.sector_median_drift_5d_pct,
                        snap.sector_p25_drift_5d_pct,
                        snap.sector_p75_drift_5d_pct
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Peers considered / with data")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        snap.peers_considered, snap.peers_with_data
                    ))
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

pub(super) fn render_periodogram_snapshot(ui: &mut egui::Ui, snap: &PeriodogramSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.periodogram_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥60 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.periodogram_label.as_str() {
            "STRONG_CYCLE" | "MODERATE_CYCLE" => UP,
            "WEAK_CYCLE" | "NO_CYCLE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — period {:.1} bars — as of {}",
                snap.symbol, snap.periodogram_label, snap.dominant_period_bars, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("pgram_summary")
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
                ui.label(
                    egui::RichText::new("Frequencies evaluated")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{}", snap.n_freqs))
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
                ui.label(egui::RichText::new("Dominant power").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3e}", snap.dominant_power))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Total power").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3e}", snap.total_power))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Dominant / total").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.dominant_power_ratio))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_permen_snapshot(ui: &mut egui::Ui, snap: &PermenSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.permen_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.permen_label.as_str() {
            "REGULAR" => UP,
            "HIGHLY_COMPLEX" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — H_norm {:.4} — as of {}",
                snap.symbol, snap.permen_label, snap.permen_normalised, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("permen_summary")
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
                ui.label(egui::RichText::new("Embed dim (m)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.embed_dim))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Patterns observed").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{}/{}",
                        snap.patterns_observed, snap.patterns_possible
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("H raw (bits)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.permen_raw))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("H normalised").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.permen_normalised))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_pickands_snapshot(ui: &mut egui::Ui, snap: &PickandsSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.pickands_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥80 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.pickands_label.as_str() {
            "WEIBULL_BOUNDED" | "GUMBEL_EXPONENTIAL" => UP,
            "FRECHET_HEAVY" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — γ̂ {:+.4} — as of {}",
                snap.symbol, snap.pickands_label, snap.gamma_hat, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("pickands_summary")
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
                ui.label(egui::RichText::new("k (order-stat)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.k_index))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("γ̂ (Pickands)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.gamma_hat))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Tail α = 1/γ̂").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.tail_index))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("x_k").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.5}", snap.x_k))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("x_2k").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.5}", snap.x_2k))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("x_4k").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.5}", snap.x_4k))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_pproot_snapshot(ui: &mut egui::Ui, snap: &PprootSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.pproot_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 closes.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.pproot_label.as_str() {
            "STATIONARY_STRONG" | "STATIONARY_WEAK" => UP,
            "UNIT_ROOT" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — Z(t) {:+.3} — as of {}",
                snap.symbol, snap.pproot_label, snap.z_t, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("pproot_summary")
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
                ui.label(egui::RichText::new("ρ̂").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.5}", snap.rho_hat))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Raw t(ρ=1)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}", snap.t_rho))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("PP Z(ρ)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}", snap.z_rho))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("PP Z(t)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}", snap.z_t))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Lag truncation q").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.lag_truncation))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_psr_snapshot(ui: &mut egui::Ui, snap: &ProbabilisticSharpeSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.psr_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.psr_label.as_str() {
            "VERY_HIGH" | "HIGH" => UP,
            "VERY_LOW" | "LOW" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — PSR {:.4} — SR {:.3} — skew {:+.3} — kurt {:.2} — as of {}",
                snap.symbol,
                snap.psr_label,
                snap.psr,
                snap.sharpe,
                snap.skewness,
                snap.kurtosis,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("psr_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("PSR(SR*=0)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.psr))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Annualized Sharpe").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.sharpe))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Skewness γ₃").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.skewness))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Kurtosis γ₄").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.kurtosis))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("SR benchmark").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.sr_benchmark))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_ptd_snapshot(ui: &mut egui::Ui, snap: &PriceTargetDispersion) {
    ui.separator();
    if snap.symbol.is_empty() || snap.num_analysts <= 0 {
        ui.label(
            egui::RichText::new("No data — run UPDG / PT for this symbol, then click Compute.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.consensus_label.as_str() {
            "BULLISH" => UP,
            "NEUTRAL" => AXIS_TEXT,
            "BEARISH" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — {} analysts — as of {}",
                snap.symbol, snap.consensus_label, snap.num_analysts, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("ptd_grid")
            .striped(true)
            .num_columns(2)
            .min_col_width(180.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Current price").small().strong());
                ui.label(
                    egui::RichText::new(format!("${:.2}", snap.current_price))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Target high / low").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "${:.2} / ${:.2}",
                        snap.target_high, snap.target_low
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Target mean / median").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "${:.2} / ${:.2}",
                        snap.target_mean, snap.target_median
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Dispersion %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}%", snap.dispersion_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Spread % (vs current)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.1}%", snap.spread_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Implied return (median)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:+.1}%", snap.implied_return_median_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Implied return (mean)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:+.1}%", snap.implied_return_mean_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Upside to high / Downside to low")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:+.1}% / {:+.1}%",
                        snap.upside_to_high_pct, snap.downside_to_low_pct
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_ptfs_snapshot(ui: &mut egui::Ui, snap: &PiotroskiSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.checks.is_empty() {
        ui.label(
            egui::RichText::new(
                "No data — run FA (Financials) with 2+ annual periods, then click Compute.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.strength_label.as_str() {
            "STRONG" => UP,
            "MIXED" => AXIS_TEXT,
            "WEAK" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — F-Score {}/9 — {} — {} vs {} — as of {}",
                snap.symbol,
                snap.f_score,
                snap.strength_label,
                snap.current_period,
                snap.prior_period,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.label(
            egui::RichText::new(format!(
                "Profitability {}/4 · Leverage/Liquidity {}/3 · Efficiency {}/2",
                snap.profitability_score, snap.leverage_score, snap.efficiency_score,
            ))
            .small()
            .color(AXIS_TEXT),
        );
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("ptfs_grid")
                .striped(true)
                .num_columns(5)
                .min_col_width(80.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Category")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Check")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Passed")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Current")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Prior")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.end_row();
                    for c in &snap.checks {
                        let check_color = if c.passed { UP } else { DOWN };
                        let check_text = if c.passed { "PASS" } else { "FAIL" };
                        ui.label(egui::RichText::new(&c.category).small().monospace());
                        ui.label(egui::RichText::new(&c.name).small().monospace().strong());
                        ui.label(
                            egui::RichText::new(check_text)
                                .color(check_color)
                                .small()
                                .monospace()
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.2}", c.value_current))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.2}", c.value_prior))
                                .small()
                                .monospace(),
                        );
                        ui.end_row();
                    }
                });
        });
    }
}

pub(super) fn render_qrk_snapshot(ui: &mut egui::Ui, snap: &QualityRankSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.rank_label == "NO_DATA" {
        ui.label(egui::RichText::new("No data — needs a QUAL snapshot on the subject, fundamentals w/ sector, and ≥3 peers in the same sector with QUAL snapshots.")
            .color(AXIS_TEXT).small());
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.rank_label.as_str() {
            "TOP_DECILE" | "TOP_QUARTILE" | "ABOVE_MEDIAN" => UP,
            "BELOW_MEDIAN" | "BOTTOM_QUARTILE" | "BOTTOM_DECILE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — pct {:.0} — rank {}/{} — sector {} — as of {}",
                snap.symbol,
                snap.rank_label,
                snap.percentile_rank,
                snap.rank_position,
                snap.peers_considered + 1,
                snap.sector,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("qrk_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Subject composite").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}", snap.composite_score))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Sector median / p25 / p75")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.1} / {:.1} / {:.1}",
                        snap.sector_median_score, snap.sector_p25, snap.sector_p75
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Peers considered / with data")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        snap.peers_considered, snap.peers_with_data
                    ))
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

pub(super) fn render_qual_snapshot(ui: &mut egui::Ui, snap: &QualitySnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.quality_label == "NO_DATA" {
        ui.label(
            egui::RichText::new(
                "No data — needs at least one of PTFS / MARGINS / ACRL / LEV cached.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.quality_label.as_str() {
            "HIGH_QUALITY" | "QUALITY" => UP,
            "POOR" | "WEAK" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — composite {:.1} — as of {}",
                snap.symbol, snap.quality_label, snap.composite_score, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("qual_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Piotroski F").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{}/9 ({})",
                        snap.piotroski_score, snap.piotroski_label
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Operating margin").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{:.2}% ({})",
                        snap.operating_margin_pct, snap.margin_trend_label
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Cash conversion").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{:.0}% ({})",
                        snap.cash_conversion_pct, snap.accruals_trend_label
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Leverage").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{} — D/EBITDA {:.2}",
                        snap.leverage_summary, snap.debt_to_ebitda
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Inputs used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}/4", snap.inputs_available))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.components.is_empty() {
            ui.separator();
            ui.label(
                egui::RichText::new("Component contributions")
                    .strong()
                    .small()
                    .color(AXIS_TEXT),
            );
            egui::Grid::new("qual_comps")
                .striped(true)
                .num_columns(4)
                .min_col_width(130.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Component").strong().small());
                    ui.label(egui::RichText::new("Value").strong().small());
                    ui.label(egui::RichText::new("Score").strong().small());
                    ui.label(egui::RichText::new("Weight").strong().small());
                    ui.end_row();
                    for c in &snap.components {
                        ui.label(egui::RichText::new(&c.name).small().strong());
                        ui.label(egui::RichText::new(&c.value).small().monospace());
                        ui.label(
                            egui::RichText::new(format!("{:.1}", c.score))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.0}%", c.weight))
                                .small()
                                .monospace(),
                        );
                        ui.end_row();
                    }
                });
        }
    }
}

pub(super) fn render_rachev_snapshot(ui: &mut egui::Ui, snap: &RachevSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.rachev_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.rachev_label.as_str() {
            "RIGHT_HEAVY" | "STRONG_RIGHT_TAIL" => UP,
            "STRONG_LEFT_TAIL" | "LEFT_HEAVY" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — Rachev(5%)={:.3} — as of {}",
                snap.symbol, snap.rachev_label, snap.rachev_5pct, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("rachev_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Returns used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("ES right 5% (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.es_right_5pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("ES left 5% (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.es_left_5pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Rachev 5%").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.rachev_5pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("ES right 1% (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.es_right_1pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("ES left 1% (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.es_left_1pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Rachev 1%").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.rachev_1pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_rankac_snapshot(ui: &mut egui::Ui, snap: &RankacSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.rankac_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.rankac_label.as_str() {
            "INDEPENDENT" | "WEAK_DEPENDENCE" => UP,
            "STRONG_DEPENDENCE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — max|ρ| {:.4} — as of {}",
                snap.symbol, snap.rankac_label, snap.max_abs_rho, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("rankac_summary")
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
                ui.label(egui::RichText::new("ρ(lag 1)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.rho_lag1))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("ρ(lag 5)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.rho_lag5))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("ρ(lag 10)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.rho_lag10))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("mean |ρ|").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.mean_abs_rho))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("max |ρ|").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.max_abs_rho))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_recfact_snapshot(ui: &mut egui::Ui, snap: &RecfactSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.recfact_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — needs ≥20 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.recfact_label.as_str() {
            "EXCELLENT" | "GOOD" => UP,
            "DEEP_LOSS" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — RF {:.4} — as of {}",
                snap.symbol, snap.recfact_label, snap.recovery_factor, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("recfact_summary")
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
                ui.label(egui::RichText::new("Cum return (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.cum_return_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Max drawdown (%)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.max_drawdown_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Recovery factor").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.recovery_factor))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_regime_snapshot(ui: &mut egui::Ui, snap: &RegimeSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.inputs_available == 0 {
        ui.label(
            egui::RichText::new(
                "No data — run VOLE, TECH and/or HRA for this symbol, then click Compute.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.regime_label.as_str() {
            "TRENDING" => UP,
            "VOLATILE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — composite: {:.1} / 100 — as of {}",
                snap.symbol, snap.regime_label, snap.composite_score, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("regime_sub")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Realized vol").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{:.2}% ({})",
                        snap.realized_vol_pct, snap.vol_source
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("ADX").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1} — {}", snap.adx_value, snap.trend_summary))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("1Y return").small().strong());
                let rc = if snap.return_1y_pct > 0.0 {
                    UP
                } else if snap.return_1y_pct < 0.0 {
                    DOWN
                } else {
                    AXIS_TEXT
                };
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.return_1y_pct))
                        .small()
                        .monospace()
                        .color(rc),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Sharpe").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.sharpe_ratio))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Trend strength score").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1} / 100", snap.trend_strength_score))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Volatility score").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1} / 100", snap.volatility_score))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Return score").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1} / 100", snap.return_score))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Inputs available").small().strong());
                ui.label(
                    egui::RichText::new(format!("{} / 3", snap.inputs_available))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_relepsgr_snapshot(ui: &mut egui::Ui, snap: &RelativeEpsGrowthSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.relative_label == "NO_DATA" {
        ui.label(egui::RichText::new("No data — needs ≥4 annual income rows on subject, fundamentals w/ sector, and ≥3 peers in the same sector with ≥4 annual EPS rows.")
            .color(AXIS_TEXT).small());
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.relative_label.as_str() {
            "FAR_ABOVE" | "ABOVE" => UP,
            "BELOW" | "FAR_BELOW" | "CAGR_NEGATIVE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — {:.1}% CAGR — gap {:+.1}pp — sector {} — as of {}",
                snap.symbol,
                snap.relative_label,
                snap.symbol_cagr_pct,
                snap.gap_to_median_pp,
                snap.sector,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("relepsgr_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Latest / earliest EPS")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.2} / {:.2} ({} yrs)",
                        snap.latest_eps, snap.earliest_eps, snap.years_used
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Sector median / p25 / p75 CAGR")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.1}% / {:.1}% / {:.1}%",
                        snap.sector_median_cagr_pct,
                        snap.sector_p25_cagr_pct,
                        snap.sector_p75_cagr_pct
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Peers considered / with data")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        snap.peers_considered, snap.peers_with_data
                    ))
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

pub(super) fn render_relvol_snapshot(ui: &mut egui::Ui, snap: &RelVolSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.activity_label == "INSUFFICIENT_DATA" {
        ui.label(egui::RichText::new("No data — run HP (historical prices, ≥20 bars) for this symbol, then click Compute.")
            .color(AXIS_TEXT).small());
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.activity_label.as_str() {
            "EXTREME" | "HIGH" => UP,
            "LOW" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — {} — {:.2}× (20d) — as of {}",
                snap.symbol,
                snap.activity_label,
                snap.direction_label,
                snap.rel_volume_20d,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("relvol_sub")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Current volume").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.0}", snap.current_volume))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Avg volume (5d / 20d / 60d)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.0} / {:.0} / {:.0}",
                        snap.avg_volume_5d, snap.avg_volume_20d, snap.avg_volume_60d
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Rel volume (5d / 20d / 60d)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.2}× / {:.2}× / {:.2}×",
                        snap.rel_volume_5d, snap.rel_volume_20d, snap.rel_volume_60d
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Vol trend (5d vs 20d)")
                        .small()
                        .strong(),
                );
                let tc = if snap.volume_trend_5d_pct > 0.0 {
                    UP
                } else if snap.volume_trend_5d_pct < 0.0 {
                    DOWN
                } else {
                    AXIS_TEXT
                };
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.volume_trend_5d_pct))
                        .small()
                        .monospace()
                        .color(tc),
                );
                ui.end_row();
                ui.label(egui::RichText::new("60d percentile").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.0}", snap.volume_percentile_60d))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_renyient_snapshot(ui: &mut egui::Ui, snap: &RenyientSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.renyient_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.renyient_label.as_str() {
            "HIGHLY_DISPERSED" => UP,
            "CONCENTRATED" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — H_norm {:.4} — as of {}",
                snap.symbol, snap.renyient_label, snap.renyi_normalised, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("renyient_summary")
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
                ui.label(egui::RichText::new("Histogram bins").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.num_bins))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("α").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}", snap.alpha))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("H₂ raw (bits)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.renyi_raw))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("H₂ normalised").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.renyi_normalised))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Collision prob").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.collision_prob))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_retquant_snapshot(ui: &mut egui::Ui, snap: &RetquantSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.retquant_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.retquant_label.as_str() {
            "SYMMETRIC" => UP,
            "LEFT_TAIL_HEAVY" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — asymm {:.3} — IQR {:.3}% — as of {}",
                snap.symbol, snap.retquant_label, snap.tail_asymmetry, snap.iqr_pct, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("retquant_summary")
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
                ui.label(egui::RichText::new("P1").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}%", snap.p01_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("P5").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}%", snap.p05_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("P10").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}%", snap.p10_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("P25").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}%", snap.p25_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("P50 (median)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}%", snap.p50_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("P75").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}%", snap.p75_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("P90").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}%", snap.p90_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("P95").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}%", snap.p95_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("P99").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}%", snap.p99_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("IQR (P75−P25)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}%", snap.iqr_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Tail asymmetry").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.tail_asymmetry))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_revrank_snapshot(ui: &mut egui::Ui, snap: &RevenueGrowthRankSnapshot) {
    ui.separator();
    if snap.symbol.is_empty()
        || snap.relative_label == "NO_DATA"
        || snap.relative_label == "INSUFFICIENT_DATA"
    {
        ui.label(egui::RichText::new("No data — needs ≥3y of income statements on the subject and ≥3 sector peers w/ matching history.")
            .color(AXIS_TEXT).small());
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.relative_label.as_str() {
            "FAR_ABOVE_SECTOR" | "ABOVE_SECTOR" => UP,
            "BELOW_SECTOR" | "FAR_BELOW_SECTOR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — {:+.2}pp vs sector — sector {} — as of {}",
                snap.symbol, snap.relative_label, snap.gap_to_median_pp, snap.sector, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("revrank_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Subject 3y rev CAGR").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.symbol_cagr_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Latest / earliest revenue")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "${:.2}B / ${:.2}B ({} yrs)",
                        snap.latest_revenue / 1e9,
                        snap.earliest_revenue / 1e9,
                        snap.years_used
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Sector median / p25 / p75 CAGR")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:+.2}% / {:+.2}% / {:+.2}%",
                        snap.sector_median_cagr_pct,
                        snap.sector_p25_cagr_pct,
                        snap.sector_p75_cagr_pct
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Peers considered / with data")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        snap.peers_considered, snap.peers_with_data
                    ))
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

pub(super) fn render_risk_snapshot(ui: &mut egui::Ui, snap: &RiskSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.risk_label == "NO_DATA" {
        ui.label(
            egui::RichText::new(
                "No data — needs at least one of VOLE / BETA / LIQ / SHRT / ALTZ cached.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.risk_label.as_str() {
            "LOW_RISK" => UP,
            "DISTRESSED" | "HIGH_RISK" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — composite {:.1} (higher = riskier) — as of {}",
                snap.symbol, snap.risk_label, snap.composite_score, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("risk_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Realized vol").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}%", snap.realized_vol_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Beta 1Y").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.beta_1y))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Liquidity").small().strong());
                ui.label(
                    egui::RichText::new(&snap.liquidity_tier)
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Short % float / DTC").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{:.1}% / {:.1}",
                        snap.short_percent_of_float, snap.days_to_cover
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Altman Z").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2} ({})", snap.altman_z, snap.altman_zone))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Inputs used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}/5", snap.inputs_available))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.components.is_empty() {
            ui.separator();
            ui.label(
                egui::RichText::new("Component contributions")
                    .strong()
                    .small()
                    .color(AXIS_TEXT),
            );
            egui::Grid::new("risk_comps")
                .striped(true)
                .num_columns(4)
                .min_col_width(130.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Component").strong().small());
                    ui.label(egui::RichText::new("Value").strong().small());
                    ui.label(egui::RichText::new("Score").strong().small());
                    ui.label(egui::RichText::new("Weight").strong().small());
                    ui.end_row();
                    for c in &snap.components {
                        ui.label(egui::RichText::new(&c.name).small().strong());
                        ui.label(egui::RichText::new(&c.value).small().monospace());
                        ui.label(
                            egui::RichText::new(format!("{:.1}", c.score))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.0}%", c.weight))
                                .small()
                                .monospace(),
                        );
                        ui.end_row();
                    }
                });
        }
    }
}

pub(super) fn render_robvol_snapshot(ui: &mut egui::Ui, snap: &RobVolSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.robvol_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.robvol_label.as_str() {
            "CLEAN" => UP,
            "HEAVY_OUTLIERS" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — MAD ratio {:.3} — as of {}",
                snap.symbol, snap.robvol_label, snap.mad_ratio, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("robvol_summary")
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
                ui.label(egui::RichText::new("Classical σ (annual)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.classical_sigma))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("MAD σ (annual)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.mad_sigma))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("IQR σ (annual)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.iqr_sigma))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("MAD ratio").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.mad_ratio))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("IQR ratio").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.iqr_ratio))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_rollsprd_snapshot(ui: &mut egui::Ui, snap: &RollSpreadSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.roll_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else if snap.roll_label == "INVALID_POSITIVE_COV" {
        ui.label(
            egui::RichText::new(format!(
                "{} — INVALID — first-lag cov {:+.6} (≥0) — Roll model undefined (trending series)",
                snap.symbol, snap.first_lag_cov,
            ))
            .strong()
            .color(DOWN),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).small());
        }
    } else {
        let color = match snap.roll_label.as_str() {
            "TIGHT" => UP,
            "WIDE" | "VERY_WIDE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — spread {:.4} ({:.1} bps) — {} bars — as of {}",
                snap.symbol,
                snap.roll_label,
                snap.implicit_spread,
                snap.implicit_spread_bps,
                snap.bars_used,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("rollsprd_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("First-lag cov").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.6}", snap.first_lag_cov))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Mean price").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.mean_price))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Implicit spread").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.implicit_spread))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Implicit spread (bps)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.implicit_spread_bps))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_rrk_snapshot(ui: &mut egui::Ui, snap: &RiskRankSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.rank_label == "NO_DATA" {
        ui.label(egui::RichText::new("No data — needs a RISK snapshot on the subject, fundamentals w/ sector, and ≥3 peers in the same sector with RISK snapshots.")
            .color(AXIS_TEXT).small());
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.rank_label.as_str() {
            "SAFEST_DECILE" | "SAFEST_QUARTILE" | "ABOVE_MEDIAN_SAFE" => UP,
            "BELOW_MEDIAN_RISKY" | "BOTTOM_QUARTILE_RISKY" | "RISKIEST_DECILE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — safe pct {:.0} — rank {}/{} — sector {} — as of {}",
                snap.symbol,
                snap.rank_label,
                snap.percentile_rank,
                snap.rank_position,
                snap.peers_considered + 1,
                snap.sector,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("rrk_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Subject composite (higher = riskier)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.1}", snap.composite_score))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Sector median / p25 / p75 (risk)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.1} / {:.1} / {:.1}",
                        snap.sector_median_score, snap.sector_p25, snap.sector_p75
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Peers considered / with data")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        snap.peers_considered, snap.peers_with_data
                    ))
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

pub(super) fn render_rsvol_snapshot(ui: &mut egui::Ui, snap: &RogersSatchellVolSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.vol_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 OHLC bars.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.vol_label.as_str() {
            "VERY_LOW" | "LOW" => UP,
            "HIGH" | "VERY_HIGH" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(egui::RichText::new(format!(
            "{} — {} — {:.2}% annualized ({:.3}% daily) — {} bars — drift-independent — as of {}",
            snap.symbol, snap.vol_label, snap.annualized_vol_pct, snap.daily_vol_pct, snap.bars_used, snap.as_of,
        )).strong().color(color));
    }
}

pub(super) fn render_runstest_snapshot(ui: &mut egui::Ui, snap: &RunsTestSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.runs_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥20 signed returns.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.runs_label.as_str() {
            "RANDOM" => UP,
            "STRONG_CLUST" | "MOD_CLUST" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — z {:+.3} — p {:.4} — runs {}/{:.1} — as of {}",
                snap.symbol,
                snap.runs_label,
                snap.z_statistic,
                snap.p_value,
                snap.runs_observed,
                snap.runs_expected,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("runstest_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Runs observed").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.runs_observed))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Runs expected").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.runs_expected))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Runs std").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.runs_std))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("z-statistic").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.z_statistic))
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
                ui.label(egui::RichText::new("Positive days").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.positive_days))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Negative days").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.negative_days))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Reject randomness").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.reject_randomness))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_rvol_snapshot(ui: &mut egui::Ui, snap: &RealizedVolSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.windows.is_empty() {
        ui.label(
            egui::RichText::new("No data — run HP for this symbol, then click Compute.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        ui.label(
            egui::RichText::new(format!(
                "{} — last ${:.2} — IV {:.1}% vs gap {:+.1}% — {} — as of {}",
                snap.symbol,
                snap.last_close,
                snap.current_atm_iv_pct,
                snap.iv_rv_gap_pct,
                snap.regime_label,
                snap.as_of,
            ))
            .strong()
            .color(AXIS_TEXT),
        );
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("rvol_grid")
                .striped(true)
                .num_columns(4)
                .min_col_width(100.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Window")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Days")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Realized Vol %")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Percentile")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.end_row();
                    for w in &snap.windows {
                        ui.label(egui::RichText::new(&w.label).small().monospace().strong());
                        ui.label(
                            egui::RichText::new(format!("{}", w.trading_days))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.2}", w.realized_vol_pct))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.0}%", w.percentile))
                                .small()
                                .monospace(),
                        );
                        ui.end_row();
                    }
                });
        });
    }
}

pub(super) fn render_sadf_snapshot(ui: &mut egui::Ui, snap: &SadfSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.sadf_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥60 closes.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.sadf_label.as_str() {
            "STABLE" => UP,
            "EXPLOSIVE_CONFIRMED" | "EXPLOSIVE_LIKELY" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — SADF {:+.3} — as of {}",
                snap.symbol, snap.sadf_label, snap.sadf_stat, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("sadf_summary")
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
                ui.label(egui::RichText::new("Min window r0").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.min_window))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Full-sample ADF t").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}", snap.adf_full))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("SADF statistic").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}", snap.sadf_stat))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Argmax end index").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.sadf_argmax_end))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Critical 5%").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.critical_95))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Reject null").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.reject_null))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_sampen_snapshot(ui: &mut egui::Ui, snap: &SampenSnapshot) {
    ui.separator();
    if snap.symbol.is_empty()
        || snap.sampen_label == "INSUFFICIENT_DATA"
        || snap.sampen_label == "UNDEFINED"
    {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.sampen_label.as_str() {
            "REGULAR" => UP,
            "HIGHLY_COMPLEX" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — SampEn {:.4} — as of {}",
                snap.symbol, snap.sampen_label, snap.sampen, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("sampen_summary")
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
                ui.label(egui::RichText::new("Embed dim (m)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.embed_dim))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Tolerance (r)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.tolerance))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("A count (m+1)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.a_count))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("B count (m)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.b_count))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Sample entropy").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.sampen))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_seag_snapshot(ui: &mut egui::Ui, snap: &SeasonalitySnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.months.is_empty() {
        ui.label(
            egui::RichText::new("No data — run HP to populate bar history, then click Compute.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        ui.label(
            egui::RichText::new(format!(
                "{} — {} years covered — best {} · worst {} — as of {}",
                snap.symbol, snap.years_covered, snap.best_month, snap.worst_month, snap.as_of
            ))
            .strong()
            .color(AXIS_TEXT),
        );
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label(
                egui::RichText::new("Monthly seasonality")
                    .strong()
                    .color(AXIS_TEXT),
            );
            egui::Grid::new("seag_months_grid")
                .striped(true)
                .num_columns(6)
                .min_col_width(70.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Month")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(egui::RichText::new("Avg").color(AXIS_TEXT).small().strong());
                    ui.label(
                        egui::RichText::new("Median")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(egui::RichText::new("σ").color(AXIS_TEXT).small().strong());
                    ui.label(
                        egui::RichText::new("Pos/Tot")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Best/Worst")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.end_row();
                    for m in &snap.months {
                        if m.total_years == 0 {
                            continue;
                        }
                        let c = if m.avg_return_pct >= 0.0 { UP } else { DOWN };
                        ui.label(egui::RichText::new(&m.label).small().monospace().strong());
                        ui.label(
                            egui::RichText::new(format!("{:+.2}%", m.avg_return_pct))
                                .color(c)
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:+.2}%", m.median_return_pct))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.2}", m.stdev_pct))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{}/{}", m.positive_years, m.total_years))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "{:+.1}% / {:+.1}%",
                                m.best_return_pct, m.worst_return_pct
                            ))
                            .small()
                            .monospace(),
                        );
                        ui.end_row();
                    }
                });
            ui.separator();
            ui.label(
                egui::RichText::new("Day-of-week seasonality")
                    .strong()
                    .color(AXIS_TEXT),
            );
            egui::Grid::new("seag_dow_grid")
                .striped(true)
                .num_columns(3)
                .min_col_width(90.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Day").color(AXIS_TEXT).small().strong());
                    ui.label(
                        egui::RichText::new("Avg log-ret")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Pos/Tot")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.end_row();
                    for d in &snap.dow {
                        let c = if d.avg_return_pct >= 0.0 { UP } else { DOWN };
                        ui.label(egui::RichText::new(&d.label).small().monospace().strong());
                        ui.label(
                            egui::RichText::new(format!("{:+.3}%", d.avg_return_pct))
                                .color(c)
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{}/{}", d.positive_days, d.total_days))
                                .small()
                                .monospace(),
                        );
                        ui.end_row();
                    }
                });
        });
    }
}

pub(super) fn render_sectr_snapshot(ui: &mut egui::Ui, snap: &SectorRotationSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.sectors_total == 0 {
        ui.label(
            egui::RichText::new(
                "No data — run INDU (sector performance) first, then click Compute.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.strength_label.as_str() {
            "LEADER" => UP,
            "LAGGARD" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — sector: {} — rank {}/{} — as of {}",
                snap.symbol,
                snap.strength_label,
                snap.symbol_sector,
                snap.sector_rank,
                snap.sectors_total,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("sectr_grid")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Symbol sector change").small().strong());
                let c = if snap.symbol_sector_change_pct > 0.0 {
                    UP
                } else if snap.symbol_sector_change_pct < 0.0 {
                    DOWN
                } else {
                    AXIS_TEXT
                };
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.symbol_sector_change_pct))
                        .small()
                        .monospace()
                        .color(c),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Average sector change")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.avg_sector_change_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Median sector change").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.median_sector_change_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Relative strength vs avg")
                        .small()
                        .strong(),
                );
                let cr = if snap.relative_strength_pct > 0.0 {
                    UP
                } else if snap.relative_strength_pct < 0.0 {
                    DOWN
                } else {
                    AXIS_TEXT
                };
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.relative_strength_pct))
                        .small()
                        .monospace()
                        .color(cr),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Market breadth (positive %)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.0}%", snap.breadth_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Strongest sector").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{} ({:+.2}%)",
                        snap.strongest_sector, snap.strongest_sector_pct
                    ))
                    .small()
                    .monospace()
                    .color(UP),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Weakest sector").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{} ({:+.2}%)",
                        snap.weakest_sector, snap.weakest_sector_pct
                    ))
                    .small()
                    .monospace()
                    .color(DOWN),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_sharpr_snapshot(ui: &mut egui::Ui, snap: &SharpeRatioSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.sharpe_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.sharpe_label.as_str() {
            "GOOD" | "EXCELLENT" => UP,
            "POOR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — Sharpe {:.3} (ann {:.3}) — {} bars — as of {}",
                snap.symbol,
                snap.sharpe_label,
                snap.sharpe_ratio,
                snap.sharpe_ratio_ann,
                snap.bars_used,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("sharpr_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Mean log return").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.mean_log_return))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Stdev log return").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.stdev_log_return))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Mean return (ann)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.mean_return_ann))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Stdev return (ann)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.stdev_return_ann))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Sharpe (raw)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.sharpe_ratio))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Sharpe (annualized)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.sharpe_ratio_ann))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_shrank_snapshot(ui: &mut egui::Ui, snap: &ShortInterestRankSnapshot) {
    ui.separator();
    if snap.symbol.is_empty()
        || snap.rank_label == "INSUFFICIENT_DATA"
        || snap.rank_label == "NO_DATA"
    {
        ui.label(
            egui::RichText::new(
                "No data — needs ≥3 sector peers with short_percent_of_float in Fundamentals.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        // Risk-inverted: SAFEST is green, RISKIEST is red.
        let color = match snap.rank_label.as_str() {
            "SAFEST_DECILE" | "SAFEST_QUARTILE" | "ABOVE_MEDIAN_SAFE" => UP,
            "BELOW_MEDIAN_RISKY" | "BOTTOM_QUARTILE_RISKY" | "RISKIEST_DECILE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — short {:.2}% — pct {:.0} — rank {}/{} — sector {} — as of {}",
                snap.symbol,
                snap.rank_label,
                snap.short_pct_of_float,
                snap.percentile_rank,
                snap.rank_position,
                snap.peers_considered + 1,
                snap.sector,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("shrank_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Subject short % of float")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.short_pct_of_float))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Sector median / p25 / p75 short")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.2}% / {:.2}% / {:.2}%",
                        snap.sector_median_short_pct,
                        snap.sector_p25_short_pct,
                        snap.sector_p75_short_pct
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Peers considered / with data")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        snap.peers_considered, snap.peers_with_data
                    ))
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

pub(super) fn render_shrt_snapshot(ui: &mut egui::Ui, snap: &ShortInterestSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() {
        ui.label(
            egui::RichText::new(
                "No data — run FA (Fundamentals/SharesFloat) + HP, then click Compute.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.squeeze_risk_label.as_str() {
            "LOW" => UP,
            "ELEVATED" => AXIS_TEXT,
            "HIGH" | "EXTREME" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — as of {}",
                snap.symbol, snap.squeeze_risk_label, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("shrt_grid")
            .striped(true)
            .num_columns(2)
            .min_col_width(180.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Short % of float").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.short_percent_of_float))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Days to cover").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}", snap.days_to_cover))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Short shares").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.0}M", snap.short_shares / 1e6))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Float").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.0}M", snap.shares_float / 1e6))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Shares outstanding").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.0}M", snap.shares_outstanding / 1e6))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Avg daily vol (20d)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.0}K", snap.avg_daily_volume_20d / 1e3))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Short ratio (reported)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.short_ratio_reported))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Utilization proxy").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}%", snap.utilization_proxy_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.note.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new(&snap.note).color(AXIS_TEXT).small());
        }
    }
}

pub(super) fn render_sizef_snapshot(ui: &mut egui::Ui, snap: &SizeFactorSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.rank_label == "NO_DATA" {
        ui.label(egui::RichText::new("No data — needs fundamentals w/ market_cap on the subject and ≥3 sector peers with market_cap.")
            .color(AXIS_TEXT).small());
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.rank_label.as_str() {
            "TOP_DECILE" | "TOP_QUARTILE" | "ABOVE_MEDIAN" => UP,
            "BELOW_MEDIAN" | "BOTTOM_QUARTILE" | "BOTTOM_DECILE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — {} — pct {:.0} — rank {}/{} — sector {} — as of {}",
                snap.symbol,
                snap.tier_label,
                snap.rank_label,
                snap.percentile_rank,
                snap.rank_position,
                snap.peers_considered + 1,
                snap.sector,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("sizef_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Subject market cap").small().strong());
                ui.label(
                    egui::RichText::new(format!("${:.2}B", snap.market_cap / 1e9))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("log(cap)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.log_market_cap))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Sector median / p25 / p75 cap")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "${:.2}B / ${:.2}B / ${:.2}B",
                        snap.sector_median_cap / 1e9,
                        snap.sector_p25_cap / 1e9,
                        snap.sector_p75_cap / 1e9
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Peers considered / with data")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        snap.peers_considered, snap.peers_with_data
                    ))
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

pub(super) fn render_skew_snapshot(ui: &mut egui::Ui, snap: &VolatilitySkew) {
    ui.separator();
    if snap.symbol.is_empty() || snap.expiries.is_empty() {
        ui.label(
            egui::RichText::new(
                "No data — run OMON for this symbol first to cache the chain, then click Compute.",
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
                "{} — underlying ${:.2} — {} expiries — as of {}",
                snap.symbol,
                snap.underlying_price,
                snap.expiries.len(),
                snap.as_of
            ))
            .strong()
            .color(AXIS_TEXT),
        );
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for ex in &snap.expiries {
                ui.label(
                    egui::RichText::new(format!(
                        "Expiry {} ({} days) — ATM IV {:.1}% — skew 25Δ≈ {:+.2}%",
                        ex.expiration, ex.days_to_expiry, ex.atm_iv_pct, ex.put_call_skew_25d_pct
                    ))
                    .strong()
                    .color(AXIS_TEXT),
                );
                egui::Grid::new(format!("skew_grid_{}", ex.expiration))
                    .striped(true)
                    .num_columns(5)
                    .min_col_width(80.0)
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Strike")
                                .color(AXIS_TEXT)
                                .small()
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new("Moneyness")
                                .color(AXIS_TEXT)
                                .small()
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new("Call IV")
                                .color(AXIS_TEXT)
                                .small()
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new("Put IV")
                                .color(AXIS_TEXT)
                                .small()
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new("Combined")
                                .color(AXIS_TEXT)
                                .small()
                                .strong(),
                        );
                        ui.end_row();
                        for p in &ex.points {
                            ui.label(
                                egui::RichText::new(format!("{:.2}", p.strike))
                                    .small()
                                    .monospace()
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:+.2}%", p.moneyness_pct))
                                    .small()
                                    .monospace(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:.1}%", p.call_iv_pct))
                                    .small()
                                    .monospace(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:.1}%", p.put_iv_pct))
                                    .small()
                                    .monospace(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:.1}%", p.combined_iv_pct))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                        }
                    });
                ui.separator();
            }
        });
    }
}

pub(super) fn render_skspec_snapshot(ui: &mut egui::Ui, snap: &SkspecSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.skspec_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥60 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.skspec_label.as_str() {
            "STABLE_POSITIVE" | "STABLE_NEGATIVE" => UP,
            "UNSTABLE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — mean skew {:+.3} — as of {}",
                snap.symbol, snap.skspec_label, snap.mean_skew, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("skspec_summary")
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
                ui.label(egui::RichText::new("Window size").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.window_size))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Mean skew").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}", snap.mean_skew))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Std of skew").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.std_skew))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Min skew").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}", snap.min_skew))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Max skew").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}", snap.max_skew))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Range (max−min)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.range_skew))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_specent_snapshot(ui: &mut egui::Ui, snap: &SpecentSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.specent_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.specent_label.as_str() {
            "PERIODIC" => UP,
            "NOISE_LIKE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — H_norm {:.4} — as of {}",
                snap.symbol, snap.specent_label, snap.spectral_entropy_norm, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("specent_summary")
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
                ui.label(egui::RichText::new("Freq bins").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.num_freqs))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("H raw (bits)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.spectral_entropy_raw))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("H normalised").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.spectral_entropy_norm))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Peak freq idx").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.peak_freq_idx))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Peak power share").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.peak_power_share))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_squeezerank_snapshot(ui: &mut egui::Ui, snap: &SqueezeRankSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.squeezerank_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new(
                "No data — need ≥5 symbols with SQUEEZE rows. Try the Watchlist Refresh first.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
    } else {
        let color = match snap.squeezerank_label.as_str() {
            "TOP_1PCT" | "TOP_5PCT" => DOWN,
            "TOP_10PCT" => AXIS_TEXT,
            _ => UP,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — rank {}/{} — percentile {:.1} — as of {}",
                snap.symbol,
                snap.squeezerank_label,
                snap.rank,
                snap.peer_count,
                snap.percentile,
                snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("sqzrank_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(180.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Composite score").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}", snap.composite_score))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Rank").small().strong());
                ui.label(
                    egui::RichText::new(format!("{} / {}", snap.rank, snap.peer_count))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Percentile").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}", snap.percentile))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_sterling_snapshot(ui: &mut egui::Ui, snap: &SterlingRatioSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.sterling_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.sterling_label.as_str() {
            "GOOD" | "EXCELLENT" => UP,
            "POOR" | "VERY_POOR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — ratio {:.3} — ann ret {:+.2}% — mean worst {} dd {:.2}% — as of {}",
                snap.symbol,
                snap.sterling_label,
                snap.sterling_ratio,
                snap.annualized_return_pct,
                snap.worst_n,
                snap.mean_worst_dd_pct,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("sterling_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Sterling ratio").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.sterling_ratio))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Annualized return %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}%", snap.annualized_return_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Mean worst-N drawdown %")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.3}%", snap.mean_worst_dd_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Worst-N size").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.worst_n))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Total dd events").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.dd_event_count))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Bars used").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.bars_used))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_surpstk_snapshot(ui: &mut egui::Ui, snap: &EarningsSurpriseStreakSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.streak_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new(
                "No data — needs ≥4 cached earnings surprise rows for the subject.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.streak_label.as_str() {
            "HOT_STREAK" | "BEAT_TREND" => UP,
            "MISS_TREND" | "COLD_STREAK" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — beat rate {:.0}% — current {} × {} — as of {}",
                snap.symbol,
                snap.streak_label,
                snap.beat_rate_pct,
                snap.current_streak_type,
                snap.current_streak_len,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("surpstk_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Events (beats/misses/inlines)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} ({} / {} / {})",
                        snap.total_events, snap.beats, snap.misses, snap.inlines
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Longest beat / miss streak")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        snap.longest_beat_streak, snap.longest_miss_streak
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Avg surprise %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.avg_surprise_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Latest event").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{} — {} — {:+.2}%",
                        snap.latest_event_date,
                        snap.latest_event_label,
                        snap.latest_event_surprise_pct
                    ))
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

pub(super) fn render_svm_snapshot(ui: &mut egui::Ui, snap: &SvmSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.rows.is_empty() {
        ui.label(
            egui::RichText::new(
                "No data — run DDM/DCF/PEERS for this symbol first, then click Compute.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
    } else {
        let color = if snap.upside_mid_pct >= 0.0 { UP } else { DOWN };
        ui.label(
            egui::RichText::new(format!(
                "{} — current ${:.2} — fair mid ${:.2} ({:+.2}%)",
                snap.symbol, snap.current_price, snap.fair_mid, snap.upside_mid_pct
            ))
            .strong()
            .size(16.0)
            .color(color),
        );
        ui.label(
            egui::RichText::new(format!(
                "Fair range ${:.2} – ${:.2} — as of {}",
                snap.fair_low, snap.fair_high, snap.as_of
            ))
            .color(AXIS_TEXT)
            .small(),
        );
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("svm_grid")
                .striped(true)
                .num_columns(5)
                .min_col_width(110.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Model")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Implied")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Upside")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Confidence")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Source")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.end_row();
                    for r in &snap.rows {
                        let rc = if r.upside_pct >= 0.0 { UP } else { DOWN };
                        ui.label(egui::RichText::new(&r.model).small().monospace().strong());
                        ui.label(
                            egui::RichText::new(format!("${:.2}", r.implied_price))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:+.2}%", r.upside_pct))
                                .color(rc)
                                .small()
                                .monospace(),
                        );
                        ui.label(egui::RichText::new(&r.confidence).small().monospace());
                        ui.label(egui::RichText::new(&r.source).small().monospace());
                        ui.end_row();
                    }
                });
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

pub(super) fn render_tech_snapshot(ui: &mut egui::Ui, snap: &TechnicalSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.indicators.is_empty() {
        ui.label(
            egui::RichText::new("No data — run HP for this symbol, then click Compute.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        ui.label(
            egui::RichText::new(format!(
                "{} — last close ${:.2} — {} — as of {}",
                snap.symbol, snap.last_close, snap.trend_summary, snap.as_of
            ))
            .strong()
            .color(AXIS_TEXT),
        );
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("tech_grid")
                .striped(true)
                .num_columns(5)
                .min_col_width(80.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Indicator")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Value")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Secondary")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Signal")
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
                    for ind in &snap.indicators {
                        let color = match ind.signal.as_str() {
                            "bullish" | "oversold" => UP,
                            "bearish" | "overbought" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(&ind.name).small().monospace().strong());
                        ui.label(
                            egui::RichText::new(format!("{:.3}", ind.value))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.3}", ind.value_secondary))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(&ind.signal)
                                .color(color)
                                .small()
                                .monospace()
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new(&ind.note)
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

pub(super) fn render_tlrank_snapshot(ui: &mut egui::Ui, snap: &ThirtyDayLiquidityRankSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.rank_label == "NO_DATA" {
        ui.label(
            egui::RichText::new(
                "No data — needs cached daily bars for the subject and at least 3 same-sector peers.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.rank_label.as_str() {
            "TOP_DECILE" | "TOP_QUARTILE" | "ABOVE_MEDIAN" => UP,
            "BELOW_MEDIAN" | "BOTTOM_QUARTILE" | "BOTTOM_DECILE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — tier {} — 30d ADV$ ${:.1}M — rank {}/{} — as of {}",
                snap.symbol,
                snap.rank_label,
                snap.tier_label,
                snap.avg_30d_dollar_volume / 1e6,
                snap.rank_position,
                snap.peers_considered + 1,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("tlrank_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(230.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Subject 30d ADV$ / valid bars")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "${:.1}M / {}",
                        snap.avg_30d_dollar_volume / 1e6,
                        snap.bars_used
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Sector median / p25 / p75 30d ADV$")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "${:.1}M / ${:.1}M / ${:.1}M",
                        snap.sector_median_dollar_volume / 1e6,
                        snap.sector_p25_dollar_volume / 1e6,
                        snap.sector_p75_dollar_volume / 1e6
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Percentile / peers considered")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.0} / {} with data ({})",
                        snap.percentile_rank, snap.peers_with_data, snap.peers_considered
                    ))
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

pub(super) fn render_tra_snapshot(ui: &mut egui::Ui, snap: &TotalReturnSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.windows.is_empty() {
        ui.label(
            egui::RichText::new("No data — run HP and DVD for this symbol, then click Compute.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        ui.label(
            egui::RichText::new(format!(
                "{} — last close ${:.2} — TTM div ${:.2} ({:.2}%) — as of {}",
                snap.symbol,
                snap.last_close,
                snap.trailing_12m_dividends,
                snap.trailing_12m_yield_pct,
                snap.as_of
            ))
            .strong()
            .color(AXIS_TEXT),
        );
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("tra_grid")
                .striped(true)
                .num_columns(6)
                .min_col_width(80.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Window")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Price %")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Div %")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Total %")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Annualized")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("N divs")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.end_row();
                    for w in &snap.windows {
                        let c = if w.total_return_pct >= 0.0 { UP } else { DOWN };
                        ui.label(egui::RichText::new(&w.label).small().monospace().strong());
                        ui.label(
                            egui::RichText::new(format!("{:+.2}%", w.price_return_pct))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:+.2}%", w.dividend_yield_pct))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:+.2}%", w.total_return_pct))
                                .color(c)
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:+.2}%", w.annualized_pct))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{}", w.n_dividends))
                                .small()
                                .monospace(),
                        );
                        ui.end_row();
                    }
                });
        });
    }
}

pub(super) fn render_tsi_snapshot(ui: &mut egui::Ui, snap: &TsiSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.tsi_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥60 closes.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.tsi_label.as_str() {
            "STRONG_BULL" | "BULL" => UP,
            "BEAR" | "STRONG_BEAR" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — TSI {:+.2} — as of {}",
                snap.symbol, snap.tsi_label, snap.tsi_value, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("tsi_summary")
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
                ui.label(egui::RichText::new("EMA long / short").small().strong());
                ui.label(
                    egui::RichText::new(format!("{} / {}", snap.ema_long, snap.ema_short))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("TSI value").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}", snap.tsi_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Signal (EMA short)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}", snap.signal_value))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("TSI − signal").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}", snap.tsi_minus_signal))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_turnpts_snapshot(ui: &mut egui::Ui, snap: &TurnPtsSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.turnpts_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥40 closes.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.turnpts_label.as_str() {
            "RANDOM_IID" => UP,
            "OVER_TURNING" | "UNDER_TURNING" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — z {:+.3} — as of {}",
                snap.symbol, snap.turnpts_label, snap.z_stat, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("turnpts_summary")
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
                ui.label(egui::RichText::new("Observed turning pts").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.observed_turnpts))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Expected 2(n−2)/3").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}", snap.expected_turnpts))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Variance").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.variance_turnpts))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("z-statistic").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}", snap.z_stat))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("p (2-sided)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.p_value_two_sided))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Reject randomness").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.reject_null))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_ulcer_snapshot(ui: &mut egui::Ui, snap: &UlcerIndexSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.ulcer_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.ulcer_label.as_str() {
            "LOW_PAIN" => UP,
            "HIGH" | "SEVERE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — UI {:.2} — Martin {:.2} — {} bars — as of {}",
                snap.symbol,
                snap.ulcer_label,
                snap.ulcer_index,
                snap.martin_ratio,
                snap.bars_used,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("ulcer_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Ulcer index").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.ulcer_index))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Mean drawdown %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.mean_drawdown_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Max drawdown %").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.max_drawdown_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("% in drawdown").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}%", snap.pct_in_drawdown))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Annualized return").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.2}%", snap.annualized_return_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Martin ratio (UPI)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.martin_ratio))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_updgrank_snapshot(ui: &mut egui::Ui, snap: &UpgradeDowngradeRankSnapshot) {
    ui.separator();
    if snap.symbol.is_empty()
        || snap.rank_label == "INSUFFICIENT_DATA"
        || snap.rank_label == "NO_DATA"
    {
        ui.label(
            egui::RichText::new("No data — needs ≥3 sector peers with UPDM snapshots.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.rank_label.as_str() {
            "TOP_DECILE" | "TOP_QUARTILE" | "ABOVE_MEDIAN" => UP,
            "BELOW_MEDIAN" | "BOTTOM_QUARTILE" | "BOTTOM_DECILE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — bias {} — pct {:.0} — rank {}/{} — sector {} — as of {}",
                snap.symbol,
                snap.rank_label,
                snap.bias_label,
                snap.percentile_rank,
                snap.rank_position,
                snap.peers_considered + 1,
                snap.sector,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("updgrank_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Subject net rating changes 90d")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("{:+}", snap.net_90d))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Sector median / p25 / p75 net")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:+.1} / {:+.1} / {:+.1}",
                        snap.sector_median_net_90d,
                        snap.sector_p25_net_90d,
                        snap.sector_p75_net_90d
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Peers considered / with data")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        snap.peers_considered, snap.peers_with_data
                    ))
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

pub(super) fn render_updm_snapshot(ui: &mut egui::Ui, snap: &UpdmSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.total_actions == 0 {
        ui.label(
            egui::RichText::new("No data — run UPDG for this symbol, then click Compute.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.bias_label.as_str() {
            "BULLISH" => UP,
            "BEARISH" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — trend: {} — total actions: {} — as of {}",
                snap.symbol, snap.bias_label, snap.trend_label, snap.total_actions, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("updm_grid")
            .striped(true)
            .num_columns(4)
            .spacing([14.0, 3.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Window").strong().small());
                ui.label(egui::RichText::new("Upgrades").strong().small().color(UP));
                ui.label(
                    egui::RichText::new("Downgrades")
                        .strong()
                        .small()
                        .color(DOWN),
                );
                ui.label(egui::RichText::new("Net").strong().small());
                ui.end_row();
                ui.label(egui::RichText::new("30d").monospace().small());
                ui.label(
                    egui::RichText::new(format!("{}", snap.upgrades_30d))
                        .monospace()
                        .small(),
                );
                ui.label(
                    egui::RichText::new(format!("{}", snap.downgrades_30d))
                        .monospace()
                        .small(),
                );
                let c30 = if snap.net_30d > 0 {
                    UP
                } else if snap.net_30d < 0 {
                    DOWN
                } else {
                    AXIS_TEXT
                };
                ui.label(
                    egui::RichText::new(format!("{:+}", snap.net_30d))
                        .monospace()
                        .small()
                        .color(c30),
                );
                ui.end_row();
                ui.label(egui::RichText::new("90d").monospace().small());
                ui.label(
                    egui::RichText::new(format!("{}", snap.upgrades_90d))
                        .monospace()
                        .small(),
                );
                ui.label(
                    egui::RichText::new(format!("{}", snap.downgrades_90d))
                        .monospace()
                        .small(),
                );
                let c90 = if snap.net_90d > 0 {
                    UP
                } else if snap.net_90d < 0 {
                    DOWN
                } else {
                    AXIS_TEXT
                };
                ui.label(
                    egui::RichText::new(format!("{:+}", snap.net_90d))
                        .monospace()
                        .small()
                        .color(c90),
                );
                ui.end_row();
                ui.label(egui::RichText::new("180d").monospace().small());
                ui.label(
                    egui::RichText::new(format!("{}", snap.upgrades_180d))
                        .monospace()
                        .small(),
                );
                ui.label(
                    egui::RichText::new(format!("{}", snap.downgrades_180d))
                        .monospace()
                        .small(),
                );
                let c180 = if snap.net_180d > 0 {
                    UP
                } else if snap.net_180d < 0 {
                    DOWN
                } else {
                    AXIS_TEXT
                };
                ui.label(
                    egui::RichText::new(format!("{:+}", snap.net_180d))
                        .monospace()
                        .small()
                        .color(c180),
                );
                ui.end_row();
            });
        ui.separator();
        egui::Grid::new("updm_sub")
            .striped(true)
            .num_columns(2)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Initiations (90d)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.initiations_90d))
                        .monospace()
                        .small(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Maintains (90d)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.maintains_90d))
                        .monospace()
                        .small(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Latest action").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{} — {} — {}",
                        snap.latest_date, snap.latest_firm, snap.latest_action
                    ))
                    .monospace()
                    .small(),
                );
                ui.end_row();
                if !snap.latest_to_grade.is_empty() {
                    ui.label(egui::RichText::new("Latest to-grade").small().strong());
                    ui.label(
                        egui::RichText::new(&snap.latest_to_grade)
                            .monospace()
                            .small(),
                    );
                    ui.end_row();
                }
            });
    }
}

pub(super) fn render_upr_snapshot(ui: &mut egui::Ui, snap: &UprSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.upr_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.upr_label.as_str() {
            "HIGH_UPSIDE" | "VERY_HIGH_UPSIDE" => UP,
            "LOW_UPSIDE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — UPR {:.4} — as of {}",
                snap.symbol, snap.upr_label, snap.upr, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("upr_summary")
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
                ui.label(egui::RichText::new("UPM₁").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.upm1))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("LPM₂").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.8}", snap.lpm2))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Downside dev").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.downside_dev))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("UPR").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.upr))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_val_snapshot(ui: &mut egui::Ui, snap: &ValueSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.value_label == "NO_DATA" {
        ui.label(egui::RichText::new("No data — needs FUNDAMENTALS cached for this symbol + sector peers, ideally FCFY too.")
            .color(AXIS_TEXT).small());
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.value_label.as_str() {
            "DEEP_VALUE" | "VALUE" => UP,
            "EXPENSIVE" | "PREMIUM" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — composite {:.1} — sector {} — peers {} — as of {}",
                snap.symbol,
                snap.value_label,
                snap.composite_score,
                if snap.sector.is_empty() {
                    "?".to_string()
                } else {
                    snap.sector.clone()
                },
                snap.peers_considered,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("val_metrics")
            .striped(true)
            .num_columns(3)
            .min_col_width(150.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Metric").strong().small());
                ui.label(egui::RichText::new("Symbol").strong().small());
                ui.label(egui::RichText::new("Sector Median").strong().small());
                ui.end_row();
                ui.label(egui::RichText::new("P/E").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.pe_ratio))
                        .small()
                        .monospace(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.pe_sector_median))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Forward P/E").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.forward_pe))
                        .small()
                        .monospace(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.forward_pe_sector_median))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("P/B").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.price_to_book))
                        .small()
                        .monospace(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.price_to_book_sector_median))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("P/S").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.price_to_sales))
                        .small()
                        .monospace(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.price_to_sales_sector_median))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("EV/EBITDA").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.ev_to_ebitda))
                        .small()
                        .monospace(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.2}", snap.ev_to_ebitda_sector_median))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("FCF Yield").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.fcf_yield_pct))
                        .small()
                        .monospace(),
                );
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.fcf_yield_sector_median_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
        if !snap.components.is_empty() {
            ui.separator();
            ui.label(
                egui::RichText::new("Component contributions")
                    .strong()
                    .small()
                    .color(AXIS_TEXT),
            );
            egui::Grid::new("val_comps")
                .striped(true)
                .num_columns(4)
                .min_col_width(130.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Component").strong().small());
                    ui.label(egui::RichText::new("Value").strong().small());
                    ui.label(egui::RichText::new("Score").strong().small());
                    ui.label(egui::RichText::new("Weight").strong().small());
                    ui.end_row();
                    for c in &snap.components {
                        ui.label(egui::RichText::new(&c.name).small().strong());
                        ui.label(egui::RichText::new(&c.value).small().monospace());
                        ui.label(
                            egui::RichText::new(format!("{:.1}", c.score))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.0}%", c.weight))
                                .small()
                                .monospace(),
                        );
                        ui.end_row();
                    }
                });
        }
    }
}

pub(super) fn render_varhalf_snapshot(ui: &mut egui::Ui, snap: &VarHalfSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.varhalf_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥50 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.varhalf_label.as_str() {
            "FAST_REVERT" => UP,
            "VERY_PERSISTENT" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — HL {:.1} days — β {:.4} — as of {}",
                snap.symbol, snap.varhalf_label, snap.half_life_days, snap.ar1_beta, snap.as_of
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("varhalf_summary")
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
                ui.label(egui::RichText::new("Vol observations").small().strong());
                ui.label(
                    egui::RichText::new(format!("{}", snap.vol_obs))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("AR(1) β").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.ar1_beta))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("AR(1) α").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.ar1_alpha))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("R²").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.ar1_r2))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Half-life (days)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}", snap.half_life_days))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_varratio_snapshot(ui: &mut egui::Ui, snap: &VarianceRatioSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.rw_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥40 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.rw_label.as_str() {
            "TRENDING" | "STRONG_TREND" => UP,
            "STRONG_REVERT" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — VR(5) {:.3} — z(5) {:+.2} — {} bars — as of {}",
                snap.symbol, snap.rw_label, snap.vr_5, snap.z_stat_5, snap.bars_used, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("varratio_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("VR(2)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.vr_2))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("VR(5)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.vr_5))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("VR(10)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.vr_10))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("VR(20)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.vr_20))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("z-stat(2)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}", snap.z_stat_2))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("z-stat(5)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.3}", snap.z_stat_5))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_volcluster_snapshot(ui: &mut egui::Ui, snap: &VolClusterSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.cluster_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.cluster_label.as_str() {
            "NONE" => UP,
            "STRONG" | "VERY_STRONG" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — |r| lag1 ACF {:.3} — {} bars — as of {}",
                snap.symbol, snap.cluster_label, snap.abs_acf_lag1, snap.bars_used, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("volcluster_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("r² ACF (lag 1 / 5 / 20)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.3} / {:.3} / {:.3}",
                        snap.sq_acf_lag1, snap.sq_acf_lag5, snap.sq_acf_lag20
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("|r| ACF (lag 1 / 5 / 20)")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.3} / {:.3} / {:.3}",
                        snap.abs_acf_lag1, snap.abs_acf_lag5, snap.abs_acf_lag20
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_vole_snapshot(ui: &mut egui::Ui, snap: &OhlcVolSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.estimators.is_empty() {
        ui.label(
            egui::RichText::new("No data — run HP for this symbol, then click Compute.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        ui.label(
            egui::RichText::new(format!(
                "{} — preferred {} = {:.2}% · {} trading days · as of {}",
                snap.symbol,
                snap.preferred_label,
                snap.preferred_estimate_pct,
                snap.trading_days,
                snap.as_of,
            ))
            .strong()
            .color(AXIS_TEXT),
        );
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("vole_grid")
                .striped(true)
                .num_columns(4)
                .min_col_width(100.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Estimator")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Annualized %")
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Efficiency vs CtC")
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
                    for e in &snap.estimators {
                        ui.label(egui::RichText::new(&e.name).small().monospace().strong());
                        ui.label(
                            egui::RichText::new(format!("{:.2}", e.annualized_vol_pct))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.2}x", e.efficiency_vs_close))
                                .small()
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(&e.note)
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

pub(super) fn render_volofvol_snapshot(ui: &mut egui::Ui, snap: &VolOfVolSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.cv_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥50 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.cv_label.as_str() {
            "STABLE" => UP,
            "UNSTABLE" | "CHAOTIC" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — CV {:.3} — {} RV points — as of {}",
                snap.symbol, snap.cv_label, snap.cv_rv20, snap.bars_used, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("volofvol_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Mean RV20").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.mean_rv20))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Stdev RV20").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.stdev_rv20))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Min RV20").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.min_rv20))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Max RV20").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.max_rv20))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Latest RV20").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.6}", snap.latest_rv20))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("CV (stdev/mean)").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.cv_rv20))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_volratio_snapshot(ui: &mut egui::Ui, snap: &VolumeRatioSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.flow_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥20 bars with volume.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.flow_label.as_str() {
            "ACCUMULATION" | "SLIGHT_ACCUMULATION" => UP,
            "DISTRIBUTION" | "SLIGHT_DISTRIBUTION" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — ratio {:.2} — {} bars — as of {}",
                snap.symbol, snap.flow_label, snap.up_down_volume_ratio, snap.bars_used, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("volratio_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Avg up-day / down-day volume")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.0} / {:.0}",
                        snap.avg_up_volume, snap.avg_down_volume
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Median up-day / down-day volume")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.0} / {:.0}",
                        snap.median_up_volume, snap.median_down_volume
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Up/down volume ratio").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.3}", snap.up_down_volume_ratio))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Max up-day / down-day volume")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.0} / {:.0}",
                        snap.max_up_volume, snap.max_down_volume
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Up / down days count").small().strong());
                ui.label(
                    egui::RichText::new(format!("{} / {}", snap.up_days, snap.down_days))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_vrk_snapshot(ui: &mut egui::Ui, snap: &ValueRankSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.rank_label == "NO_DATA" {
        ui.label(egui::RichText::new("No data — needs a VAL snapshot on the subject and ≥3 VAL-carrying peers in the same sector.")
            .color(AXIS_TEXT).small());
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.rank_label.as_str() {
            "TOP_DECILE" | "TOP_QUARTILE" | "ABOVE_MEDIAN" => UP,
            "BELOW_MEDIAN" | "BOTTOM_QUARTILE" | "BOTTOM_DECILE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — pct {:.0} — rank {}/{} — sector {} — as of {}",
                snap.symbol,
                snap.rank_label,
                snap.percentile_rank,
                snap.rank_position,
                snap.peers_considered + 1,
                snap.sector,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("vrk_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Subject composite").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1}", snap.composite_score))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Sector median / p25 / p75")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.1} / {:.1} / {:.1}",
                        snap.sector_median_score, snap.sector_p25, snap.sector_p75
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Peers considered / with data")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        snap.peers_considered, snap.peers_with_data
                    ))
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

pub(super) fn render_wacc_snapshot(ui: &mut egui::Ui, snap: &WaccSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() {
        ui.label(
            egui::RichText::new("No data — click Fetch to pull FMP profile + balance sheet.")
                .color(AXIS_TEXT)
                .small(),
        );
        ui.label(
            egui::RichText::new(
                "Tip: run GY first to cache the latest 10Y Treasury yield as your risk-free rate.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
    } else {
        let fmt_money = |v: f64| -> String {
            if v >= 1e12 {
                format!("${:.2}T", v / 1e12)
            } else if v >= 1e9 {
                format!("${:.2}B", v / 1e9)
            } else if v >= 1e6 {
                format!("${:.1}M", v / 1e6)
            } else {
                format!("${:.0}", v)
            }
        };
        ui.label(
            egui::RichText::new(format!("{} — WACC {:.2}%", snap.symbol, snap.wacc_pct,))
                .strong()
                .size(16.0)
                .color(if snap.wacc_pct > 0.0 { UP } else { AXIS_TEXT }),
        );
        ui.label(
            egui::RichText::new(format!("as of {}", snap.as_of))
                .color(AXIS_TEXT)
                .small(),
        );
        ui.separator();
        egui::Grid::new("wacc_grid")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                let row = |ui: &mut egui::Ui, k: &str, v: String| {
                    ui.label(egui::RichText::new(k).color(AXIS_TEXT).small());
                    ui.label(egui::RichText::new(v).small().monospace().strong());
                    ui.end_row();
                };
                row(ui, "Beta (β)", format!("{:.3}", snap.beta));
                row(
                    ui,
                    "Risk-free rate (Rf, 10Y)",
                    format!("{:.2}%", snap.risk_free_pct),
                );
                row(
                    ui,
                    "Equity risk premium (ERP)",
                    format!("{:.2}%", snap.equity_risk_premium_pct),
                );
                row(
                    ui,
                    "Cost of equity (Re = Rf + β·ERP)",
                    format!("{:.2}%", snap.cost_of_equity_pct),
                );
                row(
                    ui,
                    "Pre-tax cost of debt (Rd)",
                    format!("{:.2}%", snap.pre_tax_cost_of_debt_pct),
                );
                row(
                    ui,
                    "Effective tax rate",
                    format!("{:.2}%", snap.tax_rate_pct),
                );
                row(
                    ui,
                    "After-tax cost of debt",
                    format!("{:.2}%", snap.after_tax_cost_of_debt_pct),
                );
                row(ui, "Market cap (E)", fmt_money(snap.market_cap));
                row(ui, "Total debt (D)", fmt_money(snap.total_debt));
                row(
                    ui,
                    "Equity weight (wE)",
                    format!("{:.1}%", snap.equity_weight * 100.0),
                );
                row(
                    ui,
                    "Debt weight (wD)",
                    format!("{:.1}%", snap.debt_weight * 100.0),
                );
                row(ui, "WACC", format!("{:.2}%", snap.wacc_pct));
            });
        ui.separator();
        ui.label(
            egui::RichText::new("CAPM formula: WACC = wE × (Rf + β × ERP) + wD × Rd × (1 - t)")
                .color(AXIS_TEXT)
                .small()
                .italics(),
        );
    }
}

pub(super) fn render_wickbias_snapshot(ui: &mut egui::Ui, snap: &WickBiasSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.bias_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — need ≥20 non-flat bars.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.bias_label.as_str() {
            "BUYER_LEAN" | "BUYER_DEFEND" => UP,
            "SELLER_LEAN" | "SELLER_REJECT" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — bias {:+.4} — {} bars — as of {}",
                snap.symbol, snap.bias_label, snap.wick_bias_score, snap.bars_used, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("wickbias_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Avg upper wick share").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.avg_upper_wick))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Avg lower wick share").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.avg_lower_wick))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Median upper wick").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.median_upper_wick))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Median lower wick").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.median_lower_wick))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Avg body share").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.4}", snap.avg_body_share))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Bias score").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:+.4}", snap.wick_bias_score))
                        .small()
                        .monospace(),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_zeroret_snapshot(ui: &mut egui::Ui, snap: &ZeroReturnSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.zero_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                .color(AXIS_TEXT)
                .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.zero_label.as_str() {
            "HIGHLY_LIQUID" | "LIQUID" => UP,
            "ILLIQUID" | "VERY_ILLIQUID" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — zero {:.2}% ({}/{}) — longest streak {} — ε {:.0e} — as of {}",
                snap.symbol,
                snap.zero_label,
                snap.zero_day_pct,
                snap.zero_day_count,
                snap.bars_used,
                snap.longest_zero_streak,
                snap.epsilon,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
    }
}

pub(super) fn render_cdl_closing_marubozu_snapshot(
    ui: &mut egui::Ui,
    snap: &CdlClosingMarubozuSnapshot,
) {
    ui.separator();
    if snap.symbol.is_empty() || snap.cdl_closing_marubozu_label == "INSUFFICIENT_DATA" {
        ui.label(
            egui::RichText::new("No data — HP cache needs ≥2 bars.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let color = match snap.cdl_closing_marubozu_label.as_str() {
            "BULLISH_PATTERN" => UP,
            "BEARISH_PATTERN" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(egui::RichText::new(format!("{} — {} — value {} — body {:.1}% — opening_shadow {:.1}% — closing_shadow {:.1}% — close {:.4} — as of {}",
            snap.symbol, snap.cdl_closing_marubozu_label, snap.pattern_value, snap.body_pct_range, snap.opening_shadow_pct, snap.closing_shadow_pct, snap.last_close, snap.as_of)).strong().color(color));
        ui.separator();
        egui::Grid::new("cdl_closing_marubozu_summary")
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

pub(super) fn render_momentum_snapshot(ui: &mut egui::Ui, snap: &MomentumSnapshot) {
    ui.separator();
    if snap.symbol.is_empty() || snap.bars_used == 0 {
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
        let color = match snap.regime_label.as_str() {
            "STRONG" => UP,
            "CRASH" | "WEAK" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — trend: {} — bars: {} — as of {}",
                snap.symbol, snap.regime_label, snap.trend_label, snap.bars_used, snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("mom_grid")
            .striped(true)
            .num_columns(2)
            .min_col_width(220.0)
            .show(ui, |ui| {
                let pct_row = |ui: &mut egui::Ui, label: &str, val: f64| {
                    ui.label(egui::RichText::new(label).small().strong());
                    let c = if val > 0.0 {
                        UP
                    } else if val < 0.0 {
                        DOWN
                    } else {
                        AXIS_TEXT
                    };
                    ui.label(
                        egui::RichText::new(format!("{:+.2}%", val))
                            .small()
                            .monospace()
                            .color(c),
                    );
                    ui.end_row();
                };
                pct_row(ui, "Return 1m", snap.return_1m_pct);
                pct_row(ui, "Return 3m", snap.return_3m_pct);
                pct_row(ui, "Return 6m", snap.return_6m_pct);
                pct_row(ui, "Return 12m", snap.return_12m_pct);
                pct_row(ui, "Return 12-1", snap.return_12_1_pct);
                ui.label(egui::RichText::new("Annualized vol").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.2}%", snap.vol_annualized_pct))
                        .small()
                        .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Vol-adjusted score").small().strong());
                let cv = if snap.vol_adjusted_score > 0.0 {
                    UP
                } else if snap.vol_adjusted_score < 0.0 {
                    DOWN
                } else {
                    AXIS_TEXT
                };
                ui.label(
                    egui::RichText::new(format!("{:+.3}", snap.vol_adjusted_score))
                        .small()
                        .monospace()
                        .color(cv),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Composite score").small().strong());
                ui.label(
                    egui::RichText::new(format!("{:.1} / 100", snap.composite_score))
                        .small()
                        .monospace()
                        .color(color),
                );
                ui.end_row();
            });
    }
}

pub(super) fn render_shortrank_delta_snapshot(
    ui: &mut egui::Ui,
    snap: &ShortInterestDeltaRankSnapshot,
) {
    ui.separator();
    if snap.symbol.is_empty()
        || snap.rank_label == "NO_DATA"
        || snap.rank_label == "INSUFFICIENT_DATA"
    {
        ui.label(
            egui::RichText::new(
                "No data — needs short-interest history for the subject and at least 3 same-sector peers. History accumulates from fundamentals scrapes and SHORT_INTEREST fetches.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
        if !snap.note.is_empty() {
            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
        }
    } else {
        let color = match snap.rank_label.as_str() {
            "SAFEST_DECILE" | "SAFEST_QUARTILE" | "ABOVE_MEDIAN_SAFE" => UP,
            "BELOW_MEDIAN_RISKY" | "BOTTOM_QUARTILE_RISKY" | "RISKIEST_DECILE" => DOWN,
            _ => AXIS_TEXT,
        };
        ui.label(
            egui::RichText::new(format!(
                "{} — {} — {} {:+.2} pts — rank {}/{} — as of {}",
                snap.symbol,
                snap.rank_label,
                snap.subject_trend_label,
                snap.delta_short_pct_points,
                snap.rank_position,
                snap.peers_considered + 1,
                snap.as_of,
            ))
            .strong()
            .color(color),
        );
        ui.separator();
        egui::Grid::new("shortrank_delta_summary")
            .striped(true)
            .num_columns(2)
            .min_col_width(240.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Window / history span")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{}d / {} → {}",
                        snap.lookback_days, snap.history_start_date, snap.history_end_date
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Short % float / delta")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.2}% from {:.2}% ({:+.2} pts)",
                        snap.latest_short_pct_of_float,
                        snap.prior_short_pct_of_float,
                        snap.delta_short_pct_points
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(egui::RichText::new("Short ratio / prior").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{:.2} / {:.2}",
                        snap.latest_short_ratio, snap.prior_short_ratio
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Sector median / p25 / p75 delta")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:+.2} / {:+.2} / {:+.2}",
                        snap.sector_median_delta_pct_pts,
                        snap.sector_p25_delta_pct_pts,
                        snap.sector_p75_delta_pct_pts
                    ))
                    .small()
                    .monospace(),
                );
                ui.end_row();
                ui.label(
                    egui::RichText::new("Percentile / peers considered")
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.0} / {} with data ({})",
                        snap.percentile_rank, snap.peers_with_data, snap.peers_considered
                    ))
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
