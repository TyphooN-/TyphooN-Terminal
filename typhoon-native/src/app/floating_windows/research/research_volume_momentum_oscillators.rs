use super::*;

impl TyphooNApp {
    pub(super) fn render_research_volume_momentum_oscillators_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if self.show_mass_win {
            if self.mass_win_symbol.is_empty() {
                self.mass_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_mass_win;
            egui::Window::new("MASS — Mass Index (Dorsey, 1992)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.mass_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.mass_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.mass_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_mass(&conn, &sym_u)
                                    {
                                        self.mass_win_snapshot = snap;
                                        self.mass_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.mass_win_symbol.to_uppercase();
                            self.mass_win_loading = true;
                            self.mass_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMassSnapshot { symbol: sym });
                        }
                        if self.mass_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_mass_snapshot(ui, &self.mass_win_snapshot);
                });
            self.show_mass_win = open;
        }

        if self.show_chaikosc_win {
            if self.chaikosc_win_symbol.is_empty() {
                self.chaikosc_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_chaikosc_win;
            egui::Window::new("CHAIKOSC — Chaikin Oscillator (3/10)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.chaikosc_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.chaikosc_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.chaikosc_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_chaikosc(&conn, &sym_u)
                                    {
                                        self.chaikosc_win_snapshot = snap;
                                        self.chaikosc_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.chaikosc_win_symbol.to_uppercase();
                            self.chaikosc_win_loading = true;
                            self.chaikosc_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeChaikoscSnapshot { symbol: sym });
                        }
                        if self.chaikosc_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_chaikosc_snapshot(ui, &self.chaikosc_win_snapshot);
                });
            self.show_chaikosc_win = open;
        }

        if self.show_klinger_win {
            if self.klinger_win_symbol.is_empty() {
                self.klinger_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_klinger_win;
            egui::Window::new("KLINGER — Klinger Volume Oscillator (34/55/13)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.klinger_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.klinger_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.klinger_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_klinger(&conn, &sym_u)
                                    {
                                        self.klinger_win_snapshot = snap;
                                        self.klinger_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.klinger_win_symbol.to_uppercase();
                            self.klinger_win_loading = true;
                            self.klinger_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeKlingerSnapshot { symbol: sym });
                        }
                        if self.klinger_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_klinger_snapshot(ui, &self.klinger_win_snapshot);
                });
            self.show_klinger_win = open;
        }

        if self.show_stochrsi_win {
            if self.stochrsi_win_symbol.is_empty() {
                self.stochrsi_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_stochrsi_win;
            egui::Window::new("STOCHRSI — Stochastic RSI (14/14/3/3)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.stochrsi_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.stochrsi_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.stochrsi_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_stochrsi(&conn, &sym_u)
                                    {
                                        self.stochrsi_win_snapshot = snap;
                                        self.stochrsi_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.stochrsi_win_symbol.to_uppercase();
                            self.stochrsi_win_loading = true;
                            self.stochrsi_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeStochRsiSnapshot { symbol: sym });
                        }
                        if self.stochrsi_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_stochrsi_snapshot(ui, &self.stochrsi_win_snapshot);
                });
            self.show_stochrsi_win = open;
        }
    }
}
