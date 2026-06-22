use super::*;

impl TyphooNApp {
    pub(super) fn render_research_jump_unitroot_multifractal_tsi_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "BNSJUMP — Barndorff-Nielsen-Shephard Jump-Test Z",
                default_size: [560.0, 320.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_bnsjump,
            &mut self.bnsjump_symbol,
            &mut self.bnsjump_loading,
            &mut self.bnsjump_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_bnsjump(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeBnsjumpSnapshot { symbol },
            super::render::render_bnsjump_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "PPROOT — Phillips-Perron Unit-Root Test",
                default_size: [560.0, 320.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_pproot,
            &mut self.pproot_symbol,
            &mut self.pproot_loading,
            &mut self.pproot_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_pproot(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputePprootSnapshot { symbol },
            super::render::render_pproot_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if self.show_mfdfa {
            if self.mfdfa_symbol.is_empty() {
                self.mfdfa_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_mfdfa;
            egui::Window::new("MFDFA — Multifractal DFA (q ∈ {-2, 0, +2})")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.mfdfa_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.mfdfa_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.mfdfa_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_mfdfa(&conn, &sym_u)
                                    {
                                        self.mfdfa_snapshot = snap;
                                        self.mfdfa_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.mfdfa_symbol.to_uppercase();
                            self.mfdfa_loading = true;
                            self.mfdfa_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMfdfaSnapshot { symbol: sym });
                        }
                        if self.mfdfa_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_mfdfa_snapshot(ui, &self.mfdfa_snapshot);
                });
            self.show_mfdfa = open;
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "HILLKS — Hill-Tail KS Goodness-of-Fit",
                default_size: [540.0, 300.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_hillks,
            &mut self.hillks_symbol,
            &mut self.hillks_loading,
            &mut self.hillks_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_hillks(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeHillksSnapshot { symbol },
            super::render::render_hillks_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if self.show_tsi {
            if self.tsi_symbol.is_empty() {
                self.tsi_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_tsi;
            egui::Window::new("TSI — True Strength Index (Blau 1991)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.tsi_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.tsi_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.tsi_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_tsi(&conn, &sym_u)
                                    {
                                        self.tsi_snapshot = snap;
                                        self.tsi_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.tsi_symbol.to_uppercase();
                            self.tsi_loading = true;
                            self.tsi_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeTsiSnapshot { symbol: sym });
                        }
                        if self.tsi_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_tsi_snapshot(ui, &self.tsi_snapshot);
                });
            self.show_tsi = open;
        }
    }
}
