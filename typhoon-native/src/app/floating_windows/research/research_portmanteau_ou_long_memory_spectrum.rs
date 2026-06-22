use super::*;

impl TyphooNApp {
    pub(super) fn render_research_portmanteau_ou_long_memory_spectrum_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if self.show_mcleodli {
            if self.mcleodli_symbol.is_empty() {
                self.mcleodli_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_mcleodli;
            egui::Window::new("MCLEODLI — McLeod-Li Squared-Returns Portmanteau")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.mcleodli_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.mcleodli_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.mcleodli_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_mcleodli(&conn, &sym_u)
                                    {
                                        self.mcleodli_snapshot = snap;
                                        self.mcleodli_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.mcleodli_symbol.to_uppercase();
                            self.mcleodli_loading = true;
                            self.mcleodli_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMcLeodLiSnapshot { symbol: sym });
                        }
                        if self.mcleodli_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_mcleodli_snapshot(ui, &self.mcleodli_snapshot);
                });
            self.show_mcleodli = open;
        }

        if self.show_oufit {
            if self.oufit_symbol.is_empty() {
                self.oufit_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_oufit;
            egui::Window::new("OUFIT — Ornstein-Uhlenbeck Mean-Reversion Fit")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 340.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.oufit_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.oufit_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.oufit_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_oufit(&conn, &sym_u)
                                    {
                                        self.oufit_snapshot = snap;
                                        self.oufit_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.oufit_symbol.to_uppercase();
                            self.oufit_loading = true;
                            self.oufit_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeOuFitSnapshot { symbol: sym });
                        }
                        if self.oufit_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_oufit_snapshot(ui, &self.oufit_snapshot);
                });
            self.show_oufit = open;
        }

        if self.show_gph {
            if self.gph_symbol.is_empty() {
                self.gph_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_gph;
            egui::Window::new("GPH — Geweke-Porter-Hudak Long-Memory d̂")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.gph_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.gph_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.gph_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_gph(&conn, &sym_u)
                                    {
                                        self.gph_snapshot = snap;
                                        self.gph_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.gph_symbol.to_uppercase();
                            self.gph_loading = true;
                            self.gph_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeGphSnapshot { symbol: sym });
                        }
                        if self.gph_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_gph_snapshot(ui, &self.gph_snapshot);
                });
            self.show_gph = open;
        }

        if self.show_burgspec {
            if self.burgspec_symbol.is_empty() {
                self.burgspec_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_burgspec;
            egui::Window::new("BURGSPEC — Burg Maximum-Entropy AR Spectrum")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 340.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.burgspec_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.burgspec_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.burgspec_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_burgspec(&conn, &sym_u)
                                    {
                                        self.burgspec_snapshot = snap;
                                        self.burgspec_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.burgspec_symbol.to_uppercase();
                            self.burgspec_loading = true;
                            self.burgspec_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeBurgSpecSnapshot { symbol: sym });
                        }
                        if self.burgspec_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_burgspec_snapshot(ui, &self.burgspec_snapshot);
                });
            self.show_burgspec = open;
        }

        if self.show_kendalltau {
            if self.kendalltau_symbol.is_empty() {
                self.kendalltau_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_kendalltau;
            egui::Window::new("KENDALLTAU — Kendall's Tau Lag-1 Rank Autocorrelation")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.kendalltau_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.kendalltau_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.kendalltau_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_kendalltau(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.kendalltau_snapshot = snap;
                                        self.kendalltau_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.kendalltau_symbol.to_uppercase();
                            self.kendalltau_loading = true;
                            self.kendalltau_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeKendallTauSnapshot { symbol: sym });
                        }
                        if self.kendalltau_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_kendalltau_snapshot(ui, &self.kendalltau_snapshot);
                });
            self.show_kendalltau = open;
        }
    }
}
