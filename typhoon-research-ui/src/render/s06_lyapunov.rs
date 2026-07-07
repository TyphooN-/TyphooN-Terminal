// Segment of the research snapshot renderers — see render.rs (this file is
// order-preserving; segments are mechanical splits, not semantic groups).
#[allow(unused_imports)]
use crate::theme::{AXIS_TEXT, BTN_GREEN_TEXT, BTN_RED_TEXT, DOWN, UP};
#[allow(unused_imports)]
use typhoon_engine::core::research::*;

pub fn render_lyapunov_snapshot(ui: &mut egui::Ui, snap: &LyapunovSnapshot) {
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

pub fn render_margins_snapshot(ui: &mut egui::Ui, snap: &MarginsSnapshot) {
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

pub fn render_mcleodli_snapshot(ui: &mut egui::Ui, snap: &McLeodLiSnapshot) {
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

pub fn render_mfdfa_snapshot(ui: &mut egui::Ui, snap: &MfdfaSnapshot) {
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

pub fn render_mngr_snapshot(ui: &mut egui::Ui, snap: &InsiderActivitySnapshot) {
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

pub fn render_mnkendall_snapshot(ui: &mut egui::Ui, snap: &MannKendallSnapshot) {
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

pub fn render_momf_snapshot(ui: &mut egui::Ui, snap: &MomentumRankSnapshot) {
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

pub fn render_monthseas_snapshot(ui: &mut egui::Ui, snap: &MonthlySeasonalitySnapshot) {
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

pub fn render_mrhl_snapshot(ui: &mut egui::Ui, snap: &MeanReversionHalfLifeSnapshot) {
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

pub fn render_msent_snapshot(ui: &mut egui::Ui, snap: &MsentSnapshot) {
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

pub fn render_omega_snapshot(ui: &mut egui::Ui, snap: &OmegaRatioSnapshot) {
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

pub fn render_omon_snapshot(ui: &mut egui::Ui, snap: &OptionsChainSnapshot) {
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
                        let strike_key = |strike: f64| (strike * 1_000_000.0).round() as i64;
                        let call_by_strike: std::collections::HashMap<i64, _> = exp
                            .calls
                            .iter()
                            .map(|c| (strike_key(c.strike), c))
                            .collect();
                        let put_by_strike: std::collections::HashMap<i64, _> =
                            exp.puts.iter().map(|p| (strike_key(p.strike), p)).collect();
                        let mut seen_strikes: std::collections::HashSet<i64> =
                            call_by_strike.keys().copied().collect();
                        let mut strikes: Vec<f64> = exp.calls.iter().map(|c| c.strike).collect();
                        for p in &exp.puts {
                            if seen_strikes.insert(strike_key(p.strike)) {
                                strikes.push(p.strike);
                            }
                        }
                        strikes
                            .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                        for k in strikes.iter().take(40) {
                            let key = strike_key(*k);
                            let call = call_by_strike.get(&key).copied();
                            let put = put_by_strike.get(&key).copied();
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

pub fn render_operank_snapshot(ui: &mut egui::Ui, snap: &OperatingQualityRankSnapshot) {
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

pub fn render_oufit_snapshot(ui: &mut egui::Ui, snap: &OuFitSnapshot) {
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

pub fn render_pacf_snapshot(ui: &mut egui::Ui, snap: &PacfSnapshot) {
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

pub fn render_painratio_snapshot(ui: &mut egui::Ui, snap: &PainRatioSnapshot) {
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

pub fn render_parkinson_snapshot(ui: &mut egui::Ui, snap: &ParkinsonVolSnapshot) {
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

pub fn render_pead_snapshot(ui: &mut egui::Ui, snap: &PeadSnapshot) {
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

pub fn render_peadrank_snapshot(ui: &mut egui::Ui, snap: &PeadRankSnapshot) {
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

pub fn render_periodogram_snapshot(ui: &mut egui::Ui, snap: &PeriodogramSnapshot) {
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

pub fn render_permen_snapshot(ui: &mut egui::Ui, snap: &PermenSnapshot) {
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

pub fn render_pickands_snapshot(ui: &mut egui::Ui, snap: &PickandsSnapshot) {
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

pub fn render_pproot_snapshot(ui: &mut egui::Ui, snap: &PprootSnapshot) {
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

pub fn render_psr_snapshot(ui: &mut egui::Ui, snap: &ProbabilisticSharpeSnapshot) {
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

pub fn render_ptd_snapshot(ui: &mut egui::Ui, snap: &PriceTargetDispersion) {
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

pub fn render_ptfs_snapshot(ui: &mut egui::Ui, snap: &PiotroskiSnapshot) {
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

pub fn render_qrk_snapshot(ui: &mut egui::Ui, snap: &QualityRankSnapshot) {
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

pub fn render_qual_snapshot(ui: &mut egui::Ui, snap: &QualitySnapshot) {
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

pub fn render_rachev_snapshot(ui: &mut egui::Ui, snap: &RachevSnapshot) {
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

pub fn render_rankac_snapshot(ui: &mut egui::Ui, snap: &RankacSnapshot) {
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

pub fn render_recfact_snapshot(ui: &mut egui::Ui, snap: &RecfactSnapshot) {
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

pub fn render_regime_snapshot(ui: &mut egui::Ui, snap: &RegimeSnapshot) {
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

pub fn render_relepsgr_snapshot(ui: &mut egui::Ui, snap: &RelativeEpsGrowthSnapshot) {
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
