use super::*;

impl TyphooNApp {
    pub(super) fn render_research_round27_windows(&mut self, ctx: &egui::Context) {
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

        // ── Research Round 27 windows ──
        if self.show_omega {
            if self.omega_symbol.is_empty() {
                self.omega_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_omega;
            egui::Window::new("OMEGA — Omega Ratio (τ=0)")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.omega_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.omega_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.omega_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_omega(&conn, &sym_u)
                                    {
                                        self.omega_snapshot = snap;
                                        self.omega_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.omega_symbol.to_uppercase();
                            self.omega_loading = true;
                            self.omega_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeOmegaSnapshot { symbol: sym });
                        }
                        if self.omega_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.omega_snapshot;
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
                });
            self.show_omega = open;
        }

        if self.show_dfa {
            if self.dfa_symbol.is_empty() {
                self.dfa_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_dfa;
            egui::Window::new("DFA — Detrended Fluctuation Analysis")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.dfa_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.dfa_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.dfa_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_dfa(&conn, &sym_u)
                                    {
                                        self.dfa_snapshot = snap;
                                        self.dfa_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.dfa_symbol.to_uppercase();
                            self.dfa_loading = true;
                            self.dfa_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeDfaSnapshot { symbol: sym });
                        }
                        if self.dfa_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.dfa_snapshot;
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
                });
            self.show_dfa = open;
        }

        if self.show_burke {
            if self.burke_symbol.is_empty() {
                self.burke_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_burke;
            egui::Window::new("BURKE — Burke Ratio (Σdd² adjusted)")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.burke_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.burke_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.burke_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_burke(&conn, &sym_u)
                                    {
                                        self.burke_snapshot = snap;
                                        self.burke_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.burke_symbol.to_uppercase();
                            self.burke_loading = true;
                            self.burke_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeBurkeSnapshot { symbol: sym });
                        }
                        if self.burke_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.burke_snapshot;
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
                                    egui::RichText::new(format!(
                                        "{:+.3}%",
                                        snap.annualized_return_pct
                                    ))
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
                                    egui::RichText::new(format!(
                                        "-{:.2}%",
                                        snap.worst_event_dd_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_burke = open;
        }

        if self.show_monthseas {
            if self.monthseas_symbol.is_empty() {
                self.monthseas_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_monthseas;
            egui::Window::new("MONTHSEAS — Monthly Seasonality")
                .open(&mut open)
                .resizable(true)
                .default_size([720.0, 540.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.monthseas_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.monthseas_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.monthseas_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_monthseas(&conn, &sym_u)
                                    {
                                        self.monthseas_snapshot = snap;
                                        self.monthseas_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.monthseas_symbol.to_uppercase();
                            self.monthseas_loading = true;
                            self.monthseas_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMonthseasSnapshot { symbol: sym });
                        }
                        if self.monthseas_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.monthseas_snapshot;
                    if snap.symbol.is_empty() || snap.season_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new(
                                "No data — HP cache needs ≥120 bars across ≥1 year.",
                            )
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
                            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct",
                            "Nov", "Dec",
                        ];
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — {} yrs — best {} ({:.0}%) / worst {} ({:.0}%) — as of {}",
                            snap.symbol, snap.season_label, snap.years_covered,
                            MONTHS[snap.best_month_idx], snap.best_month_hit_pct,
                            MONTHS[snap.worst_month_idx], snap.worst_month_hit_pct,
                            snap.as_of,
                        )).strong().color(color));
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
                                        egui::RichText::new(format!(
                                            "{:.1}%",
                                            snap.month_hit_pct[m]
                                        ))
                                        .small()
                                        .monospace(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{:+.3}%",
                                            snap.month_mean_ret_pct[m]
                                        ))
                                        .small()
                                        .monospace(),
                                    );
                                    ui.end_row();
                                }
                            });
                    }
                });
            self.show_monthseas = open;
        }

        if self.show_rollsprd {
            if self.rollsprd_symbol.is_empty() {
                self.rollsprd_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_rollsprd;
            egui::Window::new("ROLLSPRD — Roll's Implicit Bid-Ask Spread")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.rollsprd_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.rollsprd_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.rollsprd_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_rollsprd(&conn, &sym_u) {
                                        self.rollsprd_snapshot = snap;
                                        self.rollsprd_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.rollsprd_symbol.to_uppercase();
                            self.rollsprd_loading = true;
                            self.rollsprd_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeRollsprdSnapshot { symbol: sym });
                        }
                        if self.rollsprd_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.rollsprd_snapshot;
                    if snap.symbol.is_empty() || snap.roll_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥30 bars.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else if snap.roll_label == "INVALID_POSITIVE_COV" {
                        ui.label(egui::RichText::new(format!(
                            "{} — INVALID — first-lag cov {:+.6} (≥0) — Roll model undefined (trending series)",
                            snap.symbol, snap.first_lag_cov,
                        )).strong().color(DOWN));
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).small());
                        }
                    } else {
                        let color = match snap.roll_label.as_str() {
                            "TIGHT" => UP,
                            "WIDE" | "VERY_WIDE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — spread {:.4} ({:.1} bps) — {} bars — as of {}",
                            snap.symbol, snap.roll_label, snap.implicit_spread, snap.implicit_spread_bps, snap.bars_used, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("rollsprd_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("First-lag cov").small().strong());
                            ui.label(egui::RichText::new(format!("{:+.6}", snap.first_lag_cov)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Mean price").small().strong());
                            ui.label(egui::RichText::new(format!("{:.4}", snap.mean_price)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Implicit spread").small().strong());
                            ui.label(egui::RichText::new(format!("{:.4}", snap.implicit_spread)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Implicit spread (bps)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}", snap.implicit_spread_bps)).small().monospace());
                            ui.end_row();
                        });
                    }
                });
            self.show_rollsprd = open;
        }
    }
}
