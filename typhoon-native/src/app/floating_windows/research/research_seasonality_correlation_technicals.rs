use super::*;

impl TyphooNApp {
    pub(super) fn render_research_seasonality_correlation_technicals_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──

        // SEAG — Seasonality (monthly + day-of-week)
        if self.show_seag {
            if self.seag_symbol.is_empty() {
                self.seag_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_seag;
            egui::Window::new("SEAG — Seasonality Analysis")
                .open(&mut open)
                .resizable(true)
                .default_size([620.0, 480.0])
                .max_size([620.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.seag_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.seag_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.seag_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_seasonality(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.seag_snapshot = snap;
                                        self.seag_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.seag_symbol.to_uppercase();
                            self.seag_loading = true;
                            self.seag_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeSeasonalitySnapshot { symbol: sym });
                        }
                        if self.seag_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_seag_snapshot(ui, &self.seag_snapshot);
                });
            self.show_seag = open;
        }

        // COR — Correlation Matrix
        if self.show_cor {
            if self.cor_symbol.is_empty() {
                self.cor_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cor;
            egui::Window::new("COR — Correlation Matrix")
                .open(&mut open)
                .resizable(true)
                .default_size([620.0, 440.0])
                .max_size([620.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cor_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.cor_symbol = chart_sym_research.clone();
                        }
                        ui.label(
                            egui::RichText::new("Window (days)")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        ui.add(egui::DragValue::new(&mut self.cor_window_days).range(30..=1260));
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.cor_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_correlation(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.cor_snapshot = snap;
                                        self.cor_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cor_symbol.to_uppercase();
                            self.cor_loading = true;
                            self.cor_symbol = sym.clone();
                            let window_days = self.cor_window_days;
                            // Build peer series JSON on the main thread where the cache lives.
                            let peer_json = if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let peer_syms =
                                        typhoon_engine::core::research::get_peers(&conn, &sym)
                                            .unwrap_or(None)
                                            .unwrap_or_default();
                                    let mut peers_raw: Vec<(
                                        String,
                                        Vec<typhoon_engine::core::research::HistoricalPriceRow>,
                                    )> = Vec::new();
                                    for p in &peer_syms {
                                        if p.eq_ignore_ascii_case(&sym) {
                                            continue;
                                        }
                                        if let Ok(Some(mut rows)) =
                                            typhoon_engine::core::research::get_historical_price(
                                                &conn, p,
                                            )
                                        {
                                            if rows.len() >= 2
                                                && rows[0].date > rows[rows.len() - 1].date
                                            {
                                                rows.reverse();
                                            }
                                            peers_raw.push((p.to_uppercase(), rows));
                                        }
                                    }
                                    serde_json::to_string(&peers_raw)
                                        .unwrap_or_else(|_| "[]".to_string())
                                } else {
                                    "[]".to_string()
                                }
                            } else {
                                "[]".to_string()
                            };
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCorrelationMatrix {
                                symbol: sym,
                                window_days,
                                peer_series_json: peer_json,
                            });
                        }
                        if self.cor_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_cor_snapshot(ui, &self.cor_snapshot);
                });
            self.show_cor = open;
        }

        // TRA — Total Return Analysis
        if self.show_tra {
            if self.tra_symbol.is_empty() {
                self.tra_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_tra;
            egui::Window::new("TRA — Total Return Analysis")
                .open(&mut open)
                .resizable(true)
                .default_size([600.0, 420.0])
                .max_size([600.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.tra_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.tra_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.tra_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_total_return(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.tra_snapshot = snap;
                                        self.tra_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.tra_symbol.to_uppercase();
                            self.tra_loading = true;
                            self.tra_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeTotalReturnSnapshot { symbol: sym });
                        }
                        if self.tra_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_tra_snapshot(ui, &self.tra_snapshot);
                });
            self.show_tra = open;
        }

        // TECH — Technical Indicators
        if self.show_tech {
            if self.tech_symbol.is_empty() {
                self.tech_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_tech;
            egui::Window::new("TECH — Technical Indicators")
                .open(&mut open)
                .resizable(true)
                .default_size([620.0, 460.0])
                .max_size([620.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.tech_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.tech_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.tech_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_technicals(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.tech_snapshot = snap;
                                        self.tech_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.tech_symbol.to_uppercase();
                            self.tech_loading = true;
                            self.tech_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeTechnicalsSnapshot { symbol: sym });
                        }
                        if self.tech_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_tech_snapshot(ui, &self.tech_snapshot);
                });
            self.show_tech = open;
        }

        // SKEW — Volatility Skew / Smile
        if self.show_skew {
            if self.skew_symbol.is_empty() {
                self.skew_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_skew;
            egui::Window::new("SKEW — Implied Volatility Skew")
                .open(&mut open)
                .resizable(true)
                .default_size([680.0, 480.0])
                .max_size([680.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.skew_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.skew_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.skew_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_vol_skew(&conn, &sym_u)
                                    {
                                        self.skew_snapshot = snap;
                                        self.skew_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.skew_symbol.to_uppercase();
                            self.skew_loading = true;
                            self.skew_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeVolSkewSnapshot { symbol: sym });
                        }
                        if self.skew_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_skew_snapshot(ui, &self.skew_snapshot);
                });
            self.show_skew = open;
        }
    }
}
