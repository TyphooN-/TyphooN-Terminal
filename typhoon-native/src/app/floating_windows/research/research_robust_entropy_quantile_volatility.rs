use super::*;

impl TyphooNApp {
    pub(super) fn render_research_robust_entropy_quantile_volatility_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "ROBVOL — Robust Volatility",
                default_size: [520.0, 300.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_robvol,
            &mut self.robvol_symbol,
            &mut self.robvol_loading,
            &mut self.robvol_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_robvol(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeRobvolSnapshot { symbol },
            super::render::render_robvol_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "RENYIENT — Rényi Entropy (α=2)",
                default_size: [520.0, 300.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_renyient,
            &mut self.renyient_symbol,
            &mut self.renyient_loading,
            &mut self.renyient_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_renyient(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeRenyientSnapshot { symbol },
            super::render::render_renyient_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "RETQUANT — Return Quantile Profile",
                default_size: [600.0, 360.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_retquant,
            &mut self.retquant_symbol,
            &mut self.retquant_loading,
            &mut self.retquant_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_retquant(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeRetquantSnapshot { symbol },
            super::render::render_retquant_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if self.show_msent {
            if self.msent_symbol.is_empty() {
                self.msent_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_msent;
            egui::Window::new("MSENT — Multiscale Entropy")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.msent_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.msent_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.msent_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_msent(&conn, &sym_u)
                                    {
                                        self.msent_snapshot = snap;
                                        self.msent_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.msent_symbol.to_uppercase();
                            self.msent_loading = true;
                            self.msent_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMsentSnapshot { symbol: sym });
                        }
                        if self.msent_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_msent_snapshot(ui, &self.msent_snapshot);
                });
            self.show_msent = open;
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "EWMAVOL — EWMA Volatility (RiskMetrics)",
                default_size: [520.0, 300.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_ewmavol,
            &mut self.ewmavol_symbol,
            &mut self.ewmavol_loading,
            &mut self.ewmavol_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_ewmavol(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeEwmavolSnapshot { symbol },
            super::render::render_ewmavol_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
