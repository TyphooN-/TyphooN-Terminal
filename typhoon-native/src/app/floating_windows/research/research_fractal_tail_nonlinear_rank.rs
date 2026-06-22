use super::*;

impl TyphooNApp {
    pub(super) fn render_research_fractal_tail_nonlinear_rank_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if self.show_higuchi {
            if self.higuchi_symbol.is_empty() {
                self.higuchi_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_higuchi;
            egui::Window::new("HIGUCHI — Higuchi Fractal Dimension (1988)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.higuchi_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.higuchi_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.higuchi_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_higuchi(&conn, &sym_u)
                                    {
                                        self.higuchi_snapshot = snap;
                                        self.higuchi_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.higuchi_symbol.to_uppercase();
                            self.higuchi_loading = true;
                            self.higuchi_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeHiguchiSnapshot { symbol: sym });
                        }
                        if self.higuchi_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_higuchi_snapshot(ui, &self.higuchi_snapshot);
                });
            self.show_higuchi = open;
        }

        if self.show_pickands {
            if self.pickands_symbol.is_empty() {
                self.pickands_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_pickands;
            egui::Window::new("PICKANDS — Pickands 1975 Tail-Index Estimator")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.pickands_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.pickands_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.pickands_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_pickands(&conn, &sym_u)
                                    {
                                        self.pickands_snapshot = snap;
                                        self.pickands_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.pickands_symbol.to_uppercase();
                            self.pickands_loading = true;
                            self.pickands_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputePickandsSnapshot { symbol: sym });
                        }
                        if self.pickands_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_pickands_snapshot(ui, &self.pickands_snapshot);
                });
            self.show_pickands = open;
        }

        if self.show_kappa3 {
            if self.kappa3_symbol.is_empty() {
                self.kappa3_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_kappa3;
            egui::Window::new("KAPPA3 — Kaplan-Knowles 2004 Kappa-3 Ratio")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.kappa3_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.kappa3_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.kappa3_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_kappa3(&conn, &sym_u)
                                    {
                                        self.kappa3_snapshot = snap;
                                        self.kappa3_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.kappa3_symbol.to_uppercase();
                            self.kappa3_loading = true;
                            self.kappa3_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeKappa3Snapshot { symbol: sym });
                        }
                        if self.kappa3_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_kappa3_snapshot(ui, &self.kappa3_snapshot);
                });
            self.show_kappa3 = open;
        }

        if self.show_lyapunov {
            if self.lyapunov_symbol.is_empty() {
                self.lyapunov_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_lyapunov;
            egui::Window::new("LYAPUNOV — Largest Lyapunov Exponent (Rosenstein 1993)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.lyapunov_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.lyapunov_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.lyapunov_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_lyapunov(&conn, &sym_u)
                                    {
                                        self.lyapunov_snapshot = snap;
                                        self.lyapunov_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.lyapunov_symbol.to_uppercase();
                            self.lyapunov_loading = true;
                            self.lyapunov_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeLyapunovSnapshot { symbol: sym });
                        }
                        if self.lyapunov_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_lyapunov_snapshot(ui, &self.lyapunov_snapshot);
                });
            self.show_lyapunov = open;
        }

        if self.show_rankac {
            if self.rankac_symbol.is_empty() {
                self.rankac_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_rankac;
            egui::Window::new("RANKAC — Spearman Rank Autocorrelation (lags 1/5/10)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.rankac_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.rankac_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.rankac_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_rankac(&conn, &sym_u)
                                    {
                                        self.rankac_snapshot = snap;
                                        self.rankac_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.rankac_symbol.to_uppercase();
                            self.rankac_loading = true;
                            self.rankac_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRankacSnapshot { symbol: sym });
                        }
                        if self.rankac_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_rankac_snapshot(ui, &self.rankac_snapshot);
                });
            self.show_rankac = open;
        }
    }
}
