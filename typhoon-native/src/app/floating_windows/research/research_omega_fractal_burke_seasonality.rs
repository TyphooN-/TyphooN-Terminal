use super::*;

impl TyphooNApp {
    pub(super) fn render_research_omega_fractal_burke_seasonality_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
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
                    super::render::render_omega_snapshot(ui, &self.omega_snapshot);
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
                    super::render::render_dfa_snapshot(ui, &self.dfa_snapshot);
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
                    super::render::render_burke_snapshot(ui, &self.burke_snapshot);
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
                    super::render::render_monthseas_snapshot(ui, &self.monthseas_snapshot);
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
                        ui.add(
                            egui::TextEdit::singleline(&mut self.rollsprd_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.rollsprd_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.rollsprd_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_rollsprd(&conn, &sym_u)
                                    {
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
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRollsprdSnapshot { symbol: sym });
                        }
                        if self.rollsprd_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_rollsprd_snapshot(ui, &self.rollsprd_snapshot);
                });
            self.show_rollsprd = open;
        }
    }
}
