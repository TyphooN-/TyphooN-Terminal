use super::*;

impl TyphooNApp {
    pub(super) fn render_research_autocorrelation_hurst_volume_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──

        // AUTOCOR — Autocorrelation at multiple lags
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "AUTOCOR — Return Autocorrelation",
                default_size: [640.0, 420.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_autocor,
            &mut self.autocor_symbol,
            &mut self.autocor_loading,
            &mut self.autocor_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_autocor(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeAutocorSnapshot { symbol },
            super::render::render_autocor_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // HURST — Hurst exponent via R/S
        if self.show_hurst {
            if self.hurst_symbol.is_empty() {
                self.hurst_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_hurst;
            egui::Window::new("HURST — Hurst Exponent (R/S)")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.hurst_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.hurst_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.hurst_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_hurst(&conn, &sym_u)
                                    {
                                        self.hurst_snapshot = snap;
                                        self.hurst_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.hurst_symbol.to_uppercase();
                            self.hurst_loading = true;
                            self.hurst_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeHurstSnapshot { symbol: sym });
                        }
                        if self.hurst_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_hurst_snapshot(ui, &self.hurst_snapshot);
                });
            self.show_hurst = open;
        }

        // HITRATE — Multi-horizon hit rate
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "HITRATE — Multi-Horizon Win Rate",
                default_size: [640.0, 420.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_hitrate,
            &mut self.hitrate_symbol,
            &mut self.hitrate_loading,
            &mut self.hitrate_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_hitrate(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeHitrateSnapshot { symbol },
            super::render::render_hitrate_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // GLASYM — Gain/loss asymmetry
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "GLASYM — Gain/Loss Asymmetry",
                default_size: [640.0, 420.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_glasym,
            &mut self.glasym_symbol,
            &mut self.glasym_loading,
            &mut self.glasym_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_glasym(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeGlasymSnapshot { symbol },
            super::render::render_glasym_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // VOLRATIO — Up vs down volume ratio
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "VOLRATIO — Up/Down Volume Ratio",
                default_size: [640.0, 440.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_volratio,
            &mut self.volratio_symbol,
            &mut self.volratio_loading,
            &mut self.volratio_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_volratio(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeVolratioSnapshot { symbol },
            super::render::render_volratio_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
