// Segment of the research snapshot renderers — see render.rs (this file is
// order-preserving; segments are mechanical splits, not semantic groups).
#[allow(unused_imports)]
use crate::theme::{AXIS_TEXT, BTN_GREEN_TEXT, BTN_RED_TEXT, DOWN, UP};
#[allow(unused_imports)]
use typhoon_engine::core::research::*;

pub fn render_updm_snapshot(ui: &mut egui::Ui, snap: &UpdmSnapshot) {
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

pub fn render_upr_snapshot(ui: &mut egui::Ui, snap: &UprSnapshot) {
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

pub fn render_val_snapshot(ui: &mut egui::Ui, snap: &ValueSnapshot) {
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

pub fn render_varhalf_snapshot(ui: &mut egui::Ui, snap: &VarHalfSnapshot) {
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

pub fn render_varratio_snapshot(ui: &mut egui::Ui, snap: &VarianceRatioSnapshot) {
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

pub fn render_volcluster_snapshot(ui: &mut egui::Ui, snap: &VolClusterSnapshot) {
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

pub fn render_vole_snapshot(ui: &mut egui::Ui, snap: &OhlcVolSnapshot) {
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

pub fn render_volofvol_snapshot(ui: &mut egui::Ui, snap: &VolOfVolSnapshot) {
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

pub fn render_volratio_snapshot(ui: &mut egui::Ui, snap: &VolumeRatioSnapshot) {
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

pub fn render_vrk_snapshot(ui: &mut egui::Ui, snap: &ValueRankSnapshot) {
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

pub fn render_wacc_snapshot(ui: &mut egui::Ui, snap: &WaccSnapshot) {
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

pub fn render_wickbias_snapshot(ui: &mut egui::Ui, snap: &WickBiasSnapshot) {
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

pub fn render_zeroret_snapshot(ui: &mut egui::Ui, snap: &ZeroReturnSnapshot) {
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

pub fn render_cdl_closing_marubozu_snapshot(ui: &mut egui::Ui, snap: &CdlClosingMarubozuSnapshot) {
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

pub fn render_momentum_snapshot(ui: &mut egui::Ui, snap: &MomentumSnapshot) {
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

pub fn render_shortrank_delta_snapshot(ui: &mut egui::Ui, snap: &ShortInterestDeltaRankSnapshot) {
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

pub fn render_dividend_history(ui: &mut egui::Ui, rows: &[DividendRecord]) {
    ui.separator();
    if rows.is_empty() {
        ui.label(
            egui::RichText::new("No dividend history — click Load Cached or Fetch.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        // Summary line: TTM dividend sum + count
        let ttm_cut = (chrono::Utc::now() - chrono::Duration::days(365))
            .format("%Y-%m-%d")
            .to_string();
        let ttm_sum: f64 = rows
            .iter()
            .filter(|d| d.ex_date.as_str() >= ttm_cut.as_str())
            .map(|d| d.amount)
            .sum();
        let ttm_count = rows
            .iter()
            .filter(|d| d.ex_date.as_str() >= ttm_cut.as_str())
            .count();
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("TTM: ${:.4}", ttm_sum))
                    .strong()
                    .color(UP),
            );
            ui.label(
                egui::RichText::new(format!("({} payments)", ttm_count))
                    .color(AXIS_TEXT)
                    .small(),
            );
            ui.label(
                egui::RichText::new(format!("total records: {}", rows.len()))
                    .color(AXIS_TEXT)
                    .small(),
            );
        });
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("dvd_grid")
                    .striped(true)
                    .num_columns(6)
                    .spacing([12.0, 2.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Ex-Date").strong());
                        ui.label(egui::RichText::new("Pay Date").strong());
                        ui.label(egui::RichText::new("Record").strong());
                        ui.label(egui::RichText::new("Amount").strong());
                        ui.label(egui::RichText::new("Adj").strong());
                        ui.label(egui::RichText::new("Label").strong());
                        ui.end_row();
                        for d in rows.iter().take(200) {
                            ui.label(egui::RichText::new(&d.ex_date).monospace().small());
                            ui.label(egui::RichText::new(&d.pay_date).monospace().small());
                            ui.label(egui::RichText::new(&d.record_date).monospace().small());
                            ui.label(
                                egui::RichText::new(format!("${:.4}", d.amount))
                                    .color(UP)
                                    .monospace(),
                            );
                            ui.label(
                                egui::RichText::new(format!("${:.4}", d.adjusted_amount))
                                    .monospace()
                                    .small(),
                            );
                            ui.label(egui::RichText::new(&d.label).color(AXIS_TEXT).small());
                            ui.end_row();
                        }
                    });
            });
    }
}

pub fn render_earnings_estimates(ui: &mut egui::Ui, rows: &[EarningsEstimate]) {
    ui.separator();
    if rows.is_empty() {
        ui.label(
            egui::RichText::new("No forward estimates — click Load Cached or Fetch.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("eeb_grid")
                    .striped(true)
                    .num_columns(8)
                    .spacing([10.0, 2.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Period").strong());
                        ui.label(egui::RichText::new("EPS Avg").strong());
                        ui.label(egui::RichText::new("EPS Low").strong());
                        ui.label(egui::RichText::new("EPS High").strong());
                        ui.label(egui::RichText::new("Rev Avg").strong());
                        ui.label(egui::RichText::new("Rev Low").strong());
                        ui.label(egui::RichText::new("Rev High").strong());
                        ui.label(egui::RichText::new("#Analysts").strong());
                        ui.end_row();
                        for e in rows.iter().take(40) {
                            ui.label(egui::RichText::new(&e.date).monospace().small());
                            ui.label(
                                egui::RichText::new(format!("{:.2}", e.eps_avg))
                                    .color(UP)
                                    .monospace(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:.2}", e.eps_low))
                                    .monospace()
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:.2}", e.eps_high))
                                    .monospace()
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "${:.0}M",
                                    e.revenue_avg / 1_000_000.0
                                ))
                                .monospace(),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "${:.0}M",
                                    e.revenue_low / 1_000_000.0
                                ))
                                .monospace()
                                .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "${:.0}M",
                                    e.revenue_high / 1_000_000.0
                                ))
                                .monospace()
                                .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "{}",
                                    e.num_analysts_eps.max(e.num_analysts_rev)
                                ))
                                .color(AXIS_TEXT)
                                .small(),
                            );
                            ui.end_row();
                        }
                    });
            });
    }
}

pub fn render_eps_surprises(ui: &mut egui::Ui, rows: &[EarningsSurprise]) {
    ui.separator();
    if rows.is_empty() {
        ui.label(
            egui::RichText::new("No EPS history — click Load Cached or Fetch.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let beats = rows.iter().filter(|s| s.surprise > 0.0).count();
        let misses = rows.iter().filter(|s| s.surprise < 0.0).count();
        let avg_surprise: f64 = rows.iter().take(8).map(|s| s.surprise_pct).sum::<f64>()
            / rows.iter().take(8).count().max(1) as f64;
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("{} quarters", rows.len())).strong());
            ui.label(
                egui::RichText::new(format!("Beats: {}", beats))
                    .color(UP)
                    .monospace()
                    .small(),
            );
            ui.label(
                egui::RichText::new(format!("Misses: {}", misses))
                    .color(DOWN)
                    .monospace()
                    .small(),
            );
            let avg_col = if avg_surprise >= 0.0 { UP } else { DOWN };
            ui.label(
                egui::RichText::new(format!("8Q avg: {:+.2}%", avg_surprise))
                    .color(avg_col)
                    .strong()
                    .monospace()
                    .small(),
            );
        });
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("eps_grid")
                    .striped(true)
                    .num_columns(5)
                    .spacing([20.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Date").strong());
                        ui.label(egui::RichText::new("Actual").strong());
                        ui.label(egui::RichText::new("Estimate").strong());
                        ui.label(egui::RichText::new("Surprise").strong());
                        ui.label(egui::RichText::new("Surprise %").strong());
                        ui.end_row();
                        for s in rows.iter() {
                            let col = if s.surprise >= 0.0 { UP } else { DOWN };
                            ui.label(egui::RichText::new(&s.date).monospace().small());
                            ui.label(
                                egui::RichText::new(format!("${:.2}", s.eps_actual))
                                    .strong()
                                    .monospace()
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!("${:.2}", s.eps_estimate))
                                    .monospace()
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:+.2}", s.surprise))
                                    .color(col)
                                    .monospace()
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:+.2}%", s.surprise_pct))
                                    .color(col)
                                    .strong()
                                    .monospace()
                                    .small(),
                            );
                            ui.end_row();
                        }
                    });
            });
    }
}

