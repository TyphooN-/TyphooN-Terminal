use super::*;

impl TyphooNApp {
    pub(super) fn render_research_ohlc_volatility_cvar_calendar_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "PARKINSON — H-L Range Volatility",
                default_size: [560.0, 360.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_parkinson,
            &mut self.parkinson_symbol,
            &mut self.parkinson_loading,
            &mut self.parkinson_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_parkinson(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeParkinsonSnapshot { symbol },
            super::render::render_parkinson_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if self.show_gkvol {
            if self.gkvol_symbol.is_empty() {
                self.gkvol_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_gkvol;
            egui::Window::new("GKVOL — Garman-Klass OHLC Volatility")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.gkvol_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.gkvol_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.gkvol_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_gkvol(&conn, &sym_u)
                                    {
                                        self.gkvol_snapshot = snap;
                                        self.gkvol_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.gkvol_symbol.to_uppercase();
                            self.gkvol_loading = true;
                            self.gkvol_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeGkvolSnapshot { symbol: sym });
                        }
                        if self.gkvol_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_gkvol_snapshot(ui, &self.gkvol_snapshot);
                });
            self.show_gkvol = open;
        }

        if self.show_rsvol {
            if self.rsvol_symbol.is_empty() {
                self.rsvol_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_rsvol;
            egui::Window::new("RSVOL — Rogers-Satchell OHLC Volatility")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 340.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.rsvol_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.rsvol_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.rsvol_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_rsvol(&conn, &sym_u)
                                    {
                                        self.rsvol_snapshot = snap;
                                        self.rsvol_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.rsvol_symbol.to_uppercase();
                            self.rsvol_loading = true;
                            self.rsvol_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRsvolSnapshot { symbol: sym });
                        }
                        if self.rsvol_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_rsvol_snapshot(ui, &self.rsvol_snapshot);
                });
            self.show_rsvol = open;
        }

        if self.show_cvar {
            if self.cvar_symbol.is_empty() {
                self.cvar_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cvar;
            egui::Window::new("CVAR — Conditional VaR / Expected Shortfall")
                .open(&mut open)
                .resizable(true)
                .default_size([600.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cvar_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.cvar_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.cvar_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_cvar(&conn, &sym_u)
                                    {
                                        self.cvar_snapshot = snap;
                                        self.cvar_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cvar_symbol.to_uppercase();
                            self.cvar_loading = true;
                            self.cvar_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCvarSnapshot { symbol: sym });
                        }
                        if self.cvar_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_cvar_snapshot(ui, &self.cvar_snapshot);
                });
            self.show_cvar = open;
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "DOWEFFECT — Day-of-Week Intraday Seasonality",
                default_size: [640.0, 460.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_doweffect,
            &mut self.doweffect_symbol,
            &mut self.doweffect_loading,
            &mut self.doweffect_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_doweffect(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeDoweffectSnapshot { symbol },
            super::render::render_doweffect_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
