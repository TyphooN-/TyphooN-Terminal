use super::*;

impl TyphooNApp {
    pub(super) fn render_research_solvency_scores_volatility_targets_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // Solvency, quality, volatility-estimator, EPS-beat, and price-target research

        // ALTZ — Altman Z-Score
        if self.show_altz {
            if self.altz_symbol.is_empty() {
                self.altz_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_altz;
            egui::Window::new("ALTZ — Altman Z-Score")
                .open(&mut open)
                .resizable(true)
                .default_size([620.0, 420.0])
                .max_size([620.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.altz_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.altz_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.altz_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_altman_z(&conn, &sym_u)
                                    {
                                        self.altz_snapshot = snap;
                                        self.altz_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.altz_symbol.to_uppercase();
                            self.altz_loading = true;
                            self.altz_symbol = sym.clone();
                            let market_value_equity = if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    if let Ok(Some(fa)) =
                                        typhoon_engine::core::fundamentals::get_fundamentals(
                                            &conn, &sym,
                                        )
                                    {
                                        fa.market_cap.unwrap_or(0.0)
                                    } else {
                                        0.0
                                    }
                                } else {
                                    0.0
                                }
                            } else {
                                0.0
                            };
                            let _ = self.broker_tx.send(BrokerCmd::ComputeAltmanZSnapshot {
                                symbol: sym,
                                market_value_equity,
                            });
                        }
                        if self.altz_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_altz_snapshot(ui, &self.altz_snapshot);
                });
            self.show_altz = open;
        }

        // PTFS — Piotroski F-Score
        if self.show_ptfs {
            if self.ptfs_symbol.is_empty() {
                self.ptfs_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ptfs;
            egui::Window::new("PTFS — Piotroski F-Score")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 480.0])
                .max_size([640.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ptfs_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ptfs_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ptfs_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_piotroski(&conn, &sym_u)
                                    {
                                        self.ptfs_snapshot = snap;
                                        self.ptfs_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ptfs_symbol.to_uppercase();
                            self.ptfs_loading = true;
                            self.ptfs_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputePiotroskiSnapshot { symbol: sym });
                        }
                        if self.ptfs_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_ptfs_snapshot(ui, &self.ptfs_snapshot);
                });
            self.show_ptfs = open;
        }

        // VOLE — OHLC Volatility Estimators
        if self.show_vole {
            if self.vole_symbol.is_empty() {
                self.vole_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_vole;
            egui::Window::new("VOLE — OHLC Volatility Estimators")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 360.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.vole_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.vole_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.vole_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ohlc_vol(&conn, &sym_u)
                                    {
                                        self.vole_snapshot = snap;
                                        self.vole_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.vole_symbol.to_uppercase();
                            self.vole_loading = true;
                            self.vole_symbol = sym.clone();
                            let bars_json = if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let mut bars: Vec<
                                        typhoon_engine::core::research::HistoricalPriceRow,
                                    > = typhoon_engine::core::research::get_historical_price(
                                        &conn, &sym,
                                    )
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default();
                                    if bars.len() >= 2 && bars[0].date > bars[bars.len() - 1].date {
                                        bars.reverse();
                                    }
                                    serde_json::to_string(&bars).unwrap_or_default()
                                } else {
                                    String::new()
                                }
                            } else {
                                String::new()
                            };
                            let _ = self.broker_tx.send(BrokerCmd::ComputeOhlcVolSnapshot {
                                symbol: sym,
                                window_days: 60,
                                bars_json,
                            });
                        }
                        if self.vole_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_vole_snapshot(ui, &self.vole_snapshot);
                });
            self.show_vole = open;
        }

        // EPSB — EPS Beat Streak & Surprise
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "EPSB — EPS Beat Streak",
                default_size: [560.0, 380.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_epsb,
            &mut self.epsb_symbol,
            &mut self.epsb_loading,
            &mut self.epsb_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_eps_beat(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeEpsBeatSnapshot { symbol },
            super::render::render_epsb_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // PTD — Price Target Dispersion & Implied Return
        if self.show_ptd {
            if self.ptd_symbol.is_empty() {
                self.ptd_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ptd;
            egui::Window::new("PTD — Price Target Dispersion")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ptd_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ptd_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ptd_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_price_target_dispersion(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.ptd_snapshot = snap;
                                        self.ptd_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ptd_symbol.to_uppercase();
                            self.ptd_loading = true;
                            self.ptd_symbol = sym.clone();
                            let current_price = if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    if let Ok(Some(fa)) =
                                        typhoon_engine::core::fundamentals::get_fundamentals(
                                            &conn, &sym,
                                        )
                                    {
                                        fa.stock_price.unwrap_or(0.0)
                                    } else {
                                        0.0
                                    }
                                } else {
                                    0.0
                                }
                            } else {
                                0.0
                            };
                            let _ = self.broker_tx.send(
                                BrokerCmd::ComputePriceTargetDispersionSnapshot {
                                    symbol: sym,
                                    current_price,
                                },
                            );
                        }
                        if self.ptd_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_ptd_snapshot(ui, &self.ptd_snapshot);
                });
            self.show_ptd = open;
        }
    }
}
