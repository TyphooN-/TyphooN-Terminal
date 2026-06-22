use super::*;

impl TyphooNApp {
    pub(super) fn render_research_gap_volatility_mean_reversion_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if self.show_drawup {
            if self.drawup_symbol.is_empty() {
                self.drawup_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_drawup;
            egui::Window::new("DRAWUP — Rally History")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.drawup_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.drawup_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.drawup_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_drawup(&conn, &sym_u)
                                    {
                                        self.drawup_snapshot = snap;
                                        self.drawup_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.drawup_symbol.to_uppercase();
                            self.drawup_loading = true;
                            self.drawup_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeDrawupSnapshot { symbol: sym });
                        }
                        if self.drawup_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_drawup_snapshot(ui, &self.drawup_snapshot);
                });
            self.show_drawup = open;
        }

        if self.show_gapstats {
            if self.gapstats_symbol.is_empty() {
                self.gapstats_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_gapstats;
            egui::Window::new("GAPSTATS — Overnight Gap Statistics")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.gapstats_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.gapstats_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.gapstats_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_gapstats(&conn, &sym_u)
                                    {
                                        self.gapstats_snapshot = snap;
                                        self.gapstats_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.gapstats_symbol.to_uppercase();
                            self.gapstats_loading = true;
                            self.gapstats_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeGapstatsSnapshot { symbol: sym });
                        }
                        if self.gapstats_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_gapstats_snapshot(ui, &self.gapstats_snapshot);
                });
            self.show_gapstats = open;
        }

        if self.show_volcluster {
            if self.volcluster_symbol.is_empty() {
                self.volcluster_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_volcluster;
            egui::Window::new("VOLCLUSTER — Volatility Clustering ACF")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.volcluster_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.volcluster_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.volcluster_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_volcluster(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.volcluster_snapshot = snap;
                                        self.volcluster_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.volcluster_symbol.to_uppercase();
                            self.volcluster_loading = true;
                            self.volcluster_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeVolclusterSnapshot { symbol: sym });
                        }
                        if self.volcluster_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_volcluster_snapshot(ui, &self.volcluster_snapshot);
                });
            self.show_volcluster = open;
        }

        if self.show_closeplc {
            if self.closeplc_symbol.is_empty() {
                self.closeplc_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_closeplc;
            egui::Window::new("CLOSEPLC — Close Placement in Daily Range")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.closeplc_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.closeplc_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.closeplc_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_closeplc(&conn, &sym_u)
                                    {
                                        self.closeplc_snapshot = snap;
                                        self.closeplc_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.closeplc_symbol.to_uppercase();
                            self.closeplc_loading = true;
                            self.closeplc_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCloseplcSnapshot { symbol: sym });
                        }
                        if self.closeplc_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_closeplc_snapshot(ui, &self.closeplc_snapshot);
                });
            self.show_closeplc = open;
        }

        if self.show_mrhl {
            if self.mrhl_symbol.is_empty() {
                self.mrhl_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_mrhl;
            egui::Window::new("MRHL — Mean-Reversion Half-Life (AR1)")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.mrhl_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.mrhl_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.mrhl_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_mrhl(&conn, &sym_u)
                                    {
                                        self.mrhl_snapshot = snap;
                                        self.mrhl_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.mrhl_symbol.to_uppercase();
                            self.mrhl_loading = true;
                            self.mrhl_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMrhlSnapshot { symbol: sym });
                        }
                        if self.mrhl_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_mrhl_snapshot(ui, &self.mrhl_snapshot);
                });
            self.show_mrhl = open;
        }
    }
}
