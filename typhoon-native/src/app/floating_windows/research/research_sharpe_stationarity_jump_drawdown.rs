use super::*;

impl TyphooNApp {
    pub(super) fn render_research_sharpe_stationarity_jump_drawdown_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if self.show_psr {
            if self.psr_symbol.is_empty() {
                self.psr_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_psr;
            egui::Window::new("PSR — Probabilistic Sharpe Ratio")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.psr_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.psr_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.psr_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_psr(&conn, &sym_u)
                                    {
                                        self.psr_snapshot = snap;
                                        self.psr_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.psr_symbol.to_uppercase();
                            self.psr_loading = true;
                            self.psr_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputePsrSnapshot { symbol: sym });
                        }
                        if self.psr_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_psr_snapshot(ui, &self.psr_snapshot);
                });
            self.show_psr = open;
        }

        if self.show_adf {
            if self.adf_symbol.is_empty() {
                self.adf_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_adf;
            egui::Window::new("ADF — Dickey-Fuller Unit-Root Test")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.adf_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.adf_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.adf_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_adf(&conn, &sym_u)
                                    {
                                        self.adf_snapshot = snap;
                                        self.adf_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.adf_symbol.to_uppercase();
                            self.adf_loading = true;
                            self.adf_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeAdfSnapshot { symbol: sym });
                        }
                        if self.adf_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_adf_snapshot(ui, &self.adf_snapshot);
                });
            self.show_adf = open;
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "MNKENDALL — Mann-Kendall Trend Test",
                default_size: [580.0, 380.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_mnkendall,
            &mut self.mnkendall_symbol,
            &mut self.mnkendall_loading,
            &mut self.mnkendall_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_mnkendall(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeMnkendallSnapshot { symbol },
            super::render::render_mnkendall_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "BIPOWER — Bipower Variation / Jump Ratio",
                default_size: [580.0, 380.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_bipower,
            &mut self.bipower_symbol,
            &mut self.bipower_loading,
            &mut self.bipower_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_bipower(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeBipowerSnapshot { symbol },
            super::render::render_bipower_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if self.show_dddur {
            if self.dddur_symbol.is_empty() {
                self.dddur_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_dddur;
            egui::Window::new("DDDUR — Drawdown Duration Statistics")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.dddur_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.dddur_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.dddur_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_dddur(&conn, &sym_u)
                                    {
                                        self.dddur_snapshot = snap;
                                        self.dddur_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.dddur_symbol.to_uppercase();
                            self.dddur_loading = true;
                            self.dddur_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeDddurSnapshot { symbol: sym });
                        }
                        if self.dddur_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_dddur_snapshot(ui, &self.dddur_snapshot);
                });
            self.show_dddur = open;
        }
    }
}
