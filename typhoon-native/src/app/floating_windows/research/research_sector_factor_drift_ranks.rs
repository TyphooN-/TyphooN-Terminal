use super::*;

impl TyphooNApp {
    pub(super) fn render_research_sector_factor_drift_ranks_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──

        // VRK — Value Rank vs sector peers
        if self.show_vrk {
            if self.vrk_symbol.is_empty() {
                self.vrk_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_vrk;
            egui::Window::new("VRK — Value Rank vs Sector Peers")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 360.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.vrk_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.vrk_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.vrk_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_vrk(&conn, &sym_u)
                                    {
                                        self.vrk_snapshot = snap;
                                        self.vrk_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.vrk_symbol.to_uppercase();
                            self.vrk_loading = true;
                            self.vrk_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeVrkSnapshot { symbol: sym });
                        }
                        if self.vrk_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_vrk_snapshot(ui, &self.vrk_snapshot);
                });
            self.show_vrk = open;
        }

        // QRK — Quality Rank vs sector peers
        if self.show_qrk {
            if self.qrk_symbol.is_empty() {
                self.qrk_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_qrk;
            egui::Window::new("QRK — Quality Rank vs Sector Peers")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 360.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.qrk_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.qrk_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.qrk_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_qrk(&conn, &sym_u)
                                    {
                                        self.qrk_snapshot = snap;
                                        self.qrk_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.qrk_symbol.to_uppercase();
                            self.qrk_loading = true;
                            self.qrk_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeQrkSnapshot { symbol: sym });
                        }
                        if self.qrk_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_qrk_snapshot(ui, &self.qrk_snapshot);
                });
            self.show_qrk = open;
        }

        // RRK — Risk Rank vs sector peers (inverted — higher pct = SAFER)
        if self.show_rrk {
            if self.rrk_symbol.is_empty() {
                self.rrk_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_rrk;
            egui::Window::new("RRK — Risk Rank vs Sector Peers (Higher = Safer)")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 360.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.rrk_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.rrk_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.rrk_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_rrk(&conn, &sym_u)
                                    {
                                        self.rrk_snapshot = snap;
                                        self.rrk_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.rrk_symbol.to_uppercase();
                            self.rrk_loading = true;
                            self.rrk_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRrkSnapshot { symbol: sym });
                        }
                        if self.rrk_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_rrk_snapshot(ui, &self.rrk_snapshot);
                });
            self.show_rrk = open;
        }

        // RELEPSGR — Relative 3y EPS CAGR vs sector median
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "RELEPSGR — Relative 3y EPS CAGR vs Sector",
                default_size: [640.0, 380.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_relepsgr,
            &mut self.relepsgr_symbol,
            &mut self.relepsgr_loading,
            &mut self.relepsgr_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_relepsgr(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeRelepsgrSnapshot { symbol },
            super::render::render_relepsgr_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // PEAD — Post-Earnings-Announcement Drift
        if self.show_pead {
            if self.pead_symbol.is_empty() {
                self.pead_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_pead;
            egui::Window::new("PEAD — Post-Earnings-Announcement Drift")
                .open(&mut open)
                .resizable(true)
                .default_size([720.0, 480.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.pead_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.pead_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.pead_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_pead(&conn, &sym_u)
                                    {
                                        self.pead_snapshot = snap;
                                        self.pead_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.pead_symbol.to_uppercase();
                            self.pead_loading = true;
                            self.pead_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputePeadSnapshot { symbol: sym });
                        }
                        if self.pead_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_pead_snapshot(ui, &self.pead_snapshot);
                });
            self.show_pead = open;
        }
    }
}
