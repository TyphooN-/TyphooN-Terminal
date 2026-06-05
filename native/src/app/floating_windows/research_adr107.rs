use super::*;

impl TyphooNApp {
    pub(super) fn render_research_adr107_windows(&mut self, ctx: &egui::Context) {
        // ── Godel parity research windows (ADR-107) ───────────────────────
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

        // DES — Company Description (profile + peers + earnings + press)
        if self.show_company_desc {
            if self.desc_symbol.is_empty() {
                self.desc_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_company_desc;
            let mut open_url: Option<String> = None;
            egui::Window::new("DES — Company Description")
                .open(&mut open)
                .resizable(true)
                .default_size([780.0, 560.0])
                .max_size([780.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.desc_symbol).desired_width(90.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.desc_symbol = chart_sym_research.clone();
                        }
                        ui.separator();
                        if ui
                            .add_enabled(
                                !self.desc_loading,
                                egui::Button::new("Load Cached").fill(BTN_GREEN),
                            )
                            .clicked()
                        {
                            let sym = self.desc_symbol.trim().to_uppercase();
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    self.desc_profile =
                                        typhoon_engine::core::research::get_profile(&conn, &sym)
                                            .ok()
                                            .flatten();
                                    self.desc_peers =
                                        typhoon_engine::core::research::get_peers(&conn, &sym)
                                            .ok()
                                            .flatten()
                                            .unwrap_or_default();
                                    self.desc_earnings =
                                        typhoon_engine::core::research::get_earnings_history(
                                            &conn, &sym,
                                        )
                                        .ok()
                                        .flatten()
                                        .unwrap_or_default();
                                    self.desc_press =
                                        typhoon_engine::core::research::get_press_releases(
                                            &conn, &sym,
                                        )
                                        .ok()
                                        .flatten()
                                        .unwrap_or_default();
                                }
                            }
                        }
                        if ui
                            .add_enabled(
                                !self.desc_loading && !self.finnhub_key.is_empty(),
                                egui::Button::new("Fetch All").fill(BTN_BLUE),
                            )
                            .clicked()
                        {
                            let sym = self.desc_symbol.trim().to_uppercase();
                            if !sym.is_empty() {
                                self.desc_loading = true;
                                let fk = self.finnhub_key.clone();
                                let _ = self.broker_tx.send(BrokerCmd::FetchCompanyProfile {
                                    symbol: sym.clone(),
                                    finnhub_key: fk.clone(),
                                });
                                let _ = self.broker_tx.send(BrokerCmd::FetchStockPeers {
                                    symbol: sym.clone(),
                                    finnhub_key: fk.clone(),
                                });
                                let _ = self.broker_tx.send(BrokerCmd::FetchEarningsHistory {
                                    symbol: sym.clone(),
                                    finnhub_key: fk.clone(),
                                });
                                let _ = self.broker_tx.send(BrokerCmd::FetchPressReleases {
                                    symbol: sym,
                                    finnhub_key: fk,
                                });
                            }
                        }
                        if self.desc_loading {
                            ui.spinner();
                        }
                    });
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            if let Some(p) = self.desc_profile.clone() {
                                ui.heading(
                                    egui::RichText::new(format!("{}  —  {}", p.symbol, p.name))
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}  •  {}  •  {}",
                                        p.exchange, p.country, p.currency
                                    ))
                                    .color(AXIS_TEXT),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Sector: {}    Industry: {}",
                                        p.sector, p.industry
                                    ))
                                    .color(AXIS_TEXT),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "IPO: {}    Market Cap: ${:.0}M    Shares Out: {:.1}M",
                                        p.ipo_date, p.market_cap, p.shares_outstanding
                                    ))
                                    .color(AXIS_TEXT),
                                );
                                if !p.website.is_empty() {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("Website:").color(AXIS_TEXT));
                                        if ui.link(&p.website).clicked() {
                                            open_url = Some(p.website.clone());
                                        }
                                    });
                                }
                                if !p.phone.is_empty() {
                                    ui.label(
                                        egui::RichText::new(format!("Phone: {}", p.phone))
                                            .color(AXIS_TEXT),
                                    );
                                }
                            } else {
                                ui.label(
                                    egui::RichText::new(
                                        "No profile cached — click Fetch All (needs Finnhub key).",
                                    )
                                    .color(AXIS_TEXT),
                                );
                            }
                            ui.separator();
                            ui.collapsing(
                                egui::RichText::new(format!("Peers ({})", self.desc_peers.len()))
                                    .strong(),
                                |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        for peer in self.desc_peers.iter() {
                                            ui.label(
                                                egui::RichText::new(peer).color(BTN_BLUE_TEXT),
                                            );
                                        }
                                    });
                                },
                            );
                            ui.collapsing(
                                egui::RichText::new(format!(
                                    "Earnings History ({})",
                                    self.desc_earnings.len()
                                ))
                                .strong(),
                                |ui| {
                                    egui::Grid::new("des_earnings_grid").striped(true).show(
                                        ui,
                                        |ui| {
                                            ui.label(egui::RichText::new("Period").strong());
                                            ui.label(egui::RichText::new("Actual").strong());
                                            ui.label(egui::RichText::new("Estimate").strong());
                                            ui.label(egui::RichText::new("Surprise %").strong());
                                            ui.end_row();
                                            for r in self.desc_earnings.iter().take(12) {
                                                ui.label(&r.period);
                                                ui.label(
                                                    r.actual
                                                        .map(|v| format!("{:.2}", v))
                                                        .unwrap_or_default(),
                                                );
                                                ui.label(
                                                    r.estimate
                                                        .map(|v| format!("{:.2}", v))
                                                        .unwrap_or_default(),
                                                );
                                                let col = match r.surprise_pct {
                                                    Some(v) if v > 0.0 => BTN_GREEN_TEXT,
                                                    Some(v) if v < 0.0 => BTN_RED_TEXT,
                                                    _ => AXIS_TEXT,
                                                };
                                                ui.label(
                                                    egui::RichText::new(
                                                        r.surprise_pct
                                                            .map(|v| format!("{:+.1}%", v))
                                                            .unwrap_or_default(),
                                                    )
                                                    .color(col),
                                                );
                                                ui.end_row();
                                            }
                                        },
                                    );
                                },
                            );
                            ui.collapsing(
                                egui::RichText::new(format!(
                                    "Recent Press ({})",
                                    self.desc_press.len()
                                ))
                                .strong(),
                                |ui| {
                                    for pr in self.desc_press.iter().take(20) {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(&pr.datetime)
                                                    .color(AXIS_TEXT)
                                                    .small(),
                                            );
                                            if ui.link(&pr.headline).clicked() {
                                                open_url = Some(pr.url.clone());
                                            }
                                        });
                                    }
                                },
                            );
                        });
                });
            self.show_company_desc = open;
            if let Some(u) = open_url {
                ctx.open_url(egui::OpenUrl::new_tab(u));
            }
        }

        // IPO Calendar
        if self.show_ipo_calendar {
            let mut open = self.show_ipo_calendar;
            egui::Window::new("IPO Calendar")
                .open(&mut open)
                .resizable(true)
                .default_size([760.0, 480.0])
                .max_size([760.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                !self.ipo_loading && !self.finnhub_key.is_empty(),
                                egui::Button::new("Fetch (±30d)").fill(BTN_BLUE),
                            )
                            .clicked()
                        {
                            self.ipo_loading = true;
                            let _ = self.broker_tx.send(BrokerCmd::FetchIpoCalendar {
                                finnhub_key: self.finnhub_key.clone(),
                                days_ahead: 30,
                                days_back: 30,
                            });
                        }
                        if ui
                            .add_enabled(
                                !self.ipo_loading,
                                egui::Button::new("Load Cached").fill(BTN_GREEN),
                            )
                            .clicked()
                        {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    self.ipo_events =
                                        typhoon_engine::core::research::get_ipo_calendar(&conn)
                                            .ok()
                                            .flatten()
                                            .unwrap_or_default();
                                }
                            }
                        }
                        if self.ipo_loading {
                            ui.spinner();
                        }
                        ui.label(
                            egui::RichText::new(format!("{} events", self.ipo_events.len()))
                                .color(AXIS_TEXT),
                        );
                    });
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let mut rows = self.ipo_events.clone();
                            rows.sort_by(|a, b| {
                                let ord = match self.ipo_sort_col {
                                    0 => a.date.cmp(&b.date),
                                    1 => a.symbol.cmp(&b.symbol),
                                    2 => a.name.cmp(&b.name),
                                    3 => a.exchange.cmp(&b.exchange),
                                    4 => a.price_range.cmp(&b.price_range),
                                    5 => a.shares.cmp(&b.shares),
                                    6 => a.total_value.total_cmp(&b.total_value),
                                    7 => a.status.cmp(&b.status),
                                    _ => a.date.cmp(&b.date),
                                };
                                if self.ipo_sort_asc {
                                    ord
                                } else {
                                    ord.reverse()
                                }
                            });
                            egui::Grid::new("ipo_grid").striped(true).show(ui, |ui| {
                                sortable_header(
                                    ui,
                                    "Date",
                                    0,
                                    &mut self.ipo_sort_col,
                                    &mut self.ipo_sort_asc,
                                );
                                sortable_header(
                                    ui,
                                    "Symbol",
                                    1,
                                    &mut self.ipo_sort_col,
                                    &mut self.ipo_sort_asc,
                                );
                                sortable_header(
                                    ui,
                                    "Name",
                                    2,
                                    &mut self.ipo_sort_col,
                                    &mut self.ipo_sort_asc,
                                );
                                sortable_header(
                                    ui,
                                    "Exchange",
                                    3,
                                    &mut self.ipo_sort_col,
                                    &mut self.ipo_sort_asc,
                                );
                                sortable_header(
                                    ui,
                                    "Price Range",
                                    4,
                                    &mut self.ipo_sort_col,
                                    &mut self.ipo_sort_asc,
                                );
                                sortable_header(
                                    ui,
                                    "Shares",
                                    5,
                                    &mut self.ipo_sort_col,
                                    &mut self.ipo_sort_asc,
                                );
                                sortable_header(
                                    ui,
                                    "Total Value",
                                    6,
                                    &mut self.ipo_sort_col,
                                    &mut self.ipo_sort_asc,
                                );
                                sortable_header(
                                    ui,
                                    "Status",
                                    7,
                                    &mut self.ipo_sort_col,
                                    &mut self.ipo_sort_asc,
                                );
                                ui.end_row();
                                for e in rows.iter() {
                                    ui.label(&e.date);
                                    ui.label(
                                        egui::RichText::new(&e.symbol)
                                            .color(BTN_BLUE_TEXT)
                                            .strong(),
                                    );
                                    ui.label(&e.name);
                                    ui.label(&e.exchange);
                                    ui.label(&e.price_range);
                                    ui.label(if e.shares > 0 {
                                        format!("{}", e.shares)
                                    } else {
                                        String::new()
                                    });
                                    ui.label(if e.total_value > 0.0 {
                                        format!("${:.1}M", e.total_value / 1e6)
                                    } else {
                                        String::new()
                                    });
                                    ui.label(&e.status);
                                    ui.end_row();
                                }
                            });
                        });
                });
            self.show_ipo_calendar = open;
        }

        // Earnings History (ERN)
        if self.show_earnings_history {
            if self.earnings_history_symbol.is_empty() {
                self.earnings_history_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_earnings_history;
            egui::Window::new("ERN — Earnings History")
                .open(&mut open)
                .resizable(true)
                .default_size([700.0, 460.0])
                .max_size([700.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.earnings_history_symbol)
                                .desired_width(90.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.earnings_history_symbol = chart_sym_research.clone();
                        }
                        if ui
                            .add_enabled(
                                !self.earnings_history_loading,
                                egui::Button::new("Load Cached").fill(BTN_GREEN),
                            )
                            .clicked()
                        {
                            let sym = self.earnings_history_symbol.trim().to_uppercase();
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    self.earnings_history_rows =
                                        typhoon_engine::core::research::get_earnings_history(
                                            &conn, &sym,
                                        )
                                        .ok()
                                        .flatten()
                                        .unwrap_or_default();
                                }
                            }
                        }
                        if ui
                            .add_enabled(
                                !self.earnings_history_loading && !self.finnhub_key.is_empty(),
                                egui::Button::new("Fetch").fill(BTN_BLUE),
                            )
                            .clicked()
                        {
                            let sym = self.earnings_history_symbol.trim().to_uppercase();
                            if !sym.is_empty() {
                                self.earnings_history_loading = true;
                                let _ = self.broker_tx.send(BrokerCmd::FetchEarningsHistory {
                                    symbol: sym,
                                    finnhub_key: self.finnhub_key.clone(),
                                });
                            }
                        }
                        if self.earnings_history_loading {
                            ui.spinner();
                        }
                    });
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let mut rows = self.earnings_history_rows.clone();
                            rows.sort_by(|a, b| {
                                let ord = match self.earnings_history_sort_col {
                                    0 => a.period.cmp(&b.period),
                                    1 => a.quarter.cmp(&b.quarter),
                                    2 => a.year.cmp(&b.year),
                                    3 => a
                                        .actual
                                        .unwrap_or(f64::NEG_INFINITY)
                                        .total_cmp(&b.actual.unwrap_or(f64::NEG_INFINITY)),
                                    4 => a
                                        .estimate
                                        .unwrap_or(f64::NEG_INFINITY)
                                        .total_cmp(&b.estimate.unwrap_or(f64::NEG_INFINITY)),
                                    5 => a
                                        .surprise
                                        .unwrap_or(f64::NEG_INFINITY)
                                        .total_cmp(&b.surprise.unwrap_or(f64::NEG_INFINITY)),
                                    6 => a
                                        .surprise_pct
                                        .unwrap_or(f64::NEG_INFINITY)
                                        .total_cmp(&b.surprise_pct.unwrap_or(f64::NEG_INFINITY)),
                                    _ => a.period.cmp(&b.period),
                                };
                                if self.earnings_history_sort_asc {
                                    ord
                                } else {
                                    ord.reverse()
                                }
                            });
                            egui::Grid::new("ern_grid").striped(true).show(ui, |ui| {
                                sortable_header(
                                    ui,
                                    "Period",
                                    0,
                                    &mut self.earnings_history_sort_col,
                                    &mut self.earnings_history_sort_asc,
                                );
                                sortable_header(
                                    ui,
                                    "Qtr",
                                    1,
                                    &mut self.earnings_history_sort_col,
                                    &mut self.earnings_history_sort_asc,
                                );
                                sortable_header(
                                    ui,
                                    "Year",
                                    2,
                                    &mut self.earnings_history_sort_col,
                                    &mut self.earnings_history_sort_asc,
                                );
                                sortable_header(
                                    ui,
                                    "Actual",
                                    3,
                                    &mut self.earnings_history_sort_col,
                                    &mut self.earnings_history_sort_asc,
                                );
                                sortable_header(
                                    ui,
                                    "Estimate",
                                    4,
                                    &mut self.earnings_history_sort_col,
                                    &mut self.earnings_history_sort_asc,
                                );
                                sortable_header(
                                    ui,
                                    "Surprise",
                                    5,
                                    &mut self.earnings_history_sort_col,
                                    &mut self.earnings_history_sort_asc,
                                );
                                sortable_header(
                                    ui,
                                    "Surprise %",
                                    6,
                                    &mut self.earnings_history_sort_col,
                                    &mut self.earnings_history_sort_asc,
                                );
                                ui.end_row();
                                for r in rows.iter() {
                                    ui.label(&r.period);
                                    ui.label(
                                        r.quarter.map(|v| format!("Q{}", v)).unwrap_or_default(),
                                    );
                                    ui.label(r.year.map(|v| v.to_string()).unwrap_or_default());
                                    ui.label(
                                        r.actual.map(|v| format!("{:.2}", v)).unwrap_or_default(),
                                    );
                                    ui.label(
                                        r.estimate.map(|v| format!("{:.2}", v)).unwrap_or_default(),
                                    );
                                    ui.label(
                                        r.surprise
                                            .map(|v| format!("{:+.2}", v))
                                            .unwrap_or_default(),
                                    );
                                    let col = match r.surprise_pct {
                                        Some(v) if v > 0.0 => BTN_GREEN_TEXT,
                                        Some(v) if v < 0.0 => BTN_RED_TEXT,
                                        _ => AXIS_TEXT,
                                    };
                                    ui.label(
                                        egui::RichText::new(
                                            r.surprise_pct
                                                .map(|v| format!("{:+.1}%", v))
                                                .unwrap_or_default(),
                                        )
                                        .color(col),
                                    );
                                    ui.end_row();
                                }
                            });
                        });
                });
            self.show_earnings_history = open;
        }

        // Stock Peers
        if self.show_peers {
            if self.peers_symbol.is_empty() {
                self.peers_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_peers;
            let mut jump_to: Option<String> = None;
            egui::Window::new("PEERS — Stock Peers")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 420.0])
                .max_size([520.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.peers_symbol).desired_width(90.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.peers_symbol = chart_sym_research.clone();
                        }
                        if ui
                            .add_enabled(
                                !self.peers_loading,
                                egui::Button::new("Load Cached").fill(BTN_GREEN),
                            )
                            .clicked()
                        {
                            let sym = self.peers_symbol.trim().to_uppercase();
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    self.peers_list =
                                        typhoon_engine::core::research::get_peers(&conn, &sym)
                                            .ok()
                                            .flatten()
                                            .unwrap_or_default();
                                }
                            }
                        }
                        if ui
                            .add_enabled(
                                !self.peers_loading && !self.finnhub_key.is_empty(),
                                egui::Button::new("Fetch").fill(BTN_BLUE),
                            )
                            .clicked()
                        {
                            let sym = self.peers_symbol.trim().to_uppercase();
                            if !sym.is_empty() {
                                self.peers_loading = true;
                                let _ = self.broker_tx.send(BrokerCmd::FetchStockPeers {
                                    symbol: sym,
                                    finnhub_key: self.finnhub_key.clone(),
                                });
                            }
                        }
                        if self.peers_loading {
                            ui.spinner();
                        }
                    });
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for peer in self.peers_list.iter() {
                                if ui
                                    .button(egui::RichText::new(peer).color(BTN_BLUE_TEXT).strong())
                                    .on_hover_text("Click to load in chart")
                                    .clicked()
                                {
                                    jump_to = Some(peer.clone());
                                }
                            }
                        });
                });
            self.show_peers = open;
            if let Some(sym) = jump_to {
                self.symbol_input = sym.clone();
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    chart.symbol = sym.clone();
                    if let Some(ref cache_arc) = self.cache {
                        let mut gpu = self.gpu_indicators.take();
                        if !chart.try_load(Arc::as_ref(cache_arc), &mut self.log, gpu.as_mut()) {
                            self.queue_chart_reload(self.active_tab);
                        }
                        self.gpu_indicators = gpu;
                    }
                }
                self.log
                    .push_back(LogEntry::info(format!("Chart: {}", sym)));
            }
        }

        // Press Releases
        if self.show_press_releases {
            if self.press_symbol.is_empty() {
                self.press_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_press_releases;
            let mut open_url: Option<String> = None;
            egui::Window::new("PRESS — Press Releases")
                .open(&mut open)
                .resizable(true)
                .default_size([820.0, 520.0])
                .max_size([820.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.press_symbol).desired_width(90.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.press_symbol = chart_sym_research.clone();
                        }
                        if ui
                            .add_enabled(
                                !self.press_loading,
                                egui::Button::new("Load Cached").fill(BTN_GREEN),
                            )
                            .clicked()
                        {
                            let sym = self.press_symbol.trim().to_uppercase();
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    self.press_releases_list =
                                        typhoon_engine::core::research::get_press_releases(
                                            &conn, &sym,
                                        )
                                        .ok()
                                        .flatten()
                                        .unwrap_or_default();
                                }
                            }
                        }
                        if ui
                            .add_enabled(
                                !self.press_loading && !self.finnhub_key.is_empty(),
                                egui::Button::new("Fetch").fill(BTN_BLUE),
                            )
                            .clicked()
                        {
                            let sym = self.press_symbol.trim().to_uppercase();
                            if !sym.is_empty() {
                                self.press_loading = true;
                                let _ = self.broker_tx.send(BrokerCmd::FetchPressReleases {
                                    symbol: sym,
                                    finnhub_key: self.finnhub_key.clone(),
                                });
                            }
                        }
                        if self.press_loading {
                            ui.spinner();
                        }
                    });
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for pr in self.press_releases_list.iter() {
                                ui.group(|ui| {
                                    ui.label(
                                        egui::RichText::new(&pr.datetime).color(AXIS_TEXT).small(),
                                    );
                                    if ui
                                        .link(egui::RichText::new(&pr.headline).strong())
                                        .clicked()
                                    {
                                        open_url = Some(pr.url.clone());
                                    }
                                    if !pr.description.is_empty() {
                                        ui.label(
                                            egui::RichText::new(&pr.description).color(AXIS_TEXT),
                                        );
                                    }
                                });
                            }
                        });
                });
            self.show_press_releases = open;
            if let Some(u) = open_url {
                ctx.open_url(egui::OpenUrl::new_tab(u));
            }
        }

        // Social Sentiment
        if self.show_sentiment {
            if self.sentiment_symbol.is_empty() {
                self.sentiment_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_sentiment;
            egui::Window::new("SENTIMENT — Social Sentiment")
                .open(&mut open)
                .resizable(true)
                .default_size([720.0, 460.0])
                .max_size([720.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.sentiment_symbol)
                                .desired_width(90.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.sentiment_symbol = chart_sym_research.clone();
                        }
                        if ui
                            .add_enabled(
                                !self.sentiment_loading,
                                egui::Button::new("Load Cached").fill(BTN_GREEN),
                            )
                            .clicked()
                        {
                            let sym = self.sentiment_symbol.trim().to_uppercase();
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    self.sentiment_rows =
                                        typhoon_engine::core::research::get_sentiment(&conn, &sym)
                                            .ok()
                                            .flatten()
                                            .unwrap_or_default();
                                }
                            }
                        }
                        if ui
                            .add_enabled(
                                !self.sentiment_loading && !self.finnhub_key.is_empty(),
                                egui::Button::new("Fetch").fill(BTN_BLUE),
                            )
                            .clicked()
                        {
                            let sym = self.sentiment_symbol.trim().to_uppercase();
                            if !sym.is_empty() {
                                self.sentiment_loading = true;
                                let _ = self.broker_tx.send(BrokerCmd::FetchSocialSentiment {
                                    symbol: sym,
                                    finnhub_key: self.finnhub_key.clone(),
                                });
                            }
                        }
                        if self.sentiment_loading {
                            ui.spinner();
                        }
                    });
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let mut rows = self.sentiment_rows.clone();
                            rows.sort_by(|a, b| {
                                let ord = match self.sentiment_sort_col {
                                    0 => a.source.cmp(&b.source),
                                    1 => a.at_time.cmp(&b.at_time),
                                    2 => a.mention.cmp(&b.mention),
                                    3 => a.positive_mention.cmp(&b.positive_mention),
                                    4 => a.negative_mention.cmp(&b.negative_mention),
                                    5 => a.score.total_cmp(&b.score),
                                    _ => a.at_time.cmp(&b.at_time),
                                };
                                if self.sentiment_sort_asc {
                                    ord
                                } else {
                                    ord.reverse()
                                }
                            });
                            egui::Grid::new("sentiment_grid")
                                .striped(true)
                                .show(ui, |ui| {
                                    sortable_header(
                                        ui,
                                        "Source",
                                        0,
                                        &mut self.sentiment_sort_col,
                                        &mut self.sentiment_sort_asc,
                                    );
                                    sortable_header(
                                        ui,
                                        "Time",
                                        1,
                                        &mut self.sentiment_sort_col,
                                        &mut self.sentiment_sort_asc,
                                    );
                                    sortable_header(
                                        ui,
                                        "Mentions",
                                        2,
                                        &mut self.sentiment_sort_col,
                                        &mut self.sentiment_sort_asc,
                                    );
                                    sortable_header(
                                        ui,
                                        "Pos",
                                        3,
                                        &mut self.sentiment_sort_col,
                                        &mut self.sentiment_sort_asc,
                                    );
                                    sortable_header(
                                        ui,
                                        "Neg",
                                        4,
                                        &mut self.sentiment_sort_col,
                                        &mut self.sentiment_sort_asc,
                                    );
                                    sortable_header(
                                        ui,
                                        "Score",
                                        5,
                                        &mut self.sentiment_sort_col,
                                        &mut self.sentiment_sort_asc,
                                    );
                                    ui.end_row();
                                    for r in rows.iter() {
                                        ui.label(&r.source);
                                        ui.label(&r.at_time);
                                        ui.label(format!("{}", r.mention));
                                        ui.label(
                                            egui::RichText::new(format!("{}", r.positive_mention))
                                                .color(BTN_GREEN_TEXT),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{}", r.negative_mention))
                                                .color(BTN_RED_TEXT),
                                        );
                                        let col = if r.score > 0.0 {
                                            BTN_GREEN_TEXT
                                        } else if r.score < 0.0 {
                                            BTN_RED_TEXT
                                        } else {
                                            AXIS_TEXT
                                        };
                                        ui.label(
                                            egui::RichText::new(format!("{:+.2}", r.score))
                                                .color(col),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });
                });
            self.show_sentiment = open;
        }

        // Transcripts — two-pane list → body
        if self.show_transcripts {
            if self.transcripts_symbol.is_empty() {
                self.transcripts_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_transcripts;
            egui::Window::new("TRANSCRIPTS — Earnings Calls")
                .open(&mut open)
                .resizable(true)
                .default_size([960.0, 620.0])
                .max_size([960.0, 640.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.transcripts_symbol).desired_width(90.0));
                        if ui.button("Use Chart").clicked() { self.transcripts_symbol = chart_sym_research.clone(); }
                        if ui.add_enabled(!self.transcripts_loading_list, egui::Button::new("Load Cached").fill(BTN_GREEN)).clicked() {
                            let sym = self.transcripts_symbol.trim().to_uppercase();
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    self.transcripts_list = typhoon_engine::core::research::get_transcript_list(&conn, &sym).ok().flatten().unwrap_or_default();
                                }
                            }
                            self.transcripts_selected = None;
                            self.transcripts_body = None;
                            self.transcripts_summary = None;
                            self.transcripts_summary_for = (String::new(), 0, 0);
                        }
                        if ui.add_enabled(!self.transcripts_loading_list && !self.fmp_key.is_empty(), egui::Button::new("Fetch List").fill(BTN_BLUE)).clicked() {
                            let sym = self.transcripts_symbol.trim().to_uppercase();
                            if !sym.is_empty() {
                                self.transcripts_loading_list = true;
                                self.transcripts_selected = None;
                                self.transcripts_body = None;
                                self.transcripts_summary = None;
                                self.transcripts_summary_for = (String::new(), 0, 0);
                                let _ = self.broker_tx.send(BrokerCmd::FetchTranscriptList { symbol: sym, fmp_key: self.fmp_key.clone() });
                            }
                        }
                        if self.transcripts_loading_list || self.transcripts_loading_body { ui.spinner(); }
                    });
                    ui.separator();
                    let avail_h = ui.available_height();
                    ui.horizontal(|ui| {
                        // Left: transcript list
                        ui.allocate_ui_with_layout(
                            egui::vec2(260.0, avail_h),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                egui::ScrollArea::vertical().id_salt("tx_list").auto_shrink([false, false]).show(ui, |ui| {
                                    for (idx, m) in self.transcripts_list.iter().enumerate() {
                                        let selected = self.transcripts_selected == Some(idx);
                                        let label = format!("Q{} {}  —  {}", m.quarter, m.year, m.date);
                                        let resp = ui.selectable_label(selected, egui::RichText::new(label).color(if selected { BTN_BLUE_TEXT } else { AXIS_TEXT }));
                                        if resp.clicked() {
                                            self.transcripts_selected = Some(idx);
                                            // Load cached body first, fetch if missing.
                                            let sym = m.symbol.clone();
                                            let q = m.quarter;
                                            let y = m.year;
                                            let mut cached: Option<typhoon_engine::core::research::Transcript> = None;
                                            if let Some(ref cache) = self.cache {
                                                if let Ok(conn) = cache.connection() {
                                                    cached = typhoon_engine::core::research::get_transcript(&conn, &sym, q, y).ok().flatten();
                                                }
                                            }
                                            if let Some(t) = cached {
                                                self.transcripts_body = Some(t);
                                                self.transcripts_summary = None;
                                                self.transcripts_summary_for = (String::new(), 0, 0);
                                            } else if !self.fmp_key.is_empty() {
                                                self.transcripts_loading_body = true;
                                                self.transcripts_summary = None;
                                                self.transcripts_summary_for = (String::new(), 0, 0);
                                                let _ = self.broker_tx.send(BrokerCmd::FetchTranscriptBody { symbol: sym, quarter: q, year: y, fmp_key: self.fmp_key.clone() });
                                            }
                                        }
                                    }
                                });
                            },
                        );
                        ui.separator();
                        // Right: body
                        ui.allocate_ui_with_layout(
                            egui::vec2(ui.available_width(), avail_h),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                egui::ScrollArea::vertical().id_salt("tx_body").auto_shrink([false, false]).show(ui, |ui| {
                                    if let Some(t) = self.transcripts_body.clone() {
                                        ui.heading(format!("{}  —  Q{} {}", t.symbol, t.quarter, t.year));
                                        ui.label(egui::RichText::new(&t.date).color(AXIS_TEXT));
                                        ui.separator();
                                        let summary_key = (t.symbol.clone(), t.quarter, t.year);
                                        if self.transcripts_summary_for != summary_key {
                                            self.transcripts_summary = Some(
                                                typhoon_engine::core::research::summarize_transcript(&t),
                                            );
                                            self.transcripts_summary_for = summary_key;
                                        }
                                        if let Some(summary) = self.transcripts_summary.clone() {
                                            ui.label(egui::RichText::new(&summary.headline).color(BTN_BLUE_TEXT).strong());
                                            if !summary.bullets.is_empty() {
                                                egui::CollapsingHeader::new(egui::RichText::new("Summary bullets").small().strong())
                                                    .id_salt("transcripts_summary_bullets")
                                                    .default_open(true)
                                                    .show(ui, |ui| {
                                                        for b in &summary.bullets {
                                                            ui.label(egui::RichText::new(format!("\u{2022} {}", b)).small().color(egui::Color32::from_rgb(210, 210, 220)));
                                                        }
                                                    });
                                            }
                                            if !summary.sections.is_empty() {
                                                egui::CollapsingHeader::new(egui::RichText::new("Extracted sections").small().strong())
                                                    .id_salt("transcripts_summary_sections")
                                                    .default_open(false)
                                                    .show(ui, |ui| {
                                                        for section in &summary.sections {
                                                            ui.label(egui::RichText::new(&section.title).color(BTN_BLUE_TEXT).strong().small());
                                                            ui.label(egui::RichText::new(&section.body).small().color(egui::Color32::from_rgb(200, 200, 210)));
                                                            ui.add_space(4.0);
                                                        }
                                                    });
                                            }
                                            ui.separator();
                                        }
                                        ui.label(egui::RichText::new(&t.content).size(13.0));
                                    } else {
                                        ui.label(egui::RichText::new("Select a call from the list.").color(AXIS_TEXT));
                                    }
                                });
                            },
                        );
                    });
                });
            self.show_transcripts = open;
        }

        // Commodities (GLCO) — global commodities futures dashboard
        if self.show_commodities {
            let mut open = self.show_commodities;
            egui::Window::new("GLCO — Global Commodities")
                .open(&mut open)
                .resizable(true)
                .default_size([680.0, 560.0])
                .max_size([680.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.add_enabled(!self.commodities_loading, egui::Button::new("Refresh").fill(BTN_BLUE)).clicked() {
                            self.commodities_loading = true;
                            let _ = self.broker_tx.send(BrokerCmd::FetchCommoditiesQuotes);
                        }
                        if self.commodities_loading { ui.spinner(); }
                        if let Some(t) = self.commodities_last_fetch {
                            let secs = t.elapsed().as_secs();
                            ui.label(egui::RichText::new(format!("Updated {}s ago", secs)).color(AXIS_TEXT));
                        }
                    });
                    ui.separator();
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                        let groups = [("Metals", "Metals"), ("Energy", "Energy"), ("Grains", "Grains"), ("Softs", "Softs"), ("Livestock", "Livestock")];
                        for (label, _) in groups.iter() {
                            ui.collapsing(egui::RichText::new(*label).strong(), |ui| {
                                egui::Grid::new(format!("glco_{}", label)).striped(true).show(ui, |ui| {
                                    ui.label(egui::RichText::new("Symbol").strong());
                                    ui.label(egui::RichText::new("Name").strong());
                                    ui.label(egui::RichText::new("Price").strong());
                                    ui.label(egui::RichText::new("Change").strong());
                                    ui.label(egui::RichText::new("%").strong());
                                    ui.end_row();
                                    for (idx, q) in self.commodities_quotes.iter().enumerate() {
                                        let group = typhoon_engine::core::research::COMMODITIES_UNIVERSE
                                            .get(idx).map(|(_, _, g)| *g).unwrap_or("");
                                        if group != *label { continue; }
                                        ui.label(egui::RichText::new(&q.symbol).color(BTN_BLUE_TEXT));
                                        ui.label(&q.display);
                                        ui.label(format!("{:.2}", q.price));
                                        let col = if q.change > 0.0 { BTN_GREEN_TEXT }
                                            else if q.change < 0.0 { BTN_RED_TEXT } else { AXIS_TEXT };
                                        ui.label(egui::RichText::new(format!("{:+.2}", q.change)).color(col));
                                        ui.label(egui::RichText::new(format!("{:+.2}%", q.change_pct)).color(col));
                                        ui.end_row();
                                    }
                                });
                            });
                        }
                    });
                });
            self.show_commodities = open;
        }

        // TAS — Time & Sales tape
        if self.show_tas {
            if self.tas_symbol.is_empty() {
                self.tas_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_tas;
            egui::Window::new("TAS — Time & Sales")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 620.0])
                .max_size([520.0, 640.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.tas_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() {
                            self.tas_symbol = chart_sym_research.clone();
                            self.tas_rows.clear();
                        }
                        let pause_label = if self.tas_paused { "Resume" } else { "Pause" };
                        let pause_fill = if self.tas_paused { BTN_GREEN } else { BTN_MG };
                        if ui.add(egui::Button::new(pause_label).fill(pause_fill)).clicked() {
                            self.tas_paused = !self.tas_paused;
                        }
                        if ui.add(egui::Button::new("Clear").fill(BTN_RED)).clicked() { self.tas_rows.clear(); }
                        ui.label(egui::RichText::new(format!("{} prints", self.tas_rows.len())).color(AXIS_TEXT));
                    });
                    ui.separator();
                    ui.label(egui::RichText::new("Live trades appear here as the WebSocket stream delivers them. Load the same symbol in the chart to start streaming.").color(AXIS_TEXT).small());
                    ui.separator();
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                        egui::Grid::new("tas_grid").striped(true).num_columns(4).spacing([16.0, 2.0]).show(ui, |ui| {
                            ui.label(egui::RichText::new("Time").strong());
                            ui.label(egui::RichText::new("Price").strong());
                            ui.label(egui::RichText::new("Size").strong());
                            ui.label(egui::RichText::new("Side").strong());
                            ui.end_row();
                            for (_sym, price, size, side, ts) in self.tas_rows.iter().take(500) {
                                let ts_short = ts.split('T').nth(1).map(|t| t.split('.').next().unwrap_or(t)).unwrap_or(ts.as_str());
                                ui.label(egui::RichText::new(ts_short).color(AXIS_TEXT).monospace());
                                let col = match side.as_str() {
                                    "buy" => BTN_GREEN_TEXT,
                                    "sell" => BTN_RED_TEXT,
                                    _ => AXIS_TEXT,
                                };
                                ui.label(egui::RichText::new(format!("{:.4}", price)).color(col).monospace());
                                ui.label(egui::RichText::new(format!("{:.0}", size)).monospace());
                                ui.label(egui::RichText::new(side.to_uppercase()).color(col).monospace().small());
                                ui.end_row();
                            }
                        });
                    });
                });
            self.show_tas = open;
        }
    }
}
