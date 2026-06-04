use super::*;

impl TyphooNApp {
    pub(super) fn render_research_dividends_earnings_upgrades_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research: String = self
            .charts
            .get(self.active_tab)
            .map(|c| {
                c.symbol
                    .split(':')
                    .rev()
                    .nth(1)
                    .or_else(|| c.symbol.split(':').last())
                    .unwrap_or("AAPL")
                    .to_string()
            })
            .unwrap_or_else(|| "AAPL".to_string());

        // ── Research Godel Parity Round 2 windows ─────────────────────

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
                    ui.separator();
                    if self.dividend_history.is_empty() {
                        ui.label(
                            egui::RichText::new(
                                "No dividend history — click Load Cached or Fetch.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                    } else {
                        // Summary line: TTM dividend sum + count
                        let ttm_cut = (chrono::Utc::now() - chrono::Duration::days(365))
                            .format("%Y-%m-%d")
                            .to_string();
                        let ttm_sum: f64 = self
                            .dividend_history
                            .iter()
                            .filter(|d| d.ex_date.as_str() >= ttm_cut.as_str())
                            .map(|d| d.amount)
                            .sum();
                        let ttm_count = self
                            .dividend_history
                            .iter()
                            .filter(|d| d.ex_date.as_str() >= ttm_cut.as_str())
                            .count();
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("TTM: ${:.4}", ttm_sum))
                                    .strong()
                                    .color(UP),
                            );
                            ui.label(
                                egui::RichText::new(format!("({} payments)", ttm_count))
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "total records: {}",
                                    self.dividend_history.len()
                                ))
                                .color(AXIS_TEXT)
                                .small(),
                            );
                        });
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                egui::Grid::new("dvd_grid")
                                    .striped(true)
                                    .num_columns(6)
                                    .spacing([12.0, 2.0])
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("Ex-Date").strong());
                                        ui.label(egui::RichText::new("Pay Date").strong());
                                        ui.label(egui::RichText::new("Record").strong());
                                        ui.label(egui::RichText::new("Amount").strong());
                                        ui.label(egui::RichText::new("Adj").strong());
                                        ui.label(egui::RichText::new("Label").strong());
                                        ui.end_row();
                                        for d in self.dividend_history.iter().take(200) {
                                            ui.label(
                                                egui::RichText::new(&d.ex_date).monospace().small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(&d.pay_date)
                                                    .monospace()
                                                    .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(&d.record_date)
                                                    .monospace()
                                                    .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("${:.4}", d.amount))
                                                    .color(UP)
                                                    .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "${:.4}",
                                                    d.adjusted_amount
                                                ))
                                                .monospace()
                                                .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(&d.label)
                                                    .color(AXIS_TEXT)
                                                    .small(),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
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
                    ui.separator();
                    if self.earnings_estimates.is_empty() {
                        ui.label(
                            egui::RichText::new(
                                "No forward estimates — click Load Cached or Fetch.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                    } else {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                egui::Grid::new("eeb_grid")
                                    .striped(true)
                                    .num_columns(8)
                                    .spacing([10.0, 2.0])
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("Period").strong());
                                        ui.label(egui::RichText::new("EPS Avg").strong());
                                        ui.label(egui::RichText::new("EPS Low").strong());
                                        ui.label(egui::RichText::new("EPS High").strong());
                                        ui.label(egui::RichText::new("Rev Avg").strong());
                                        ui.label(egui::RichText::new("Rev Low").strong());
                                        ui.label(egui::RichText::new("Rev High").strong());
                                        ui.label(egui::RichText::new("#Analysts").strong());
                                        ui.end_row();
                                        for e in self.earnings_estimates.iter().take(40) {
                                            ui.label(
                                                egui::RichText::new(&e.date).monospace().small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("{:.2}", e.eps_avg))
                                                    .color(UP)
                                                    .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("{:.2}", e.eps_low))
                                                    .monospace()
                                                    .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("{:.2}", e.eps_high))
                                                    .monospace()
                                                    .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "${:.0}M",
                                                    e.revenue_avg / 1_000_000.0
                                                ))
                                                .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "${:.0}M",
                                                    e.revenue_low / 1_000_000.0
                                                ))
                                                .monospace()
                                                .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "${:.0}M",
                                                    e.revenue_high / 1_000_000.0
                                                ))
                                                .monospace()
                                                .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{}",
                                                    e.num_analysts_eps.max(e.num_analysts_rev)
                                                ))
                                                .color(AXIS_TEXT)
                                                .small(),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
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
                    ui.separator();
                    if self.rating_changes.is_empty() {
                        ui.label(
                            egui::RichText::new("No rating changes — click Load Cached or Fetch.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                egui::Grid::new("updg_grid")
                                    .striped(true)
                                    .num_columns(6)
                                    .spacing([12.0, 2.0])
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("Date").strong());
                                        ui.label(egui::RichText::new("Firm").strong());
                                        ui.label(egui::RichText::new("Action").strong());
                                        ui.label(egui::RichText::new("From").strong());
                                        ui.label(egui::RichText::new("To").strong());
                                        ui.label(egui::RichText::new("Target").strong());
                                        ui.end_row();
                                        for r in self.rating_changes.iter().take(200) {
                                            ui.label(
                                                egui::RichText::new(&r.date).monospace().small(),
                                            );
                                            ui.label(egui::RichText::new(&r.firm).small());
                                            let act_col = match r.action.as_str() {
                                                "upgrade" => BTN_GREEN_TEXT,
                                                "downgrade" => BTN_RED_TEXT,
                                                "initiation" => {
                                                    egui::Color32::from_rgb(100, 200, 255)
                                                }
                                                _ => AXIS_TEXT,
                                            };
                                            ui.label(
                                                egui::RichText::new(r.action.to_uppercase())
                                                    .color(act_col)
                                                    .small()
                                                    .strong(),
                                            );
                                            ui.label(
                                                egui::RichText::new(&r.from_grade)
                                                    .color(AXIS_TEXT)
                                                    .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(&r.to_grade).small().strong(),
                                            );
                                            if r.price_target > 0.0 {
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "${:.2}",
                                                        r.price_target
                                                    ))
                                                    .color(UP)
                                                    .monospace(),
                                                );
                                            } else {
                                                ui.label(
                                                    egui::RichText::new("—")
                                                        .color(AXIS_TEXT)
                                                        .small(),
                                                );
                                            }
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
                });
            self.show_rating_changes = open;
        }
    }
}
