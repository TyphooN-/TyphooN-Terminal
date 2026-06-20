use super::*;

mod rank_highlow_windows;
mod volatility_correlation_windows;
mod volatility_drawdown_momentum;

impl TyphooNApp {
    pub(super) fn render_research_behavior_distribution_stats_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research: String = self
            .charts
            .get(self.active_tab)
            .map(|c| {
                c.symbol
                    .split(':')
                    .rev()
                    .nth(1)
                    .or_else(|| c.symbol.split(':').last())
                    .unwrap_or("AAPL")
                    .to_string()
            })
            .unwrap_or_else(|| "AAPL".to_string());

        self.render_volatility_drawdown_momentum_windows(ctx, &chart_sym_research);

        self.render_rank_highlow_windows(ctx, &chart_sym_research);

        self.render_volatility_correlation_windows(ctx, &chart_sym_research);

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

        // VRP — implied-vs-realized vol premium
        if self.show_vrp {
            if self.vrp_symbol.is_empty() {
                self.vrp_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_vrp;
            egui::Window::new("VRP — Vol Risk Premium")
                .open(&mut open)
                .resizable(true)
                .default_size([700.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.vrp_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.vrp_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.vrp_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_vrp(&conn, &sym_u)
                                    {
                                        self.vrp_snapshot = snap;
                                        self.vrp_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.vrp_symbol.to_uppercase();
                            self.vrp_loading = true;
                            self.vrp_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeVrpSnapshot { symbol: sym });
                        }
                        if self.vrp_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.vrp_snapshot;
                    if snap.symbol.is_empty() || snap.premium_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new(
                                "No data — needs cached IVOL and RVCONE snapshots for the subject.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.premium_label.as_str() {
                            "CHEAP_IV" => UP,
                            "RICH_IV" | "EXTREME_RICH" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — IV {:.1}% vs RV20 {:.1}% ({:.2}x) — as of {}",
                                snap.symbol,
                                snap.premium_label,
                                snap.current_atm_iv_pct,
                                snap.rv20_pct,
                                snap.iv_to_rv20_ratio,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("vrp_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(240.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("IV rank / percentile / observations")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.0} / {:.0} / {}",
                                        snap.iv_rank, snap.iv_percentile, snap.iv_observation_count
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("RV20 / RV60 / RV252").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.1}% / {:.1}% / {:.1}%",
                                        snap.rv20_pct, snap.rv60_pct, snap.rv252_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("IV-RV20 / IV-RV252").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.1} pts ({:.2}x) / {:+.1} pts ({:.2}x)",
                                        snap.iv_minus_rv20_pct,
                                        snap.iv_to_rv20_ratio,
                                        snap.iv_minus_rv252_pct,
                                        snap.iv_to_rv252_ratio
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("RV cone label / 20d percentile")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {:.0}",
                                        snap.rv_cone_label, snap.rv20_percentile
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
            self.show_vrp = open;
        }

        // RETSKEW — Return Distribution Skewness
        if self.show_retskew {
            if self.retskew_symbol.is_empty() {
                self.retskew_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_retskew;
            egui::Window::new("RETSKEW — Return Distribution Skewness")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.retskew_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.retskew_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.retskew_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_retskew(&conn, &sym_u)
                                    {
                                        self.retskew_snapshot = snap;
                                        self.retskew_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.retskew_symbol.to_uppercase();
                            self.retskew_loading = true;
                            self.retskew_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRetskewSnapshot { symbol: sym });
                        }
                        if self.retskew_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.retskew_snapshot;
                    if snap.symbol.is_empty() || snap.skew_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — needs ≥20 cached daily bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.skew_label.as_str() {
                            "STRONG_RIGHT" | "RIGHT" => UP,
                            "STRONG_LEFT" | "LEFT" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — skew {:+.3} — up-day share {:.1}% — {} bars — as of {}",
                                snap.symbol,
                                snap.skew_label,
                                snap.skewness,
                                snap.positive_return_pct,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("retskew_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Skewness").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.skewness))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Mean / stdev log return")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.5} / {:.5}",
                                        snap.mean_log_return, snap.stdev_log_return
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Largest up / down (single session)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.2}% / {:+.2}%",
                                        snap.largest_up_pct, snap.largest_down_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Up-day share").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.1}%",
                                        snap.positive_return_pct
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
            self.show_retskew = open;
        }

