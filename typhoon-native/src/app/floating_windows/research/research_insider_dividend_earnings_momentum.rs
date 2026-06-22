use super::*;

impl TyphooNApp {
    pub(super) fn render_research_insider_dividend_earnings_momentum_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // MNGR — Insider Activity Bias
        if self.show_mngr {
            if self.mngr_symbol.is_empty() {
                self.mngr_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_mngr;
            egui::Window::new("MNGR — Insider Activity Bias")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.mngr_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.mngr_symbol = chart_sym_research.clone();
                        }
                        ui.label(egui::RichText::new("Window (days):").color(AXIS_TEXT));
                        ui.add(egui::DragValue::new(&mut self.mngr_window_days).range(30..=365));
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.mngr_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_insider_activity(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.mngr_snapshot = snap;
                                        self.mngr_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.mngr_symbol.to_uppercase();
                            self.mngr_loading = true;
                            self.mngr_symbol = sym.clone();
                            let _ =
                                self.broker_tx
                                    .send(BrokerCmd::ComputeInsiderActivitySnapshot {
                                        symbol: sym,
                                        window_days: self.mngr_window_days,
                                    });
                        }
                        if self.mngr_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_mngr_snapshot(ui, &self.mngr_snapshot);
                });
            self.show_mngr = open;
        }

        // DIVG — Dividend Growth Analysis
        if self.show_divg {
            if self.divg_symbol.is_empty() {
                self.divg_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_divg;
            egui::Window::new("DIVG — Dividend Growth Analysis")
                .open(&mut open)
                .resizable(true)
                .default_size([600.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.divg_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.divg_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.divg_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_divg(&conn, &sym_u)
                                    {
                                        self.divg_snapshot = snap;
                                        self.divg_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.divg_symbol.to_uppercase();
                            self.divg_loading = true;
                            self.divg_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeDivgSnapshot { symbol: sym });
                        }
                        if self.divg_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_divg_snapshot(ui, &self.divg_snapshot);
                });
            self.show_divg = open;
        }

        // EARM — Earnings Momentum Trend
        if self.show_earm {
            if self.earm_symbol.is_empty() {
                self.earm_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_earm;
            egui::Window::new("EARM — Earnings Momentum Trend")
                .open(&mut open)
                .resizable(true)
                .default_size([620.0, 460.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.earm_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.earm_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.earm_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_earm(&conn, &sym_u)
                                    {
                                        self.earm_snapshot = snap;
                                        self.earm_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.earm_symbol.to_uppercase();
                            self.earm_loading = true;
                            self.earm_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeEarmSnapshot { symbol: sym });
                        }
                        if self.earm_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_earm_snapshot(ui, &self.earm_snapshot);
                });
            self.show_earm = open;
        }

        // SECTR — Sector Rotation Strength
        if self.show_sectr {
            if self.sectr_symbol.is_empty() {
                self.sectr_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_sectr;
            egui::Window::new("SECTR — Sector Rotation Strength")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.sectr_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.sectr_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.sectr_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_sector_rotation(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.sectr_snapshot = snap;
                                        self.sectr_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.sectr_symbol.to_uppercase();
                            self.sectr_loading = true;
                            self.sectr_symbol = sym.clone();
                            let symbol_sector = if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    if let Ok(Some(fa)) =
                                        typhoon_engine::core::fundamentals::get_fundamentals(
                                            &conn, &sym,
                                        )
                                    {
                                        fa.sector
                                    } else {
                                        String::new()
                                    }
                                } else {
                                    String::new()
                                }
                            } else {
                                String::new()
                            };
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeSectorRotationSnapshot {
                                    symbol: sym,
                                    symbol_sector,
                                });
                        }
                        if self.sectr_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_sectr_snapshot(ui, &self.sectr_snapshot);
                });
            self.show_sectr = open;
        }

        // UPDM — Upgrade/Downgrade Momentum
        if self.show_updm {
            if self.updm_symbol.is_empty() {
                self.updm_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_updm;
            egui::Window::new("UPDM — Upgrade/Downgrade Momentum")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.updm_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.updm_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.updm_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_updm(&conn, &sym_u)
                                    {
                                        self.updm_snapshot = snap;
                                        self.updm_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.updm_symbol.to_uppercase();
                            self.updm_loading = true;
                            self.updm_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeUpdmSnapshot { symbol: sym });
                        }
                        if self.updm_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_updm_snapshot(ui, &self.updm_snapshot);
                });
            self.show_updm = open;
        }
    }
}
