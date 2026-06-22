use super::*;

impl TyphooNApp {
    pub(super) fn render_research_normality_lmoments_price_impact_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if self.show_ksnorm {
            if self.ksnorm_symbol.is_empty() {
                self.ksnorm_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ksnorm;
            egui::Window::new("KSNORM — Kolmogorov-Smirnov Normality Test")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ksnorm_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ksnorm_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ksnorm_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ksnorm(&conn, &sym_u)
                                    {
                                        self.ksnorm_snapshot = snap;
                                        self.ksnorm_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ksnorm_symbol.to_uppercase();
                            self.ksnorm_loading = true;
                            self.ksnorm_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeKsnormSnapshot { symbol: sym });
                        }
                        if self.ksnorm_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_ksnorm_snapshot(ui, &self.ksnorm_snapshot);
                });
            self.show_ksnorm = open;
        }

        if self.show_adtest {
            if self.adtest_symbol.is_empty() {
                self.adtest_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_adtest;
            egui::Window::new("ADTEST — Anderson-Darling Normality Test")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.adtest_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.adtest_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.adtest_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_adtest(&conn, &sym_u)
                                    {
                                        self.adtest_snapshot = snap;
                                        self.adtest_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.adtest_symbol.to_uppercase();
                            self.adtest_loading = true;
                            self.adtest_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeAdtestSnapshot { symbol: sym });
                        }
                        if self.adtest_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_adtest_snapshot(ui, &self.adtest_snapshot);
                });
            self.show_adtest = open;
        }

        if self.show_lmom {
            if self.lmom_symbol.is_empty() {
                self.lmom_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_lmom;
            egui::Window::new("LMOM — L-Moments (Hosking 1990)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.lmom_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.lmom_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.lmom_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_lmom(&conn, &sym_u)
                                    {
                                        self.lmom_snapshot = snap;
                                        self.lmom_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.lmom_symbol.to_uppercase();
                            self.lmom_loading = true;
                            self.lmom_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeLmomSnapshot { symbol: sym });
                        }
                        if self.lmom_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_lmom_snapshot(ui, &self.lmom_snapshot);
                });
            self.show_lmom = open;
        }

        if self.show_kylelam {
            if self.kylelam_symbol.is_empty() {
                self.kylelam_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_kylelam;
            egui::Window::new("KYLELAM — Kyle's Price-Impact λ")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.kylelam_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.kylelam_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.kylelam_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_kylelam(&conn, &sym_u)
                                    {
                                        self.kylelam_snapshot = snap;
                                        self.kylelam_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.kylelam_symbol.to_uppercase();
                            self.kylelam_loading = true;
                            self.kylelam_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeKylelamSnapshot { symbol: sym });
                        }
                        if self.kylelam_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_kylelam_snapshot(ui, &self.kylelam_snapshot);
                });
            self.show_kylelam = open;
        }
    }
}
