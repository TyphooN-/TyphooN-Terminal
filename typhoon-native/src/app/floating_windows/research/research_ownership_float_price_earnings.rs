use super::*;

impl TyphooNApp {
    pub(super) fn render_research_ownership_float_price_earnings_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──

        // INS — Insider Trades (SEC Form-4)
        if self.show_insider_trades {
            if self.insider_symbol.is_empty() {
                self.insider_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_insider_trades;
            egui::Window::new("INS — Insider Trades")
                .open(&mut open)
                .resizable(true)
                .default_size([820.0, 480.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.insider_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.insider_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.insider_symbol.to_uppercase();
                                    if let Ok(Some(rows)) =
                                        typhoon_engine::core::research::get_insider_trades(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.insider_trades = rows;
                                        self.insider_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        let have_key = !self.fmp_key.is_empty();
                        if ui
                            .add_enabled(have_key, egui::Button::new("Fetch").fill(BTN_MG))
                            .clicked()
                        {
                            let sym = self.insider_symbol.to_uppercase();
                            self.insider_loading = true;
                            self.insider_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::FetchInsiderTrades {
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
                        if self.insider_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_insider_trades(ui, &self.insider_trades);
                });
            self.show_insider_trades = open;
        }

        // HDS — Institutional Holders (13F)
        if self.show_inst_holders {
            if self.inst_holders_symbol.is_empty() {
                self.inst_holders_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_inst_holders;
            egui::Window::new("HDS — Institutional Holders")
                .open(&mut open)
                .resizable(true)
                .default_size([720.0, 460.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.inst_holders_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.inst_holders_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.inst_holders_symbol.to_uppercase();
                                    if let Ok(Some(rows)) =
                                        typhoon_engine::core::research::get_institutional_holders(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.institutional_holders = rows;
                                        self.inst_holders_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        let have_key = !self.fmp_key.is_empty();
                        if ui
                            .add_enabled(have_key, egui::Button::new("Fetch").fill(BTN_MG))
                            .clicked()
                        {
                            let sym = self.inst_holders_symbol.to_uppercase();
                            self.inst_holders_loading = true;
                            self.inst_holders_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::FetchInstitutionalHolders {
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
                        if self.inst_holders_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_institutional_holders(ui, &self.institutional_holders);
                });
            self.show_inst_holders = open;
        }

        // FLOAT — Shares Float snapshot
        if self.show_shares_float {
            if self.float_symbol.is_empty() {
                self.float_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_shares_float;
            egui::Window::new("FLOAT — Shares Outstanding")
                .open(&mut open)
                .resizable(true)
                .default_size([460.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.float_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.float_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.float_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_shares_float(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.shares_float = snap;
                                        self.float_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        let have_key = !self.fmp_key.is_empty();
                        if ui
                            .add_enabled(have_key, egui::Button::new("Fetch").fill(BTN_MG))
                            .clicked()
                        {
                            let sym = self.float_symbol.to_uppercase();
                            self.float_loading = true;
                            self.float_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::FetchSharesFloat {
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
                        if self.float_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let sf = &self.shares_float;
                    if sf.symbol.is_empty() && sf.outstanding_shares == 0.0 {
                        ui.label(
                            egui::RichText::new("No shares float — click Load Cached or Fetch.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        egui::Grid::new("float_grid")
                            .num_columns(2)
                            .spacing([24.0, 6.0])
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Symbol").color(AXIS_TEXT));
                                ui.label(egui::RichText::new(&sf.symbol).strong().monospace());
                                ui.end_row();
                                ui.label(egui::RichText::new("As of").color(AXIS_TEXT));
                                ui.label(egui::RichText::new(&sf.date).monospace());
                                ui.end_row();
                                ui.label(egui::RichText::new("Outstanding").color(AXIS_TEXT));
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2}M",
                                        sf.outstanding_shares / 1e6
                                    ))
                                    .strong()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Float").color(AXIS_TEXT));
                                ui.label(
                                    egui::RichText::new(format!("{:.2}M", sf.float_shares / 1e6))
                                        .strong()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Free Float %").color(AXIS_TEXT));
                                ui.label(
                                    egui::RichText::new(format!("{:.2}%", sf.free_float_pct))
                                        .strong()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Source").color(AXIS_TEXT));
                                ui.label(egui::RichText::new(&sf.source).small());
                                ui.end_row();
                            });
                    }
                });
            self.show_shares_float = open;
        }

        // HP — Historical Price table
        if self.show_hist_price {
            if self.hp_symbol.is_empty() {
                self.hp_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_hist_price;
            egui::Window::new("HP — Historical Price")
                .open(&mut open)
                .resizable(true)
                .default_size([760.0, 520.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.hp_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.hp_symbol = chart_sym_research.clone();
                        }
                        ui.label(egui::RichText::new("Bars:").color(AXIS_TEXT));
                        ui.add(egui::Slider::new(&mut self.hp_limit, 30..=1000).show_value(true));
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.hp_symbol.to_uppercase();
                                    if let Ok(Some(rows)) =
                                        typhoon_engine::core::research::get_historical_price(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.hp_rows = rows;
                                        self.hp_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        let have_key = !self.fmp_key.is_empty();
                        if ui
                            .add_enabled(have_key, egui::Button::new("Fetch").fill(BTN_MG))
                            .clicked()
                        {
                            let sym = self.hp_symbol.to_uppercase();
                            self.hp_loading = true;
                            self.hp_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::FetchHistoricalPrice {
                                symbol: sym,
                                fmp_key: self.fmp_key.clone(),
                                limit: self.hp_limit,
                            });
                        }
                        if ui.button("Copy CSV").clicked() && !self.hp_rows.is_empty() {
                            let mut csv = String::from(
                                "date,open,high,low,close,adj_close,volume,change,change_pct\n",
                            );
                            for r in self.hp_rows.iter() {
                                csv.push_str(&format!(
                                    "{},{},{},{},{},{},{},{},{:.4}\n",
                                    r.date,
                                    r.open,
                                    r.high,
                                    r.low,
                                    r.close,
                                    r.adj_close,
                                    r.volume,
                                    r.change,
                                    r.change_pct
                                ));
                            }
                            ui.ctx().copy_text(csv);
                        }
                        if self.fmp_key.is_empty() {
                            ui.label(
                                egui::RichText::new("(add FMP key in Settings)")
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                        }
                        if self.hp_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_hp_rows(ui, &self.hp_rows);
                });
            self.show_hist_price = open;
        }

        // EPS — Earnings Surprise history
        if self.show_eps_surprise {
            if self.eps_symbol.is_empty() {
                self.eps_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_eps_surprise;
            egui::Window::new("EPS — Earnings Surprise")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.eps_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.eps_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.eps_symbol.to_uppercase();
                                    if let Ok(Some(rows)) =
                                        typhoon_engine::core::research::get_earnings_surprises(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.eps_surprises = rows;
                                        self.eps_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        let have_key = !self.fmp_key.is_empty();
                        if ui
                            .add_enabled(have_key, egui::Button::new("Fetch").fill(BTN_MG))
                            .clicked()
                        {
                            let sym = self.eps_symbol.to_uppercase();
                            self.eps_loading = true;
                            self.eps_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::FetchEarningsSurprises {
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
                        if self.eps_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_eps_surprises(ui, &self.eps_surprises);
                });
            self.show_eps_surprise = open;
        }
    }
}
