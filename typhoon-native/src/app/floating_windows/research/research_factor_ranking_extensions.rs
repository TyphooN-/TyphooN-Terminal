use super::*;

impl TyphooNApp {
    pub(super) fn render_research_factor_ranking_extensions_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        // SIZEF — Size Factor Rank vs Sector Peers
        if self.show_sizef {
            if self.sizef_symbol.is_empty() {
                self.sizef_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_sizef;
            egui::Window::new("SIZEF — Size Factor Rank vs Sector Peers")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 360.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.sizef_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.sizef_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.sizef_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_sizef(&conn, &sym_u)
                                    {
                                        self.sizef_snapshot = snap;
                                        self.sizef_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.sizef_symbol.to_uppercase();
                            self.sizef_loading = true;
                            self.sizef_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeSizefSnapshot { symbol: sym });
                        }
                        if self.sizef_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_sizef_snapshot(ui, &self.sizef_snapshot);
                });
            self.show_sizef = open;
        }

        // MOMF — Momentum Factor Rank vs Sector Peers
        if self.show_momf {
            if self.momf_symbol.is_empty() {
                self.momf_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_momf;
            egui::Window::new("MOMF — Momentum Factor Rank vs Sector Peers")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 360.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.momf_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.momf_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.momf_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_momf(&conn, &sym_u)
                                    {
                                        self.momf_snapshot = snap;
                                        self.momf_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.momf_symbol.to_uppercase();
                            self.momf_loading = true;
                            self.momf_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMomfSnapshot { symbol: sym });
                        }
                        if self.momf_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_momf_snapshot(ui, &self.momf_snapshot);
                });
            self.show_momf = open;
        }

        // PEADRANK — Post-Earnings Drift Rank vs Sector Peers
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "PEADRANK — PEAD Drift Rank vs Sector Peers",
                default_size: [640.0, 360.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_peadrank,
            &mut self.peadrank_symbol,
            &mut self.peadrank_loading,
            &mut self.peadrank_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_peadrank(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputePeadrankSnapshot { symbol },
            super::render::render_peadrank_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // FQM — Fundamental Quality Meter
        if self.show_fqm {
            if self.fqm_symbol.is_empty() {
                self.fqm_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_fqm;
            egui::Window::new("FQM — Fundamental Quality Meter")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.fqm_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.fqm_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.fqm_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_fqm(&conn, &sym_u)
                                    {
                                        self.fqm_snapshot = snap;
                                        self.fqm_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.fqm_symbol.to_uppercase();
                            self.fqm_loading = true;
                            self.fqm_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeFqmSnapshot { symbol: sym });
                        }
                        if self.fqm_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_fqm_snapshot(ui, &self.fqm_snapshot);
                });
            self.show_fqm = open;
        }

        // REVRANK — Relative 3y Revenue CAGR vs Sector Median
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "REVRANK — Relative 3y Revenue CAGR vs Sector",
                default_size: [640.0, 380.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_revrank,
            &mut self.revrank_symbol,
            &mut self.revrank_loading,
            &mut self.revrank_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_revrank(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeRevrankSnapshot { symbol },
            super::render::render_revrank_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // LEVRANK — Leverage Rank vs Sector Peers
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "LEVRANK — Leverage Rank vs Sector",
                default_size: [640.0, 380.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_levrank,
            &mut self.levrank_symbol,
            &mut self.levrank_loading,
            &mut self.levrank_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_levrank(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeLevrankSnapshot { symbol },
            super::render::render_levrank_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // OPERANK — Operating Quality Rank vs Sector Peers
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "OPERANK — Operating Quality Rank vs Sector",
                default_size: [640.0, 380.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_operank,
            &mut self.operank_symbol,
            &mut self.operank_loading,
            &mut self.operank_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_operank(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeOperankSnapshot { symbol },
            super::render::render_operank_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // FQMRANK — Fundamental Quality Meter Rank vs Sector Peers
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "FQMRANK — Fundamental Quality Rank vs Sector",
                default_size: [640.0, 380.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_fqmrank,
            &mut self.fqmrank_symbol,
            &mut self.fqmrank_loading,
            &mut self.fqmrank_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_fqmrank(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeFqmrankSnapshot { symbol },
            super::render::render_fqmrank_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // LIQRANK — Liquidity Rank vs Sector Peers
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "LIQRANK — Liquidity Rank vs Sector",
                default_size: [640.0, 380.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_liqrank,
            &mut self.liqrank_symbol,
            &mut self.liqrank_loading,
            &mut self.liqrank_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_liqrank(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeLiqrankSnapshot { symbol },
            super::render::render_liqrank_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // TLRANK — 30-day Liquidity Rank vs Sector Peers
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "TLRANK — 30-Day Liquidity Rank",
                default_size: [660.0, 400.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_tlrank,
            &mut self.tlrank_symbol,
            &mut self.tlrank_loading,
            &mut self.tlrank_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_tlrank(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeTlrankSnapshot { symbol },
            super::render::render_tlrank_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
