use super::*;

impl TyphooNApp {
    pub(super) fn render_research_gap_volatility_mean_reversion_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "DRAWUP — Rally History",
                default_size: [640.0, 440.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_drawup,
            &mut self.drawup_symbol,
            &mut self.drawup_loading,
            &mut self.drawup_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_drawup(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeDrawupSnapshot { symbol },
            super::render::render_drawup_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "GAPSTATS — Overnight Gap Statistics",
                default_size: [640.0, 440.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_gapstats,
            &mut self.gapstats_symbol,
            &mut self.gapstats_loading,
            &mut self.gapstats_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_gapstats(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeGapstatsSnapshot { symbol },
            super::render::render_gapstats_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if self.show_volcluster {
            if self.volcluster_symbol.is_empty() {
                self.volcluster_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_volcluster;
            egui::Window::new("VOLCLUSTER — Volatility Clustering ACF")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.volcluster_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.volcluster_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.volcluster_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_volcluster(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.volcluster_snapshot = snap;
                                        self.volcluster_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.volcluster_symbol.to_uppercase();
                            self.volcluster_loading = true;
                            self.volcluster_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeVolclusterSnapshot { symbol: sym });
                        }
                        if self.volcluster_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_volcluster_snapshot(ui, &self.volcluster_snapshot);
                });
            self.show_volcluster = open;
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "CLOSEPLC — Close Placement in Daily Range",
                default_size: [640.0, 440.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_closeplc,
            &mut self.closeplc_symbol,
            &mut self.closeplc_loading,
            &mut self.closeplc_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_closeplc(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeCloseplcSnapshot { symbol },
            super::render::render_closeplc_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if self.show_mrhl {
            if self.mrhl_symbol.is_empty() {
                self.mrhl_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_mrhl;
            egui::Window::new("MRHL — Mean-Reversion Half-Life (AR1)")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.mrhl_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.mrhl_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.mrhl_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_mrhl(&conn, &sym_u)
                                    {
                                        self.mrhl_snapshot = snap;
                                        self.mrhl_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.mrhl_symbol.to_uppercase();
                            self.mrhl_loading = true;
                            self.mrhl_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMrhlSnapshot { symbol: sym });
                        }
                        if self.mrhl_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_mrhl_snapshot(ui, &self.mrhl_snapshot);
                });
            self.show_mrhl = open;
        }
    }
}
