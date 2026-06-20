use super::*;

impl TyphooNApp {
    pub(super) fn render_research_ownership_float_price_earnings_windows(
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
                    ui.separator();
                    if self.insider_trades.is_empty() {
                        ui.label(
                            egui::RichText::new("No insider trades — click Load Cached or Fetch.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        // Derive a quick net-flow summary.
                        let (mut bought, mut sold) = (0.0_f64, 0.0_f64);
                        for t in self.insider_trades.iter() {
                            match t.acquisition_disposition.as_str() {
                                "A" => bought += t.value_usd,
                                "D" => sold += t.value_usd,
                                _ => {}
                            }
                        }
                        let net = bought - sold;
                        let net_col = if net >= 0.0 { UP } else { DOWN };
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} filings",
                                    self.insider_trades.len()
                                ))
                                .strong(),
                            );
                            ui.label(
                                egui::RichText::new(format!("Buys: ${:.1}M", bought / 1e6))
                                    .color(UP)
                                    .monospace()
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!("Sells: ${:.1}M", sold / 1e6))
                                    .color(DOWN)
                                    .monospace()
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!("Net: ${:.1}M", net / 1e6))
                                    .color(net_col)
                                    .strong()
                                    .monospace()
                                    .small(),
                            );
                        });
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                egui::Grid::new("ins_grid")
                                    .striped(true)
                                    .num_columns(7)
                                    .spacing([14.0, 4.0])
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("Filed").strong());
                                        ui.label(egui::RichText::new("Tx Date").strong());
                                        ui.label(egui::RichText::new("Insider").strong());
                                        ui.label(egui::RichText::new("Type").strong());
                                        ui.label(egui::RichText::new("Shares").strong());
                                        ui.label(egui::RichText::new("Price").strong());
                                        ui.label(egui::RichText::new("Value").strong());
                                        ui.end_row();
                                        for t in self.insider_trades.iter().take(100) {
                                            let dir_col = match t.acquisition_disposition.as_str() {
                                                "A" => UP,
                                                "D" => DOWN,
                                                _ => AXIS_TEXT,
                                            };
                                            ui.label(
                                                egui::RichText::new(&t.filing_date)
                                                    .monospace()
                                                    .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(&t.transaction_date)
                                                    .monospace()
                                                    .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(&t.reporting_name).small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(&t.transaction_type)
                                                    .color(dir_col)
                                                    .monospace()
                                                    .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("{:.0}", t.shares))
                                                    .monospace()
                                                    .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("${:.2}", t.price))
                                                    .monospace()
                                                    .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "${:.1}k",
                                                    t.value_usd / 1e3
                                                ))
                                                .color(dir_col)
                                                .monospace()
                                                .small(),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
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
                    ui.separator();
                    if self.institutional_holders.is_empty() {
                        ui.label(
                            egui::RichText::new(
                                "No institutional holders — click Load Cached or Fetch.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                    } else {
                        let total: f64 = self.institutional_holders.iter().map(|h| h.shares).sum();
                        ui.label(
                            egui::RichText::new(format!(
                                "{} holders — {:.1}M total shares",
                                self.institutional_holders.len(),
                                total / 1e6
                            ))
                            .strong(),
                        );
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                egui::Grid::new("hds_grid")
                                    .striped(true)
                                    .num_columns(4)
                                    .spacing([18.0, 4.0])
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("Holder").strong());
                                        ui.label(egui::RichText::new("Shares").strong());
                                        ui.label(egui::RichText::new("QoQ Δ").strong());
                                        ui.label(egui::RichText::new("Reported").strong());
                                        ui.end_row();
                                        for h in self.institutional_holders.iter().take(200) {
                                            let chg_col = if h.change > 0.0 {
                                                UP
                                            } else if h.change < 0.0 {
                                                DOWN
                                            } else {
                                                AXIS_TEXT
                                            };
                                            ui.label(egui::RichText::new(&h.holder).small());
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:.2}M",
                                                    h.shares / 1e6
                                                ))
                                                .monospace()
                                                .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:+.2}M",
                                                    h.change / 1e6
                                                ))
                                                .color(chg_col)
                                                .monospace()
                                                .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(&h.date_reported)
                                                    .monospace()
                                                    .small(),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
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
                    ui.separator();
                    if self.hp_rows.is_empty() {
                        ui.label(
                            egui::RichText::new("No historical bars — click Load Cached or Fetch.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new(format!("{} daily bars", self.hp_rows.len()))
                                .strong(),
                        );
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                egui::Grid::new("hp_grid")
                                    .striped(true)
                                    .num_columns(8)
                                    .spacing([14.0, 4.0])
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("Date").strong());
                                        ui.label(egui::RichText::new("Open").strong());
                                        ui.label(egui::RichText::new("High").strong());
                                        ui.label(egui::RichText::new("Low").strong());
                                        ui.label(egui::RichText::new("Close").strong());
                                        ui.label(egui::RichText::new("Volume").strong());
                                        ui.label(egui::RichText::new("Chg").strong());
                                        ui.label(egui::RichText::new("Chg %").strong());
                                        ui.end_row();
                                        for r in self.hp_rows.iter() {
                                            let chg_col = if r.change >= 0.0 { UP } else { DOWN };
                                            ui.label(
                                                egui::RichText::new(&r.date).monospace().small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("{:.2}", r.open))
                                                    .monospace()
                                                    .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("{:.2}", r.high))
                                                    .monospace()
                                                    .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("{:.2}", r.low))
                                                    .monospace()
                                                    .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("{:.2}", r.close))
                                                    .strong()
                                                    .monospace()
                                                    .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:.1}M",
                                                    r.volume / 1e6
                                                ))
                                                .color(AXIS_TEXT)
                                                .monospace()
                                                .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("{:+.2}", r.change))
                                                    .color(chg_col)
                                                    .monospace()
                                                    .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:+.2}%",
                                                    r.change_pct
                                                ))
                                                .color(chg_col)
                                                .monospace()
                                                .small(),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
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
                    ui.separator();
                    if self.eps_surprises.is_empty() {
                        ui.label(
                            egui::RichText::new("No EPS history — click Load Cached or Fetch.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let beats = self
                            .eps_surprises
                            .iter()
                            .filter(|s| s.surprise > 0.0)
                            .count();
                        let misses = self
                            .eps_surprises
                            .iter()
                            .filter(|s| s.surprise < 0.0)
                            .count();
                        let avg_surprise: f64 = self
                            .eps_surprises
                            .iter()
                            .take(8)
                            .map(|s| s.surprise_pct)
                            .sum::<f64>()
                            / self.eps_surprises.iter().take(8).count().max(1) as f64;
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} quarters",
                                    self.eps_surprises.len()
                                ))
                                .strong(),
                            );
                            ui.label(
                                egui::RichText::new(format!("Beats: {}", beats))
                                    .color(UP)
                                    .monospace()
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!("Misses: {}", misses))
                                    .color(DOWN)
                                    .monospace()
                                    .small(),
                            );
                            let avg_col = if avg_surprise >= 0.0 { UP } else { DOWN };
                            ui.label(
                                egui::RichText::new(format!("8Q avg: {:+.2}%", avg_surprise))
                                    .color(avg_col)
                                    .strong()
                                    .monospace()
                                    .small(),
                            );
                        });
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                egui::Grid::new("eps_grid")
                                    .striped(true)
                                    .num_columns(5)
                                    .spacing([20.0, 4.0])
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("Date").strong());
                                        ui.label(egui::RichText::new("Actual").strong());
                                        ui.label(egui::RichText::new("Estimate").strong());
                                        ui.label(egui::RichText::new("Surprise").strong());
                                        ui.label(egui::RichText::new("Surprise %").strong());
                                        ui.end_row();
                                        for s in self.eps_surprises.iter() {
                                            let col = if s.surprise >= 0.0 { UP } else { DOWN };
                                            ui.label(
                                                egui::RichText::new(&s.date).monospace().small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "${:.2}",
                                                    s.eps_actual
                                                ))
                                                .strong()
                                                .monospace()
                                                .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "${:.2}",
                                                    s.eps_estimate
                                                ))
                                                .monospace()
                                                .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("{:+.2}", s.surprise))
                                                    .color(col)
                                                    .monospace()
                                                    .small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:+.2}%",
                                                    s.surprise_pct
                                                ))
                                                .color(col)
                                                .strong()
                                                .monospace()
                                                .small(),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
                });
            self.show_eps_surprise = open;
        }
    }
}
