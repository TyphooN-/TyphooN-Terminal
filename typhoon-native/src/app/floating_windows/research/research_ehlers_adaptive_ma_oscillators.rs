use super::*;

impl TyphooNApp {
    pub(super) fn render_research_ehlers_adaptive_ma_oscillators_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research WMA / RAINBOW / MESA_SINE / FRAMA / IBS windows ──

        if self.show_wma_win {
            if self.wma_win_symbol.is_empty() {
                self.wma_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_wma_win;
            egui::Window::new("WMA — Weighted Moving Average (linearly-weighted SMA, N=20)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.wma_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.wma_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.wma_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_wma(&conn, &sym_u)
                                    {
                                        self.wma_win_snapshot = snap;
                                        self.wma_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.wma_win_symbol.to_uppercase();
                            self.wma_win_loading = true;
                            self.wma_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeWmaSnapshot { symbol: sym });
                        }
                        if self.wma_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_wma_snapshot(ui, &self.wma_win_snapshot);
                });
            self.show_wma_win = open;
        }

        if self.show_rainbow_win {
            if self.rainbow_win_symbol.is_empty() {
                self.rainbow_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_rainbow_win;
            egui::Window::new("RAINBOW — Rainbow MA Oscillator (10-level recursive SMA stack)")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.rainbow_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.rainbow_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.rainbow_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_rainbow(&conn, &sym_u)
                                    {
                                        self.rainbow_win_snapshot = snap;
                                        self.rainbow_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.rainbow_win_symbol.to_uppercase();
                            self.rainbow_win_loading = true;
                            self.rainbow_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRainbowSnapshot { symbol: sym });
                        }
                        if self.rainbow_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_rainbow_snapshot(ui, &self.rainbow_win_snapshot);
                });
            self.show_rainbow_win = open;
        }

        if self.show_mesa_sine_win {
            if self.mesa_sine_win_symbol.is_empty() {
                self.mesa_sine_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_mesa_sine_win;
            egui::Window::new("MESA_SINE — Ehlers MESA Sine Wave (cycle phase + lead-sine)")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.mesa_sine_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.mesa_sine_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.mesa_sine_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_mesa_sine(&conn, &sym_u)
                                    {
                                        self.mesa_sine_win_snapshot = snap;
                                        self.mesa_sine_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.mesa_sine_win_symbol.to_uppercase();
                            self.mesa_sine_win_loading = true;
                            self.mesa_sine_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMesaSineSnapshot { symbol: sym });
                        }
                        if self.mesa_sine_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_mesa_sine_snapshot(ui, &self.mesa_sine_win_snapshot);
                });
            self.show_mesa_sine_win = open;
        }

        if self.show_frama_win {
            if self.frama_win_symbol.is_empty() {
                self.frama_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_frama_win;
            egui::Window::new("FRAMA — Fractal Adaptive Moving Average (Ehlers, D-driven α)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.frama_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.frama_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.frama_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_frama(&conn, &sym_u)
                                    {
                                        self.frama_win_snapshot = snap;
                                        self.frama_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.frama_win_symbol.to_uppercase();
                            self.frama_win_loading = true;
                            self.frama_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeFramaSnapshot { symbol: sym });
                        }
                        if self.frama_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_frama_snapshot(ui, &self.frama_win_snapshot);
                });
            self.show_frama_win = open;
        }

        if self.show_ibs_win {
            if self.ibs_win_symbol.is_empty() {
                self.ibs_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ibs_win;
            egui::Window::new("IBS — Internal Bar Strength ((close−low)/(high−low) + 14-bar SMA)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ibs_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ibs_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ibs_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ibs(&conn, &sym_u)
                                    {
                                        self.ibs_win_snapshot = snap;
                                        self.ibs_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ibs_win_symbol.to_uppercase();
                            self.ibs_win_loading = true;
                            self.ibs_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeIbsSnapshot { symbol: sym });
                        }
                        if self.ibs_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_ibs_snapshot(ui, &self.ibs_win_snapshot);
                });
            self.show_ibs_win = open;
        }
    }
}
