use super::*;

impl TyphooNApp {
    pub(super) fn render_research_sterling_kelly_stat_tests_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──

        if self.show_sterling {
            if self.sterling_symbol.is_empty() {
                self.sterling_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_sterling;
            egui::Window::new("STERLING — Sterling Ratio")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 360.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.sterling_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.sterling_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.sterling_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_sterling(&conn, &sym_u)
                                    {
                                        self.sterling_snapshot = snap;
                                        self.sterling_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.sterling_symbol.to_uppercase();
                            self.sterling_loading = true;
                            self.sterling_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeSterlingSnapshot { symbol: sym });
                        }
                        if self.sterling_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_sterling_snapshot(ui, &self.sterling_snapshot);
                });
            self.show_sterling = open;
        }

        if self.show_kellyf {
            if self.kellyf_symbol.is_empty() {
                self.kellyf_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_kellyf;
            egui::Window::new("KELLYF — Kelly Fraction / Optimal Leverage")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.kellyf_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.kellyf_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.kellyf_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_kellyf(&conn, &sym_u)
                                    {
                                        self.kellyf_snapshot = snap;
                                        self.kellyf_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.kellyf_symbol.to_uppercase();
                            self.kellyf_loading = true;
                            self.kellyf_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeKellyfSnapshot { symbol: sym });
                        }
                        if self.kellyf_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_kellyf_snapshot(ui, &self.kellyf_snapshot);
                });
            self.show_kellyf = open;
        }

        if self.show_ljungb {
            if self.ljungb_symbol.is_empty() {
                self.ljungb_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ljungb;
            egui::Window::new("LJUNGB — Ljung-Box Q-Statistic (h=10)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 340.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ljungb_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ljungb_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ljungb_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ljungb(&conn, &sym_u)
                                    {
                                        self.ljungb_snapshot = snap;
                                        self.ljungb_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ljungb_symbol.to_uppercase();
                            self.ljungb_loading = true;
                            self.ljungb_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeLjungbSnapshot { symbol: sym });
                        }
                        if self.ljungb_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_ljungb_snapshot(ui, &self.ljungb_snapshot);
                });
            self.show_ljungb = open;
        }

        if self.show_runstest {
            if self.runstest_symbol.is_empty() {
                self.runstest_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_runstest;
            egui::Window::new("RUNSTEST — Wald-Wolfowitz Runs Test")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.runstest_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.runstest_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.runstest_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_runstest(&conn, &sym_u)
                                    {
                                        self.runstest_snapshot = snap;
                                        self.runstest_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.runstest_symbol.to_uppercase();
                            self.runstest_loading = true;
                            self.runstest_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRunstestSnapshot { symbol: sym });
                        }
                        if self.runstest_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_runstest_snapshot(ui, &self.runstest_snapshot);
                });
            self.show_runstest = open;
        }

        if self.show_zeroret {
            if self.zeroret_symbol.is_empty() {
                self.zeroret_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_zeroret;
            egui::Window::new("ZERORET — Zero-Return-Day Fraction (Lesmond-Ogden-Trzcinka)")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 340.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.zeroret_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.zeroret_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.zeroret_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_zeroret(&conn, &sym_u)
                                    {
                                        self.zeroret_snapshot = snap;
                                        self.zeroret_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.zeroret_symbol.to_uppercase();
                            self.zeroret_loading = true;
                            self.zeroret_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeZeroretSnapshot { symbol: sym });
                        }
                        if self.zeroret_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_zeroret_snapshot(ui, &self.zeroret_snapshot);
                });
            self.show_zeroret = open;
        }
    }
}