pub fn render_esg_rows(ui: &mut egui::Ui, rows: &[EsgScore]) {
    ui.separator();
    if rows.is_empty() {
        ui.label(
            egui::RichText::new("No ESG data — click Load Cached or Fetch.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("esg_grid")
                    .striped(true)
                    .num_columns(5)
                    .spacing([20.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Year").strong());
                        ui.label(egui::RichText::new("Environmental").strong());
                        ui.label(egui::RichText::new("Social").strong());
                        ui.label(egui::RichText::new("Governance").strong());
                        ui.label(egui::RichText::new("Overall").strong());
                        ui.end_row();
                        let score_color = |s: f64| -> egui::Color32 {
                            if s >= 70.0 {
                                UP
                            } else if s >= 50.0 {
                                AXIS_TEXT
                            } else {
                                DOWN
                            }
                        };
                        for e in rows.iter() {
                            ui.label(
                                egui::RichText::new(format!("{}", e.year))
                                    .monospace()
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:.1}", e.environmental_score))
                                    .color(score_color(e.environmental_score))
                                    .monospace(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:.1}", e.social_score))
                                    .color(score_color(e.social_score))
                                    .monospace(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:.1}", e.governance_score))
                                    .color(score_color(e.governance_score))
                                    .monospace(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:.1}", e.esg_score))
                                    .color(score_color(e.esg_score))
                                    .monospace()
                                    .strong(),
                            );
                            ui.end_row();
                        }
                    });
            });
    }
}

