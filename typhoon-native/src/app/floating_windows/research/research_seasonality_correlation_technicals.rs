use super::*;

impl TyphooNApp {
    pub(super) fn render_research_seasonality_correlation_technicals_windows(
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
                    ui.separator();
                    let snap = &self.seag_snapshot;
                    if snap.symbol.is_empty() || snap.months.is_empty() {
                        ui.label(
                            egui::RichText::new(
                                "No data — run HP to populate bar history, then click Compute.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} years covered — best {} · worst {} — as of {}",
                                snap.symbol,
                                snap.years_covered,
                                snap.best_month,
                                snap.worst_month,
                                snap.as_of
                            ))
                            .strong()
                            .color(AXIS_TEXT),
                        );
                        ui.separator();
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("Monthly seasonality")
                                    .strong()
                                    .color(AXIS_TEXT),
                            );
                            egui::Grid::new("seag_months_grid")
                                .striped(true)
                                .num_columns(6)
                                .min_col_width(70.0)
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new("Month")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Avg")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Median")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("σ").color(AXIS_TEXT).small().strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Pos/Tot")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Best/Worst")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.end_row();
                                    for m in &snap.months {
                                        if m.total_years == 0 {
                                            continue;
                                        }
                                        let c = if m.avg_return_pct >= 0.0 { UP } else { DOWN };
                                        ui.label(
                                            egui::RichText::new(&m.label)
                                                .small()
                                                .monospace()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{:+.2}%",
                                                m.avg_return_pct
                                            ))
                                            .color(c)
                                            .small()
                                            .monospace(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{:+.2}%",
                                                m.median_return_pct
                                            ))
                                            .small()
                                            .monospace(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{:.2}", m.stdev_pct))
                                                .small()
                                                .monospace(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{}/{}",
                                                m.positive_years, m.total_years
                                            ))
                                            .small()
                                            .monospace(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{:+.1}% / {:+.1}%",
                                                m.best_return_pct, m.worst_return_pct
                                            ))
                                            .small()
                                            .monospace(),
                                        );
                                        ui.end_row();
                                    }
                                });
                            ui.separator();
                            ui.label(
                                egui::RichText::new("Day-of-week seasonality")
                                    .strong()
                                    .color(AXIS_TEXT),
                            );
                            egui::Grid::new("seag_dow_grid")
                                .striped(true)
                                .num_columns(3)
                                .min_col_width(90.0)
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new("Day")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Avg log-ret")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Pos/Tot")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.end_row();
                                    for d in &snap.dow {
                                        let c = if d.avg_return_pct >= 0.0 { UP } else { DOWN };
                                        ui.label(
                                            egui::RichText::new(&d.label)
                                                .small()
                                                .monospace()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{:+.3}%",
                                                d.avg_return_pct
                                            ))
                                            .color(c)
                                            .small()
                                            .monospace(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{}/{}",
                                                d.positive_days, d.total_days
                                            ))
                                            .small()
                                            .monospace(),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });
                    }
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
                        ui.add(egui::TextEdit::singleline(&mut self.cor_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cor_symbol = chart_sym_research.clone(); }
                        ui.label(egui::RichText::new("Window (days)").color(AXIS_TEXT).small());
                        ui.add(egui::DragValue::new(&mut self.cor_window_days).range(30..=1260));
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.cor_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_correlation(&conn, &sym_u) {
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
                                    let peer_syms = typhoon_engine::core::research::get_peers(&conn, &sym)
                                        .unwrap_or(None).unwrap_or_default();
                                    let mut peers_raw: Vec<(String, Vec<typhoon_engine::core::research::HistoricalPriceRow>)> = Vec::new();
                                    for p in &peer_syms {
                                        if p.eq_ignore_ascii_case(&sym) { continue; }
                                        if let Ok(Some(mut rows)) = typhoon_engine::core::research::get_historical_price(&conn, p) {
                                            if rows.len() >= 2 && rows[0].date > rows[rows.len()-1].date { rows.reverse(); }
                                            peers_raw.push((p.to_uppercase(), rows));
                                        }
                                    }
                                    serde_json::to_string(&peers_raw).unwrap_or_else(|_| "[]".to_string())
                                } else { "[]".to_string() }
                            } else { "[]".to_string() };
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCorrelationMatrix {
                                symbol: sym, window_days, peer_series_json: peer_json,
                            });
                        }
                        if self.cor_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.cor_snapshot;
                    if snap.symbol.is_empty() || snap.cells.is_empty() {
                        ui.label(egui::RichText::new("No data — run PEERS + HP for the symbol and its peers, then click Compute.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        ui.label(egui::RichText::new(format!(
                            "{} — {}-day window — avg |ρ| {:.2} — highest {} · lowest {} — as of {}",
                            snap.symbol, snap.window_days, snap.mean_correlation,
                            snap.highest_corr_symbol, snap.lowest_corr_symbol, snap.as_of))
                            .strong().color(AXIS_TEXT));
                        ui.separator();
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            egui::Grid::new("cor_grid").striped(true).num_columns(4).min_col_width(100.0).show(ui, |ui| {
                                ui.label(egui::RichText::new("Peer").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("ρ").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("β").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("N").color(AXIS_TEXT).small().strong());
                                ui.end_row();
                                for c in &snap.cells {
                                    let color = if c.correlation >= 0.0 { UP } else { DOWN };
                                    ui.label(egui::RichText::new(&c.peer_symbol).small().monospace().strong());
                                    ui.label(egui::RichText::new(format!("{:+.3}", c.correlation)).color(color).small().monospace());
                                    ui.label(egui::RichText::new(format!("{:+.3}", c.beta_vs_peer)).small().monospace());
                                    ui.label(egui::RichText::new(format!("{}", c.n_observations)).small().monospace());
                                    ui.end_row();
                                }
                            });
                        });
                    }
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
                    ui.separator();
                    let snap = &self.tra_snapshot;
                    if snap.symbol.is_empty() || snap.windows.is_empty() {
                        ui.label(
                            egui::RichText::new(
                                "No data — run HP and DVD for this symbol, then click Compute.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — last close ${:.2} — TTM div ${:.2} ({:.2}%) — as of {}",
                                snap.symbol,
                                snap.last_close,
                                snap.trailing_12m_dividends,
                                snap.trailing_12m_yield_pct,
                                snap.as_of
                            ))
                            .strong()
                            .color(AXIS_TEXT),
                        );
                        ui.separator();
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            egui::Grid::new("tra_grid")
                                .striped(true)
                                .num_columns(6)
                                .min_col_width(80.0)
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new("Window")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Price %")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Div %")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Total %")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Annualized")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("N divs")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.end_row();
                                    for w in &snap.windows {
                                        let c = if w.total_return_pct >= 0.0 { UP } else { DOWN };
                                        ui.label(
                                            egui::RichText::new(&w.label)
                                                .small()
                                                .monospace()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{:+.2}%",
                                                w.price_return_pct
                                            ))
                                            .small()
                                            .monospace(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{:+.2}%",
                                                w.dividend_yield_pct
                                            ))
                                            .small()
                                            .monospace(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{:+.2}%",
                                                w.total_return_pct
                                            ))
                                            .color(c)
                                            .small()
                                            .monospace(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{:+.2}%",
                                                w.annualized_pct
                                            ))
                                            .small()
                                            .monospace(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{}", w.n_dividends))
                                                .small()
                                                .monospace(),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });
                    }
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
                    ui.separator();
                    let snap = &self.tech_snapshot;
                    if snap.symbol.is_empty() || snap.indicators.is_empty() {
                        ui.label(
                            egui::RichText::new(
                                "No data — run HP for this symbol, then click Compute.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — last close ${:.2} — {} — as of {}",
                                snap.symbol, snap.last_close, snap.trend_summary, snap.as_of
                            ))
                            .strong()
                            .color(AXIS_TEXT),
                        );
                        ui.separator();
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            egui::Grid::new("tech_grid")
                                .striped(true)
                                .num_columns(5)
                                .min_col_width(80.0)
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new("Indicator")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Value")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Secondary")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Signal")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Note")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.end_row();
                                    for ind in &snap.indicators {
                                        let color = match ind.signal.as_str() {
                                            "bullish" | "oversold" => UP,
                                            "bearish" | "overbought" => DOWN,
                                            _ => AXIS_TEXT,
                                        };
                                        ui.label(
                                            egui::RichText::new(&ind.name)
                                                .small()
                                                .monospace()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{:.3}", ind.value))
                                                .small()
                                                .monospace(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{:.3}",
                                                ind.value_secondary
                                            ))
                                            .small()
                                            .monospace(),
                                        );
                                        ui.label(
                                            egui::RichText::new(&ind.signal)
                                                .color(color)
                                                .small()
                                                .monospace()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new(&ind.note)
                                                .color(AXIS_TEXT)
                                                .small()
                                                .monospace(),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });
                    }
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
                        ui.add(egui::TextEdit::singleline(&mut self.skew_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.skew_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.skew_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_vol_skew(&conn, &sym_u) {
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
                            let _ = self.broker_tx.send(BrokerCmd::ComputeVolSkewSnapshot { symbol: sym });
                        }
                        if self.skew_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.skew_snapshot;
                    if snap.symbol.is_empty() || snap.expiries.is_empty() {
                        ui.label(egui::RichText::new("No data — run OMON for this symbol first to cache the chain, then click Compute.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        ui.label(egui::RichText::new(format!(
                            "{} — underlying ${:.2} — {} expiries — as of {}",
                            snap.symbol, snap.underlying_price, snap.expiries.len(), snap.as_of))
                            .strong().color(AXIS_TEXT));
                        ui.separator();
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for ex in &snap.expiries {
                                ui.label(egui::RichText::new(format!(
                                    "Expiry {} ({} days) — ATM IV {:.1}% — skew 25Δ≈ {:+.2}%",
                                    ex.expiration, ex.days_to_expiry, ex.atm_iv_pct, ex.put_call_skew_25d_pct))
                                    .strong().color(AXIS_TEXT));
                                egui::Grid::new(format!("skew_grid_{}", ex.expiration)).striped(true).num_columns(5).min_col_width(80.0).show(ui, |ui| {
                                    ui.label(egui::RichText::new("Strike").color(AXIS_TEXT).small().strong());
                                    ui.label(egui::RichText::new("Moneyness").color(AXIS_TEXT).small().strong());
                                    ui.label(egui::RichText::new("Call IV").color(AXIS_TEXT).small().strong());
                                    ui.label(egui::RichText::new("Put IV").color(AXIS_TEXT).small().strong());
                                    ui.label(egui::RichText::new("Combined").color(AXIS_TEXT).small().strong());
                                    ui.end_row();
                                    for p in &ex.points {
                                        ui.label(egui::RichText::new(format!("{:.2}", p.strike)).small().monospace().strong());
                                        ui.label(egui::RichText::new(format!("{:+.2}%", p.moneyness_pct)).small().monospace());
                                        ui.label(egui::RichText::new(format!("{:.1}%", p.call_iv_pct)).small().monospace());
                                        ui.label(egui::RichText::new(format!("{:.1}%", p.put_iv_pct)).small().monospace());
                                        ui.label(egui::RichText::new(format!("{:.1}%", p.combined_iv_pct)).small().monospace());
                                        ui.end_row();
                                    }
                                });
                                ui.separator();
                            }
                        });
                    }
                });
            self.show_skew = open;
        }
    }
}
