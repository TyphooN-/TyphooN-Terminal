use super::*;

impl TyphooNApp {
    pub(super) fn render_research_ichimoku_supertrend_channels_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if self.show_ichimoku_win {
            if self.ichimoku_win_symbol.is_empty() {
                self.ichimoku_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ichimoku_win;
            egui::Window::new("ICHIMOKU — Kinko Hyo Cloud")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 340.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ichimoku_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ichimoku_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ichimoku_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ichimoku(&conn, &sym_u)
                                    {
                                        self.ichimoku_win_snapshot = snap;
                                        self.ichimoku_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ichimoku_win_symbol.to_uppercase();
                            self.ichimoku_win_loading = true;
                            self.ichimoku_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeIchimokuSnapshot { symbol: sym });
                        }
                        if self.ichimoku_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_ichimoku_snapshot(ui, &self.ichimoku_win_snapshot);
                });
            self.show_ichimoku_win = open;
        }

        if self.show_supertrend_win {
            if self.supertrend_win_symbol.is_empty() {
                self.supertrend_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_supertrend_win;
            egui::Window::new("SUPERTREND — ATR Trailing Stop")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.supertrend_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.supertrend_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.supertrend_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_supertrend(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.supertrend_win_snapshot = snap;
                                        self.supertrend_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.supertrend_win_symbol.to_uppercase();
                            self.supertrend_win_loading = true;
                            self.supertrend_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeSupertrendSnapshot { symbol: sym });
                        }
                        if self.supertrend_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_supertrend_snapshot(ui, &self.supertrend_win_snapshot);
                });
            self.show_supertrend_win = open;
        }

        if self.show_keltner_win {
            if self.keltner_win_symbol.is_empty() {
                self.keltner_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_keltner_win;
            egui::Window::new("KELTNER — Channels (EMA 20 ± 2·ATR 10)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.keltner_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.keltner_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.keltner_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_keltner(&conn, &sym_u)
                                    {
                                        self.keltner_win_snapshot = snap;
                                        self.keltner_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.keltner_win_symbol.to_uppercase();
                            self.keltner_win_loading = true;
                            self.keltner_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeKeltnerSnapshot { symbol: sym });
                        }
                        if self.keltner_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_keltner_snapshot(ui, &self.keltner_win_snapshot);
                });
            self.show_keltner_win = open;
        }

        if self.show_fisher_win {
            if self.fisher_win_symbol.is_empty() {
                self.fisher_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_fisher_win;
            egui::Window::new("FISHER — Ehlers Fisher Transform")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.fisher_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.fisher_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.fisher_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_fisher(&conn, &sym_u)
                                    {
                                        self.fisher_win_snapshot = snap;
                                        self.fisher_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.fisher_win_symbol.to_uppercase();
                            self.fisher_win_loading = true;
                            self.fisher_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeFisherSnapshot { symbol: sym });
                        }
                        if self.fisher_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_fisher_snapshot(ui, &self.fisher_win_snapshot);
                });
            self.show_fisher_win = open;
        }

        if self.show_aroon_win {
            if self.aroon_win_symbol.is_empty() {
                self.aroon_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_aroon_win;
            egui::Window::new("AROON — Up / Down / Oscillator (25)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.aroon_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.aroon_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.aroon_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_aroon(&conn, &sym_u)
                                    {
                                        self.aroon_win_snapshot = snap;
                                        self.aroon_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.aroon_win_symbol.to_uppercase();
                            self.aroon_win_loading = true;
                            self.aroon_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeAroonSnapshot { symbol: sym });
                        }
                        if self.aroon_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_aroon_snapshot(ui, &self.aroon_win_snapshot);
                });
            self.show_aroon_win = open;
        }
    }
}
