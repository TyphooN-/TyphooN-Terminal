use super::*;

impl TyphooNApp {
    pub(super) fn render_research_tail_arch_pain_structural_var_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if self.show_hilltail {
            if self.hilltail_symbol.is_empty() {
                self.hilltail_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_hilltail;
            egui::Window::new("HILLTAIL — Hill Tail-Index Estimator")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.hilltail_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.hilltail_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.hilltail_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_hilltail(&conn, &sym_u)
                                    {
                                        self.hilltail_snapshot = snap;
                                        self.hilltail_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.hilltail_symbol.to_uppercase();
                            self.hilltail_loading = true;
                            self.hilltail_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeHilltailSnapshot { symbol: sym });
                        }
                        if self.hilltail_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_hilltail_snapshot(ui, &self.hilltail_snapshot);
                });
            self.show_hilltail = open;
        }

        if self.show_archlm {
            if self.archlm_symbol.is_empty() {
                self.archlm_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_archlm;
            egui::Window::new("ARCHLM — Engle ARCH-LM Test")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.archlm_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.archlm_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.archlm_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_archlm(&conn, &sym_u)
                                    {
                                        self.archlm_snapshot = snap;
                                        self.archlm_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.archlm_symbol.to_uppercase();
                            self.archlm_loading = true;
                            self.archlm_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeArchlmSnapshot { symbol: sym });
                        }
                        if self.archlm_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_archlm_snapshot(ui, &self.archlm_snapshot);
                });
            self.show_archlm = open;
        }

        if self.show_painratio {
            if self.painratio_symbol.is_empty() {
                self.painratio_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_painratio;
            egui::Window::new("PAINRATIO — Pain Index + Pain Ratio")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 360.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.painratio_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.painratio_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.painratio_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_painratio(&conn, &sym_u)
                                    {
                                        self.painratio_snapshot = snap;
                                        self.painratio_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.painratio_symbol.to_uppercase();
                            self.painratio_loading = true;
                            self.painratio_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputePainratioSnapshot { symbol: sym });
                        }
                        if self.painratio_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_painratio_snapshot(ui, &self.painratio_snapshot);
                });
            self.show_painratio = open;
        }

        if self.show_cusum {
            if self.cusum_symbol.is_empty() {
                self.cusum_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cusum;
            egui::Window::new("CUSUM — Brown-Durbin-Evans Structural Break Test")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cusum_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.cusum_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.cusum_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_cusum(&conn, &sym_u)
                                    {
                                        self.cusum_snapshot = snap;
                                        self.cusum_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cusum_symbol.to_uppercase();
                            self.cusum_loading = true;
                            self.cusum_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCusumSnapshot { symbol: sym });
                        }
                        if self.cusum_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_cusum_snapshot(ui, &self.cusum_snapshot);
                });
            self.show_cusum = open;
        }

        if self.show_cfvar {
            if self.cfvar_symbol.is_empty() {
                self.cfvar_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cfvar;
            egui::Window::new("CFVAR — Cornish-Fisher Modified VaR")
                .open(&mut open)
                .resizable(true)
                .default_size([620.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cfvar_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.cfvar_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.cfvar_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_cfvar(&conn, &sym_u)
                                    {
                                        self.cfvar_snapshot = snap;
                                        self.cfvar_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cfvar_symbol.to_uppercase();
                            self.cfvar_loading = true;
                            self.cfvar_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCfvarSnapshot { symbol: sym });
                        }
                        if self.cfvar_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_cfvar_snapshot(ui, &self.cfvar_snapshot);
                });
            self.show_cfvar = open;
        }
    }
}
