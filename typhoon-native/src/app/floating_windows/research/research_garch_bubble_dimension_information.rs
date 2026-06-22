use super::*;

impl TyphooNApp {
    pub(super) fn render_research_garch_bubble_dimension_information_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if self.show_garch11 {
            if self.garch11_symbol.is_empty() {
                self.garch11_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_garch11;
            egui::Window::new("GARCH11 — GARCH(1,1) Conditional Volatility Fit")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 340.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.garch11_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.garch11_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.garch11_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_garch11(&conn, &sym_u)
                                    {
                                        self.garch11_snapshot = snap;
                                        self.garch11_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.garch11_symbol.to_uppercase();
                            self.garch11_loading = true;
                            self.garch11_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeGarch11Snapshot { symbol: sym });
                        }
                        if self.garch11_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_garch11_snapshot(ui, &self.garch11_snapshot);
                });
            self.show_garch11 = open;
        }

        if self.show_sadf {
            if self.sadf_symbol.is_empty() {
                self.sadf_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_sadf;
            egui::Window::new("SADF — Phillips-Wu-Yu Sup-ADF Bubble Test")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.sadf_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.sadf_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.sadf_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_sadf(&conn, &sym_u)
                                    {
                                        self.sadf_snapshot = snap;
                                        self.sadf_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.sadf_symbol.to_uppercase();
                            self.sadf_loading = true;
                            self.sadf_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeSadfSnapshot { symbol: sym });
                        }
                        if self.sadf_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_sadf_snapshot(ui, &self.sadf_snapshot);
                });
            self.show_sadf = open;
        }

        if self.show_cordim {
            if self.cordim_symbol.is_empty() {
                self.cordim_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cordim;
            egui::Window::new("CORDIM — Grassberger-Procaccia Correlation Dimension D2")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cordim_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.cordim_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.cordim_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_cordim(&conn, &sym_u)
                                    {
                                        self.cordim_snapshot = snap;
                                        self.cordim_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cordim_symbol.to_uppercase();
                            self.cordim_loading = true;
                            self.cordim_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCordimSnapshot { symbol: sym });
                        }
                        if self.cordim_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_cordim_snapshot(ui, &self.cordim_snapshot);
                });
            self.show_cordim = open;
        }

        if self.show_skspec {
            if self.skspec_symbol.is_empty() {
                self.skspec_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_skspec;
            egui::Window::new("SKSPEC — Rolling-Window Skewness Spectrum")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.skspec_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.skspec_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.skspec_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_skspec(&conn, &sym_u)
                                    {
                                        self.skspec_snapshot = snap;
                                        self.skspec_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.skspec_symbol.to_uppercase();
                            self.skspec_loading = true;
                            self.skspec_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeSkspecSnapshot { symbol: sym });
                        }
                        if self.skspec_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_skspec_snapshot(ui, &self.skspec_snapshot);
                });
            self.show_skspec = open;
        }

        if self.show_automi {
            if self.automi_symbol.is_empty() {
                self.automi_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_automi;
            egui::Window::new("AUTOMI — Auto Mutual Information (Info-Theoretic ACF)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.automi_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.automi_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.automi_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_automi(&conn, &sym_u)
                                    {
                                        self.automi_snapshot = snap;
                                        self.automi_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.automi_symbol.to_uppercase();
                            self.automi_loading = true;
                            self.automi_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeAutomiSnapshot { symbol: sym });
                        }
                        if self.automi_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_automi_snapshot(ui, &self.automi_snapshot);
                });
            self.show_automi = open;
        }
    }
}
