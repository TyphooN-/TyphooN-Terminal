// Segment of the research snapshot renderers — see render.rs (this file is
// order-preserving; segments are mechanical splits, not semantic groups).
#[allow(unused_imports)]
use crate::theme::{AXIS_TEXT, BTN_GREEN_TEXT, BTN_RED_TEXT, DOWN, UP};
#[allow(unused_imports)]
use typhoon_engine::core::research::*;

pub fn render_relvol_snapshot(ui: &mut egui::Ui, snap: &RelVolSnapshot) {
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

pub fn render_renyient_snapshot(ui: &mut egui::Ui, snap: &RenyientSnapshot) {
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

pub fn render_retquant_snapshot(ui: &mut egui::Ui, snap: &RetquantSnapshot) {
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

pub fn render_revrank_snapshot(ui: &mut egui::Ui, snap: &RevenueGrowthRankSnapshot) {
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

pub fn render_risk_snapshot(ui: &mut egui::Ui, snap: &RiskSnapshot) {
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

pub fn render_robvol_snapshot(ui: &mut egui::Ui, snap: &RobVolSnapshot) {
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

pub fn render_rollsprd_snapshot(ui: &mut egui::Ui, snap: &RollSpreadSnapshot) {
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

pub fn render_rrk_snapshot(ui: &mut egui::Ui, snap: &RiskRankSnapshot) {
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

pub fn render_rsvol_snapshot(ui: &mut egui::Ui, snap: &RogersSatchellVolSnapshot) {
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

pub fn render_runstest_snapshot(ui: &mut egui::Ui, snap: &RunsTestSnapshot) {
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

pub fn render_rvol_snapshot(ui: &mut egui::Ui, snap: &RealizedVolSnapshot) {
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

pub fn render_sadf_snapshot(ui: &mut egui::Ui, snap: &SadfSnapshot) {
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

pub fn render_sampen_snapshot(ui: &mut egui::Ui, snap: &SampenSnapshot) {
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

pub fn render_seag_snapshot(ui: &mut egui::Ui, snap: &SeasonalitySnapshot) {
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

pub fn render_sectr_snapshot(ui: &mut egui::Ui, snap: &SectorRotationSnapshot) {
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

pub fn render_sharpr_snapshot(ui: &mut egui::Ui, snap: &SharpeRatioSnapshot) {
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

pub fn render_shrank_snapshot(ui: &mut egui::Ui, snap: &ShortInterestRankSnapshot) {
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

pub fn render_shrt_snapshot(ui: &mut egui::Ui, snap: &ShortInterestSnapshot) {
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

pub fn render_sizef_snapshot(ui: &mut egui::Ui, snap: &SizeFactorSnapshot) {
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

pub fn render_skew_snapshot(ui: &mut egui::Ui, snap: &VolatilitySkew) {
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

pub fn render_skspec_snapshot(ui: &mut egui::Ui, snap: &SkspecSnapshot) {
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

pub fn render_specent_snapshot(ui: &mut egui::Ui, snap: &SpecentSnapshot) {
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

pub fn render_squeezerank_snapshot(ui: &mut egui::Ui, snap: &SqueezeRankSnapshot) {
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

pub fn render_sterling_snapshot(ui: &mut egui::Ui, snap: &SterlingRatioSnapshot) {
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

pub fn render_surpstk_snapshot(ui: &mut egui::Ui, snap: &EarningsSurpriseStreakSnapshot) {
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

pub fn render_svm_snapshot(ui: &mut egui::Ui, snap: &SvmSnapshot) {
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

pub fn render_tech_snapshot(ui: &mut egui::Ui, snap: &TechnicalSnapshot) {
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

pub fn render_tlrank_snapshot(ui: &mut egui::Ui, snap: &ThirtyDayLiquidityRankSnapshot) {
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

pub fn render_tra_snapshot(ui: &mut egui::Ui, snap: &TotalReturnSnapshot) {
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

pub fn render_tsi_snapshot(ui: &mut egui::Ui, snap: &TsiSnapshot) {
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

pub fn render_turnpts_snapshot(ui: &mut egui::Ui, snap: &TurnPtsSnapshot) {
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

pub fn render_ulcer_snapshot(ui: &mut egui::Ui, snap: &UlcerIndexSnapshot) {
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

pub fn render_updgrank_snapshot(ui: &mut egui::Ui, snap: &UpgradeDowngradeRankSnapshot) {
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