        // RETKURT — Return Distribution Excess Kurtosis
        if self.show_retkurt {
            if self.retkurt_symbol.is_empty() {
                self.retkurt_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_retkurt;
            egui::Window::new("RETKURT — Return Distribution Kurtosis")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.retkurt_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.retkurt_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.retkurt_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_retkurt(&conn, &sym_u) {
                                        self.retkurt_snapshot = snap;
                                        self.retkurt_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.retkurt_symbol.to_uppercase();
                            self.retkurt_loading = true;
                            self.retkurt_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeRetkurtSnapshot { symbol: sym });
                        }
                        if self.retkurt_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.retkurt_snapshot;
                    if snap.symbol.is_empty() || snap.kurt_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — needs ≥20 cached daily bars.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.kurt_label.as_str() {
                            "PLATYKURTIC" | "NORMAL" => UP,
                            "FAT" | "EXTREME_FAT" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — excess kurt {:+.2} — |z|>2 outliers {} ({:.1}%) — {} bars — as of {}",
                            snap.symbol, snap.kurt_label, snap.excess_kurtosis,
                            snap.outlier_2sigma_count, snap.outlier_2sigma_pct,
                            snap.bars_used, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("retkurt_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Excess kurtosis").small().strong());
                            ui.label(egui::RichText::new(format!("{:+.3}", snap.excess_kurtosis)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("|z|>2 outliers (count / %)").small().strong());
                            ui.label(egui::RichText::new(format!("{} / {:.2}% (normal ≈ 4.55%)",
                                snap.outlier_2sigma_count, snap.outlier_2sigma_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("|z|>3 outliers").small().strong());
                            ui.label(egui::RichText::new(format!("{} (normal ≈ 0.27%)", snap.outlier_3sigma_count)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Mean / stdev log return").small().strong());
                            ui.label(egui::RichText::new(format!("{:+.5} / {:.5}", snap.mean_log_return, snap.stdev_log_return)).small().monospace());
                            ui.end_row();
                        });
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
                        }
                    }
                });
            self.show_retkurt = open;
        }

        // TAILR — Tail Ratio
        if self.show_tailr {
            if self.tailr_symbol.is_empty() {
                self.tailr_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_tailr;
            egui::Window::new("TAILR — Tail Ratio")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.tailr_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.tailr_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.tailr_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_tailr(&conn, &sym_u)
                                    {
                                        self.tailr_snapshot = snap;
                                        self.tailr_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.tailr_symbol.to_uppercase();
                            self.tailr_loading = true;
                            self.tailr_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeTailrSnapshot { symbol: sym });
                        }
                        if self.tailr_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.tailr_snapshot;
                    if snap.symbol.is_empty() || snap.bias_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — needs ≥20 cached daily bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.bias_label.as_str() {
                            "UPSIDE_HEAVY" | "SLIGHT_UPSIDE" => UP,
                            "DOWNSIDE_HEAVY" | "SLIGHT_DOWNSIDE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — 95/5 ratio {:.2} — {} bars — as of {}",
                                snap.symbol,
                                snap.bias_label,
                                snap.tail_ratio,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("tailr_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("95th / 5th percentile returns")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.2}% / {:+.2}%",
                                        snap.pct_95_return, snap.pct_05_return
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("99th / 1st percentile returns")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.2}% / {:+.2}%",
                                        snap.pct_99_return, snap.pct_01_return
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Tail ratio 95/5").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.tail_ratio))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Tail ratio 99/1").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.tail_ratio_99_01))
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
            self.show_tailr = open;
        }

        // RUNLEN — Up/Down Day Run Length Stats
        if self.show_runlen {
            if self.runlen_symbol.is_empty() {
                self.runlen_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_runlen;
            egui::Window::new("RUNLEN — Up/Down Day Run Lengths")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.runlen_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.runlen_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.runlen_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_runlen(&conn, &sym_u) {
                                        self.runlen_snapshot = snap;
                                        self.runlen_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.runlen_symbol.to_uppercase();
                            self.runlen_loading = true;
                            self.runlen_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeRunlenSnapshot { symbol: sym });
                        }
                        if self.runlen_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.runlen_snapshot;
                    if snap.symbol.is_empty() || snap.trend_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — needs ≥20 cached daily bars.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.trend_label.as_str() {
                            "TRENDING" | "STRONG_TRENDING" => UP,
                            "CHOPPY" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        let run_desc = if snap.current_run_length > 0 {
                            format!("{} up", snap.current_run_length)
                        } else if snap.current_run_length < 0 {
                            format!("{} down", snap.current_run_length.abs())
                        } else {
                            "flat".into()
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — avg up {:.2} / avg down {:.2} — current {} — {} bars — as of {}",
                            snap.symbol, snap.trend_label, snap.avg_up_run, snap.avg_down_run,
                            run_desc, snap.bars_used, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("runlen_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Avg up / down run length").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2} / {:.2}", snap.avg_up_run, snap.avg_down_run)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Longest up / down run").small().strong());
                            ui.label(egui::RichText::new(format!("{} / {}", snap.longest_up_run, snap.longest_down_run)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Up / down runs count").small().strong());
                            ui.label(egui::RichText::new(format!("{} / {}", snap.up_runs_count, snap.down_runs_count)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Current run").small().strong());
                            ui.label(egui::RichText::new(&run_desc).small().monospace());
                            ui.end_row();
                        });
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
                        }
                    }
                });
            self.show_runlen = open;
        }

        // DAYRANGE — Daily Range Analysis
        if self.show_dayrange {
            if self.dayrange_symbol.is_empty() {
                self.dayrange_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_dayrange;
            egui::Window::new("DAYRANGE — Daily Range Analysis")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.dayrange_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.dayrange_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.dayrange_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_dayrange(&conn, &sym_u) {
                                        self.dayrange_snapshot = snap;
                                        self.dayrange_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.dayrange_symbol.to_uppercase();
                            self.dayrange_loading = true;
                            self.dayrange_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeDayrangeSnapshot { symbol: sym });
                        }
                        if self.dayrange_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.dayrange_snapshot;
                    if snap.symbol.is_empty() || snap.range_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — needs ≥20 cached daily bars.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.range_label.as_str() {
                            "TIGHT" | "COMPRESSED" => UP,
                            "EXPANDED" | "VERY_EXPANDED" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — 60d {:.2}% vs 252d {:.2}% — ratio {:.2} — {} bars — as of {}",
                            snap.symbol, snap.range_label, snap.avg_range_60_pct, snap.avg_range_252_pct,
                            snap.compression_ratio, snap.bars_used, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("dayrange_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("60d / 252d avg range (high-low)/close").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}% / {:.2}%", snap.avg_range_60_pct, snap.avg_range_252_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Compression ratio (60d / 252d)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}", snap.compression_ratio)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Widest / narrowest range").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}% / {:.2}%", snap.widest_range_pct, snap.narrowest_range_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Latest bar range").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}%", snap.latest_range_pct)).small().monospace());
                            ui.end_row();
                        });
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
                        }
                    }
                });
            self.show_dayrange = open;
        }
    }
}