pub fn render_etf_holdings(ui: &mut egui::Ui, rows: &[EtfHolding]) {
    ui.separator();
    if rows.is_empty() {
        ui.label(egui::RichText::new("No ETF holdings — click Load Cached or Fetch. Pass an ETF ticker (SPY, QQQ, IWM, VTI, …).").color(AXIS_TEXT).small());
    } else {
        let total_weight: f64 = rows.iter().map(|h| h.weight_pct).sum();
        let total_value: f64 = rows.iter().map(|h| h.market_value).sum();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("{} holdings", rows.len())).strong());
            ui.label(
                egui::RichText::new(format!(
                    "(sum weight: {:.2}%, AUM: ${:.1}B)",
                    total_weight,
                    total_value / 1e9
                ))
                .color(AXIS_TEXT)
                .small(),
            );
        });
        ui.separator();
        let fmt_money = |v: f64| -> String {
            if v.abs() >= 1e9 {
                format!("${:.2}B", v / 1e9)
            } else if v.abs() >= 1e6 {
                format!("${:.1}M", v / 1e6)
            } else if v.abs() >= 1e3 {
                format!("${:.0}K", v / 1e3)
            } else {
                format!("${:.0}", v)
            }
        };
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("etf_grid")
                    .striped(true)
                    .num_columns(5)
                    .spacing([14.0, 3.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Symbol").strong());
                        ui.label(egui::RichText::new("Name").strong());
                        ui.label(egui::RichText::new("Weight %").strong());
                        ui.label(egui::RichText::new("Shares").strong());
                        ui.label(egui::RichText::new("Market Value").strong());
                        ui.end_row();
                        for h in rows.iter().take(500) {
                            ui.label(egui::RichText::new(&h.symbol).monospace().strong());
                            let short_name: String = h.name.chars().take(40).collect();
                            ui.label(egui::RichText::new(short_name).small());
                            ui.label(
                                egui::RichText::new(format!("{:.2}%", h.weight_pct))
                                    .color(UP)
                                    .monospace(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:.0}", h.shares))
                                    .monospace()
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(fmt_money(h.market_value))
                                    .monospace()
                                    .small(),
                            );
                            ui.end_row();
                        }
                    });
            });
    }
}

