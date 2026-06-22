use super::*;

impl TyphooNApp {
    pub(super) fn render_research_aroon_macd_variable_ma_windows(&mut self, ctx: &egui::Context) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if self.show_aroonosc_win {
            if self.aroonosc_win_symbol.is_empty() {
                self.aroonosc_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_aroonosc_win;
            egui::Window::new("AROONOSC — Aroon Oscillator (period 14)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.aroonosc_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.aroonosc_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.aroonosc_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_aroonosc(&conn, &sym_u)
                                    {
                                        self.aroonosc_win_snapshot = snap;
                                        self.aroonosc_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.aroonosc_win_symbol.to_uppercase();
                            self.aroonosc_win_loading = true;
                            self.aroonosc_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeAroonoscSnapshot { symbol: sym });
                        }
                        if self.aroonosc_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_aroonosc_snapshot(ui, &self.aroonosc_win_snapshot);
                });
            self.show_aroonosc_win = open;
        }

        if self.show_minmaxindex_win {
            if self.minmaxindex_win_symbol.is_empty() {
                self.minmaxindex_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_minmaxindex_win;
            egui::Window::new("MINMAXINDEX — combined min+max recency (period 30)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.minmaxindex_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.minmaxindex_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.minmaxindex_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_minmaxindex(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.minmaxindex_win_snapshot = snap;
                                        self.minmaxindex_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.minmaxindex_win_symbol.to_uppercase();
                            self.minmaxindex_win_loading = true;
                            self.minmaxindex_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMinMaxIndexSnapshot { symbol: sym });
                        }
                        if self.minmaxindex_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_minmaxindex_snapshot(ui, &self.minmaxindex_win_snapshot);
                });
            self.show_minmaxindex_win = open;
        }

        if self.show_macdext_win {
            if self.macdext_win_symbol.is_empty() {
                self.macdext_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_macdext_win;
            egui::Window::new("MACDEXT — MACD with SMA (12/26/9)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 290.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.macdext_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.macdext_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.macdext_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_macdext(&conn, &sym_u)
                                    {
                                        self.macdext_win_snapshot = snap;
                                        self.macdext_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.macdext_win_symbol.to_uppercase();
                            self.macdext_win_loading = true;
                            self.macdext_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMacdextSnapshot { symbol: sym });
                        }
                        if self.macdext_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_macdext_snapshot(ui, &self.macdext_win_snapshot);
                });
            self.show_macdext_win = open;
        }

        if self.show_macdfix_win {
            if self.macdfix_win_symbol.is_empty() {
                self.macdfix_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_macdfix_win;
            egui::Window::new("MACDFIX — MACD with hardcoded EMA 12/26 + signal 9")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.macdfix_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.macdfix_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.macdfix_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_macdfix(&conn, &sym_u)
                                    {
                                        self.macdfix_win_snapshot = snap;
                                        self.macdfix_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.macdfix_win_symbol.to_uppercase();
                            self.macdfix_win_loading = true;
                            self.macdfix_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMacdfixSnapshot { symbol: sym });
                        }
                        if self.macdfix_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_macdfix_snapshot(ui, &self.macdfix_win_snapshot);
                });
            self.show_macdfix_win = open;
        }

        if self.show_mavp_win {
            if self.mavp_win_symbol.is_empty() {
                self.mavp_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_mavp_win;
            egui::Window::new("MAVP — Moving Average with Variable Period (5..30 ramp)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.mavp_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.mavp_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.mavp_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_mavp(&conn, &sym_u)
                                    {
                                        self.mavp_win_snapshot = snap;
                                        self.mavp_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.mavp_win_symbol.to_uppercase();
                            self.mavp_win_loading = true;
                            self.mavp_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMavpSnapshot { symbol: sym });
                        }
                        if self.mavp_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_mavp_snapshot(ui, &self.mavp_win_snapshot);
                });
            self.show_mavp_win = open;
        }
    }
}
