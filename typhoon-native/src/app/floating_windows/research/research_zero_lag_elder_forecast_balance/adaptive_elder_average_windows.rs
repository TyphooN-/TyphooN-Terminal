use super::*;

impl TyphooNApp {
    pub(super) fn render_adaptive_elder_average_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
        // ── Research section ──
        if self.show_alma_win {
            if self.alma_win_symbol.is_empty() {
                self.alma_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_alma_win;
            egui::Window::new(
                "ALMA — Arnaud Legoux Moving Average (Gaussian, length 20, σ=6, offset 0.85)",
            )
            .open(&mut open)
            .resizable(true)
            .default_size([580.0, 280.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.alma_win_symbol).desired_width(100.0),
                    );
                    if ui.button("Use Chart").clicked() {
                        self.alma_win_symbol = chart_sym_research.clone();
                    }
                    if ui.button("Load Cached").clicked() {
                        if let Some(ref cache) = self.cache {
                            if let Ok(conn) = cache.connection() {
                                let sym_u = self.alma_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) =
                                    typhoon_engine::core::research::get_alma(&conn, &sym_u)
                                {
                                    self.alma_win_snapshot = snap;
                                    self.alma_win_symbol = sym_u;
                                }
                            }
                        }
                    }
                    if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                        let sym = self.alma_win_symbol.to_uppercase();
                        self.alma_win_loading = true;
                        self.alma_win_symbol = sym.clone();
                        let _ = self
                            .broker_tx
                            .send(BrokerCmd::ComputeAlmaSnapshot { symbol: sym });
                    }
                    if self.alma_win_loading {
                        ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                    }
                });
                ui.separator();
                let snap = &self.alma_win_snapshot;
                if snap.symbol.is_empty() || snap.alma_label == "INSUFFICIENT_DATA" {
                    ui.label(
                        egui::RichText::new("No data — HP cache needs ≥21 bars with OHLC.")
                            .color(AXIS_TEXT)
                            .small(),
                    );
                } else {
                    let color = match snap.alma_label.as_str() {
                        "STRONG_BULL" | "BULL" => UP,
                        "STRONG_BEAR" | "BEAR" => DOWN,
                        _ => AXIS_TEXT,
                    };
                    ui.label(
                        egui::RichText::new(format!(
                            "{} — {} — ALMA {:.4} — close {:.4} — dev {:+.2}% — as of {}",
                            snap.symbol,
                            snap.alma_label,
                            snap.alma_value,
                            snap.last_close,
                            snap.deviation_pct,
                            snap.as_of
                        ))
                        .strong()
                        .color(color),
                    );
                    ui.separator();
                    egui::Grid::new("alma_summary")
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
                            ui.label(egui::RichText::new("Offset").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:.3}", snap.offset))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("Sigma").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:.2}", snap.sigma))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("ALMA").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:.4}", snap.alma_value))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("ALMA prev").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:.4}", snap.alma_prev))
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
            self.show_alma_win = open;
        }

        if self.show_zlema_win {
            if self.zlema_win_symbol.is_empty() {
                self.zlema_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_zlema_win;
            egui::Window::new("ZLEMA — Zero-Lag Exponential Moving Average (length 20, lag 9)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 270.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.zlema_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.zlema_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.zlema_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_zlema(&conn, &sym_u)
                                    {
                                        self.zlema_win_snapshot = snap;
                                        self.zlema_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.zlema_win_symbol.to_uppercase();
                            self.zlema_win_loading = true;
                            self.zlema_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeZlemaSnapshot { symbol: sym });
                        }
                        if self.zlema_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.zlema_win_snapshot;
                    if snap.symbol.is_empty() || snap.zlema_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥31 bars with OHLC.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.zlema_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — ZLEMA {:.4} — close {:.4} — dev {:+.2}% — as of {}",
                                snap.symbol,
                                snap.zlema_label,
                                snap.zlema_value,
                                snap.last_close,
                                snap.deviation_pct,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("zlema_summary")
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
                                ui.label(egui::RichText::new("Lag shift").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.lag_shift))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ZLEMA").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.zlema_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ZLEMA prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.zlema_prev))
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
            self.show_zlema_win = open;
        }

        if self.show_elderray_win {
            if self.elderray_win_symbol.is_empty() {
                self.elderray_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_elderray_win;
            egui::Window::new("ELDERRAY — Elder Bull/Bear Power (EMA13)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 290.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.elderray_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.elderray_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.elderray_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_elderray(&conn, &sym_u)
                                    {
                                        self.elderray_win_snapshot = snap;
                                        self.elderray_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.elderray_win_symbol.to_uppercase();
                            self.elderray_win_loading = true;
                            self.elderray_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeElderRaySnapshot { symbol: sym });
                        }
                        if self.elderray_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.elderray_win_snapshot;
                    if snap.symbol.is_empty() || snap.elder_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥15 bars with OHLC.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.elder_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — Bull {:+.4} — Bear {:+.4} — EMA13 {:.4} — as of {}",
                                snap.symbol,
                                snap.elder_label,
                                snap.bull_power,
                                snap.bear_power,
                                snap.ema13,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("elderray_summary")
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
                                ui.label(egui::RichText::new("EMA length").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.ema_length))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("EMA13").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.ema13))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("EMA13 prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.ema13_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Bull Power (H − EMA13)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.bull_power))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Bull Power prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.bull_power_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Bear Power (L − EMA13)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.bear_power))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Bear Power prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.bear_power_prev))
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
            self.show_elderray_win = open;
        }
    }
}