pub fn render_executives(ui: &mut egui::Ui, rows: &[Executive]) {
    ui.separator();
    if rows.is_empty() {
        ui.label(
            egui::RichText::new("No officers on file — click Load Cached or Fetch.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        // Aggregate total comp
        let total_comp: f64 = rows.iter().map(|e| e.compensation).sum();
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("Total comp: ${:.1}M", total_comp / 1e6))
                    .strong()
                    .color(UP),
            );
            ui.label(
                egui::RichText::new(format!("({} officers)", rows.len()))
                    .color(AXIS_TEXT)
                    .small(),
            );
        });
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("mgmt_grid")
                    .striped(true)
                    .num_columns(6)
                    .spacing([14.0, 3.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Name").strong());
                        ui.label(egui::RichText::new("Position").strong());
                        ui.label(egui::RichText::new("Age").strong());
                        ui.label(egui::RichText::new("Sex").strong());
                        ui.label(egui::RichText::new("Since").strong());
                        ui.label(egui::RichText::new("Compensation").strong());
                        ui.end_row();
                        for e in rows.iter() {
                            ui.label(egui::RichText::new(&e.name).small().strong());
                            ui.label(egui::RichText::new(&e.position).color(AXIS_TEXT).small());
                            if e.age > 0 {
                                ui.label(
                                    egui::RichText::new(format!("{}", e.age))
                                        .monospace()
                                        .small(),
                                );
                            } else {
                                ui.label(egui::RichText::new("—").color(AXIS_TEXT).small());
                            }
                            ui.label(egui::RichText::new(&e.sex).monospace().small());
                            ui.label(egui::RichText::new(&e.since).monospace().small());
                            if e.compensation > 0.0 {
                                ui.label(
                                    egui::RichText::new(format!("${:.2}M", e.compensation / 1e6))
                                        .color(UP)
                                        .monospace(),
                                );
                            } else {
                                ui.label(egui::RichText::new("—").color(AXIS_TEXT).small());
                            }
                            ui.end_row();
                        }
                    });
            });
    }
}

pub fn render_hp_rows(ui: &mut egui::Ui, rows: &[HistoricalPriceRow]) {
    ui.separator();
    if rows.is_empty() {
        ui.label(
            egui::RichText::new("No historical bars — click Load Cached or Fetch.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        ui.label(egui::RichText::new(format!("{} daily bars", rows.len())).strong());
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("hp_grid")
                    .striped(true)
                    .num_columns(8)
                    .spacing([14.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Date").strong());
                        ui.label(egui::RichText::new("Open").strong());
                        ui.label(egui::RichText::new("High").strong());
                        ui.label(egui::RichText::new("Low").strong());
                        ui.label(egui::RichText::new("Close").strong());
                        ui.label(egui::RichText::new("Volume").strong());
                        ui.label(egui::RichText::new("Chg").strong());
                        ui.label(egui::RichText::new("Chg %").strong());
                        ui.end_row();
                        for r in rows.iter() {
                            let chg_col = if r.change >= 0.0 { UP } else { DOWN };
                            ui.label(egui::RichText::new(&r.date).monospace().small());
                            ui.label(
                                egui::RichText::new(format!("{:.2}", r.open))
                                    .monospace()
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:.2}", r.high))
                                    .monospace()
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:.2}", r.low))
                                    .monospace()
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:.2}", r.close))
                                    .strong()
                                    .monospace()
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:.1}M", r.volume / 1e6))
                                    .color(AXIS_TEXT)
                                    .monospace()
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:+.2}", r.change))
                                    .color(chg_col)
                                    .monospace()
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:+.2}%", r.change_pct))
                                    .color(chg_col)
                                    .monospace()
                                    .small(),
                            );
                            ui.end_row();
                        }
                    });
            });
    }
}

