use super::*;

impl TyphooNApp {
    pub(super) fn render_research_ohlc_price_transforms_windows(&mut self, ctx: &egui::Context) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── : AVGPRICE / MEDPRICE / TYPPRICE / WCLPRICE / VARIANCE ──
        if self.show_avgprice_win {
            if self.avgprice_win_symbol.is_empty() {
                self.avgprice_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_avgprice_win;
            egui::Window::new("AVGPRICE — OHLC average (O+H+L+C)/4")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.avgprice_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.avgprice_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.avgprice_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_avgprice(&conn, &sym_u)
                                    {
                                        self.avgprice_win_snapshot = snap;
                                        self.avgprice_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.avgprice_win_symbol.to_uppercase();
                            self.avgprice_win_loading = true;
                            self.avgprice_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeAvgpriceSnapshot { symbol: sym });
                        }
                        if self.avgprice_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_avgprice_snapshot(ui, &self.avgprice_win_snapshot);
                });
            self.show_avgprice_win = open;
        }

        if self.show_medprice_win {
            if self.medprice_win_symbol.is_empty() {
                self.medprice_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_medprice_win;
            egui::Window::new("MEDPRICE — range median (H+L)/2")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.medprice_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.medprice_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.medprice_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_medprice(&conn, &sym_u)
                                    {
                                        self.medprice_win_snapshot = snap;
                                        self.medprice_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.medprice_win_symbol.to_uppercase();
                            self.medprice_win_loading = true;
                            self.medprice_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMedpriceSnapshot { symbol: sym });
                        }
                        if self.medprice_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_medprice_snapshot(ui, &self.medprice_win_snapshot);
                });
            self.show_medprice_win = open;
        }

        if self.show_typprice_win {
            if self.typprice_win_symbol.is_empty() {
                self.typprice_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_typprice_win;
            egui::Window::new("TYPPRICE — typical price (H+L+C)/3")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.typprice_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.typprice_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.typprice_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_typprice(&conn, &sym_u)
                                    {
                                        self.typprice_win_snapshot = snap;
                                        self.typprice_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.typprice_win_symbol.to_uppercase();
                            self.typprice_win_loading = true;
                            self.typprice_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeTypPriceSnapshot { symbol: sym });
                        }
                        if self.typprice_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_typprice_snapshot(ui, &self.typprice_win_snapshot);
                });
            self.show_typprice_win = open;
        }

        if self.show_wclprice_win {
            if self.wclprice_win_symbol.is_empty() {
                self.wclprice_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_wclprice_win;
            egui::Window::new("WCLPRICE — weighted close (H+L+2C)/4")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.wclprice_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.wclprice_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.wclprice_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_wclprice(&conn, &sym_u)
                                    {
                                        self.wclprice_win_snapshot = snap;
                                        self.wclprice_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.wclprice_win_symbol.to_uppercase();
                            self.wclprice_win_loading = true;
                            self.wclprice_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeWclPriceSnapshot { symbol: sym });
                        }
                        if self.wclprice_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_wclprice_snapshot(ui, &self.wclprice_win_snapshot);
                });
            self.show_wclprice_win = open;
        }

        if self.show_variance_win {
            if self.variance_win_symbol.is_empty() {
                self.variance_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_variance_win;
            egui::Window::new("VARIANCE — close variance (5-bar population, TA-Lib default)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.variance_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.variance_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.variance_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_variance(&conn, &sym_u)
                                    {
                                        self.variance_win_snapshot = snap;
                                        self.variance_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.variance_win_symbol.to_uppercase();
                            self.variance_win_loading = true;
                            self.variance_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeVarianceSnapshot { symbol: sym });
                        }
                        if self.variance_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_variance_snapshot(ui, &self.variance_win_snapshot);
                });
            self.show_variance_win = open;
        }
    }
}
