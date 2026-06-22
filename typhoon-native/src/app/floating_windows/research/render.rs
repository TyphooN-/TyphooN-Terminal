//! Pure snapshot-display renderers for the research floating windows (ADR-125
//! Phase 1 step 3): the per-window display bodies, free functions over
//! `(&mut egui::Ui, &Snapshot)` with no `TyphooNApp` access — the egui analog of
//! the symbol-investigation packet's formatter layer. Crate-movable: the future
//! `typhoon-research-ui` crate may depend on egui directly.
use crate::app::common::{AXIS_TEXT, DOWN, UP};
use typhoon_engine::core::research::{
    AvgpriceSnapshot, MedpriceSnapshot, TypPriceSnapshot, VarianceSnapshot, WclPriceSnapshot,
};

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
