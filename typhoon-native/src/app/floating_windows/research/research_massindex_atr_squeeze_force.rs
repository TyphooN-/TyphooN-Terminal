use super::*;

impl TyphooNApp {
    pub(super) fn render_research_massindex_atr_squeeze_force_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if self.show_mass_index_win {
            if self.mass_index_win_symbol.is_empty() {
                self.mass_index_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_mass_index_win;
            egui::Window::new("MASSINDEX — Dorsey Mass Index (EMA/EMA ratio, reversal bulge)")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.mass_index_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.mass_index_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.mass_index_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_mass_index(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.mass_index_win_snapshot = snap;
                                        self.mass_index_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.mass_index_win_symbol.to_uppercase();
                            self.mass_index_win_loading = true;
                            self.mass_index_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMassIndexSnapshot { symbol: sym });
                        }
                        if self.mass_index_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_mass_index_snapshot(ui, &self.mass_index_win_snapshot);
                });
            self.show_mass_index_win = open;
        }

        if self.show_natr_win {
            if self.natr_win_symbol.is_empty() {
                self.natr_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_natr_win;
            egui::Window::new("NATR — Normalized ATR (TA-Lib, 100 × ATR / close)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.natr_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.natr_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.natr_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_natr(&conn, &sym_u)
                                    {
                                        self.natr_win_snapshot = snap;
                                        self.natr_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.natr_win_symbol.to_uppercase();
                            self.natr_win_loading = true;
                            self.natr_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeNatrSnapshot { symbol: sym });
                        }
                        if self.natr_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_natr_snapshot(ui, &self.natr_win_snapshot);
                });
            self.show_natr_win = open;
        }

        if self.show_ttm_squeeze_win {
            if self.ttm_squeeze_win_symbol.is_empty() {
                self.ttm_squeeze_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ttm_squeeze_win;
            egui::Window::new("TTM_SQUEEZE — Carter's BB ⊂ KC Regime + Momentum (20)")
                .open(&mut open)
                .resizable(true)
                .default_size([600.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ttm_squeeze_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ttm_squeeze_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ttm_squeeze_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ttm_squeeze(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.ttm_squeeze_win_snapshot = snap;
                                        self.ttm_squeeze_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ttm_squeeze_win_symbol.to_uppercase();
                            self.ttm_squeeze_win_loading = true;
                            self.ttm_squeeze_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeTtmSqueezeSnapshot { symbol: sym });
                        }
                        if self.ttm_squeeze_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_ttm_squeeze_snapshot(ui, &self.ttm_squeeze_win_snapshot);
                });
            self.show_ttm_squeeze_win = open;
        }

        if self.show_force_index_win {
            if self.force_index_win_symbol.is_empty() {
                self.force_index_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_force_index_win;
            egui::Window::new("FORCE_INDEX — Elder Force Index (EMA of volume × Δclose, 13)")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.force_index_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.force_index_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.force_index_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_force_index(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.force_index_win_snapshot = snap;
                                        self.force_index_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.force_index_win_symbol.to_uppercase();
                            self.force_index_win_loading = true;
                            self.force_index_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeForceIndexSnapshot { symbol: sym });
                        }
                        if self.force_index_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_force_index_snapshot(ui, &self.force_index_win_snapshot);
                });
            self.show_force_index_win = open;
        }

        if self.show_trange_win {
            if self.trange_win_symbol.is_empty() {
                self.trange_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_trange_win;
            egui::Window::new("TRANGE — True Range (raw, single-bar, gap-aware)")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.trange_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.trange_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.trange_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_trange(&conn, &sym_u)
                                    {
                                        self.trange_win_snapshot = snap;
                                        self.trange_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.trange_win_symbol.to_uppercase();
                            self.trange_win_loading = true;
                            self.trange_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeTrangeSnapshot { symbol: sym });
                        }
                        if self.trange_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_trange_snapshot(ui, &self.trange_win_snapshot);
                });
            self.show_trange_win = open;
        }
    }
}
