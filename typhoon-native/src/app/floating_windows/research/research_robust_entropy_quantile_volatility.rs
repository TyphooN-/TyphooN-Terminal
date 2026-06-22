use super::*;

impl TyphooNApp {
    pub(super) fn render_research_robust_entropy_quantile_volatility_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if self.show_robvol {
            if self.robvol_symbol.is_empty() {
                self.robvol_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_robvol;
            egui::Window::new("ROBVOL — Robust Volatility")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.robvol_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.robvol_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.robvol_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_robvol(&conn, &sym_u)
                                    {
                                        self.robvol_snapshot = snap;
                                        self.robvol_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.robvol_symbol.to_uppercase();
                            self.robvol_loading = true;
                            self.robvol_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRobvolSnapshot { symbol: sym });
                        }
                        if self.robvol_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_robvol_snapshot(ui, &self.robvol_snapshot);
                });
            self.show_robvol = open;
        }

        if self.show_renyient {
            if self.renyient_symbol.is_empty() {
                self.renyient_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_renyient;
            egui::Window::new("RENYIENT — Rényi Entropy (α=2)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.renyient_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.renyient_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.renyient_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_renyient(&conn, &sym_u)
                                    {
                                        self.renyient_snapshot = snap;
                                        self.renyient_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.renyient_symbol.to_uppercase();
                            self.renyient_loading = true;
                            self.renyient_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRenyientSnapshot { symbol: sym });
                        }
                        if self.renyient_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_renyient_snapshot(ui, &self.renyient_snapshot);
                });
            self.show_renyient = open;
        }

        if self.show_retquant {
            if self.retquant_symbol.is_empty() {
                self.retquant_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_retquant;
            egui::Window::new("RETQUANT — Return Quantile Profile")
                .open(&mut open)
                .resizable(true)
                .default_size([600.0, 360.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.retquant_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.retquant_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.retquant_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_retquant(&conn, &sym_u)
                                    {
                                        self.retquant_snapshot = snap;
                                        self.retquant_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.retquant_symbol.to_uppercase();
                            self.retquant_loading = true;
                            self.retquant_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRetquantSnapshot { symbol: sym });
                        }
                        if self.retquant_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_retquant_snapshot(ui, &self.retquant_snapshot);
                });
            self.show_retquant = open;
        }

        if self.show_msent {
            if self.msent_symbol.is_empty() {
                self.msent_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_msent;
            egui::Window::new("MSENT — Multiscale Entropy")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.msent_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.msent_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.msent_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_msent(&conn, &sym_u)
                                    {
                                        self.msent_snapshot = snap;
                                        self.msent_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.msent_symbol.to_uppercase();
                            self.msent_loading = true;
                            self.msent_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMsentSnapshot { symbol: sym });
                        }
                        if self.msent_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_msent_snapshot(ui, &self.msent_snapshot);
                });
            self.show_msent = open;
        }

        if self.show_ewmavol {
            if self.ewmavol_symbol.is_empty() {
                self.ewmavol_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ewmavol;
            egui::Window::new("EWMAVOL — EWMA Volatility (RiskMetrics)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ewmavol_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ewmavol_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ewmavol_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ewmavol(&conn, &sym_u)
                                    {
                                        self.ewmavol_snapshot = snap;
                                        self.ewmavol_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ewmavol_symbol.to_uppercase();
                            self.ewmavol_loading = true;
                            self.ewmavol_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeEwmavolSnapshot { symbol: sym });
                        }
                        if self.ewmavol_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_ewmavol_snapshot(ui, &self.ewmavol_snapshot);
                });
            self.show_ewmavol = open;
        }
    }
}
