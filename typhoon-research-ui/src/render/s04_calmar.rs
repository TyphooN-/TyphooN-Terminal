// Segment of the research snapshot renderers — see render.rs (this file is
// order-preserving; segments are mechanical splits, not semantic groups).
#[allow(unused_imports)]
use crate::theme::{AXIS_TEXT, BTN_GREEN_TEXT, BTN_RED_TEXT, DOWN, UP};
#[allow(unused_imports)]
use typhoon_engine::core::research::*;

pub fn render_calmar_snapshot(ui: &mut egui::Ui, snap: &CalmarRatioSnapshot) {
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

pub fn render_ccrl_snapshot(ui: &mut egui::Ui, snap: &CashCycleSnapshot) {
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

pub fn render_cfvar_snapshot(ui: &mut egui::Ui, snap: &CornishFisherSnapshot) {
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

pub fn render_closeplc_snapshot(ui: &mut egui::Ui, snap: &ClosePlacementSnapshot) {
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

pub fn render_cor_snapshot(ui: &mut egui::Ui, snap: &CorrelationMatrix) {
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

pub fn render_cordim_snapshot(ui: &mut egui::Ui, snap: &CordimSnapshot) {
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

pub fn render_covg_snapshot(ui: &mut egui::Ui, snap: &CoverageSnapshot) {
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

pub fn render_credit_snapshot(ui: &mut egui::Ui, snap: &CreditSnapshot) {
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

pub fn render_cusum_snapshot(ui: &mut egui::Ui, snap: &CusumBreakSnapshot) {
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

pub fn render_cvar_snapshot(ui: &mut egui::Ui, snap: &CVaRSnapshot) {
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

pub fn render_dcf_snapshot(ui: &mut egui::Ui, snap: &DcfSnapshot) {
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

pub fn render_dddur_snapshot(ui: &mut egui::Ui, snap: &DrawdownDurationSnapshot) {
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

pub fn render_ddm_snapshot(ui: &mut egui::Ui, snap: &DdmSnapshot) {
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

pub fn render_des_snapshot(ui: &mut egui::Ui, snap: &DailyEventStreakSnapshot) {
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

pub fn render_dfa_snapshot(ui: &mut egui::Ui, snap: &DetrendedFluctuationSnapshot) {
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

pub fn render_divg_snapshot(ui: &mut egui::Ui, snap: &DivgSnapshot) {
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

pub fn render_doweffect_snapshot(ui: &mut egui::Ui, snap: &DayOfWeekEffectSnapshot) {
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

pub fn render_downvol_snapshot(ui: &mut egui::Ui, snap: &DownsideVolSnapshot) {
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

pub fn render_drawdar_snapshot(ui: &mut egui::Ui, snap: &DrawDaRSnapshot) {
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

pub fn render_drawup_snapshot(ui: &mut egui::Ui, snap: &DrawupHistorySnapshot) {
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

pub fn render_durbinwatson_snapshot(ui: &mut egui::Ui, snap: &DurbinWatsonSnapshot) {
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

pub fn render_dvdrank_snapshot(ui: &mut egui::Ui, snap: &DividendGrowthRankSnapshot) {
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

pub fn render_dvdyieldrank_snapshot(ui: &mut egui::Ui, snap: &DividendYieldRankSnapshot) {
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

pub fn render_earm_snapshot(ui: &mut egui::Ui, snap: &EarmSnapshot) {
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

pub fn render_earmrank_snapshot(ui: &mut egui::Ui, snap: &EarningsMomentumRankSnapshot) {
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

pub fn render_effratio_snapshot(ui: &mut egui::Ui, snap: &EfficiencyRatioSnapshot) {
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

pub fn render_entropy_snapshot(ui: &mut egui::Ui, snap: &EntropySnapshot) {
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

pub fn render_epsb_snapshot(ui: &mut egui::Ui, snap: &EpsBeatSnapshot) {
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

pub fn render_ewmavol_snapshot(ui: &mut egui::Ui, snap: &EwmaVolSnapshot) {
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

pub fn render_fcfy_snapshot(ui: &mut egui::Ui, snap: &FcfYieldSnapshot) {
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

pub fn render_figi_snapshot(ui: &mut egui::Ui, snap: &FigiSnapshot) {
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

pub fn render_flow_snapshot(ui: &mut egui::Ui, snap: &FlowSnapshot) {
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

pub fn render_fqm_snapshot(ui: &mut egui::Ui, snap: &FundamentalQualityMeterSnapshot) {
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
