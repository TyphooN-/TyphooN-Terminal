use super::*;

impl TyphooNApp {
    pub(super) fn render_research_moving_average_regression_pivots_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if self.show_dema_win {
            if self.dema_win_symbol.is_empty() {
                self.dema_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_dema_win;
            egui::Window::new("DEMA — Double Exponential Moving Average (length 20)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.dema_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.dema_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.dema_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_dema(&conn, &sym_u)
                                    {
                                        self.dema_win_snapshot = snap;
                                        self.dema_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.dema_win_symbol.to_uppercase();
                            self.dema_win_loading = true;
                            self.dema_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeDemaSnapshot { symbol: sym });
                        }
                        if self.dema_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_dema_snapshot(ui, &self.dema_win_snapshot);
                });
            self.show_dema_win = open;
        }

        if self.show_tema_win {
            if self.tema_win_symbol.is_empty() {
                self.tema_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_tema_win;
            egui::Window::new("TEMA — Triple Exponential Moving Average (length 20)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.tema_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.tema_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.tema_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_tema(&conn, &sym_u)
                                    {
                                        self.tema_win_snapshot = snap;
                                        self.tema_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.tema_win_symbol.to_uppercase();
                            self.tema_win_loading = true;
                            self.tema_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeTemaSnapshot { symbol: sym });
                        }
                        if self.tema_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_tema_snapshot(ui, &self.tema_win_snapshot);
                });
            self.show_tema_win = open;
        }

        if self.show_linreg_win {
            if self.linreg_win_symbol.is_empty() {
                self.linreg_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_linreg_win;
            egui::Window::new("LINREG — Linear Regression Channel (length 20, ±2σ)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.linreg_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.linreg_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.linreg_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_linreg(&conn, &sym_u)
                                    {
                                        self.linreg_win_snapshot = snap;
                                        self.linreg_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.linreg_win_symbol.to_uppercase();
                            self.linreg_win_loading = true;
                            self.linreg_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeLinregSnapshot { symbol: sym });
                        }
                        if self.linreg_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_linreg_snapshot(ui, &self.linreg_win_snapshot);
                });
            self.show_linreg_win = open;
        }

        if self.show_pivots_win {
            if self.pivots_win_symbol.is_empty() {
                self.pivots_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_pivots_win;
            egui::Window::new("PIVOTS — Classic Floor-Trader Pivot Points (prior bar)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.pivots_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.pivots_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.pivots_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_pivots(&conn, &sym_u)
                                    {
                                        self.pivots_win_snapshot = snap;
                                        self.pivots_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.pivots_win_symbol.to_uppercase();
                            self.pivots_win_loading = true;
                            self.pivots_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputePivotsSnapshot { symbol: sym });
                        }
                        if self.pivots_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_pivots_snapshot(ui, &self.pivots_win_snapshot);
                });
            self.show_pivots_win = open;
        }

        if self.show_heikin_win {
            if self.heikin_win_symbol.is_empty() {
                self.heikin_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_heikin_win;
            egui::Window::new("HEIKIN — Heikin-Ashi Candle Sentiment Tracker")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.heikin_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.heikin_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.heikin_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_heikin(&conn, &sym_u)
                                    {
                                        self.heikin_win_snapshot = snap;
                                        self.heikin_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.heikin_win_symbol.to_uppercase();
                            self.heikin_win_loading = true;
                            self.heikin_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeHeikinSnapshot { symbol: sym });
                        }
                        if self.heikin_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_heikin_snapshot(ui, &self.heikin_win_snapshot);
                });
            self.show_heikin_win = open;
        }
    }
}
