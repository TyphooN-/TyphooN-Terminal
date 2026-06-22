use super::*;

impl TyphooNApp {
    pub(super) fn render_research_directional_movement_family_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research PLUS_DI / MINUS_DI / PLUS_DM / MINUS_DM / DX ──
        if self.show_plus_di_win {
            if self.plus_di_win_symbol.is_empty() {
                self.plus_di_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_plus_di_win;
            egui::Window::new("PLUS_DI — Wilder +DI (period 14)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.plus_di_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.plus_di_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.plus_di_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_plus_di(&conn, &sym_u)
                                    {
                                        self.plus_di_win_snapshot = snap;
                                        self.plus_di_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.plus_di_win_symbol.to_uppercase();
                            self.plus_di_win_loading = true;
                            self.plus_di_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputePlusDiSnapshot { symbol: sym });
                        }
                        if self.plus_di_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_plus_di_snapshot(ui, &self.plus_di_win_snapshot);
                });
            self.show_plus_di_win = open;
        }

        if self.show_minus_di_win {
            if self.minus_di_win_symbol.is_empty() {
                self.minus_di_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_minus_di_win;
            egui::Window::new("MINUS_DI — Wilder −DI (period 14)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.minus_di_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.minus_di_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.minus_di_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_minus_di(&conn, &sym_u)
                                    {
                                        self.minus_di_win_snapshot = snap;
                                        self.minus_di_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.minus_di_win_symbol.to_uppercase();
                            self.minus_di_win_loading = true;
                            self.minus_di_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMinusDiSnapshot { symbol: sym });
                        }
                        if self.minus_di_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_minus_di_snapshot(ui, &self.minus_di_win_snapshot);
                });
            self.show_minus_di_win = open;
        }

        if self.show_plus_dm_win {
            if self.plus_dm_win_symbol.is_empty() {
                self.plus_dm_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_plus_dm_win;
            egui::Window::new("PLUS_DM — Wilder raw +DM (period 14)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.plus_dm_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.plus_dm_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.plus_dm_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_plus_dm(&conn, &sym_u)
                                    {
                                        self.plus_dm_win_snapshot = snap;
                                        self.plus_dm_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.plus_dm_win_symbol.to_uppercase();
                            self.plus_dm_win_loading = true;
                            self.plus_dm_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputePlusDmSnapshot { symbol: sym });
                        }
                        if self.plus_dm_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_plus_dm_snapshot(ui, &self.plus_dm_win_snapshot);
                });
            self.show_plus_dm_win = open;
        }

        if self.show_minus_dm_win {
            if self.minus_dm_win_symbol.is_empty() {
                self.minus_dm_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_minus_dm_win;
            egui::Window::new("MINUS_DM — Wilder raw −DM (period 14)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.minus_dm_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.minus_dm_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.minus_dm_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_minus_dm(&conn, &sym_u)
                                    {
                                        self.minus_dm_win_snapshot = snap;
                                        self.minus_dm_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.minus_dm_win_symbol.to_uppercase();
                            self.minus_dm_win_loading = true;
                            self.minus_dm_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMinusDmSnapshot { symbol: sym });
                        }
                        if self.minus_dm_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_minus_dm_snapshot(ui, &self.minus_dm_win_snapshot);
                });
            self.show_minus_dm_win = open;
        }

        if self.show_dx_win {
            if self.dx_win_symbol.is_empty() {
                self.dx_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_dx_win;
            egui::Window::new("DX — Wilder Directional Movement Index (period 14)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.dx_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.dx_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.dx_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_dx(&conn, &sym_u)
                                    {
                                        self.dx_win_snapshot = snap;
                                        self.dx_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.dx_win_symbol.to_uppercase();
                            self.dx_win_loading = true;
                            self.dx_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeDxSnapshot { symbol: sym });
                        }
                        if self.dx_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_dx_snapshot(ui, &self.dx_win_snapshot);
                });
            self.show_dx_win = open;
        }
    }
}
