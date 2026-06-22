use super::*;

impl TyphooNApp {
    pub(super) fn render_research_entropy_tail_autocorrelation_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "ENTROPY — Shannon Return Entropy",
                default_size: [520.0, 300.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_entropy,
            &mut self.entropy_symbol,
            &mut self.entropy_loading,
            &mut self.entropy_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_entropy(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeEntropySnapshot { symbol },
            super::render::render_entropy_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "RACHEV — Conditional Tail Expectation Ratio",
                default_size: [560.0, 350.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_rachev,
            &mut self.rachev_symbol,
            &mut self.rachev_loading,
            &mut self.rachev_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_rachev(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeRachevSnapshot { symbol },
            super::render::render_rachev_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if self.show_gpr {
            if self.gpr_symbol.is_empty() {
                self.gpr_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_gpr;
            egui::Window::new("GPR — Gain-to-Pain Ratio")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 350.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.gpr_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.gpr_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.gpr_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_gpr(&conn, &sym_u)
                                    {
                                        self.gpr_snapshot = snap;
                                        self.gpr_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.gpr_symbol.to_uppercase();
                            self.gpr_loading = true;
                            self.gpr_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeGprSnapshot { symbol: sym });
                        }
                        if self.gpr_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_gpr_snapshot(ui, &self.gpr_snapshot);
                });
            self.show_gpr = open;
        }

        if self.show_pacf {
            if self.pacf_symbol.is_empty() {
                self.pacf_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_pacf;
            egui::Window::new("PACF — Partial Autocorrelation")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.pacf_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.pacf_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.pacf_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_pacf(&conn, &sym_u)
                                    {
                                        self.pacf_snapshot = snap;
                                        self.pacf_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.pacf_symbol.to_uppercase();
                            self.pacf_loading = true;
                            self.pacf_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputePacfSnapshot { symbol: sym });
                        }
                        if self.pacf_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_pacf_snapshot(ui, &self.pacf_snapshot);
                });
            self.show_pacf = open;
        }

        if self.show_apen {
            if self.apen_symbol.is_empty() {
                self.apen_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_apen;
            egui::Window::new("APEN — Approximate Entropy")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.apen_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.apen_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.apen_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_apen(&conn, &sym_u)
                                    {
                                        self.apen_snapshot = snap;
                                        self.apen_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.apen_symbol.to_uppercase();
                            self.apen_loading = true;
                            self.apen_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeApenSnapshot { symbol: sym });
                        }
                        if self.apen_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_apen_snapshot(ui, &self.apen_snapshot);
                });
            self.show_apen = open;
        }
    }
}
