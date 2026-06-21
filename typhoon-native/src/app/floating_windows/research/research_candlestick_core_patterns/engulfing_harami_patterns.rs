use super::*;

impl TyphooNApp {
    pub(super) fn render_cdl_engulfing_harami_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
        if self.show_cdl_engulfing_win {
            if self.cdl_engulfing_win_symbol.is_empty() {
                self.cdl_engulfing_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_engulfing_win;
            egui::Window::new("CDLENGULFING — Engulfing pattern (2-bar reversal)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cdl_engulfing_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.cdl_engulfing_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.cdl_engulfing_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_cdl_engulfing(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.cdl_engulfing_win_snapshot = snap;
                                        self.cdl_engulfing_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_engulfing_win_symbol.to_uppercase();
                            self.cdl_engulfing_win_loading = true;
                            self.cdl_engulfing_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCdlEngulfingSnapshot { symbol: sym });
                        }
                        if self.cdl_engulfing_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.cdl_engulfing_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_engulfing_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥3 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.cdl_engulfing_label.as_str() {
                            "BULLISH_PATTERN" => UP,
                            "BEARISH_PATTERN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — value {} — ratio {:.2}× — close {:.4} — as of {}",
                                snap.symbol,
                                snap.cdl_engulfing_label,
                                snap.pattern_value,
                                snap.body_size_ratio,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("cdl_engulfing_summary")
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
                                ui.label(egui::RichText::new("Pattern value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.pattern_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Prev value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.pattern_value_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Body size ratio (cur/prev)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.body_size_ratio))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Prior body %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2}",
                                        snap.prior_body_pct_range
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Current body %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2}",
                                        snap.current_body_pct_range
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Last bar match").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.last_bar_match))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Days since pattern").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.days_since_pattern))
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
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
                        }
                    }
                });
            self.show_cdl_engulfing_win = open;
        }

        if self.show_cdl_harami_win {
            if self.cdl_harami_win_symbol.is_empty() {
                self.cdl_harami_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_harami_win;
            egui::Window::new("CDLHARAMI — Harami / inside-bar pattern")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cdl_harami_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.cdl_harami_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.cdl_harami_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_cdl_harami(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.cdl_harami_win_snapshot = snap;
                                        self.cdl_harami_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_harami_win_symbol.to_uppercase();
                            self.cdl_harami_win_loading = true;
                            self.cdl_harami_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCdlHaramiSnapshot { symbol: sym });
                        }
                        if self.cdl_harami_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.cdl_harami_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_harami_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥3 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.cdl_harami_label.as_str() {
                            "BULLISH_PATTERN" => UP,
                            "BEARISH_PATTERN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — value {} — ratio {:.2}× — close {:.4} — as of {}",
                                snap.symbol,
                                snap.cdl_harami_label,
                                snap.pattern_value,
                                snap.body_size_ratio,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("cdl_harami_summary")
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
                                ui.label(egui::RichText::new("Pattern value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.pattern_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Prev value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.pattern_value_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Body size ratio (cur/prev)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.body_size_ratio))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Prior body %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2}",
                                        snap.prior_body_pct_range
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Current body %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2}",
                                        snap.current_body_pct_range
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Last bar match").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.last_bar_match))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Days since pattern").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.days_since_pattern))
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
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
                        }
                    }
                });
            self.show_cdl_harami_win = open;
        }
    }
}
