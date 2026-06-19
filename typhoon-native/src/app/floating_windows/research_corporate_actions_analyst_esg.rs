use super::*;

impl TyphooNApp {
    pub(super) fn render_research_corporate_actions_analyst_esg_windows(
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

        // ── Research Round 4 windows ──────────────────────────────────

        // SPLT — Stock Split History
        if self.show_splits {
            if self.splits_symbol.is_empty() {
                self.splits_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_splits;
            egui::Window::new("SPLT — Stock Split History")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 380.0])
                .max_size([540.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.splits_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.splits_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.splits_symbol.to_uppercase();
                                    if let Ok(Some(rows)) =
                                        typhoon_engine::core::research::get_stock_splits(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.splits_list = rows;
                                        self.splits_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui
                            .add_enabled(
                                !self.splits_symbol.trim().is_empty(),
                                egui::Button::new("Fetch").fill(BTN_MG),
                            )
                            .clicked()
                        {
                            let sym = self.splits_symbol.to_uppercase();
                            self.splits_loading = true;
                            self.splits_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::FetchStockSplits {
                                symbol: sym,
                                fmp_key: self.fmp_key.clone(),
                            });
                        }
                        if self.fmp_key.is_empty() {
                            ui.label(
                                egui::RichText::new(
                                    "(Yahoo fallback; add FMP key for second source)",
                                )
                                .color(AXIS_TEXT)
                                .small(),
                            );
                        }
                        if self.splits_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    if self.splits_list.is_empty() {
                        ui.label(
                            egui::RichText::new("No split history — click Load Cached or Fetch.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new(format!("{} split events", self.splits_list.len()))
                                .strong(),
                        );
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                egui::Grid::new("splt_grid")
                                    .striped(true)
                                    .num_columns(4)
                                    .spacing([20.0, 4.0])
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("Date").strong());
                                        ui.label(egui::RichText::new("Label").strong());
                                        ui.label(egui::RichText::new("Ratio").strong());
                                        ui.label(egui::RichText::new("From → To").strong());
                                        ui.end_row();
                                        for s in self.splits_list.iter() {
                                            ui.label(
                                                egui::RichText::new(&s.date).monospace().small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(&s.label)
                                                    .monospace()
                                                    .strong()
                                                    .color(UP),
                                            );
                                            let ratio = if s.denominator > 0.0 {
                                                s.numerator / s.denominator
                                            } else {
                                                0.0
                                            };
                                            ui.label(
                                                egui::RichText::new(format!("{:.3}x", ratio))
                                                    .monospace()
                                                    .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:.0} → {:.0}",
                                                    s.denominator, s.numerator
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
            self.show_splits = open;
        }

        // ETF — ETF Holdings (Constituents)
        if self.show_etf_holdings {
            if self.etf_symbol.is_empty() {
                self.etf_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_etf_holdings;
            egui::Window::new("ETF — Fund Holdings")
                .open(&mut open)
                .resizable(true)
                .default_size([820.0, 540.0])
                .max_size([820.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("ETF:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.etf_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.etf_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.etf_symbol.to_uppercase();
                                    if let Ok(Some(rows)) = typhoon_engine::core::research::get_etf_holdings(&conn, &sym_u) {
                                        self.etf_holdings = rows;
                                        self.etf_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        let have_key = !self.fmp_key.is_empty();
                        if ui.add_enabled(have_key, egui::Button::new("Fetch").fill(BTN_MG)).clicked() {
                            let sym = self.etf_symbol.to_uppercase();
                            self.etf_loading = true;
                            self.etf_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::FetchEtfHoldings { symbol: sym, fmp_key: self.fmp_key.clone() });
                        }
                        if self.fmp_key.is_empty() {
                            ui.label(egui::RichText::new("(add FMP key in Settings)").color(AXIS_TEXT).small());
                        }
                        if self.etf_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    if self.etf_holdings.is_empty() {
                        ui.label(egui::RichText::new("No ETF holdings — click Load Cached or Fetch. Pass an ETF ticker (SPY, QQQ, IWM, VTI, …).").color(AXIS_TEXT).small());
                    } else {
                        let total_weight: f64 = self.etf_holdings.iter().map(|h| h.weight_pct).sum();
                        let total_value: f64 = self.etf_holdings.iter().map(|h| h.market_value).sum();
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("{} holdings", self.etf_holdings.len())).strong());
                            ui.label(egui::RichText::new(format!("(sum weight: {:.2}%, AUM: ${:.1}B)", total_weight, total_value / 1e9)).color(AXIS_TEXT).small());
                        });
                        ui.separator();
                        let fmt_money = |v: f64| -> String {
                            if v.abs() >= 1e9 { format!("${:.2}B", v / 1e9) }
                            else if v.abs() >= 1e6 { format!("${:.1}M", v / 1e6) }
                            else if v.abs() >= 1e3 { format!("${:.0}K", v / 1e3) }
                            else { format!("${:.0}", v) }
                        };
                        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                            egui::Grid::new("etf_grid").striped(true).num_columns(5).spacing([14.0, 3.0]).show(ui, |ui| {
                                ui.label(egui::RichText::new("Symbol").strong());
                                ui.label(egui::RichText::new("Name").strong());
                                ui.label(egui::RichText::new("Weight %").strong());
                                ui.label(egui::RichText::new("Shares").strong());
                                ui.label(egui::RichText::new("Market Value").strong());
                                ui.end_row();
                                for h in self.etf_holdings.iter().take(500) {
                                    ui.label(egui::RichText::new(&h.symbol).monospace().strong());
                                    let short_name: String = h.name.chars().take(40).collect();
                                    ui.label(egui::RichText::new(short_name).small());
                                    ui.label(egui::RichText::new(format!("{:.2}%", h.weight_pct)).color(UP).monospace());
                                    ui.label(egui::RichText::new(format!("{:.0}", h.shares)).monospace().small());
                                    ui.label(egui::RichText::new(fmt_money(h.market_value)).monospace().small());
                                    ui.end_row();
                                }
                            });
                        });
                    }
                });
            self.show_etf_holdings = open;
        }

