use super::*;

impl TyphooNApp {
    pub(super) fn render_research_laguerre_pivot_midpoint_models_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research LAGUERRE_RSI / ZIGZAG / PGO / HT_TRENDLINE / MIDPOINT windows ──

        if self.show_laguerre_rsi_win {
            if self.laguerre_rsi_win_symbol.is_empty() {
                self.laguerre_rsi_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_laguerre_rsi_win;
            egui::Window::new("LAGUERRE_RSI — Ehlers 4-stage Laguerre Filter RSI (γ=0.5)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.laguerre_rsi_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.laguerre_rsi_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.laguerre_rsi_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_laguerre_rsi(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.laguerre_rsi_win_snapshot = snap;
                                        self.laguerre_rsi_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.laguerre_rsi_win_symbol.to_uppercase();
                            self.laguerre_rsi_win_loading = true;
                            self.laguerre_rsi_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeLaguerreRsiSnapshot { symbol: sym });
                        }
                        if self.laguerre_rsi_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_laguerre_rsi_snapshot(
                        ui,
                        &self.laguerre_rsi_win_snapshot,
                    );
                });
            self.show_laguerre_rsi_win = open;
        }

        if self.show_zigzag_win {
            if self.zigzag_win_symbol.is_empty() {
                self.zigzag_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_zigzag_win;
            egui::Window::new("ZIGZAG — Percent-Threshold Pivot Reversal Detector (5% default)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.zigzag_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.zigzag_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.zigzag_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_zigzag(&conn, &sym_u)
                                    {
                                        self.zigzag_win_snapshot = snap;
                                        self.zigzag_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.zigzag_win_symbol.to_uppercase();
                            self.zigzag_win_loading = true;
                            self.zigzag_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeZigzagSnapshot { symbol: sym });
                        }
                        if self.zigzag_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_zigzag_snapshot(ui, &self.zigzag_win_snapshot);
                });
            self.show_zigzag_win = open;
        }

        if self.show_pgo_win {
            if self.pgo_win_symbol.is_empty() {
                self.pgo_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_pgo_win;
            egui::Window::new(
                "PGO — Pretty Good Oscillator (Mark Johnson, (close−SMA)/EMA(TR), N=14)",
            )
            .open(&mut open)
            .resizable(true)
            .default_size([560.0, 260.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.pgo_win_symbol).desired_width(100.0),
                    );
                    if ui.button("Use Chart").clicked() {
                        self.pgo_win_symbol = chart_sym_research.clone();
                    }
                    if ui.button("Load Cached").clicked() {
                        if let Some(ref cache) = self.cache {
                            if let Ok(conn) = cache.connection() {
                                let sym_u = self.pgo_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) =
                                    typhoon_engine::core::research::get_pgo(&conn, &sym_u)
                                {
                                    self.pgo_win_snapshot = snap;
                                    self.pgo_win_symbol = sym_u;
                                }
                            }
                        }
                    }
                    if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                        let sym = self.pgo_win_symbol.to_uppercase();
                        self.pgo_win_loading = true;
                        self.pgo_win_symbol = sym.clone();
                        let _ = self
                            .broker_tx
                            .send(BrokerCmd::ComputePgoSnapshot { symbol: sym });
                    }
                    if self.pgo_win_loading {
                        ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                    }
                });
                super::render::render_pgo_snapshot(ui, &self.pgo_win_snapshot);
            });
            self.show_pgo_win = open;
        }

        if self.show_ht_trendline_win {
            if self.ht_trendline_win_symbol.is_empty() {
                self.ht_trendline_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ht_trendline_win;
            egui::Window::new(
                "HT_TRENDLINE — Hilbert Instantaneous Trendline (Ehlers, period-adaptive WMA)",
            )
            .open(&mut open)
            .resizable(true)
            .default_size([560.0, 260.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.ht_trendline_win_symbol)
                            .desired_width(100.0),
                    );
                    if ui.button("Use Chart").clicked() {
                        self.ht_trendline_win_symbol = chart_sym_research.clone();
                    }
                    if ui.button("Load Cached").clicked() {
                        if let Some(ref cache) = self.cache {
                            if let Ok(conn) = cache.connection() {
                                let sym_u = self.ht_trendline_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) =
                                    typhoon_engine::core::research::get_ht_trendline(&conn, &sym_u)
                                {
                                    self.ht_trendline_win_snapshot = snap;
                                    self.ht_trendline_win_symbol = sym_u;
                                }
                            }
                        }
                    }
                    if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                        let sym = self.ht_trendline_win_symbol.to_uppercase();
                        self.ht_trendline_win_loading = true;
                        self.ht_trendline_win_symbol = sym.clone();
                        let _ = self
                            .broker_tx
                            .send(BrokerCmd::ComputeHtTrendlineSnapshot { symbol: sym });
                    }
                    if self.ht_trendline_win_loading {
                        ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                    }
                });
                super::render::render_ht_trendline_snapshot(ui, &self.ht_trendline_win_snapshot);
            });
            self.show_ht_trendline_win = open;
        }

        if self.show_midpoint_win {
            if self.midpoint_win_symbol.is_empty() {
                self.midpoint_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_midpoint_win;
            egui::Window::new("MIDPOINT — (HHV(N) + LLV(N)) / 2 with Close Position (N=14)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.midpoint_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.midpoint_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.midpoint_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_midpoint(&conn, &sym_u)
                                    {
                                        self.midpoint_win_snapshot = snap;
                                        self.midpoint_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.midpoint_win_symbol.to_uppercase();
                            self.midpoint_win_loading = true;
                            self.midpoint_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMidpointSnapshot { symbol: sym });
                        }
                        if self.midpoint_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_midpoint_snapshot(ui, &self.midpoint_win_snapshot);
                });
            self.show_midpoint_win = open;
        }
    }
}
