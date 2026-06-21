use super::*;

impl TyphooNApp {
    pub(super) fn render_cdl_cloud_piercing_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
        if self.show_cdl_dark_cloud_cover_win {
            if self.cdl_dark_cloud_cover_win_symbol.is_empty() {
                self.cdl_dark_cloud_cover_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_dark_cloud_cover_win;
            egui::Window::new("CDLDARKCLOUDCOVER — Dark Cloud Cover (2-bar bearish reversal)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cdl_dark_cloud_cover_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.cdl_dark_cloud_cover_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.cdl_dark_cloud_cover_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_cdl_dark_cloud_cover(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.cdl_dark_cloud_cover_win_snapshot = snap;
                                        self.cdl_dark_cloud_cover_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_dark_cloud_cover_win_symbol.to_uppercase();
                            self.cdl_dark_cloud_cover_win_loading = true;
                            self.cdl_dark_cloud_cover_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCdlDarkCloudCoverSnapshot { symbol: sym });
                        }
                        if self.cdl_dark_cloud_cover_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.cdl_dark_cloud_cover_win_snapshot;
                    if snap.symbol.is_empty()
                        || snap.cdl_dark_cloud_cover_label == "INSUFFICIENT_DATA"
                    {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥3 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.cdl_dark_cloud_cover_label.as_str() {
                            "BEARISH_PATTERN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — value {} — penetration {:.2}% — close {:.4} — as of {}",
                                snap.symbol,
                                snap.cdl_dark_cloud_cover_label,
                                snap.pattern_value,
                                snap.penetration_pct,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("cdl_dark_cloud_cover_summary")
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
                                ui.label(egui::RichText::new("Penetration %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.penetration_pct))
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
            self.show_cdl_dark_cloud_cover_win = open;
        }

        if self.show_cdl_piercing_win {
            if self.cdl_piercing_win_symbol.is_empty() {
                self.cdl_piercing_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_piercing_win;
            egui::Window::new("CDLPIERCING — Piercing Line (2-bar bullish reversal)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cdl_piercing_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.cdl_piercing_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.cdl_piercing_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_cdl_piercing(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.cdl_piercing_win_snapshot = snap;
                                        self.cdl_piercing_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_piercing_win_symbol.to_uppercase();
                            self.cdl_piercing_win_loading = true;
                            self.cdl_piercing_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCdlPiercingSnapshot { symbol: sym });
                        }
                        if self.cdl_piercing_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.cdl_piercing_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_piercing_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥3 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.cdl_piercing_label.as_str() {
                            "BULLISH_PATTERN" => UP,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — value {} — penetration {:.2}% — close {:.4} — as of {}",
                                snap.symbol,
                                snap.cdl_piercing_label,
                                snap.pattern_value,
                                snap.penetration_pct,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("cdl_piercing_summary")
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
                                ui.label(egui::RichText::new("Penetration %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.penetration_pct))
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
            self.show_cdl_piercing_win = open;
        }
    }
}