        // ANR — Analyst Recommendations + Consensus Price Target
        if self.show_analyst_recs {
            if self.anr_symbol.is_empty() {
                self.anr_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_analyst_recs;
            egui::Window::new("ANR — Analyst Recommendations")
                .open(&mut open)
                .resizable(true)
                .default_size([700.0, 460.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.anr_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.anr_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.anr_symbol.to_uppercase();
                                    if let Ok(Some(rows)) =
                                        typhoon_engine::core::research::get_analyst_recs(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.analyst_recs = rows;
                                    }
                                    if let Ok(Some(pt)) =
                                        typhoon_engine::core::research::get_price_target(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.price_target = pt;
                                    }
                                    self.anr_symbol = sym_u;
                                }
                            }
                        }
                        let have_key = !self.finnhub_key.is_empty();
                        if ui
                            .add_enabled(have_key, egui::Button::new("Fetch").fill(BTN_MG))
                            .clicked()
                        {
                            let sym = self.anr_symbol.to_uppercase();
                            self.anr_loading = true;
                            self.anr_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::FetchAnalystRecs {
                                symbol: sym.clone(),
                                finnhub_key: self.finnhub_key.clone(),
                            });
                            let _ = self.broker_tx.send(BrokerCmd::FetchPriceTarget {
                                symbol: sym,
                                finnhub_key: self.finnhub_key.clone(),
                            });
                        }
                        if self.finnhub_key.is_empty() {
                            ui.label(
                                egui::RichText::new("(add Finnhub key in Settings)")
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                        }
                        if self.anr_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    // Price target header
                    if self.price_target.num_analysts > 0 {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Price Target:").strong());
                            ui.label(
                                egui::RichText::new(format!(
                                    "Mean ${:.2}",
                                    self.price_target.target_mean
                                ))
                                .color(UP)
                                .monospace()
                                .strong(),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "Median ${:.2}",
                                    self.price_target.target_median
                                ))
                                .monospace(),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "Low ${:.2}",
                                    self.price_target.target_low
                                ))
                                .color(DOWN)
                                .monospace()
                                .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "High ${:.2}",
                                    self.price_target.target_high
                                ))
                                .color(UP)
                                .monospace()
                                .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "n={}",
                                    self.price_target.num_analysts
                                ))
                                .color(AXIS_TEXT)
                                .small(),
                            );
                            if !self.price_target.last_updated.is_empty() {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "({})",
                                        self.price_target.last_updated
                                    ))
                                    .color(AXIS_TEXT)
                                    .small(),
                                );
                            }
                        });
                        ui.separator();
                    }
                    if self.analyst_recs.is_empty() {
                        ui.label(
                            egui::RichText::new(
                                "No recommendation history — click Load Cached or Fetch.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} monthly snapshots",
                                self.analyst_recs.len()
                            ))
                            .strong(),
                        );
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                egui::Grid::new("anr_grid")
                                    .striped(true)
                                    .num_columns(7)
                                    .spacing([14.0, 3.0])
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("Period").strong());
                                        ui.label(egui::RichText::new("Str Buy").strong());
                                        ui.label(egui::RichText::new("Buy").strong());
                                        ui.label(egui::RichText::new("Hold").strong());
                                        ui.label(egui::RichText::new("Sell").strong());
                                        ui.label(egui::RichText::new("Str Sell").strong());
                                        ui.label(egui::RichText::new("Score").strong());
                                        ui.end_row();
                                        for r in self.analyst_recs.iter() {
                                            ui.label(
                                                egui::RichText::new(&r.period).monospace().small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("{}", r.strong_buy))
                                                    .color(UP)
                                                    .monospace()
                                                    .strong(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("{}", r.buy))
                                                    .color(UP)
                                                    .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("{}", r.hold))
                                                    .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("{}", r.sell))
                                                    .color(DOWN)
                                                    .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("{}", r.strong_sell))
                                                    .color(DOWN)
                                                    .monospace()
                                                    .strong(),
                                            );
                                            let total = (r.strong_buy
                                                + r.buy
                                                + r.hold
                                                + r.sell
                                                + r.strong_sell)
                                                as f64;
                                            let score = if total > 0.0 {
                                                (r.strong_buy as f64 * 5.0
                                                    + r.buy as f64 * 4.0
                                                    + r.hold as f64 * 3.0
                                                    + r.sell as f64 * 2.0
                                                    + r.strong_sell as f64)
                                                    / total
                                            } else {
                                                0.0
                                            };
                                            let score_col = if score >= 4.0 {
                                                UP
                                            } else if score <= 2.0 {
                                                DOWN
                                            } else {
                                                AXIS_TEXT
                                            };
                                            ui.label(
                                                egui::RichText::new(format!("{:.2}", score))
                                                    .color(score_col)
                                                    .monospace()
                                                    .strong(),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
                });
            self.show_analyst_recs = open;
        }

        // ESG — Environmental / Social / Governance Scores
        if self.show_esg {
            if self.esg_symbol.is_empty() {
                self.esg_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_esg;
            egui::Window::new("ESG — Environmental / Social / Governance")
                .open(&mut open)
                .resizable(true)
                .default_size([620.0, 400.0])
                .max_size([620.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.esg_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.esg_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.esg_symbol.to_uppercase();
                                    if let Ok(Some(rows)) =
                                        typhoon_engine::core::research::get_esg(&conn, &sym_u)
                                    {
                                        self.esg_rows = rows;
                                        self.esg_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        let have_key = !self.fmp_key.is_empty();
                        if ui
                            .add_enabled(have_key, egui::Button::new("Fetch").fill(BTN_MG))
                            .clicked()
                        {
                            let sym = self.esg_symbol.to_uppercase();
                            self.esg_loading = true;
                            self.esg_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::FetchEsgScores {
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
                        if self.esg_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    if self.esg_rows.is_empty() {
                        ui.label(
                            egui::RichText::new("No ESG data — click Load Cached or Fetch.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                egui::Grid::new("esg_grid")
                                    .striped(true)
                                    .num_columns(5)
                                    .spacing([20.0, 4.0])
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("Year").strong());
                                        ui.label(egui::RichText::new("Environmental").strong());
                                        ui.label(egui::RichText::new("Social").strong());
                                        ui.label(egui::RichText::new("Governance").strong());
                                        ui.label(egui::RichText::new("Overall").strong());
                                        ui.end_row();
                                        let score_color = |s: f64| -> egui::Color32 {
                                            if s >= 70.0 {
                                                UP
                                            } else if s >= 50.0 {
                                                AXIS_TEXT
                                            } else {
                                                DOWN
                                            }
                                        };
                                        for e in self.esg_rows.iter() {
                                            ui.label(
                                                egui::RichText::new(format!("{}", e.year))
                                                    .monospace()
                                                    .strong(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:.1}",
                                                    e.environmental_score
                                                ))
                                                .color(score_color(e.environmental_score))
                                                .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:.1}",
                                                    e.social_score
                                                ))
                                                .color(score_color(e.social_score))
                                                .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:.1}",
                                                    e.governance_score
                                                ))
                                                .color(score_color(e.governance_score))
                                                .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("{:.1}", e.esg_score))
                                                    .color(score_color(e.esg_score))
                                                    .monospace()
                                                    .strong(),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
                });
            self.show_esg = open;
        }

        // MEMB — Index Members (Constituents)
        if self.show_index_members {
            if self.index_code.is_empty() {
                self.index_code = "SP500".to_string();
            }
            let mut open = self.show_index_members;
            egui::Window::new("MEMB — Index Constituents")
                .open(&mut open)
                .resizable(true)
                .default_size([880.0, 560.0])
                .max_size([880.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Index:").color(AXIS_TEXT));
                        for code in ["SP500", "NDX", "DJIA"] {
                            if ui
                                .selectable_label(self.index_code.eq_ignore_ascii_case(code), code)
                                .clicked()
                            {
                                self.index_code = code.to_string();
                            }
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    if let Ok(Some(rows)) =
                                        typhoon_engine::core::research::get_index_members(
                                            &conn,
                                            &self.index_code,
                                        )
                                    {
                                        self.index_members = rows;
                                    }
                                }
                            }
                        }
                        let have_key = !self.fmp_key.is_empty();
                        if ui
                            .add_enabled(have_key, egui::Button::new("Fetch").fill(BTN_MG))
                            .clicked()
                        {
                            self.memb_loading = true;
                            let _ = self.broker_tx.send(BrokerCmd::FetchIndexMembers {
                                index_code: self.index_code.clone(),
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
                        if self.memb_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                        ui.label(egui::RichText::new("Filter:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.memb_filter)
                                .desired_width(180.0)
                                .hint_text("sector / symbol"),
                        );
                    });
                    ui.separator();
                    if self.index_members.is_empty() {
                        ui.label(
                            egui::RichText::new("No constituents — click Load Cached or Fetch.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let filter = self.memb_filter.to_uppercase();
                        let filtered: Vec<&typhoon_engine::core::research::IndexMember> = self
                            .index_members
                            .iter()
                            .filter(|m| {
                                filter.is_empty()
                                    || m.symbol.to_uppercase().contains(&filter)
                                    || m.sector.to_uppercase().contains(&filter)
                                    || m.name.to_uppercase().contains(&filter)
                            })
                            .collect();
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} of {} members",
                                    filtered.len(),
                                    self.index_members.len()
                                ))
                                .strong(),
                            );
                        });
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                egui::Grid::new("memb_grid")
                                    .striped(true)
                                    .num_columns(5)
                                    .spacing([14.0, 3.0])
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("Symbol").strong());
                                        ui.label(egui::RichText::new("Name").strong());
                                        ui.label(egui::RichText::new("Sector").strong());
                                        ui.label(egui::RichText::new("HQ").strong());
                                        ui.label(egui::RichText::new("Added").strong());
                                        ui.end_row();
                                        for m in filtered.iter().take(600) {
                                            ui.label(
                                                egui::RichText::new(&m.symbol).monospace().strong(),
                                            );
                                            let short_name: String =
                                                m.name.chars().take(36).collect();
                                            ui.label(egui::RichText::new(short_name).small());
                                            ui.label(
                                                egui::RichText::new(&m.sector)
                                                    .color(AXIS_TEXT)
                                                    .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(&m.headquarters)
                                                    .color(AXIS_TEXT)
                                                    .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(&m.date_added)
                                                    .monospace()
                                                    .small(),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
                });
            self.show_index_members = open;
        }
    }
}
