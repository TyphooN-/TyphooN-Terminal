use super::*;

impl TyphooNApp {
    pub(super) fn render_research_insider_dividend_earnings_momentum_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // MNGR — Insider Activity Bias
        if self.show_mngr {
            if self.mngr_symbol.is_empty() {
                self.mngr_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_mngr;
            egui::Window::new("MNGR — Insider Activity Bias")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.mngr_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.mngr_symbol = chart_sym_research.clone();
                        }
                        ui.label(egui::RichText::new("Window (days):").color(AXIS_TEXT));
                        ui.add(egui::DragValue::new(&mut self.mngr_window_days).range(30..=365));
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.mngr_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_insider_activity(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.mngr_snapshot = snap;
                                        self.mngr_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.mngr_symbol.to_uppercase();
                            self.mngr_loading = true;
                            self.mngr_symbol = sym.clone();
                            let _ =
                                self.broker_tx
                                    .send(BrokerCmd::ComputeInsiderActivitySnapshot {
                                        symbol: sym,
                                        window_days: self.mngr_window_days,
                                    });
                        }
                        if self.mngr_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_mngr_snapshot(ui, &self.mngr_snapshot);
                });
            self.show_mngr = open;
        }

        // DIVG — Dividend Growth Analysis
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "DIVG — Dividend Growth Analysis",
                default_size: [600.0, 440.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_divg,
            &mut self.divg_symbol,
            &mut self.divg_loading,
            &mut self.divg_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_divg(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeDivgSnapshot { symbol },
            super::render::render_divg_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // EARM — Earnings Momentum Trend
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "EARM — Earnings Momentum Trend",
                default_size: [620.0, 460.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_earm,
            &mut self.earm_symbol,
            &mut self.earm_loading,
            &mut self.earm_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_earm(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeEarmSnapshot { symbol },
            super::render::render_earm_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // SECTR — Sector Rotation Strength
        if let Some(sym) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "SECTR — Sector Rotation Strength",
                default_size: [560.0, 420.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_sectr,
            &mut self.sectr_symbol,
            &mut self.sectr_loading,
            &mut self.sectr_snapshot,
            |conn, s| {
                typhoon_engine::core::research::get_sector_rotation(conn, s)
                    .ok()
                    .flatten()
            },
            |symbol| symbol,
            super::render::render_sectr_snapshot,
        ) {
            let symbol_sector = if let Some(ref cache) = self.cache {
                if let Ok(conn) = cache.connection() {
                    if let Ok(Some(fa)) =
                        typhoon_engine::core::fundamentals::get_fundamentals(&conn, &sym)
                    {
                        fa.sector
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            let _ = self
                .broker_tx
                .send(BrokerCmd::ComputeSectorRotationSnapshot {
                    symbol: sym,
                    symbol_sector,
                });
        }

        // UPDM — Upgrade/Downgrade Momentum
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "UPDM — Upgrade/Downgrade Momentum",
                default_size: [560.0, 420.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_updm,
            &mut self.updm_symbol,
            &mut self.updm_loading,
            &mut self.updm_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_updm(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeUpdmSnapshot { symbol },
            super::render::render_updm_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
