use super::*;

impl TyphooNApp {
    pub(super) fn render_research_garch_bubble_dimension_information_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "GARCH11 — GARCH(1,1) Conditional Volatility Fit",
                default_size: [560.0, 340.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_garch11,
            &mut self.garch11_symbol,
            &mut self.garch11_loading,
            &mut self.garch11_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_garch11(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeGarch11Snapshot { symbol },
            super::render::render_garch11_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if self.show_sadf {
            if self.sadf_symbol.is_empty() {
                self.sadf_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_sadf;
            egui::Window::new("SADF — Phillips-Wu-Yu Sup-ADF Bubble Test")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.sadf_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.sadf_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.sadf_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_sadf(&conn, &sym_u)
                                    {
                                        self.sadf_snapshot = snap;
                                        self.sadf_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.sadf_symbol.to_uppercase();
                            self.sadf_loading = true;
                            self.sadf_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeSadfSnapshot { symbol: sym });
                        }
                        if self.sadf_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_sadf_snapshot(ui, &self.sadf_snapshot);
                });
            self.show_sadf = open;
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "CORDIM — Grassberger-Procaccia Correlation Dimension D2",
                default_size: [540.0, 300.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_cordim,
            &mut self.cordim_symbol,
            &mut self.cordim_loading,
            &mut self.cordim_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_cordim(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeCordimSnapshot { symbol },
            super::render::render_cordim_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "SKSPEC — Rolling-Window Skewness Spectrum",
                default_size: [560.0, 320.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_skspec,
            &mut self.skspec_symbol,
            &mut self.skspec_loading,
            &mut self.skspec_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_skspec(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeSkspecSnapshot { symbol },
            super::render::render_skspec_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "AUTOMI — Auto Mutual Information (Info-Theoretic ACF)",
                default_size: [540.0, 300.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_automi,
            &mut self.automi_symbol,
            &mut self.automi_loading,
            &mut self.automi_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_automi(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeAutomiSnapshot { symbol },
            super::render::render_automi_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
