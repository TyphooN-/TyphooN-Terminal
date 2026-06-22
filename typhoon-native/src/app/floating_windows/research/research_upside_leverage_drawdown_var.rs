use super::*;

impl TyphooNApp {
    pub(super) fn render_research_upside_leverage_drawdown_var_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if self.show_upr {
            if self.upr_symbol.is_empty() {
                self.upr_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_upr;
            egui::Window::new("UPR — Upside Potential Ratio")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.upr_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.upr_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.upr_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_upr(&conn, &sym_u)
                                    {
                                        self.upr_snapshot = snap;
                                        self.upr_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.upr_symbol.to_uppercase();
                            self.upr_loading = true;
                            self.upr_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeUprSnapshot { symbol: sym });
                        }
                        if self.upr_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_upr_snapshot(ui, &self.upr_snapshot);
                });
            self.show_upr = open;
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "LEVEREFF — Leverage Effect",
                default_size: [560.0, 320.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_levereff,
            &mut self.levereff_symbol,
            &mut self.levereff_loading,
            &mut self.levereff_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_levereff(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeLevereffSnapshot { symbol },
            super::render::render_levereff_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "DRAWDAR — Drawdown-at-Risk",
                default_size: [560.0, 350.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_drawdar,
            &mut self.drawdar_symbol,
            &mut self.drawdar_loading,
            &mut self.drawdar_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_drawdar(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeDrawdarSnapshot { symbol },
            super::render::render_drawdar_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "VARHALF — Volatility Half-Life",
                default_size: [520.0, 300.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_varhalf,
            &mut self.varhalf_symbol,
            &mut self.varhalf_loading,
            &mut self.varhalf_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_varhalf(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeVarhalfSnapshot { symbol },
            super::render::render_varhalf_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if self.show_gini {
            if self.gini_symbol.is_empty() {
                self.gini_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_gini;
            egui::Window::new("GINI — Return Concentration")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.gini_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.gini_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.gini_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_gini(&conn, &sym_u)
                                    {
                                        self.gini_snapshot = snap;
                                        self.gini_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.gini_symbol.to_uppercase();
                            self.gini_loading = true;
                            self.gini_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeGiniSnapshot { symbol: sym });
                        }
                        if self.gini_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_gini_snapshot(ui, &self.gini_snapshot);
                });
            self.show_gini = open;
        }
    }
}
