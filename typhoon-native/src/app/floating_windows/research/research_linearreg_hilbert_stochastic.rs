use super::*;

impl TyphooNApp {
    pub(super) fn render_research_linearreg_hilbert_stochastic_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── egui windows ──
        if self.show_linearreg_slope_win {
            if self.linearreg_slope_win_symbol.is_empty() {
                self.linearreg_slope_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_linearreg_slope_win;
            egui::Window::new("LINEARREG_SLOPE — Least-squares slope on close (TA-Lib parity)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.linearreg_slope_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.linearreg_slope_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.linearreg_slope_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_linearreg_slope(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.linearreg_slope_win_snapshot = snap;
                                        self.linearreg_slope_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.linearreg_slope_win_symbol.to_uppercase();
                            self.linearreg_slope_win_loading = true;
                            self.linearreg_slope_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeLinearregSlopeSnapshot { symbol: sym });
                        }
                        if self.linearreg_slope_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_linearreg_slope_snapshot(
                        ui,
                        &self.linearreg_slope_win_snapshot,
                    );
                });
            self.show_linearreg_slope_win = open;
        }

        if self.show_ht_dcperiod_win {
            if self.ht_dcperiod_win_symbol.is_empty() {
                self.ht_dcperiod_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ht_dcperiod_win;
            egui::Window::new("HT_DCPERIOD — Hilbert Dominant Cycle Period (Ehlers homodyne)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ht_dcperiod_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ht_dcperiod_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ht_dcperiod_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ht_dcperiod(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.ht_dcperiod_win_snapshot = snap;
                                        self.ht_dcperiod_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ht_dcperiod_win_symbol.to_uppercase();
                            self.ht_dcperiod_win_loading = true;
                            self.ht_dcperiod_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeHtDcperiodSnapshot { symbol: sym });
                        }
                        if self.ht_dcperiod_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_ht_dcperiod_snapshot(ui, &self.ht_dcperiod_win_snapshot);
                });
            self.show_ht_dcperiod_win = open;
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "HT_TRENDMODE — Hilbert Trend vs Cycle Regime (Ehlers CV classifier)",
                default_size: [560.0, 260.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_ht_trendmode_win,
            &mut self.ht_trendmode_win_symbol,
            &mut self.ht_trendmode_win_loading,
            &mut self.ht_trendmode_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_ht_trendmode(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeHtTrendmodeSnapshot { symbol },
            super::render::render_ht_trendmode_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "ACCBANDS — Headley Acceleration Bands (SMA-20 of H×(1+4·(H-L)/(H+L)))",
                default_size: [580.0, 280.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_accbands_win,
            &mut self.accbands_win_symbol,
            &mut self.accbands_win_loading,
            &mut self.accbands_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_accbands(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeAccbandsSnapshot { symbol },
            super::render::render_accbands_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "STOCHF — Fast Stochastic (TA-Lib, unsmoothed %K + SMA-3 %D)",
                default_size: [560.0, 280.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_stochf_win,
            &mut self.stochf_win_symbol,
            &mut self.stochf_win_loading,
            &mut self.stochf_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_stochf(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeStochfSnapshot { symbol },
            super::render::render_stochf_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
