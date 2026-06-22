use super::*;

impl TyphooNApp {
    pub(super) fn render_research_autocorrelation_hurst_volume_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──

        // AUTOCOR — Autocorrelation at multiple lags
        if self.show_autocor {
            if self.autocor_symbol.is_empty() {
                self.autocor_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_autocor;
            egui::Window::new("AUTOCOR — Return Autocorrelation")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.autocor_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.autocor_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.autocor_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_autocor(&conn, &sym_u)
                                    {
                                        self.autocor_snapshot = snap;
                                        self.autocor_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.autocor_symbol.to_uppercase();
                            self.autocor_loading = true;
                            self.autocor_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeAutocorSnapshot { symbol: sym });
                        }
                        if self.autocor_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_autocor_snapshot(ui, &self.autocor_snapshot);
                });
            self.show_autocor = open;
        }

        // HURST — Hurst exponent via R/S
        if self.show_hurst {
            if self.hurst_symbol.is_empty() {
                self.hurst_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_hurst;
            egui::Window::new("HURST — Hurst Exponent (R/S)")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.hurst_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.hurst_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.hurst_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_hurst(&conn, &sym_u)
                                    {
                                        self.hurst_snapshot = snap;
                                        self.hurst_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.hurst_symbol.to_uppercase();
                            self.hurst_loading = true;
                            self.hurst_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeHurstSnapshot { symbol: sym });
                        }
                        if self.hurst_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_hurst_snapshot(ui, &self.hurst_snapshot);
                });
            self.show_hurst = open;
        }

        // HITRATE — Multi-horizon hit rate
        if self.show_hitrate {
            if self.hitrate_symbol.is_empty() {
                self.hitrate_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_hitrate;
            egui::Window::new("HITRATE — Multi-Horizon Win Rate")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.hitrate_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.hitrate_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.hitrate_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_hitrate(&conn, &sym_u)
                                    {
                                        self.hitrate_snapshot = snap;
                                        self.hitrate_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.hitrate_symbol.to_uppercase();
                            self.hitrate_loading = true;
                            self.hitrate_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeHitrateSnapshot { symbol: sym });
                        }
                        if self.hitrate_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_hitrate_snapshot(ui, &self.hitrate_snapshot);
                });
            self.show_hitrate = open;
        }

        // GLASYM — Gain/loss asymmetry
        if self.show_glasym {
            if self.glasym_symbol.is_empty() {
                self.glasym_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_glasym;
            egui::Window::new("GLASYM — Gain/Loss Asymmetry")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.glasym_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.glasym_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.glasym_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_glasym(&conn, &sym_u)
                                    {
                                        self.glasym_snapshot = snap;
                                        self.glasym_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.glasym_symbol.to_uppercase();
                            self.glasym_loading = true;
                            self.glasym_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeGlasymSnapshot { symbol: sym });
                        }
                        if self.glasym_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_glasym_snapshot(ui, &self.glasym_snapshot);
                });
            self.show_glasym = open;
        }

        // VOLRATIO — Up vs down volume ratio
        if self.show_volratio {
            if self.volratio_symbol.is_empty() {
                self.volratio_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_volratio;
            egui::Window::new("VOLRATIO — Up/Down Volume Ratio")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.volratio_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.volratio_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.volratio_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_volratio(&conn, &sym_u)
                                    {
                                        self.volratio_snapshot = snap;
                                        self.volratio_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.volratio_symbol.to_uppercase();
                            self.volratio_loading = true;
                            self.volratio_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeVolratioSnapshot { symbol: sym });
                        }
                        if self.volratio_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_volratio_snapshot(ui, &self.volratio_snapshot);
                });
            self.show_volratio = open;
        }
    }
}
