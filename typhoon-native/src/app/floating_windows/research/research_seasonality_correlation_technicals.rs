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
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "SEAG — Seasonality Analysis",
                default_size: [620.0, 480.0],
                max_size: Some([620.0, 560.0]),
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_seag,
            &mut self.seag_symbol,
            &mut self.seag_loading,
            &mut self.seag_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_seasonality(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeSeasonalitySnapshot { symbol },
            super::render::render_seag_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
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
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "TRA — Total Return Analysis",
                default_size: [600.0, 420.0],
                max_size: Some([600.0, 560.0]),
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_tra,
            &mut self.tra_symbol,
            &mut self.tra_loading,
            &mut self.tra_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_total_return(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeTotalReturnSnapshot { symbol },
            super::render::render_tra_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // TECH — Technical Indicators
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "TECH — Technical Indicators",
                default_size: [620.0, 460.0],
                max_size: Some([620.0, 560.0]),
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_tech,
            &mut self.tech_symbol,
            &mut self.tech_loading,
            &mut self.tech_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_technicals(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeTechnicalsSnapshot { symbol },
            super::render::render_tech_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // SKEW — Volatility Skew / Smile
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "SKEW — Implied Volatility Skew",
                default_size: [680.0, 480.0],
                max_size: Some([680.0, 560.0]),
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_skew,
            &mut self.skew_symbol,
            &mut self.skew_loading,
            &mut self.skew_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_vol_skew(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeVolSkewSnapshot { symbol },
            super::render::render_skew_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
