use super::*;

impl TyphooNApp {
    pub(super) fn render_research_downside_efficiency_wick_volatility_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if self.show_downvol {
            if self.downvol_symbol.is_empty() {
                self.downvol_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_downvol;
            egui::Window::new("DOWNVOL — Downside Deviation / Sortino")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.downvol_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.downvol_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.downvol_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_downvol(&conn, &sym_u)
                                    {
                                        self.downvol_snapshot = snap;
                                        self.downvol_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.downvol_symbol.to_uppercase();
                            self.downvol_loading = true;
                            self.downvol_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeDownvolSnapshot { symbol: sym });
                        }
                        if self.downvol_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_downvol_snapshot(ui, &self.downvol_snapshot);
                });
            self.show_downvol = open;
        }

        if self.show_sharpr {
            if self.sharpr_symbol.is_empty() {
                self.sharpr_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_sharpr;
            egui::Window::new("SHARPR — Sharpe Ratio (rf=0)")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.sharpr_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.sharpr_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.sharpr_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_sharpr(&conn, &sym_u)
                                    {
                                        self.sharpr_snapshot = snap;
                                        self.sharpr_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.sharpr_symbol.to_uppercase();
                            self.sharpr_loading = true;
                            self.sharpr_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeSharprSnapshot { symbol: sym });
                        }
                        if self.sharpr_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_sharpr_snapshot(ui, &self.sharpr_snapshot);
                });
            self.show_sharpr = open;
        }

        if self.show_effratio {
            if self.effratio_symbol.is_empty() {
                self.effratio_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_effratio;
            egui::Window::new("EFFRATIO — Kaufman Efficiency Ratio")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.effratio_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.effratio_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.effratio_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_effratio(&conn, &sym_u)
                                    {
                                        self.effratio_snapshot = snap;
                                        self.effratio_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.effratio_symbol.to_uppercase();
                            self.effratio_loading = true;
                            self.effratio_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeEffratioSnapshot { symbol: sym });
                        }
                        if self.effratio_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_effratio_snapshot(ui, &self.effratio_snapshot);
                });
            self.show_effratio = open;
        }

        if self.show_wickbias {
            if self.wickbias_symbol.is_empty() {
                self.wickbias_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_wickbias;
            egui::Window::new("WICKBIAS — Upper vs Lower Wick Asymmetry")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.wickbias_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.wickbias_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.wickbias_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_wickbias(&conn, &sym_u)
                                    {
                                        self.wickbias_snapshot = snap;
                                        self.wickbias_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.wickbias_symbol.to_uppercase();
                            self.wickbias_loading = true;
                            self.wickbias_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeWickbiasSnapshot { symbol: sym });
                        }
                        if self.wickbias_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_wickbias_snapshot(ui, &self.wickbias_snapshot);
                });
            self.show_wickbias = open;
        }

        if self.show_volofvol {
            if self.volofvol_symbol.is_empty() {
                self.volofvol_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_volofvol;
            egui::Window::new("VOLOFVOL — Stdev of Rolling 20d Realized Vol")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.volofvol_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.volofvol_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.volofvol_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_volofvol(&conn, &sym_u)
                                    {
                                        self.volofvol_snapshot = snap;
                                        self.volofvol_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.volofvol_symbol.to_uppercase();
                            self.volofvol_loading = true;
                            self.volofvol_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeVolofvolSnapshot { symbol: sym });
                        }
                        if self.volofvol_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_volofvol_snapshot(ui, &self.volofvol_snapshot);
                });
            self.show_volofvol = open;
        }
    }
}