pub fn render_insider_trades(ui: &mut egui::Ui, rows: &[InsiderTrade]) {
    ui.separator();
    if rows.is_empty() {
        ui.label(
            egui::RichText::new("No insider trades — click Load Cached or Fetch.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        // Derive a quick net-flow summary.
        let (mut bought, mut sold) = (0.0_f64, 0.0_f64);
        for t in rows.iter() {
            match t.acquisition_disposition.as_str() {
                "A" => bought += t.value_usd,
                "D" => sold += t.value_usd,
                _ => {}
            }
        }
        let net = bought - sold;
        let net_col = if net >= 0.0 { UP } else { DOWN };
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("{} filings", rows.len())).strong());
            ui.label(
                egui::RichText::new(format!("Buys: ${:.1}M", bought / 1e6))
                    .color(UP)
                    .monospace()
                    .small(),
            );
            ui.label(
                egui::RichText::new(format!("Sells: ${:.1}M", sold / 1e6))
                    .color(DOWN)
                    .monospace()
                    .small(),
            );
            ui.label(
                egui::RichText::new(format!("Net: ${:.1}M", net / 1e6))
                    .color(net_col)
                    .strong()
                    .monospace()
                    .small(),
            );
        });
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("ins_grid")
                    .striped(true)
                    .num_columns(7)
                    .spacing([14.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Filed").strong());
                        ui.label(egui::RichText::new("Tx Date").strong());
                        ui.label(egui::RichText::new("Insider").strong());
                        ui.label(egui::RichText::new("Type").strong());
                        ui.label(egui::RichText::new("Shares").strong());
                        ui.label(egui::RichText::new("Price").strong());
                        ui.label(egui::RichText::new("Value").strong());
                        ui.end_row();
                        for t in rows.iter().take(100) {
                            let dir_col = match t.acquisition_disposition.as_str() {
                                "A" => UP,
                                "D" => DOWN,
                                _ => AXIS_TEXT,
                            };
                            ui.label(egui::RichText::new(&t.filing_date).monospace().small());
                            ui.label(egui::RichText::new(&t.transaction_date).monospace().small());
                            ui.label(egui::RichText::new(&t.reporting_name).small());
                            ui.label(
                                egui::RichText::new(&t.transaction_type)
                                    .color(dir_col)
                                    .monospace()
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:.0}", t.shares))
                                    .monospace()
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!("${:.2}", t.price))
                                    .monospace()
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!("${:.1}k", t.value_usd / 1e3))
                                    .color(dir_col)
                                    .monospace()
                                    .small(),
                            );
                            ui.end_row();
                        }
                    });
            });
    }
}

pub fn render_institutional_holders(ui: &mut egui::Ui, rows: &[InstitutionalHolder]) {
    ui.separator();
    if rows.is_empty() {
        ui.label(
            egui::RichText::new("No institutional holders — click Load Cached or Fetch.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        let total: f64 = rows.iter().map(|h| h.shares).sum();
        ui.label(
            egui::RichText::new(format!(
                "{} holders — {:.1}M total shares",
                rows.len(),
                total / 1e6
            ))
            .strong(),
        );
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("hds_grid")
                    .striped(true)
                    .num_columns(4)
                    .spacing([18.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Holder").strong());
                        ui.label(egui::RichText::new("Shares").strong());
                        ui.label(egui::RichText::new("QoQ Δ").strong());
                        ui.label(egui::RichText::new("Reported").strong());
                        ui.end_row();
                        for h in rows.iter().take(200) {
                            let chg_col = if h.change > 0.0 {
                                UP
                            } else if h.change < 0.0 {
                                DOWN
                            } else {
                                AXIS_TEXT
                            };
                            ui.label(egui::RichText::new(&h.holder).small());
                            ui.label(
                                egui::RichText::new(format!("{:.2}M", h.shares / 1e6))
                                    .monospace()
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:+.2}M", h.change / 1e6))
                                    .color(chg_col)
                                    .monospace()
                                    .small(),
                            );
                            ui.label(egui::RichText::new(&h.date_reported).monospace().small());
                            ui.end_row();
                        }
                    });
            });
    }
}

pub fn render_rating_changes(ui: &mut egui::Ui, rows: &[RatingChange]) {
    ui.separator();
    if rows.is_empty() {
        ui.label(
            egui::RichText::new("No rating changes — click Load Cached or Fetch.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("updg_grid")
                    .striped(true)
                    .num_columns(6)
                    .spacing([12.0, 2.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Date").strong());
                        ui.label(egui::RichText::new("Firm").strong());
                        ui.label(egui::RichText::new("Action").strong());
                        ui.label(egui::RichText::new("From").strong());
                        ui.label(egui::RichText::new("To").strong());
                        ui.label(egui::RichText::new("Target").strong());
                        ui.end_row();
                        for r in rows.iter().take(200) {
                            ui.label(egui::RichText::new(&r.date).monospace().small());
                            ui.label(egui::RichText::new(&r.firm).small());
                            let act_col = match r.action.as_str() {
                                "upgrade" => BTN_GREEN_TEXT,
                                "downgrade" => BTN_RED_TEXT,
                                "initiation" => egui::Color32::from_rgb(100, 200, 255),
                                _ => AXIS_TEXT,
                            };
                            ui.label(
                                egui::RichText::new(r.action.to_uppercase())
                                    .color(act_col)
                                    .small()
                                    .strong(),
                            );
                            ui.label(egui::RichText::new(&r.from_grade).color(AXIS_TEXT).small());
                            ui.label(egui::RichText::new(&r.to_grade).small().strong());
                            if r.price_target > 0.0 {
                                ui.label(
                                    egui::RichText::new(format!("${:.2}", r.price_target))
                                        .color(UP)
                                        .monospace(),
                                );
                            } else {
                                ui.label(egui::RichText::new("—").color(AXIS_TEXT).small());
                            }
                            ui.end_row();
                        }
                    });
            });
    }
}

