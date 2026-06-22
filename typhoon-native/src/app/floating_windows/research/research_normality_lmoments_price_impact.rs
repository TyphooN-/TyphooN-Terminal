use super::*;

impl TyphooNApp {
    pub(super) fn render_research_normality_lmoments_price_impact_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "KSNORM — Kolmogorov-Smirnov Normality Test",
                default_size: [540.0, 320.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_ksnorm,
            &mut self.ksnorm_symbol,
            &mut self.ksnorm_loading,
            &mut self.ksnorm_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_ksnorm(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeKsnormSnapshot { symbol },
            super::render::render_ksnorm_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "ADTEST — Anderson-Darling Normality Test",
                default_size: [540.0, 320.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_adtest,
            &mut self.adtest_symbol,
            &mut self.adtest_loading,
            &mut self.adtest_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_adtest(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeAdtestSnapshot { symbol },
            super::render::render_adtest_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if self.show_lmom {
            if self.lmom_symbol.is_empty() {
                self.lmom_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_lmom;
            egui::Window::new("LMOM — L-Moments (Hosking 1990)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.lmom_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.lmom_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.lmom_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_lmom(&conn, &sym_u)
                                    {
                                        self.lmom_snapshot = snap;
                                        self.lmom_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.lmom_symbol.to_uppercase();
                            self.lmom_loading = true;
                            self.lmom_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeLmomSnapshot { symbol: sym });
                        }
                        if self.lmom_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_lmom_snapshot(ui, &self.lmom_snapshot);
                });
            self.show_lmom = open;
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "KYLELAM — Kyle's Price-Impact λ",
                default_size: [540.0, 320.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_kylelam,
            &mut self.kylelam_symbol,
            &mut self.kylelam_loading,
            &mut self.kylelam_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_kylelam(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeKylelamSnapshot { symbol },
            super::render::render_kylelam_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
