use super::*;

impl TyphooNApp {
    pub(super) fn render_research_calmar_ulcer_liquidity_normality_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if self.show_calmar {
            if self.calmar_symbol.is_empty() {
                self.calmar_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_calmar;
            egui::Window::new("CALMAR — Calmar Ratio (Return / Max Drawdown)")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.calmar_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.calmar_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.calmar_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_calmar(&conn, &sym_u)
                                    {
                                        self.calmar_snapshot = snap;
                                        self.calmar_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.calmar_symbol.to_uppercase();
                            self.calmar_loading = true;
                            self.calmar_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCalmarSnapshot { symbol: sym });
                        }
                        if self.calmar_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_calmar_snapshot(ui, &self.calmar_snapshot);
                });
            self.show_calmar = open;
        }

        if self.show_ulcer {
            if self.ulcer_symbol.is_empty() {
                self.ulcer_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ulcer;
            egui::Window::new("ULCER — Ulcer Index + Martin Ratio (UPI)")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ulcer_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ulcer_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ulcer_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ulcer(&conn, &sym_u)
                                    {
                                        self.ulcer_snapshot = snap;
                                        self.ulcer_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ulcer_symbol.to_uppercase();
                            self.ulcer_loading = true;
                            self.ulcer_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeUlcerSnapshot { symbol: sym });
                        }
                        if self.ulcer_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_ulcer_snapshot(ui, &self.ulcer_snapshot);
                });
            self.show_ulcer = open;
        }

        if self.show_varratio {
            if self.varratio_symbol.is_empty() {
                self.varratio_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_varratio;
            egui::Window::new("VARRATIO — Lo-MacKinlay Variance Ratio")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.varratio_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.varratio_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.varratio_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_varratio(&conn, &sym_u)
                                    {
                                        self.varratio_snapshot = snap;
                                        self.varratio_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.varratio_symbol.to_uppercase();
                            self.varratio_loading = true;
                            self.varratio_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeVarratioSnapshot { symbol: sym });
                        }
                        if self.varratio_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_varratio_snapshot(ui, &self.varratio_snapshot);
                });
            self.show_varratio = open;
        }

        if self.show_amihud {
            if self.amihud_symbol.is_empty() {
                self.amihud_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_amihud;
            egui::Window::new("AMIHUD — Amihud Illiquidity Ratio")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.amihud_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.amihud_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.amihud_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_amihud(&conn, &sym_u)
                                    {
                                        self.amihud_snapshot = snap;
                                        self.amihud_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.amihud_symbol.to_uppercase();
                            self.amihud_loading = true;
                            self.amihud_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeAmihudSnapshot { symbol: sym });
                        }
                        if self.amihud_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_amihud_snapshot(ui, &self.amihud_snapshot);
                });
            self.show_amihud = open;
        }

        if self.show_jbnorm {
            if self.jbnorm_symbol.is_empty() {
                self.jbnorm_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_jbnorm;
            egui::Window::new("JBNORM — Jarque-Bera Normality Test")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.jbnorm_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.jbnorm_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.jbnorm_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_jbnorm(&conn, &sym_u)
                                    {
                                        self.jbnorm_snapshot = snap;
                                        self.jbnorm_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.jbnorm_symbol.to_uppercase();
                            self.jbnorm_loading = true;
                            self.jbnorm_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeJbnormSnapshot { symbol: sym });
                        }
                        if self.jbnorm_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_jbnorm_snapshot(ui, &self.jbnorm_snapshot);
                });
            self.show_jbnorm = open;
        }
    }
}