pub fn render_sector_perf(ui: &mut egui::Ui, rows: &[SectorPerformance]) {
    ui.separator();
    if rows.is_empty() {
        ui.label(
            egui::RichText::new("No data — click Fetch to pull FMP sector-performance.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        // Sort descending by pct for heatmap feel
        let mut rows: Vec<&typhoon_engine::core::research::SectorPerformance> =
            rows.iter().collect();
        rows.sort_by(|a, b| {
            b.change_pct
                .partial_cmp(&a.change_pct)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let up = rows.iter().filter(|r| r.change_pct > 0.0).count();
        let down = rows.iter().filter(|r| r.change_pct < 0.0).count();
        let avg: f64 = if rows.is_empty() {
            0.0
        } else {
            rows.iter().map(|r| r.change_pct).sum::<f64>() / rows.len() as f64
        };
        ui.label(
            egui::RichText::new(format!(
                "{} sectors · {} up · {} down · avg {:+.2}%",
                rows.len(),
                up,
                down,
                avg
            ))
            .color(AXIS_TEXT)
            .small(),
        );
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                egui::Grid::new("indu_grid")
                    .striped(true)
                    .num_columns(3)
                    .min_col_width(140.0)
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Sector")
                                .color(AXIS_TEXT)
                                .small()
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new("Chg %")
                                .color(AXIS_TEXT)
                                .small()
                                .strong(),
                        );
                        ui.label(egui::RichText::new("Bar").color(AXIS_TEXT).small().strong());
                        ui.end_row();
                        let max_abs = rows
                            .iter()
                            .map(|r| r.change_pct.abs())
                            .fold(0.0_f64, f64::max)
                            .max(0.1);
                        for r in rows.iter() {
                            let col = if r.change_pct > 0.0 {
                                UP
                            } else if r.change_pct < 0.0 {
                                DOWN
                            } else {
                                AXIS_TEXT
                            };
                            ui.label(egui::RichText::new(&r.sector).small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:+.2}%", r.change_pct))
                                    .color(col)
                                    .small()
                                    .monospace()
                                    .strong(),
                            );
                            let width = (r.change_pct.abs() / max_abs * 30.0).round() as usize;
                            let bar = if r.change_pct >= 0.0 {
                                format!("▌{}", "█".repeat(width))
                            } else {
                                format!("{}▐", "█".repeat(width))
                            };
                            ui.label(egui::RichText::new(bar).color(col).monospace().small());
                            ui.end_row();
                        }
                    });
            });
    }
}

pub fn render_splits_list(ui: &mut egui::Ui, rows: &[StockSplit]) {
    ui.separator();
    if rows.is_empty() {
        ui.label(
            egui::RichText::new("No split history — click Load Cached or Fetch.")
                .color(AXIS_TEXT)
                .small(),
        );
    } else {
        ui.label(egui::RichText::new(format!("{} split events", rows.len())).strong());
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("splt_grid")
                    .striped(true)
                    .num_columns(4)
                    .spacing([20.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Date").strong());
                        ui.label(egui::RichText::new("Label").strong());
                        ui.label(egui::RichText::new("Ratio").strong());
                        ui.label(egui::RichText::new("From → To").strong());
                        ui.end_row();
                        for s in rows.iter() {
                            ui.label(egui::RichText::new(&s.date).monospace().small());
                            ui.label(egui::RichText::new(&s.label).monospace().strong().color(UP));
                            let ratio = if s.denominator > 0.0 {
                                s.numerator / s.denominator
                            } else {
                                0.0
                            };
                            ui.label(
                                egui::RichText::new(format!("{:.3}x", ratio))
                                    .monospace()
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "{:.0} → {:.0}",
                                    s.denominator, s.numerator
                                ))
                                .color(AXIS_TEXT)
                                .small(),
                            );
                            ui.end_row();
                        }
                    });
            });
    }
}
