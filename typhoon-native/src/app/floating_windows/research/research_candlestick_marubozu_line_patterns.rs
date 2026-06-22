use super::*;

impl TyphooNApp {
    pub(super) fn render_research_candlestick_marubozu_line_patterns_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── popup windows ──
        if self.show_cdl_belt_hold_win {
            if self.cdl_belt_hold_win_symbol.is_empty() {
                self.cdl_belt_hold_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_belt_hold_win;
            egui::Window::new("CDLBELTHOLD — Belt Hold")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cdl_belt_hold_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.cdl_belt_hold_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.cdl_belt_hold_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_cdl_belt_hold(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.cdl_belt_hold_win_snapshot = snap;
                                        self.cdl_belt_hold_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_belt_hold_win_symbol.to_uppercase();
                            self.cdl_belt_hold_win_loading = true;
                            self.cdl_belt_hold_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCdlBeltHoldSnapshot { symbol: sym });
                        }
                        if self.cdl_belt_hold_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_cdl_belt_hold_snapshot(
                        ui,
                        &self.cdl_belt_hold_win_snapshot,
                    );
                });
            self.show_cdl_belt_hold_win = open;
        }

        if self.show_cdl_closing_marubozu_win {
            if self.cdl_closing_marubozu_win_symbol.is_empty() {
                self.cdl_closing_marubozu_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_closing_marubozu_win;
            egui::Window::new("CDLCLOSINGMARUBOZU — Closing Marubozu")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cdl_closing_marubozu_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.cdl_closing_marubozu_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.cdl_closing_marubozu_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_cdl_closing_marubozu(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.cdl_closing_marubozu_win_snapshot = snap;
                                        self.cdl_closing_marubozu_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_closing_marubozu_win_symbol.to_uppercase();
                            self.cdl_closing_marubozu_win_loading = true;
                            self.cdl_closing_marubozu_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCdlClosingMarubozuSnapshot { symbol: sym });
                        }
                        if self.cdl_closing_marubozu_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_cdl_closing_marubozu_snapshot(
                        ui,
                        &self.cdl_closing_marubozu_win_snapshot,
                    );
                });
            self.show_cdl_closing_marubozu_win = open;
        }

        if self.show_cdl_high_wave_win {
            if self.cdl_high_wave_win_symbol.is_empty() {
                self.cdl_high_wave_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_high_wave_win;
            egui::Window::new("CDLHIGHWAVE — High Wave")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cdl_high_wave_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.cdl_high_wave_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.cdl_high_wave_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_cdl_high_wave(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.cdl_high_wave_win_snapshot = snap;
                                        self.cdl_high_wave_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_high_wave_win_symbol.to_uppercase();
                            self.cdl_high_wave_win_loading = true;
                            self.cdl_high_wave_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCdlHighWaveSnapshot { symbol: sym });
                        }
                        if self.cdl_high_wave_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_cdl_high_wave_snapshot(
                        ui,
                        &self.cdl_high_wave_win_snapshot,
                    );
                });
            self.show_cdl_high_wave_win = open;
        }

        if self.show_cdl_long_line_win {
            if self.cdl_long_line_win_symbol.is_empty() {
                self.cdl_long_line_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_long_line_win;
            egui::Window::new("CDLLONGLINE — Long Line")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cdl_long_line_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.cdl_long_line_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.cdl_long_line_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_cdl_long_line(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.cdl_long_line_win_snapshot = snap;
                                        self.cdl_long_line_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_long_line_win_symbol.to_uppercase();
                            self.cdl_long_line_win_loading = true;
                            self.cdl_long_line_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCdlLongLineSnapshot { symbol: sym });
                        }
                        if self.cdl_long_line_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_cdl_long_line_snapshot(
                        ui,
                        &self.cdl_long_line_win_snapshot,
                    );
                });
            self.show_cdl_long_line_win = open;
        }

        if self.show_cdl_short_line_win {
            if self.cdl_short_line_win_symbol.is_empty() {
                self.cdl_short_line_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_short_line_win;
            egui::Window::new("CDLSHORTLINE — Short Line")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cdl_short_line_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.cdl_short_line_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.cdl_short_line_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_cdl_short_line(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.cdl_short_line_win_snapshot = snap;
                                        self.cdl_short_line_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_short_line_win_symbol.to_uppercase();
                            self.cdl_short_line_win_loading = true;
                            self.cdl_short_line_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCdlShortLineSnapshot { symbol: sym });
                        }
                        if self.cdl_short_line_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_cdl_short_line_snapshot(
                        ui,
                        &self.cdl_short_line_win_snapshot,
                    );
                });
            self.show_cdl_short_line_win = open;
        }
    }
}
