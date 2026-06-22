use super::*;

impl TyphooNApp {
    pub(super) fn render_research_residual_iid_heteroskedastic_cycles_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if self.show_durbinwatson {
            if self.durbinwatson_symbol.is_empty() {
                self.durbinwatson_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_durbinwatson;
            egui::Window::new("DURBINWATSON — Durbin-Watson Residual Autocorrelation")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.durbinwatson_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.durbinwatson_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.durbinwatson_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_durbinwatson(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.durbinwatson_snapshot = snap;
                                        self.durbinwatson_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.durbinwatson_symbol.to_uppercase();
                            self.durbinwatson_loading = true;
                            self.durbinwatson_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeDurbinWatsonSnapshot { symbol: sym });
                        }
                        if self.durbinwatson_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_durbinwatson_snapshot(ui, &self.durbinwatson_snapshot);
                });
            self.show_durbinwatson = open;
        }

        if self.show_bdstest {
            if self.bdstest_symbol.is_empty() {
                self.bdstest_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_bdstest;
            egui::Window::new("BDSTEST — Brock-Dechert-Scheinkman iid Test")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.bdstest_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.bdstest_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.bdstest_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_bdstest(&conn, &sym_u)
                                    {
                                        self.bdstest_snapshot = snap;
                                        self.bdstest_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.bdstest_symbol.to_uppercase();
                            self.bdstest_loading = true;
                            self.bdstest_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeBdsTestSnapshot { symbol: sym });
                        }
                        if self.bdstest_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_bdstest_snapshot(ui, &self.bdstest_snapshot);
                });
            self.show_bdstest = open;
        }

        if self.show_breuschpagan {
            if self.breuschpagan_symbol.is_empty() {
                self.breuschpagan_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_breuschpagan;
            egui::Window::new("BREUSCHPAGAN — Breusch-Pagan Heteroskedasticity LM Test")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.breuschpagan_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.breuschpagan_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.breuschpagan_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_breuschpagan(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.breuschpagan_snapshot = snap;
                                        self.breuschpagan_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.breuschpagan_symbol.to_uppercase();
                            self.breuschpagan_loading = true;
                            self.breuschpagan_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeBreuschPaganSnapshot { symbol: sym });
                        }
                        if self.breuschpagan_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_breuschpagan_snapshot(ui, &self.breuschpagan_snapshot);
                });
            self.show_breuschpagan = open;
        }

        if self.show_turnpts {
            if self.turnpts_symbol.is_empty() {
                self.turnpts_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_turnpts;
            egui::Window::new("TURNPTS — Bartels Turning-Points Test")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.turnpts_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.turnpts_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.turnpts_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_turnpts(&conn, &sym_u)
                                    {
                                        self.turnpts_snapshot = snap;
                                        self.turnpts_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.turnpts_symbol.to_uppercase();
                            self.turnpts_loading = true;
                            self.turnpts_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeTurnPtsSnapshot { symbol: sym });
                        }
                        if self.turnpts_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_turnpts_snapshot(ui, &self.turnpts_snapshot);
                });
            self.show_turnpts = open;
        }

        if self.show_periodogram {
            if self.periodogram_symbol.is_empty() {
                self.periodogram_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_periodogram;
            egui::Window::new("PERIODOGRAM — Direct-DFT Dominant-Cycle Detection")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.periodogram_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.periodogram_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.periodogram_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_periodogram(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.periodogram_snapshot = snap;
                                        self.periodogram_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.periodogram_symbol.to_uppercase();
                            self.periodogram_loading = true;
                            self.periodogram_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputePeriodogramSnapshot { symbol: sym });
                        }
                        if self.periodogram_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_periodogram_snapshot(ui, &self.periodogram_snapshot);
                });
            self.show_periodogram = open;
        }
    }
}
