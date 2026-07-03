// Segment of the research snapshot renderers — see render.rs (this file is
// order-preserving; segments are mechanical splits, not semantic groups).
#[allow(unused_imports)]
use crate::theme::{AXIS_TEXT, BTN_GREEN_TEXT, BTN_RED_TEXT, DOWN, UP};
#[allow(unused_imports)]
use typhoon_engine::core::research::*;

pub fn render_fqmrank_snapshot(ui: &mut egui::Ui, snap: &FqmRankSnapshot) {
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

pub fn render_gapstats_snapshot(ui: &mut egui::Ui, snap: &GapStatsSnapshot) {
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

pub fn render_garch11_snapshot(ui: &mut egui::Ui, snap: &Garch11Snapshot) {
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

pub fn render_gini_snapshot(ui: &mut egui::Ui, snap: &GiniSnapshot) {
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

pub fn render_gkvol_snapshot(ui: &mut egui::Ui, snap: &GarmanKlassVolSnapshot) {
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

pub fn render_glasym_snapshot(ui: &mut egui::Ui, snap: &GainLossAsymmetrySnapshot) {
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

pub fn render_gph_snapshot(ui: &mut egui::Ui, snap: &GphSnapshot) {
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

pub fn render_gpr_snapshot(ui: &mut egui::Ui, snap: &GprSnapshot) {
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

pub fn render_growm_snapshot(ui: &mut egui::Ui, snap: &GrowmSnapshot) {
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

pub fn render_gy_snapshot(ui: &mut egui::Ui, snap: &GapYearlySnapshot) {
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

pub fn render_higuchi_snapshot(ui: &mut egui::Ui, snap: &HiguchiSnapshot) {
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

pub fn render_hillks_snapshot(ui: &mut egui::Ui, snap: &HillksSnapshot) {
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

pub fn render_hilltail_snapshot(ui: &mut egui::Ui, snap: &HillTailSnapshot) {
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

pub fn render_hitrate_snapshot(ui: &mut egui::Ui, snap: &HitRateSnapshot) {
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

pub fn render_hra_snapshot(ui: &mut egui::Ui, snap: &HraSnapshot) {
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

pub fn render_hurst_snapshot(ui: &mut egui::Ui, snap: &HurstSnapshot) {
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

pub fn render_insiderconc_snapshot(ui: &mut egui::Ui, snap: &InsiderConcentrationSnapshot) {
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

pub fn render_insstrk_snapshot(ui: &mut egui::Ui, snap: &InsiderStreakSnapshot) {
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

pub fn render_ivol_snapshot(ui: &mut egui::Ui, snap: &IvolSnapshot) {
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

pub fn render_jbnorm_snapshot(ui: &mut egui::Ui, snap: &JarqueBeraSnapshot) {
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

pub fn render_kappa3_snapshot(ui: &mut egui::Ui, snap: &Kappa3Snapshot) {
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

pub fn render_kellyf_snapshot(ui: &mut egui::Ui, snap: &KellyFractionSnapshot) {
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

pub fn render_kendalltau_snapshot(ui: &mut egui::Ui, snap: &KendallTauSnapshot) {
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

pub fn render_kpss_snapshot(ui: &mut egui::Ui, snap: &KpssSnapshot) {
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

pub fn render_ksnorm_snapshot(ui: &mut egui::Ui, snap: &KsnormSnapshot) {
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

pub fn render_kylelam_snapshot(ui: &mut egui::Ui, snap: &KylelamSnapshot) {
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

pub fn render_lev_snapshot(ui: &mut egui::Ui, snap: &LeverageSnapshot) {
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

pub fn render_levereff_snapshot(ui: &mut egui::Ui, snap: &LeverEffSnapshot) {
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

pub fn render_levrank_snapshot(ui: &mut egui::Ui, snap: &LeverageRankSnapshot) {
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

pub fn render_liq_snapshot(ui: &mut egui::Ui, snap: &LiquiditySnapshot) {
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

pub fn render_liqrank_snapshot(ui: &mut egui::Ui, snap: &LiquidityRankSnapshot) {
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

pub fn render_ljungb_snapshot(ui: &mut egui::Ui, snap: &LjungBoxSnapshot) {
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

pub fn render_lmom_snapshot(ui: &mut egui::Ui, snap: &LmomSnapshot) {
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

