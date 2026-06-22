use super::*;

impl TyphooNApp {
    pub(super) fn render_research_upside_leverage_drawdown_var_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if self.show_upr {
            if self.upr_symbol.is_empty() {
                self.upr_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_upr;
            egui::Window::new("UPR — Upside Potential Ratio")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.upr_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.upr_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.upr_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_upr(&conn, &sym_u)
                                    {
                                        self.upr_snapshot = snap;
                                        self.upr_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.upr_symbol.to_uppercase();
                            self.upr_loading = true;
                            self.upr_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeUprSnapshot { symbol: sym });
                        }
                        if self.upr_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_upr_snapshot(ui, &self.upr_snapshot);
                });
            self.show_upr = open;
        }

        if self.show_levereff {
            if self.levereff_symbol.is_empty() {
                self.levereff_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_levereff;
            egui::Window::new("LEVEREFF — Leverage Effect")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.levereff_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.levereff_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.levereff_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_levereff(&conn, &sym_u)
                                    {
                                        self.levereff_snapshot = snap;
                                        self.levereff_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.levereff_symbol.to_uppercase();
                            self.levereff_loading = true;
                            self.levereff_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeLevereffSnapshot { symbol: sym });
                        }
                        if self.levereff_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_levereff_snapshot(ui, &self.levereff_snapshot);
                });
            self.show_levereff = open;
        }

        if self.show_drawdar {
            if self.drawdar_symbol.is_empty() {
                self.drawdar_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_drawdar;
            egui::Window::new("DRAWDAR — Drawdown-at-Risk")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 350.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.drawdar_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.drawdar_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.drawdar_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_drawdar(&conn, &sym_u)
                                    {
                                        self.drawdar_snapshot = snap;
                                        self.drawdar_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.drawdar_symbol.to_uppercase();
                            self.drawdar_loading = true;
                            self.drawdar_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeDrawdarSnapshot { symbol: sym });
                        }
                        if self.drawdar_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_drawdar_snapshot(ui, &self.drawdar_snapshot);
                });
            self.show_drawdar = open;
        }

        if self.show_varhalf {
            if self.varhalf_symbol.is_empty() {
                self.varhalf_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_varhalf;
            egui::Window::new("VARHALF — Volatility Half-Life")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.varhalf_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.varhalf_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.varhalf_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_varhalf(&conn, &sym_u)
                                    {
                                        self.varhalf_snapshot = snap;
                                        self.varhalf_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.varhalf_symbol.to_uppercase();
                            self.varhalf_loading = true;
                            self.varhalf_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeVarhalfSnapshot { symbol: sym });
                        }
                        if self.varhalf_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_varhalf_snapshot(ui, &self.varhalf_snapshot);
                });
            self.show_varhalf = open;
        }

        if self.show_gini {
            if self.gini_symbol.is_empty() {
                self.gini_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_gini;
            egui::Window::new("GINI — Return Concentration")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.gini_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.gini_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.gini_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_gini(&conn, &sym_u)
                                    {
                                        self.gini_snapshot = snap;
                                        self.gini_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.gini_symbol.to_uppercase();
                            self.gini_loading = true;
                            self.gini_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeGiniSnapshot { symbol: sym });
                        }
                        if self.gini_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_gini_snapshot(ui, &self.gini_snapshot);
                });
            self.show_gini = open;
        }
    }
}
