use super::*;

impl TyphooNApp {
    pub(super) fn render_fundamental_acceleration_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
        // OPERANK_DELTA — operating-margin trend rank vs sector peers
        if self.show_operank_delta {
            if self.operank_delta_symbol.is_empty() {
                self.operank_delta_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_operank_delta;
            egui::Window::new("OPERANK_DELTA — Operating Margin Trend Rank")
                .open(&mut open)
                .resizable(true)
                .default_size([700.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.operank_delta_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.operank_delta_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.operank_delta_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_operank_delta(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.operank_delta_snapshot = snap;
                                        self.operank_delta_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.operank_delta_symbol.to_uppercase();
                            self.operank_delta_loading = true;
                            self.operank_delta_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeOperankDeltaSnapshot { symbol: sym });
                        }
                        if self.operank_delta_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.operank_delta_snapshot;
                    if snap.symbol.is_empty() || snap.rank_label == "NO_DATA" {
                        ui.label(
                            egui::RichText::new(
                                "No data — needs a cached MARGINS snapshot for the subject and at least 3 same-sector peers.",
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
                                "{} — {} — {} {:+.2} pts — rank {}/{} — as of {}",
                                snap.symbol,
                                snap.rank_label,
                                snap.operating_trend_label,
                                snap.operating_margin_change_pct,
                                snap.rank_position,
                                snap.peers_considered + 1,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("operank_delta_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(240.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("Basis / latest period").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {}",
                                        snap.basis, snap.latest_period
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Operating margin / change")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2}% / {:+.2} pts",
                                        snap.operating_margin_pct,
                                        snap.operating_margin_change_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Sector median / p25 / p75 change")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.2} / {:+.2} / {:+.2}",
                                        snap.sector_median_change_pct,
                                        snap.sector_p25_change_pct,
                                        snap.sector_p75_change_pct
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
                                        snap.percentile_rank,
                                        snap.peers_with_data,
                                        snap.peers_considered
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
                });
            self.show_operank_delta = open;
        }

        // DIVACC — dividend growth acceleration
        if self.show_divacc {
            if self.divacc_symbol.is_empty() {
                self.divacc_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_divacc;
            egui::Window::new("DIVACC — Dividend Acceleration")
                .open(&mut open)
                .resizable(true)
                .default_size([700.0, 430.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.divacc_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.divacc_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.divacc_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_divacc(&conn, &sym_u)
                                    {
                                        self.divacc_snapshot = snap;
                                        self.divacc_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.divacc_symbol.to_uppercase();
                            self.divacc_loading = true;
                            self.divacc_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeDivaccSnapshot { symbol: sym });
                        }
                        if self.divacc_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.divacc_snapshot;
                    if snap.symbol.is_empty() || snap.divacc_label == "NO_HISTORY" {
                        ui.label(
                            egui::RichText::new(
                                "No data — needs at least 3 full dividend years from the cached DVD history.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.divacc_label.as_str() {
                            "ACCELERATING" | "REACCELERATING" => UP,
                            "DECELERATING" | "CUTTING" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — {} annual {:.4} — accel {:+.2} pts — as of {}",
                                snap.symbol,
                                snap.divacc_label,
                                snap.latest_year,
                                snap.latest_annual_dividend,
                                snap.acceleration_pct_pts,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("divacc_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(240.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("Latest / prior y/y growth")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.2}% / {:+.2}%",
                                        snap.latest_yoy_growth_pct, snap.prior_yoy_growth_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Recent 3y avg / prior 3y avg")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.2}% / {:+.2}%",
                                        snap.recent_3y_avg_growth_pct,
                                        snap.prior_3y_avg_growth_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Consistency / consecutive growth years")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.0}% / {}",
                                        snap.consistency_score_pct,
                                        snap.consecutive_growth_years
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Payments / years covered")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {}",
                                        snap.total_payments, snap.years_covered
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
                });
            self.show_divacc = open;
        }

        // EPSACC — EPS acceleration from cached quarterly financials
        if self.show_epsacc {
            if self.epsacc_symbol.is_empty() {
                self.epsacc_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_epsacc;
            egui::Window::new("EPSACC — EPS Acceleration")
                .open(&mut open)
                .resizable(true)
                .default_size([700.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.epsacc_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.epsacc_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.epsacc_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_epsacc(&conn, &sym_u)
                                    {
                                        self.epsacc_snapshot = snap;
                                        self.epsacc_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.epsacc_symbol.to_uppercase();
                            self.epsacc_loading = true;
                            self.epsacc_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeEpsaccSnapshot { symbol: sym });
                        }
                        if self.epsacc_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.epsacc_snapshot;
                    if snap.symbol.is_empty() || snap.epsacc_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new(
                                "No data — needs at least 6 cached quarterly financial statements from FA.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.epsacc_label.as_str() {
                            "ACCELERATING" | "TURNAROUND" => UP,
                            "DECELERATING" | "EARNINGS_PRESSURE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — latest {} EPS {:.3} — accel {:+.2} pts — as of {}",
                                snap.symbol,
                                snap.epsacc_label,
                                snap.latest_period,
                                snap.latest_eps,
                                snap.acceleration_pct_pts,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("epsacc_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(240.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("Latest EPS / year-ago EPS")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.3} / {:.3}",
                                        snap.latest_eps, snap.prior_year_eps
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Latest / prior y/y growth")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.2}% / {:+.2}%",
                                        snap.latest_yoy_growth_pct, snap.prior_yoy_growth_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Recent 2q avg / prior 2q avg")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.2}% / {:+.2}%",
                                        snap.recent_2q_avg_yoy_growth_pct,
                                        snap.prior_2q_avg_yoy_growth_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Positive y/y quarters / quarters used")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {}",
                                        snap.positive_yoy_quarters, snap.quarters_used
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
                });
            self.show_epsacc = open;
        }
    }
}
