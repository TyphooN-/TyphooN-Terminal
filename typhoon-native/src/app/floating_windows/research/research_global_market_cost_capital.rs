use super::*;

impl TyphooNApp {
    pub(super) fn render_research_global_market_cost_capital_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──

        // WEI — Global Equity Indices
        if self.show_wei {
            let mut open = self.show_wei;
            egui::Window::new("WEI — Global Equity Indices")
                .open(&mut open)
                .resizable(true)
                .default_size([720.0, 520.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new("Fetch").fill(BTN_MG)).clicked() {
                            self.wei_loading = true;
                            let _ = self.broker_tx.send(BrokerCmd::FetchWorldIndices);
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    if let Ok(Some(rows)) =
                                        typhoon_engine::core::research::get_world_indices(&conn)
                                    {
                                        self.wei_indices = rows;
                                    }
                                }
                            }
                        }
                        ui.label(egui::RichText::new("Region:").color(AXIS_TEXT));
                        egui::ComboBox::from_id_salt("wei_region_filter")
                            .selected_text(if self.wei_region_filter.is_empty() {
                                "All".to_string()
                            } else {
                                self.wei_region_filter.clone()
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.wei_region_filter,
                                    String::new(),
                                    "All",
                                );
                                ui.selectable_value(
                                    &mut self.wei_region_filter,
                                    "Americas".into(),
                                    "Americas",
                                );
                                ui.selectable_value(
                                    &mut self.wei_region_filter,
                                    "EMEA".into(),
                                    "EMEA",
                                );
                                ui.selectable_value(
                                    &mut self.wei_region_filter,
                                    "Asia-Pacific".into(),
                                    "Asia-Pacific",
                                );
                            });
                        if self.wei_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    if self.wei_indices.is_empty() {
                        ui.label(
                            egui::RichText::new(
                                "No data — click Fetch to batch-quote global indices from Yahoo.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                    } else {
                        // Aggregates
                        let filt = self.wei_region_filter.clone();
                        let filtered: Vec<_> = self
                            .wei_indices
                            .iter()
                            .filter(|r| filt.is_empty() || r.region == filt)
                            .collect();
                        let up = filtered.iter().filter(|r| r.change_pct > 0.0).count();
                        let down = filtered.iter().filter(|r| r.change_pct < 0.0).count();
                        ui.label(
                            egui::RichText::new(format!(
                                "{} indices · {} advancing · {} declining",
                                filtered.len(),
                                up,
                                down,
                            ))
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .auto_shrink(false)
                            .show(ui, |ui| {
                                egui::Grid::new("wei_grid")
                                    .striped(true)
                                    .num_columns(5)
                                    .min_col_width(80.0)
                                    .show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new("Region")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Ticker")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Name")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Last")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Chg %")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.end_row();
                                        for r in filtered.iter() {
                                            let col = if r.change_pct > 0.0 {
                                                UP
                                            } else if r.change_pct < 0.0 {
                                                DOWN
                                            } else {
                                                AXIS_TEXT
                                            };
                                            ui.label(
                                                egui::RichText::new(&r.region)
                                                    .small()
                                                    .color(AXIS_TEXT),
                                            );
                                            ui.label(
                                                egui::RichText::new(&r.ticker).small().monospace(),
                                            );
                                            ui.label(egui::RichText::new(&r.display).small());
                                            ui.label(
                                                egui::RichText::new(format!("{:.2}", r.price))
                                                    .small()
                                                    .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:+.2}%",
                                                    r.change_pct
                                                ))
                                                .color(col)
                                                .small()
                                                .strong()
                                                .monospace(),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
                });
            self.show_wei = open;
        }

        // MOV — Market Movers
        if self.show_market_movers {
            let mut open = self.show_market_movers;
            egui::Window::new("MOV — Market Movers")
                .open(&mut open)
                .resizable(true)
                .default_size([860.0, 540.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        let have_key = !self.fmp_key.is_empty();
                        if ui.add_enabled(have_key, egui::Button::new("Fetch").fill(BTN_MG)).clicked() {
                            self.mov_loading = true;
                            let _ = self.broker_tx.send(BrokerCmd::FetchMarketMovers {
                                fmp_key: self.fmp_key.clone(),
                            });
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    if let Ok(Some(mov)) = typhoon_engine::core::research::get_market_movers(&conn) {
                                        self.market_movers = mov;
                                    }
                                }
                            }
                        }
                        if self.fmp_key.is_empty() {
                            ui.label(egui::RichText::new("(add FMP key in Settings)").color(AXIS_TEXT).small());
                        }
                        if self.mov_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let render_col = |ui: &mut egui::Ui, title: &str, rows: &[typhoon_engine::core::research::MarketMover]| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(title).strong());
                            egui::ScrollArea::vertical().id_salt(title).max_height(460.0).show(ui, |ui| {
                                egui::Grid::new(format!("mov_{}_grid", title)).striped(true).num_columns(4).show(ui, |ui| {
                                    ui.label(egui::RichText::new("Sym").color(AXIS_TEXT).small().strong());
                                    ui.label(egui::RichText::new("Last").color(AXIS_TEXT).small().strong());
                                    ui.label(egui::RichText::new("Chg %").color(AXIS_TEXT).small().strong());
                                    ui.label(egui::RichText::new("Vol").color(AXIS_TEXT).small().strong());
                                    ui.end_row();
                                    for m in rows.iter().take(25) {
                                        let col = if m.change_pct > 0.0 { UP }
                                                  else if m.change_pct < 0.0 { DOWN }
                                                  else { AXIS_TEXT };
                                        ui.label(egui::RichText::new(&m.symbol).small().monospace().strong());
                                        ui.label(egui::RichText::new(format!("{:.2}", m.price)).small().monospace());
                                        ui.label(egui::RichText::new(format!("{:+.2}%", m.change_pct)).color(col).small().monospace().strong());
                                        ui.label(egui::RichText::new(format!("{:.1}M", m.volume / 1e6)).small().monospace().color(AXIS_TEXT));
                                        ui.end_row();
                                    }
                                });
                            });
                        });
                    };
                    ui.horizontal(|ui| {
                        render_col(ui, "Top Gainers",  &self.market_movers.gainers);
                        ui.separator();
                        render_col(ui, "Top Losers",   &self.market_movers.losers);
                        ui.separator();
                        render_col(ui, "Most Active",  &self.market_movers.actives);
                    });
                });
            self.show_market_movers = open;
        }

        // INDU — Sector Performance heatmap
        if self.show_sector_perf {
            let mut open = self.show_sector_perf;
            egui::Window::new("INDU — Sector Performance")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        let have_key = !self.fmp_key.is_empty();
                        if ui
                            .add_enabled(have_key, egui::Button::new("Fetch").fill(BTN_MG))
                            .clicked()
                        {
                            self.indu_loading = true;
                            let _ = self.broker_tx.send(BrokerCmd::FetchSectorPerformance {
                                fmp_key: self.fmp_key.clone(),
                            });
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    if let Ok(Some(rows)) =
                                        typhoon_engine::core::research::get_sector_performance(
                                            &conn,
                                        )
                                    {
                                        self.sector_perf = rows;
                                    }
                                }
                            }
                        }
                        if self.fmp_key.is_empty() {
                            ui.label(
                                egui::RichText::new("(add FMP key in Settings)")
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                        }
                        if self.indu_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_sector_perf(ui, &self.sector_perf);
                });
            self.show_sector_perf = open;
        }

        // CACS — Corporate Actions Calendar (UI-only aggregator)
        if self.show_cacs {
            let mut open = self.show_cacs;
            egui::Window::new("CACS — Corporate Actions")
                .open(&mut open)
                .resizable(true)
                .default_size([760.0, 520.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cacs_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cacs_symbol = chart_sym_research.clone(); }
                        ui.label(egui::RichText::new("Aggregates cached Splits / Dividends / Earnings / IPO data.")
                            .color(AXIS_TEXT).small());
                    });
                    ui.separator();
                    let sym_u = self.cacs_symbol.to_uppercase();
                    if sym_u.is_empty() {
                        ui.label(egui::RichText::new("Enter a symbol to view its aggregated corporate actions timeline.")
                            .color(AXIS_TEXT).small());
                    } else if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            use typhoon_engine::core::research as rx;
                            #[derive(Clone)]
                            struct Event { date: String, kind: &'static str, detail: String }
                            let mut events: Vec<Event> = Vec::new();
                            if let Ok(Some(splits)) = rx::get_stock_splits(&conn, &sym_u) {
                                for s in splits.iter() {
                                    events.push(Event {
                                        date: s.date.clone(), kind: "SPLIT",
                                        detail: if s.label.is_empty() { format!("{}:{}", s.numerator, s.denominator) }
                                                else { s.label.clone() },
                                    });
                                }
                            }
                            if let Ok(Some(divs)) = rx::get_dividends(&conn, &sym_u) {
                                for d in divs.iter() {
                                    events.push(Event {
                                        date: d.ex_date.clone(), kind: "DIV",
                                        detail: format!("${:.4} · {}", d.amount,
                                            if d.label.is_empty() { "Regular".to_string() } else { d.label.clone() }),
                                    });
                                }
                            }
                            if let Ok(Some(surprises)) = rx::get_earnings_surprises(&conn, &sym_u) {
                                for s in surprises.iter() {
                                    events.push(Event {
                                        date: s.date.clone(), kind: "EARN",
                                        detail: format!("actual ${:.2} · est ${:.2} · {:+.2}%",
                                            s.eps_actual, s.eps_estimate, s.surprise_pct),
                                    });
                                }
                            }
                            if let Ok(Some(ipos)) = rx::get_ipo_calendar(&conn) {
                                for ev in ipos.iter().filter(|e| e.symbol.eq_ignore_ascii_case(&sym_u)) {
                                    events.push(Event {
                                        date: ev.date.clone(), kind: "IPO",
                                        detail: format!("{} @ {} ({} sh)", ev.exchange, ev.price_range, ev.shares),
                                    });
                                }
                            }
                            events.sort_by(|a, b| b.date.cmp(&a.date));
                            ui.label(egui::RichText::new(format!("{} events across all action types", events.len()))
                                .color(AXIS_TEXT).small());
                            ui.separator();
                            if events.is_empty() {
                                ui.label(egui::RichText::new(format!("No cached corporate actions for {}. Run SPLT / DVD / EPS / IPO first.", sym_u))
                                    .color(AXIS_TEXT).small());
                            } else {
                                egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                                    egui::Grid::new("cacs_grid").striped(true).num_columns(3).min_col_width(100.0).show(ui, |ui| {
                                        ui.label(egui::RichText::new("Date").color(AXIS_TEXT).small().strong());
                                        ui.label(egui::RichText::new("Type").color(AXIS_TEXT).small().strong());
                                        ui.label(egui::RichText::new("Detail").color(AXIS_TEXT).small().strong());
                                        ui.end_row();
                                        for ev in events.iter() {
                                            let col = match ev.kind {
                                                "SPLIT" => egui::Color32::from_rgb(150, 200, 255),
                                                "DIV"   => UP,
                                                "EARN"  => egui::Color32::from_rgb(255, 200, 100),
                                                "IPO"   => egui::Color32::from_rgb(200, 150, 255),
                                                _       => AXIS_TEXT,
                                            };
                                            ui.label(egui::RichText::new(&ev.date).small().monospace());
                                            ui.label(egui::RichText::new(ev.kind).color(col).small().strong().monospace());
                                            ui.label(egui::RichText::new(&ev.detail).small());
                                            ui.end_row();
                                        }
                                    });
                                });
                            }
                        }
                    } else {
                        ui.label(egui::RichText::new("Cache not available.").color(AXIS_TEXT).small());
                    }
                });
            self.show_cacs = open;
        }

        // WACC — Cost of Capital snapshot
        if self.show_wacc {
            if self.wacc_symbol.is_empty() {
                self.wacc_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_wacc;
            egui::Window::new("WACC — Cost of Capital (CAPM)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 480.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.wacc_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.wacc_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.wacc_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_wacc(&conn, &sym_u)
                                    {
                                        self.wacc_snapshot = snap;
                                        self.wacc_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        let have_key = !self.fmp_key.is_empty();
                        if ui
                            .add_enabled(have_key, egui::Button::new("Fetch").fill(BTN_MG))
                            .clicked()
                        {
                            let sym = self.wacc_symbol.to_uppercase();
                            self.wacc_loading = true;
                            self.wacc_symbol = sym.clone();
                            let rf = self
                                .treasury_yields
                                .iter()
                                .find(|y| y.tenor == "10Y")
                                .map(|y| y.yield_pct)
                                .unwrap_or(4.5);
                            let _ = self.broker_tx.send(BrokerCmd::FetchWaccSnapshot {
                                symbol: sym,
                                fmp_key: self.fmp_key.clone(),
                                risk_free_pct: rf,
                            });
                        }
                        if self.fmp_key.is_empty() {
                            ui.label(
                                egui::RichText::new("(add FMP key in Settings)")
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                        }
                        if self.wacc_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_wacc_snapshot(ui, &self.wacc_snapshot);
                });
            self.show_wacc = open;
        }
    }
}
