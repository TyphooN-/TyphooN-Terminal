use super::*;

impl TyphooNApp {
    pub(super) fn render_research_insider_dividend_earnings_momentum_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research = research_chart_symbol(
            self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
        );

        // MNGR — Insider Activity Bias
        if self.show_mngr {
            if self.mngr_symbol.is_empty() {
                self.mngr_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_mngr;
            egui::Window::new("MNGR — Insider Activity Bias")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.mngr_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.mngr_symbol = chart_sym_research.clone();
                        }
                        ui.label(egui::RichText::new("Window (days):").color(AXIS_TEXT));
                        ui.add(egui::DragValue::new(&mut self.mngr_window_days).range(30..=365));
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.mngr_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_insider_activity(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.mngr_snapshot = snap;
                                        self.mngr_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.mngr_symbol.to_uppercase();
                            self.mngr_loading = true;
                            self.mngr_symbol = sym.clone();
                            let _ =
                                self.broker_tx
                                    .send(BrokerCmd::ComputeInsiderActivitySnapshot {
                                        symbol: sym,
                                        window_days: self.mngr_window_days,
                                    });
                        }
                        if self.mngr_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.mngr_snapshot;
                    if snap.symbol.is_empty() || snap.total_trades == 0 {
                        ui.label(
                            egui::RichText::new(
                                "No data — run INS for this symbol, then click Compute.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.bias_label.as_str() {
                            "BULLISH" => UP,
                            "BEARISH" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — conviction: {} — window: {}d — as of {}",
                                snap.symbol,
                                snap.bias_label,
                                snap.conviction_label,
                                snap.window_days,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("mngr_grid")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(200.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Total trades").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.total_trades))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Buys / Sells / Other").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {} / {}",
                                        snap.buy_count, snap.sell_count, snap.other_count
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Unique insiders").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.unique_insiders))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Gross buy value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "${:.0}",
                                        snap.gross_buy_value_usd
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Gross sell value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "${:.0}",
                                        snap.gross_sell_value_usd
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Net value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("${:+.0}", snap.net_value_usd))
                                        .small()
                                        .monospace()
                                        .color(color),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Net shares").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.0}", snap.net_shares))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Buy/Sell ratio").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.buy_sell_ratio))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Latest trade").small().strong());
                                ui.label(
                                    egui::RichText::new(&snap.latest_trade_date)
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_mngr = open;
        }

        // DIVG — Dividend Growth Analysis
        if self.show_divg {
            if self.divg_symbol.is_empty() {
                self.divg_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_divg;
            egui::Window::new("DIVG — Dividend Growth Analysis")
                .open(&mut open)
                .resizable(true)
                .default_size([600.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.divg_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.divg_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.divg_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_divg(&conn, &sym_u)
                                    {
                                        self.divg_snapshot = snap;
                                        self.divg_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.divg_symbol.to_uppercase();
                            self.divg_loading = true;
                            self.divg_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeDivgSnapshot { symbol: sym });
                        }
                        if self.divg_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.divg_snapshot;
                    if snap.symbol.is_empty() || snap.total_payments == 0 {
                        ui.label(
                            egui::RichText::new(
                                "No data — run DVD for this symbol, then click Compute.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.trend_label.as_str() {
                            "GROWING" => UP,
                            "CUTTING" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — {} years covered — as of {}",
                                snap.symbol, snap.trend_label, snap.years_covered, snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("divg_grid")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(200.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Latest payment").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "${:.4} on {}",
                                        snap.latest_amount, snap.latest_payment_date
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Annualized dividend").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "${:.2}",
                                        snap.annualized_dividend
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("1Y growth").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}%", snap.cagr_1y_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("3Y CAGR").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}%", snap.cagr_3y_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("5Y CAGR").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}%", snap.cagr_5y_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Consecutive growth years")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}",
                                        snap.consecutive_growth_years
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Consistency").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.0}%",
                                        snap.consistency_score_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Total payments").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.total_payments))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                        ui.separator();
                        ui.label(
                            egui::RichText::new("Annual buckets")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        egui::Grid::new("divg_years_grid")
                            .striped(true)
                            .num_columns(4)
                            .spacing([18.0, 3.0])
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Year").strong().small());
                                ui.label(egui::RichText::new("Total").strong().small());
                                ui.label(egui::RichText::new("Payments").strong().small());
                                ui.label(egui::RichText::new("YoY%").strong().small());
                                ui.end_row();
                                for row in &snap.annual_rows {
                                    ui.label(
                                        egui::RichText::new(format!("{}", row.year))
                                            .monospace()
                                            .small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("${:.2}", row.total_amount))
                                            .monospace()
                                            .small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("{}", row.payment_count))
                                            .monospace()
                                            .small(),
                                    );
                                    let col = if row.growth_pct > 0.0 {
                                        UP
                                    } else if row.growth_pct < 0.0 {
                                        DOWN
                                    } else {
                                        AXIS_TEXT
                                    };
                                    ui.label(
                                        egui::RichText::new(format!("{:+.1}%", row.growth_pct))
                                            .monospace()
                                            .small()
                                            .color(col),
                                    );
                                    ui.end_row();
                                }
                            });
                    }
                });
            self.show_divg = open;
        }

        // EARM — Earnings Momentum Trend
        if self.show_earm {
            if self.earm_symbol.is_empty() {
                self.earm_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_earm;
            egui::Window::new("EARM — Earnings Momentum Trend")
                .open(&mut open)
                .resizable(true)
                .default_size([620.0, 460.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.earm_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.earm_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.earm_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_earm(&conn, &sym_u)
                                    {
                                        self.earm_snapshot = snap;
                                        self.earm_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.earm_symbol.to_uppercase();
                            self.earm_loading = true;
                            self.earm_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeEarmSnapshot { symbol: sym });
                        }
                        if self.earm_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.earm_snapshot;
                    if snap.symbol.is_empty() || snap.quarters_used < 5 {
                        ui.label(
                            egui::RichText::new(
                                "No data — run FA + EPS for this symbol, then click Compute.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.momentum_label.as_str() {
                            "ACCELERATING" => UP,
                            "DECELERATING" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — composite {:.0}/100 — {}Q used — as of {}",
                                snap.symbol,
                                snap.momentum_label,
                                snap.composite_score,
                                snap.quarters_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("earm_grid")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("Recent revenue growth (4Q avg)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.2}%",
                                        snap.recent_revenue_growth_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Prior revenue growth (4Q avg)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.2}%",
                                        snap.prior_revenue_growth_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Revenue acceleration").small().strong(),
                                );
                                let c_rev = if snap.revenue_acceleration_pct > 0.0 {
                                    UP
                                } else if snap.revenue_acceleration_pct < 0.0 {
                                    DOWN
                                } else {
                                    AXIS_TEXT
                                };
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.2}%",
                                        snap.revenue_acceleration_pct
                                    ))
                                    .small()
                                    .monospace()
                                    .color(c_rev),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Recent EPS surprise (4Q avg)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.2}%",
                                        snap.recent_eps_surprise_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Prior EPS surprise (4Q avg)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.2}%",
                                        snap.prior_eps_surprise_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("EPS surprise acceleration")
                                        .small()
                                        .strong(),
                                );
                                let c_eps = if snap.eps_surprise_acceleration_pct > 0.0 {
                                    UP
                                } else if snap.eps_surprise_acceleration_pct < 0.0 {
                                    DOWN
                                } else {
                                    AXIS_TEXT
                                };
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.2}%",
                                        snap.eps_surprise_acceleration_pct
                                    ))
                                    .small()
                                    .monospace()
                                    .color(c_eps),
                                );
                                ui.end_row();
                            });
                        ui.separator();
                        ui.label(
                            egui::RichText::new("Quarterly breakdown (newest first)")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        egui::Grid::new("earm_q_grid")
                            .striped(true)
                            .num_columns(6)
                            .spacing([14.0, 3.0])
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Period").strong().small());
                                ui.label(egui::RichText::new("Revenue").strong().small());
                                ui.label(egui::RichText::new("YoY%").strong().small());
                                ui.label(egui::RichText::new("EPS").strong().small());
                                ui.label(egui::RichText::new("Est").strong().small());
                                ui.label(egui::RichText::new("Surp%").strong().small());
                                ui.end_row();
                                for q in &snap.quarters {
                                    ui.label(egui::RichText::new(&q.period).monospace().small());
                                    ui.label(
                                        egui::RichText::new(format!("{:.0}M", q.revenue / 1e6))
                                            .monospace()
                                            .small(),
                                    );
                                    let c = if q.revenue_yoy_pct > 0.0 {
                                        UP
                                    } else if q.revenue_yoy_pct < 0.0 {
                                        DOWN
                                    } else {
                                        AXIS_TEXT
                                    };
                                    ui.label(
                                        egui::RichText::new(format!("{:+.1}%", q.revenue_yoy_pct))
                                            .monospace()
                                            .small()
                                            .color(c),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("{:.2}", q.eps_actual))
                                            .monospace()
                                            .small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("{:.2}", q.eps_estimate))
                                            .monospace()
                                            .small(),
                                    );
                                    let cs = if q.eps_surprise_pct > 0.0 {
                                        UP
                                    } else if q.eps_surprise_pct < 0.0 {
                                        DOWN
                                    } else {
                                        AXIS_TEXT
                                    };
                                    ui.label(
                                        egui::RichText::new(format!("{:+.1}%", q.eps_surprise_pct))
                                            .monospace()
                                            .small()
                                            .color(cs),
                                    );
                                    ui.end_row();
                                }
                            });
                    }
                });
            self.show_earm = open;
        }

        // SECTR — Sector Rotation Strength
        if self.show_sectr {
            if self.sectr_symbol.is_empty() {
                self.sectr_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_sectr;
            egui::Window::new("SECTR — Sector Rotation Strength")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.sectr_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.sectr_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.sectr_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_sector_rotation(&conn, &sym_u) {
                                        self.sectr_snapshot = snap;
                                        self.sectr_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.sectr_symbol.to_uppercase();
                            self.sectr_loading = true;
                            self.sectr_symbol = sym.clone();
                            let symbol_sector = if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    if let Ok(Some(fa)) = typhoon_engine::core::fundamentals::get_fundamentals(&conn, &sym) {
                                        fa.sector
                                    } else { String::new() }
                                } else { String::new() }
                            } else { String::new() };
                            let _ = self.broker_tx.send(BrokerCmd::ComputeSectorRotationSnapshot {
                                symbol: sym, symbol_sector,
                            });
                        }
                        if self.sectr_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.sectr_snapshot;
                    if snap.symbol.is_empty() || snap.sectors_total == 0 {
                        ui.label(egui::RichText::new("No data — run INDU (sector performance) first, then click Compute.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.strength_label.as_str() {
                            "LEADER" => UP,
                            "LAGGARD" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — sector: {} — rank {}/{} — as of {}",
                            snap.symbol, snap.strength_label, snap.symbol_sector,
                            snap.sector_rank, snap.sectors_total, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("sectr_grid").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Symbol sector change").small().strong());
                            let c = if snap.symbol_sector_change_pct > 0.0 { UP } else if snap.symbol_sector_change_pct < 0.0 { DOWN } else { AXIS_TEXT };
                            ui.label(egui::RichText::new(format!("{:+.2}%", snap.symbol_sector_change_pct)).small().monospace().color(c));
                            ui.end_row();
                            ui.label(egui::RichText::new("Average sector change").small().strong());
                            ui.label(egui::RichText::new(format!("{:+.2}%", snap.avg_sector_change_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Median sector change").small().strong());
                            ui.label(egui::RichText::new(format!("{:+.2}%", snap.median_sector_change_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Relative strength vs avg").small().strong());
                            let cr = if snap.relative_strength_pct > 0.0 { UP } else if snap.relative_strength_pct < 0.0 { DOWN } else { AXIS_TEXT };
                            ui.label(egui::RichText::new(format!("{:+.2}%", snap.relative_strength_pct)).small().monospace().color(cr));
                            ui.end_row();
                            ui.label(egui::RichText::new("Market breadth (positive %)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.0}%", snap.breadth_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Strongest sector").small().strong());
                            ui.label(egui::RichText::new(format!("{} ({:+.2}%)", snap.strongest_sector, snap.strongest_sector_pct)).small().monospace().color(UP));
                            ui.end_row();
                            ui.label(egui::RichText::new("Weakest sector").small().strong());
                            ui.label(egui::RichText::new(format!("{} ({:+.2}%)", snap.weakest_sector, snap.weakest_sector_pct)).small().monospace().color(DOWN));
                            ui.end_row();
                        });
                    }
                });
            self.show_sectr = open;
        }

        // UPDM — Upgrade/Downgrade Momentum
        if self.show_updm {
            if self.updm_symbol.is_empty() {
                self.updm_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_updm;
            egui::Window::new("UPDM — Upgrade/Downgrade Momentum")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.updm_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.updm_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.updm_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_updm(&conn, &sym_u)
                                    {
                                        self.updm_snapshot = snap;
                                        self.updm_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.updm_symbol.to_uppercase();
                            self.updm_loading = true;
                            self.updm_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeUpdmSnapshot { symbol: sym });
                        }
                        if self.updm_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.updm_snapshot;
                    if snap.symbol.is_empty() || snap.total_actions == 0 {
                        ui.label(
                            egui::RichText::new(
                                "No data — run UPDG for this symbol, then click Compute.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.bias_label.as_str() {
                            "BULLISH" => UP,
                            "BEARISH" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — trend: {} — total actions: {} — as of {}",
                                snap.symbol,
                                snap.bias_label,
                                snap.trend_label,
                                snap.total_actions,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("updm_grid")
                            .striped(true)
                            .num_columns(4)
                            .spacing([14.0, 3.0])
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Window").strong().small());
                                ui.label(
                                    egui::RichText::new("Upgrades").strong().small().color(UP),
                                );
                                ui.label(
                                    egui::RichText::new("Downgrades")
                                        .strong()
                                        .small()
                                        .color(DOWN),
                                );
                                ui.label(egui::RichText::new("Net").strong().small());
                                ui.end_row();
                                ui.label(egui::RichText::new("30d").monospace().small());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.upgrades_30d))
                                        .monospace()
                                        .small(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.downgrades_30d))
                                        .monospace()
                                        .small(),
                                );
                                let c30 = if snap.net_30d > 0 {
                                    UP
                                } else if snap.net_30d < 0 {
                                    DOWN
                                } else {
                                    AXIS_TEXT
                                };
                                ui.label(
                                    egui::RichText::new(format!("{:+}", snap.net_30d))
                                        .monospace()
                                        .small()
                                        .color(c30),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("90d").monospace().small());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.upgrades_90d))
                                        .monospace()
                                        .small(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.downgrades_90d))
                                        .monospace()
                                        .small(),
                                );
                                let c90 = if snap.net_90d > 0 {
                                    UP
                                } else if snap.net_90d < 0 {
                                    DOWN
                                } else {
                                    AXIS_TEXT
                                };
                                ui.label(
                                    egui::RichText::new(format!("{:+}", snap.net_90d))
                                        .monospace()
                                        .small()
                                        .color(c90),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("180d").monospace().small());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.upgrades_180d))
                                        .monospace()
                                        .small(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.downgrades_180d))
                                        .monospace()
                                        .small(),
                                );
                                let c180 = if snap.net_180d > 0 {
                                    UP
                                } else if snap.net_180d < 0 {
                                    DOWN
                                } else {
                                    AXIS_TEXT
                                };
                                ui.label(
                                    egui::RichText::new(format!("{:+}", snap.net_180d))
                                        .monospace()
                                        .small()
                                        .color(c180),
                                );
                                ui.end_row();
                            });
                        ui.separator();
                        egui::Grid::new("updm_sub")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(200.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Initiations (90d)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.initiations_90d))
                                        .monospace()
                                        .small(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Maintains (90d)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.maintains_90d))
                                        .monospace()
                                        .small(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Latest action").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} — {} — {}",
                                        snap.latest_date, snap.latest_firm, snap.latest_action
                                    ))
                                    .monospace()
                                    .small(),
                                );
                                ui.end_row();
                                if !snap.latest_to_grade.is_empty() {
                                    ui.label(
                                        egui::RichText::new("Latest to-grade").small().strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new(&snap.latest_to_grade)
                                            .monospace()
                                            .small(),
                                    );
                                    ui.end_row();
                                }
                            });
                    }
                });
            self.show_updm = open;
        }
    }
}
