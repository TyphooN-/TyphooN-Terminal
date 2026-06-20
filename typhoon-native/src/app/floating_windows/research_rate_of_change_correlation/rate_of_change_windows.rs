use super::*;

impl TyphooNApp {
    pub(super) fn render_rate_of_change_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
        // ── Research section ──
        if self.show_roc_win {
            if self.roc_win_symbol.is_empty() {
                self.roc_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_roc_win;
            egui::Window::new("ROC — Rate of Change (period 10)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.roc_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.roc_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.roc_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_roc(&conn, &sym_u)
                                    {
                                        self.roc_win_snapshot = snap;
                                        self.roc_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.roc_win_symbol.to_uppercase();
                            self.roc_win_loading = true;
                            self.roc_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRocSnapshot { symbol: sym });
                        }
                        if self.roc_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.roc_win_snapshot;
                    if snap.symbol.is_empty() || snap.roc_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥12 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.roc_label.as_str() {
                            "STRONG_UP" | "UP" => UP,
                            "STRONG_DOWN" | "DOWN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — ROC {:.4} — close {:.4} — lag {:.4} — as of {}",
                                snap.symbol,
                                snap.roc_label,
                                snap.roc,
                                snap.close_now,
                                snap.close_lag,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("roc_summary")
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
                                ui.label(egui::RichText::new("ROC").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.roc))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ROC prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.roc_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Close").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.close_now))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Close (lag)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.close_lag))
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
            self.show_roc_win = open;
        }

        if self.show_rocp_win {
            if self.rocp_win_symbol.is_empty() {
                self.rocp_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_rocp_win;
            egui::Window::new("ROCP — Rate of Change Percentage (period 10)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.rocp_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.rocp_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.rocp_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_rocp(&conn, &sym_u)
                                    {
                                        self.rocp_win_snapshot = snap;
                                        self.rocp_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.rocp_win_symbol.to_uppercase();
                            self.rocp_win_loading = true;
                            self.rocp_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRocpSnapshot { symbol: sym });
                        }
                        if self.rocp_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.rocp_win_snapshot;
                    if snap.symbol.is_empty() || snap.rocp_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥12 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.rocp_label.as_str() {
                            "STRONG_UP" | "UP" => UP,
                            "STRONG_DOWN" | "DOWN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — ROCP {:+.4}% — close {:.4} — as of {}",
                                snap.symbol,
                                snap.rocp_label,
                                snap.rocp_pct,
                                snap.close_now,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("rocp_summary")
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
                                ui.label(egui::RichText::new("ROCP").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.rocp))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ROCP prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.rocp_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ROCP (%)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.rocp_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Close").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.close_now))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Close (lag)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.close_lag))
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
            self.show_rocp_win = open;
        }

        if self.show_rocr_win {
            if self.rocr_win_symbol.is_empty() {
                self.rocr_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_rocr_win;
            egui::Window::new("ROCR — Rate of Change Ratio (period 10)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.rocr_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.rocr_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.rocr_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_rocr(&conn, &sym_u)
                                    {
                                        self.rocr_win_snapshot = snap;
                                        self.rocr_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.rocr_win_symbol.to_uppercase();
                            self.rocr_win_loading = true;
                            self.rocr_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRocrSnapshot { symbol: sym });
                        }
                        if self.rocr_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.rocr_win_snapshot;
                    if snap.symbol.is_empty() || snap.rocr_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥12 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.rocr_label.as_str() {
                            "STRONG_UP" | "UP" => UP,
                            "STRONG_DOWN" | "DOWN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — ROCR {:.6} — close {:.4} — lag {:.4} — as of {}",
                                snap.symbol,
                                snap.rocr_label,
                                snap.rocr,
                                snap.close_now,
                                snap.close_lag,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("rocr_summary")
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
                                ui.label(egui::RichText::new("ROCR").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.rocr))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ROCR prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.rocr_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Close").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.close_now))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Close (lag)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.close_lag))
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
            self.show_rocr_win = open;
        }

        if self.show_rocr100_win {
            if self.rocr100_win_symbol.is_empty() {
                self.rocr100_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_rocr100_win;
            egui::Window::new("ROCR100 — Rate of Change Ratio × 100 (period 10)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.rocr100_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.rocr100_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.rocr100_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_rocr100(&conn, &sym_u)
                                    {
                                        self.rocr100_win_snapshot = snap;
                                        self.rocr100_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.rocr100_win_symbol.to_uppercase();
                            self.rocr100_win_loading = true;
                            self.rocr100_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRocr100Snapshot { symbol: sym });
                        }
                        if self.rocr100_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.rocr100_win_snapshot;
                    if snap.symbol.is_empty() || snap.rocr100_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥12 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.rocr100_label.as_str() {
                            "STRONG_UP" | "UP" => UP,
                            "STRONG_DOWN" | "DOWN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — ROCR100 {:.4} — close {:.4} — lag {:.4} — as of {}",
                                snap.symbol,
                                snap.rocr100_label,
                                snap.rocr100,
                                snap.close_now,
                                snap.close_lag,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("rocr100_summary")
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
                                ui.label(egui::RichText::new("ROCR100").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.rocr100))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ROCR100 prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.rocr100_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Close").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.close_now))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Close (lag)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.close_lag))
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
            self.show_rocr100_win = open;
        }
    }
}
