use super::*;

impl TyphooNApp {
    pub(super) fn render_research_oscillator_price_momentum_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if self.show_ppo_win {
            if self.ppo_win_symbol.is_empty() {
                self.ppo_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ppo_win;
            egui::Window::new("PPO — Percentage Price Oscillator (12/26/9)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ppo_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ppo_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ppo_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ppo(&conn, &sym_u)
                                    {
                                        self.ppo_win_snapshot = snap;
                                        self.ppo_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ppo_win_symbol.to_uppercase();
                            self.ppo_win_loading = true;
                            self.ppo_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputePpoSnapshot { symbol: sym });
                        }
                        if self.ppo_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_ppo_snapshot(ui, &self.ppo_win_snapshot);
                });
            self.show_ppo_win = open;
        }

        if self.show_dpo_win {
            if self.dpo_win_symbol.is_empty() {
                self.dpo_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_dpo_win;
            egui::Window::new("DPO — Detrended Price Oscillator (20)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.dpo_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.dpo_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.dpo_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_dpo(&conn, &sym_u)
                                    {
                                        self.dpo_win_snapshot = snap;
                                        self.dpo_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.dpo_win_symbol.to_uppercase();
                            self.dpo_win_loading = true;
                            self.dpo_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeDpoSnapshot { symbol: sym });
                        }
                        if self.dpo_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_dpo_snapshot(ui, &self.dpo_win_snapshot);
                });
            self.show_dpo_win = open;
        }

        if self.show_kst_win {
            if self.kst_win_symbol.is_empty() {
                self.kst_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_kst_win;
            egui::Window::new("KST — Know Sure Thing (Pring, 1992)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.kst_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.kst_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.kst_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_kst(&conn, &sym_u)
                                    {
                                        self.kst_win_snapshot = snap;
                                        self.kst_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.kst_win_symbol.to_uppercase();
                            self.kst_win_loading = true;
                            self.kst_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeKstSnapshot { symbol: sym });
                        }
                        if self.kst_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_kst_snapshot(ui, &self.kst_win_snapshot);
                });
            self.show_kst_win = open;
        }

        if self.show_ultosc_win {
            if self.ultosc_win_symbol.is_empty() {
                self.ultosc_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ultosc_win;
            egui::Window::new("ULTOSC — Ultimate Oscillator (7/14/28)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ultosc_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ultosc_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ultosc_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ultosc(&conn, &sym_u)
                                    {
                                        self.ultosc_win_snapshot = snap;
                                        self.ultosc_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ultosc_win_symbol.to_uppercase();
                            self.ultosc_win_loading = true;
                            self.ultosc_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeUltoscSnapshot { symbol: sym });
                        }
                        if self.ultosc_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_ultosc_snapshot(ui, &self.ultosc_win_snapshot);
                });
            self.show_ultosc_win = open;
        }

        if self.show_willr_win {
            if self.willr_win_symbol.is_empty() {
                self.willr_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_willr_win;
            egui::Window::new("WILLR — Williams %R (14)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.willr_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.willr_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.willr_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_willr(&conn, &sym_u)
                                    {
                                        self.willr_win_snapshot = snap;
                                        self.willr_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.willr_win_symbol.to_uppercase();
                            self.willr_win_loading = true;
                            self.willr_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeWillrSnapshot { symbol: sym });
                        }
                        if self.willr_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_willr_snapshot(ui, &self.willr_win_snapshot);
                });
            self.show_willr_win = open;
        }
    }
}
