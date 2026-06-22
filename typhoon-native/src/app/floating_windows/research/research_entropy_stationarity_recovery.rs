use super::*;

impl TyphooNApp {
    pub(super) fn render_research_entropy_stationarity_recovery_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if self.show_sampen {
            if self.sampen_symbol.is_empty() {
                self.sampen_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_sampen;
            egui::Window::new("SAMPEN — Sample Entropy")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.sampen_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.sampen_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.sampen_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_sampen(&conn, &sym_u)
                                    {
                                        self.sampen_snapshot = snap;
                                        self.sampen_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.sampen_symbol.to_uppercase();
                            self.sampen_loading = true;
                            self.sampen_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeSampenSnapshot { symbol: sym });
                        }
                        if self.sampen_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_sampen_snapshot(ui, &self.sampen_snapshot);
                });
            self.show_sampen = open;
        }

        if self.show_permen {
            if self.permen_symbol.is_empty() {
                self.permen_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_permen;
            egui::Window::new("PERMEN — Permutation Entropy")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.permen_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.permen_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.permen_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_permen(&conn, &sym_u)
                                    {
                                        self.permen_snapshot = snap;
                                        self.permen_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.permen_symbol.to_uppercase();
                            self.permen_loading = true;
                            self.permen_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputePermenSnapshot { symbol: sym });
                        }
                        if self.permen_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_permen_snapshot(ui, &self.permen_snapshot);
                });
            self.show_permen = open;
        }

        if self.show_recfact {
            if self.recfact_symbol.is_empty() {
                self.recfact_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_recfact;
            egui::Window::new("RECFACT — Recovery Factor")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.recfact_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.recfact_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.recfact_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_recfact(&conn, &sym_u)
                                    {
                                        self.recfact_snapshot = snap;
                                        self.recfact_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.recfact_symbol.to_uppercase();
                            self.recfact_loading = true;
                            self.recfact_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRecfactSnapshot { symbol: sym });
                        }
                        if self.recfact_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_recfact_snapshot(ui, &self.recfact_snapshot);
                });
            self.show_recfact = open;
        }

        if self.show_kpss {
            if self.kpss_symbol.is_empty() {
                self.kpss_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_kpss;
            egui::Window::new("KPSS — Stationarity Test")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.kpss_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.kpss_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.kpss_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_kpss(&conn, &sym_u)
                                    {
                                        self.kpss_snapshot = snap;
                                        self.kpss_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.kpss_symbol.to_uppercase();
                            self.kpss_loading = true;
                            self.kpss_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeKpssSnapshot { symbol: sym });
                        }
                        if self.kpss_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_kpss_snapshot(ui, &self.kpss_snapshot);
                });
            self.show_kpss = open;
        }

        if self.show_specent {
            if self.specent_symbol.is_empty() {
                self.specent_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_specent;
            egui::Window::new("SPECENT — Spectral Entropy")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.specent_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.specent_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.specent_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_specent(&conn, &sym_u)
                                    {
                                        self.specent_snapshot = snap;
                                        self.specent_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.specent_symbol.to_uppercase();
                            self.specent_loading = true;
                            self.specent_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeSpecentSnapshot { symbol: sym });
                        }
                        if self.specent_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_specent_snapshot(ui, &self.specent_snapshot);
                });
            self.show_specent = open;
        }
    }
}
