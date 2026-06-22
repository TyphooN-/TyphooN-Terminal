use super::*;

impl TyphooNApp {
    pub(super) fn render_research_calmar_ulcer_liquidity_normality_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "CALMAR — Calmar Ratio (Return / Max Drawdown)",
                default_size: [640.0, 440.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_calmar,
            &mut self.calmar_symbol,
            &mut self.calmar_loading,
            &mut self.calmar_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_calmar(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeCalmarSnapshot { symbol },
            super::render::render_calmar_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if self.show_ulcer {
            if self.ulcer_symbol.is_empty() {
                self.ulcer_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ulcer;
            egui::Window::new("ULCER — Ulcer Index + Martin Ratio (UPI)")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ulcer_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ulcer_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ulcer_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ulcer(&conn, &sym_u)
                                    {
                                        self.ulcer_snapshot = snap;
                                        self.ulcer_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ulcer_symbol.to_uppercase();
                            self.ulcer_loading = true;
                            self.ulcer_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeUlcerSnapshot { symbol: sym });
                        }
                        if self.ulcer_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_ulcer_snapshot(ui, &self.ulcer_snapshot);
                });
            self.show_ulcer = open;
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "VARRATIO — Lo-MacKinlay Variance Ratio",
                default_size: [640.0, 440.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_varratio,
            &mut self.varratio_symbol,
            &mut self.varratio_loading,
            &mut self.varratio_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_varratio(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeVarratioSnapshot { symbol },
            super::render::render_varratio_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "AMIHUD — Amihud Illiquidity Ratio",
                default_size: [640.0, 440.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_amihud,
            &mut self.amihud_symbol,
            &mut self.amihud_loading,
            &mut self.amihud_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_amihud(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeAmihudSnapshot { symbol },
            super::render::render_amihud_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "JBNORM — Jarque-Bera Normality Test",
                default_size: [640.0, 440.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_jbnorm,
            &mut self.jbnorm_symbol,
            &mut self.jbnorm_loading,
            &mut self.jbnorm_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_jbnorm(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeJbnormSnapshot { symbol },
            super::render::render_jbnorm_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
