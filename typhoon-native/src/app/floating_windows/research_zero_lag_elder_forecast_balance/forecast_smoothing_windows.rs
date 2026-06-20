use super::*;

impl TyphooNApp {
    pub(super) fn render_forecast_smoothing_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
        if self.show_tsf_win {
            if self.tsf_win_symbol.is_empty() {
                self.tsf_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_tsf_win;
            egui::Window::new("TSF — Time Series Forecast (OLS slope projected one bar forward)")
                .open(&mut open).resizable(true).default_size([580.0, 290.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.tsf_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.tsf_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.tsf_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_tsf(&conn, &sym_u) { self.tsf_win_snapshot = snap; self.tsf_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.tsf_win_symbol.to_uppercase(); self.tsf_win_loading = true; self.tsf_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeTsfSnapshot { symbol: sym });
                        }
                        if self.tsf_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.tsf_win_snapshot;
                    if snap.symbol.is_empty() || snap.tsf_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥20 bars with OHLC.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.tsf_label.as_str() {
                            "LEADING_UP" | "LAGGING_UP" => UP,
                            "LEADING_DOWN" | "LAGGING_DOWN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — forecast {:.4} — close {:.4} — slope {:+.5} — R² {:.3} — as of {}", snap.symbol, snap.tsf_label, snap.forecast_value, snap.last_close, snap.slope, snap.r_squared, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("tsf_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Slope (per bar)").small().strong()); ui.label(egui::RichText::new(format!("{:+.5}", snap.slope)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Intercept").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.intercept)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Forecast (t+1)").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.forecast_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Forecast dev %").small().strong()); ui.label(egui::RichText::new(format!("{:+.2}%", snap.forecast_deviation_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("R²").small().strong()); ui.label(egui::RichText::new(format!("{:.3}", snap.r_squared)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                    }
                });
            self.show_tsf_win = open;
        }

        if self.show_rvi_win {
            if self.rvi_win_symbol.is_empty() {
                self.rvi_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_rvi_win;
            egui::Window::new(
                "RVI — Relative Vigor Index (Ehlers, length 10, 4-bar triangular weighting)",
            )
            .open(&mut open)
            .resizable(true)
            .default_size([580.0, 280.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.rvi_win_symbol).desired_width(100.0),
                    );
                    if ui.button("Use Chart").clicked() {
                        self.rvi_win_symbol = chart_sym_research.clone();
                    }
                    if ui.button("Load Cached").clicked() {
                        if let Some(ref cache) = self.cache {
                            if let Ok(conn) = cache.connection() {
                                let sym_u = self.rvi_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) =
                                    typhoon_engine::core::research::get_rvi(&conn, &sym_u)
                                {
                                    self.rvi_win_snapshot = snap;
                                    self.rvi_win_symbol = sym_u;
                                }
                            }
                        }
                    }
                    if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                        let sym = self.rvi_win_symbol.to_uppercase();
                        self.rvi_win_loading = true;
                        self.rvi_win_symbol = sym.clone();
                        let _ = self
                            .broker_tx
                            .send(BrokerCmd::ComputeRviSnapshot { symbol: sym });
                    }
                    if self.rvi_win_loading {
                        ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                    }
                });
                ui.separator();
                let snap = &self.rvi_win_snapshot;
                if snap.symbol.is_empty() || snap.rvi_label == "INSUFFICIENT_DATA" {
                    ui.label(
                        egui::RichText::new("No data — HP cache needs ≥17 bars with OHLC.")
                            .color(AXIS_TEXT)
                            .small(),
                    );
                } else {
                    let color = match snap.rvi_label.as_str() {
                        "BULL_CROSS" | "BULL" => UP,
                        "BEAR_CROSS" | "BEAR" => DOWN,
                        _ => AXIS_TEXT,
                    };
                    ui.label(
                        egui::RichText::new(format!(
                            "{} — {} — RVI {:+.4} — signal {:+.4} — close {:.4} — as of {}",
                            snap.symbol,
                            snap.rvi_label,
                            snap.rvi_value,
                            snap.signal_value,
                            snap.last_close,
                            snap.as_of
                        ))
                        .strong()
                        .color(color),
                    );
                    ui.separator();
                    egui::Grid::new("rvi_summary")
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
                            ui.label(egui::RichText::new("RVI").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:+.4}", snap.rvi_value))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("RVI prev").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:+.4}", snap.rvi_prev))
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
                            ui.label(egui::RichText::new("Signal prev").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:+.4}", snap.signal_prev))
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
            });
            self.show_rvi_win = open;
        }

        if self.show_trima_win {
            if self.trima_win_symbol.is_empty() {
                self.trima_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_trima_win;
            egui::Window::new("TRIMA — Triangular MA (SMA-of-SMA, length 20)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.trima_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.trima_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.trima_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_trima(&conn, &sym_u)
                                    {
                                        self.trima_win_snapshot = snap;
                                        self.trima_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.trima_win_symbol.to_uppercase();
                            self.trima_win_loading = true;
                            self.trima_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeTrimaSnapshot { symbol: sym });
                        }
                        if self.trima_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.trima_win_snapshot;
                    if snap.symbol.is_empty() || snap.trima_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥31 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.trima_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — TRIMA {:.4} — close {:.4} — dev {:+.2}% — as of {}",
                                snap.symbol,
                                snap.trima_label,
                                snap.trima_value,
                                snap.last_close,
                                snap.deviation_pct,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("trima_summary")
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
                                ui.label(egui::RichText::new("TRIMA").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.trima_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("TRIMA prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.trima_prev))
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
                });
            self.show_trima_win = open;
        }

        if self.show_t3_win {
            if self.t3_win_symbol.is_empty() {
                self.t3_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_t3_win;
            egui::Window::new("T3 — Tillson T3 MA (six-EMA chain, v=0.7)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.t3_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.t3_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.t3_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_t3(&conn, &sym_u)
                                    {
                                        self.t3_win_snapshot = snap;
                                        self.t3_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.t3_win_symbol.to_uppercase();
                            self.t3_win_loading = true;
                            self.t3_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeT3Snapshot { symbol: sym });
                        }
                        if self.t3_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.t3_win_snapshot;
                    if snap.symbol.is_empty() || snap.t3_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new(
                                "No data — HP cache needs ≥24 bars (6-chain warmup).",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                    } else {
                        let color = match snap.t3_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — T3 {:.4} — close {:.4} — dev {:+.2}% — as of {}",
                                snap.symbol,
                                snap.t3_label,
                                snap.t3_value,
                                snap.last_close,
                                snap.deviation_pct,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("t3_summary")
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
                                ui.label(egui::RichText::new("v (volume factor)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.v_factor))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("T3").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.t3_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("T3 prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.t3_prev))
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
                });
            self.show_t3_win = open;
        }
    }
}
