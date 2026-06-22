use super::*;

impl TyphooNApp {
    pub(super) fn render_research_jump_unitroot_multifractal_tsi_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if self.show_bnsjump {
            if self.bnsjump_symbol.is_empty() {
                self.bnsjump_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_bnsjump;
            egui::Window::new("BNSJUMP — Barndorff-Nielsen-Shephard Jump-Test Z")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.bnsjump_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.bnsjump_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.bnsjump_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_bnsjump(&conn, &sym_u)
                                    {
                                        self.bnsjump_snapshot = snap;
                                        self.bnsjump_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.bnsjump_symbol.to_uppercase();
                            self.bnsjump_loading = true;
                            self.bnsjump_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeBnsjumpSnapshot { symbol: sym });
                        }
                        if self.bnsjump_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_bnsjump_snapshot(ui, &self.bnsjump_snapshot);
                });
            self.show_bnsjump = open;
        }

        if self.show_pproot {
            if self.pproot_symbol.is_empty() {
                self.pproot_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_pproot;
            egui::Window::new("PPROOT — Phillips-Perron Unit-Root Test")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.pproot_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.pproot_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.pproot_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_pproot(&conn, &sym_u)
                                    {
                                        self.pproot_snapshot = snap;
                                        self.pproot_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.pproot_symbol.to_uppercase();
                            self.pproot_loading = true;
                            self.pproot_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputePprootSnapshot { symbol: sym });
                        }
                        if self.pproot_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_pproot_snapshot(ui, &self.pproot_snapshot);
                });
            self.show_pproot = open;
        }

        if self.show_mfdfa {
            if self.mfdfa_symbol.is_empty() {
                self.mfdfa_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_mfdfa;
            egui::Window::new("MFDFA — Multifractal DFA (q ∈ {-2, 0, +2})")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.mfdfa_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.mfdfa_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.mfdfa_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_mfdfa(&conn, &sym_u)
                                    {
                                        self.mfdfa_snapshot = snap;
                                        self.mfdfa_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.mfdfa_symbol.to_uppercase();
                            self.mfdfa_loading = true;
                            self.mfdfa_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMfdfaSnapshot { symbol: sym });
                        }
                        if self.mfdfa_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_mfdfa_snapshot(ui, &self.mfdfa_snapshot);
                });
            self.show_mfdfa = open;
        }

        if self.show_hillks {
            if self.hillks_symbol.is_empty() {
                self.hillks_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_hillks;
            egui::Window::new("HILLKS — Hill-Tail KS Goodness-of-Fit")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.hillks_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.hillks_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.hillks_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_hillks(&conn, &sym_u)
                                    {
                                        self.hillks_snapshot = snap;
                                        self.hillks_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.hillks_symbol.to_uppercase();
                            self.hillks_loading = true;
                            self.hillks_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeHillksSnapshot { symbol: sym });
                        }
                        if self.hillks_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_hillks_snapshot(ui, &self.hillks_snapshot);
                });
            self.show_hillks = open;
        }

        if self.show_tsi {
            if self.tsi_symbol.is_empty() {
                self.tsi_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_tsi;
            egui::Window::new("TSI — True Strength Index (Blau 1991)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.tsi_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.tsi_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.tsi_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_tsi(&conn, &sym_u)
                                    {
                                        self.tsi_snapshot = snap;
                                        self.tsi_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.tsi_symbol.to_uppercase();
                            self.tsi_loading = true;
                            self.tsi_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeTsiSnapshot { symbol: sym });
                        }
                        if self.tsi_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_tsi_snapshot(ui, &self.tsi_snapshot);
                });
            self.show_tsi = open;
        }
    }
}
