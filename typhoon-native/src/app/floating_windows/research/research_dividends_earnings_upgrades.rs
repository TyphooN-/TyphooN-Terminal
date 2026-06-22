use super::*;

impl TyphooNApp {
    pub(super) fn render_research_dividends_earnings_upgrades_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // Dividend, earnings-estimate, rating, and treasury research

        // DVD — Dividend History
        if self.show_dividend_history {
            if self.dividend_history_symbol.is_empty() {
                self.dividend_history_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_dividend_history;
            egui::Window::new("DVD — Dividend History")
                .open(&mut open)
                .resizable(true)
                .default_size([620.0, 480.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.dividend_history_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.dividend_history_symbol = chart_sym_research.clone();
                        }
                        let have_cache = self.cache.is_some();
                        if ui
                            .add_enabled(have_cache, egui::Button::new("Load Cached"))
                            .clicked()
                        {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.dividend_history_symbol.to_uppercase();
                                    if let Ok(Some(rows)) =
                                        typhoon_engine::core::research::get_dividends(&conn, &sym_u)
                                    {
                                        self.dividend_history = rows;
                                        self.dividend_history_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        let have_key = !self.fmp_key.is_empty();
                        if ui
                            .add_enabled(have_key, egui::Button::new("Fetch").fill(BTN_MG))
                            .clicked()
                        {
                            let sym = self.dividend_history_symbol.to_uppercase();
                            self.dividend_history_loading = true;
                            self.dividend_history_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::FetchDividendHistory {
                                symbol: sym,
                                fmp_key: self.fmp_key.clone(),
                            });
                        }
                        if self.fmp_key.is_empty() {
                            ui.label(
                                egui::RichText::new("(add FMP key in Settings)")
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                        }
                        if self.dividend_history_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_dividend_history(ui, &self.dividend_history);
                });
            self.show_dividend_history = open;
        }

        // EEB — Forward Earnings Estimates
        if self.show_earnings_estimates {
            if self.earnings_estimates_symbol.is_empty() {
                self.earnings_estimates_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_earnings_estimates;
            egui::Window::new("EEB — Earnings Estimates")
                .open(&mut open)
                .resizable(true)
                .default_size([720.0, 440.0])
                .max_size([720.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.earnings_estimates_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.earnings_estimates_symbol = chart_sym_research.clone();
                        }
                        let have_cache = self.cache.is_some();
                        if ui
                            .add_enabled(have_cache, egui::Button::new("Load Cached"))
                            .clicked()
                        {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.earnings_estimates_symbol.to_uppercase();
                                    if let Ok(Some(rows)) =
                                        typhoon_engine::core::research::get_earnings_estimates(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.earnings_estimates = rows;
                                        self.earnings_estimates_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        let have_key = !self.fmp_key.is_empty();
                        if ui
                            .add_enabled(have_key, egui::Button::new("Fetch").fill(BTN_MG))
                            .clicked()
                        {
                            let sym = self.earnings_estimates_symbol.to_uppercase();
                            self.earnings_estimates_loading = true;
                            self.earnings_estimates_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::FetchEarningsEstimates {
                                symbol: sym,
                                fmp_key: self.fmp_key.clone(),
                            });
                        }
                        if self.fmp_key.is_empty() {
                            ui.label(
                                egui::RichText::new("(add FMP key in Settings)")
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                        }
                        if self.earnings_estimates_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_earnings_estimates(ui, &self.earnings_estimates);
                });
            self.show_earnings_estimates = open;
        }

        // UPDG — Analyst Rating Changes (upgrades/downgrades)
        if self.show_rating_changes {
            if self.rating_changes_symbol.is_empty() {
                self.rating_changes_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_rating_changes;
            egui::Window::new("UPDG — Upgrades / Downgrades")
                .open(&mut open)
                .resizable(true)
                .default_size([720.0, 500.0])
                .max_size([720.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.rating_changes_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.rating_changes_symbol = chart_sym_research.clone();
                        }
                        let have_cache = self.cache.is_some();
                        if ui
                            .add_enabled(have_cache, egui::Button::new("Load Cached"))
                            .clicked()
                        {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.rating_changes_symbol.to_uppercase();
                                    if let Ok(Some(rows)) =
                                        typhoon_engine::core::research::get_rating_changes(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.rating_changes = rows;
                                        self.rating_changes_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        let have_key = !self.fmp_key.is_empty();
                        if ui
                            .add_enabled(have_key, egui::Button::new("Fetch").fill(BTN_MG))
                            .clicked()
                        {
                            let sym = self.rating_changes_symbol.to_uppercase();
                            self.rating_changes_loading = true;
                            self.rating_changes_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::FetchRatingChanges {
                                symbol: sym,
                                fmp_key: self.fmp_key.clone(),
                            });
                        }
                        if self.fmp_key.is_empty() {
                            ui.label(
                                egui::RichText::new("(add FMP key in Settings)")
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                        }
                        if self.rating_changes_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_rating_changes(ui, &self.rating_changes);
                });
            self.show_rating_changes = open;
        }
    }
}
