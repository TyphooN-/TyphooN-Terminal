use super::*;

impl TyphooNApp {
    pub(super) fn render_research_linearreg_hilbert_phase_windows(&mut self, ctx: &egui::Context) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── egui windows ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "LINEARREG — TA-Lib fitted endpoint of 14-bar least-squares close",
                default_size: [560.0, 260.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_linearreg_win,
            &mut self.linearreg_win_symbol,
            &mut self.linearreg_win_loading,
            &mut self.linearreg_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_linearreg(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeLinearregSnapshot { symbol },
            super::render::render_linearreg_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if self.show_linearreg_angle_win {
            if self.linearreg_angle_win_symbol.is_empty() {
                self.linearreg_angle_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_linearreg_angle_win;
            egui::Window::new("LINEARREG_ANGLE — atan(slope)·180/π of 14-bar fit")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.linearreg_angle_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.linearreg_angle_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.linearreg_angle_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_linearreg_angle(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.linearreg_angle_win_snapshot = snap;
                                        self.linearreg_angle_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.linearreg_angle_win_symbol.to_uppercase();
                            self.linearreg_angle_win_loading = true;
                            self.linearreg_angle_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeLinearregAngleSnapshot { symbol: sym });
                        }
                        if self.linearreg_angle_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_linearreg_angle_snapshot(
                        ui,
                        &self.linearreg_angle_win_snapshot,
                    );
                });
            self.show_linearreg_angle_win = open;
        }

        if self.show_ht_dcphase_win {
            if self.ht_dcphase_win_symbol.is_empty() {
                self.ht_dcphase_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ht_dcphase_win;
            egui::Window::new("HT_DCPHASE — Ehlers Hilbert Dominant Cycle Phase (degrees)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ht_dcphase_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ht_dcphase_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ht_dcphase_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ht_dcphase(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.ht_dcphase_win_snapshot = snap;
                                        self.ht_dcphase_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ht_dcphase_win_symbol.to_uppercase();
                            self.ht_dcphase_win_loading = true;
                            self.ht_dcphase_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeHtDcphaseSnapshot { symbol: sym });
                        }
                        if self.ht_dcphase_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_ht_dcphase_snapshot(ui, &self.ht_dcphase_win_snapshot);
                });
            self.show_ht_dcphase_win = open;
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "HT_SINE — Ehlers Sine + Leadsine cycle-turn detector",
                default_size: [560.0, 280.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_ht_sine_win,
            &mut self.ht_sine_win_symbol,
            &mut self.ht_sine_win_loading,
            &mut self.ht_sine_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_ht_sine(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeHtSineSnapshot { symbol },
            super::render::render_ht_sine_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "HT_PHASOR — Ehlers raw I/Q + magnitude + phase",
                default_size: [560.0, 280.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_ht_phasor_win,
            &mut self.ht_phasor_win_symbol,
            &mut self.ht_phasor_win_loading,
            &mut self.ht_phasor_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_ht_phasor(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeHtPhasorSnapshot { symbol },
            super::render::render_ht_phasor_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "MIDPRICE — (HHV + LLV) / 2 range midpoint (14-bar)",
                default_size: [560.0, 280.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_midprice_win,
            &mut self.midprice_win_symbol,
            &mut self.midprice_win_loading,
            &mut self.midprice_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_midprice(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeMidpriceSnapshot { symbol },
            super::render::render_midprice_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "APO — Absolute Price Oscillator (EMA12 − EMA26)",
                default_size: [560.0, 280.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_apo_win,
            &mut self.apo_win_symbol,
            &mut self.apo_win_loading,
            &mut self.apo_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_apo(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeApoSnapshot { symbol },
            super::render::render_apo_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "MOM — raw close − close[n−10] momentum",
                default_size: [560.0, 260.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_mom_win,
            &mut self.mom_win_symbol,
            &mut self.mom_win_loading,
            &mut self.mom_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_mom(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeMomSnapshot { symbol },
            super::render::render_mom_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "SAREXT — Extended Parabolic SAR (asymmetric long/short AF)",
                default_size: [620.0, 320.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_sarext_win,
            &mut self.sarext_win_symbol,
            &mut self.sarext_win_loading,
            &mut self.sarext_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_sarext(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeSarextSnapshot { symbol },
            super::render::render_sarext_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "ADXR — Average Directional Movement Rating (14-bar)",
                default_size: [560.0, 260.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_adxr_win,
            &mut self.adxr_win_symbol,
            &mut self.adxr_win_loading,
            &mut self.adxr_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_adxr(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeAdxrSnapshot { symbol },
            super::render::render_adxr_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
