use super::*;

impl TyphooNApp {
    pub(super) fn render_research_round10_windows(&mut self, ctx: &egui::Context) {
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

        // ── Research Godel Parity Round 10 ──
        // LEV — Debt Leverage & Coverage
        if self.show_lev {
            if self.lev_symbol.is_empty() {
                self.lev_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_lev;
            egui::Window::new("LEV — Debt Leverage & Coverage")
                .open(&mut open)
                .resizable(true)
                .default_size([620.0, 440.0])
                .max_size([620.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.lev_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.lev_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.lev_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_leverage(&conn, &sym_u) {
                                        self.lev_snapshot = snap;
                                        self.lev_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.lev_symbol.to_uppercase();
                            self.lev_loading = true;
                            self.lev_symbol = sym.clone();
                            let (total_debt_fund, cash_fund) = if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    if let Ok(Some(fa)) = typhoon_engine::core::fundamentals::get_fundamentals(&conn, &sym) {
                                        (fa.total_debt.unwrap_or(0.0), fa.cash_and_equivalents.unwrap_or(0.0))
                                    } else { (0.0, 0.0) }
                                } else { (0.0, 0.0) }
                            } else { (0.0, 0.0) };
                            let _ = self.broker_tx.send(BrokerCmd::ComputeLeverageSnapshot {
                                symbol: sym, total_debt_fund, cash_fund,
                            });
                        }
                        if self.lev_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.lev_snapshot;
                    if snap.symbol.is_empty() || snap.ratios.is_empty() {
                        ui.label(egui::RichText::new("No data — run FA (Financials) for this symbol, then click Compute.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — as of {}", snap.symbol, snap.solvency_summary, snap.as_of))
                            .strong().color(AXIS_TEXT));
                        ui.label(egui::RichText::new(format!(
                            "Total Debt ${:.0}M · Net Debt ${:.0}M · EBITDA TTM ${:.0}M · Interest TTM ${:.0}M · Equity ${:.0}M",
                            snap.total_debt / 1e6, snap.net_debt / 1e6,
                            snap.ebitda_ttm / 1e6, snap.interest_expense_ttm / 1e6, snap.total_equity / 1e6,
                        )).small().color(AXIS_TEXT));
                        ui.separator();
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            egui::Grid::new("lev_grid").striped(true).num_columns(5).min_col_width(80.0).show(ui, |ui| {
                                ui.label(egui::RichText::new("Ratio").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Value").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Peer Median").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Signal").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Note").color(AXIS_TEXT).small().strong());
                                ui.end_row();
                                for r in &snap.ratios {
                                    let color = match r.signal.as_str() {
                                        "HEALTHY" => UP,
                                        "ELEVATED" => AXIS_TEXT,
                                        "STRETCHED" => DOWN,
                                        _ => AXIS_TEXT,
                                    };
                                    ui.label(egui::RichText::new(&r.name).small().monospace().strong());
                                    ui.label(egui::RichText::new(format!("{:.2}", r.value)).small().monospace());
                                    ui.label(egui::RichText::new(format!("{:.2}", r.peer_median)).small().monospace());
                                    ui.label(egui::RichText::new(&r.signal).color(color).small().monospace().strong());
                                    ui.label(egui::RichText::new(&r.note).color(AXIS_TEXT).small().monospace());
                                    ui.end_row();
                                }
                            });
                        });
                    }
                });
            self.show_lev = open;
        }

        // ACRL — Earnings Quality (NI vs FCF)
        if self.show_acrl {
            if self.acrl_symbol.is_empty() {
                self.acrl_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_acrl;
            egui::Window::new("ACRL — Earnings Quality")
                .open(&mut open)
                .resizable(true)
                .default_size([620.0, 420.0])
                .max_size([620.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.acrl_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.acrl_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.acrl_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_accruals(&conn, &sym_u) {
                                        self.acrl_snapshot = snap;
                                        self.acrl_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.acrl_symbol.to_uppercase();
                            self.acrl_loading = true;
                            self.acrl_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeAccrualsSnapshot { symbol: sym });
                        }
                        if self.acrl_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.acrl_snapshot;
                    if snap.symbol.is_empty() || snap.periods.is_empty() {
                        ui.label(egui::RichText::new("No data — run FA (Financials) for this symbol, then click Compute.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — TTM NI ${:.0}M · TTM FCF ${:.0}M · cash conv {:.1}% · avg {:.1}% — as of {}",
                            snap.symbol, snap.trend_label,
                            snap.ttm_net_income / 1e6, snap.ttm_free_cash_flow / 1e6,
                            snap.ttm_cash_conversion_pct, snap.avg_cash_conversion_pct, snap.as_of,
                        )).strong().color(AXIS_TEXT));
                        ui.separator();
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            egui::Grid::new("acrl_grid").striped(true).num_columns(6).min_col_width(72.0).show(ui, |ui| {
                                ui.label(egui::RichText::new("Period").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Date").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("NI").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("FCF").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Cash Conv %").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Quality").color(AXIS_TEXT).small().strong());
                                ui.end_row();
                                for p in &snap.periods {
                                    let color = match p.quality_label.as_str() {
                                        "HIGH" => UP,
                                        "LOW" | "NEGATIVE_NI" => DOWN,
                                        _ => AXIS_TEXT,
                                    };
                                    ui.label(egui::RichText::new(&p.period).small().monospace());
                                    ui.label(egui::RichText::new(&p.date).small().monospace());
                                    ui.label(egui::RichText::new(format!("{:.0}M", p.net_income / 1e6)).small().monospace());
                                    ui.label(egui::RichText::new(format!("{:.0}M", p.free_cash_flow / 1e6)).small().monospace());
                                    ui.label(egui::RichText::new(format!("{:.1}%", p.cash_conversion_pct)).small().monospace());
                                    ui.label(egui::RichText::new(&p.quality_label).color(color).small().monospace().strong());
                                    ui.end_row();
                                }
                            });
                        });
                    }
                });
            self.show_acrl = open;
        }

        // RVOL — Realized Volatility Cone
        if self.show_rvol {
            if self.rvol_symbol.is_empty() {
                self.rvol_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_rvol;
            egui::Window::new("RVOL — Realized Volatility Cone")
                .open(&mut open)
                .resizable(true)
                .default_size([620.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.rvol_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.rvol_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.rvol_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_realized_vol(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.rvol_snapshot = snap;
                                        self.rvol_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.rvol_symbol.to_uppercase();
                            self.rvol_loading = true;
                            self.rvol_symbol = sym.clone();
                            let (bars_json, current_atm_iv_pct) = if let Some(ref cache) =
                                self.cache
                            {
                                if let Ok(conn) = cache.connection() {
                                    let mut bars: Vec<
                                        typhoon_engine::core::research::HistoricalPriceRow,
                                    > = typhoon_engine::core::research::get_historical_price(
                                        &conn, &sym,
                                    )
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default();
                                    if bars.len() >= 2 && bars[0].date > bars[bars.len() - 1].date {
                                        bars.reverse();
                                    }
                                    let iv = typhoon_engine::core::research::get_ivol(&conn, &sym)
                                        .ok()
                                        .flatten()
                                        .map(|s| s.current_atm_iv_pct)
                                        .filter(|v| *v > 0.0);
                                    (serde_json::to_string(&bars).unwrap_or_default(), iv)
                                } else {
                                    (String::new(), None)
                                }
                            } else {
                                (String::new(), None)
                            };
                            let _ = self.broker_tx.send(BrokerCmd::ComputeRealizedVolSnapshot {
                                symbol: sym,
                                current_atm_iv_pct,
                                bars_json,
                            });
                        }
                        if self.rvol_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.rvol_snapshot;
                    if snap.symbol.is_empty() || snap.windows.is_empty() {
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
                                "{} — last ${:.2} — IV {:.1}% vs gap {:+.1}% — {} — as of {}",
                                snap.symbol,
                                snap.last_close,
                                snap.current_atm_iv_pct,
                                snap.iv_rv_gap_pct,
                                snap.regime_label,
                                snap.as_of,
                            ))
                            .strong()
                            .color(AXIS_TEXT),
                        );
                        ui.separator();
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            egui::Grid::new("rvol_grid")
                                .striped(true)
                                .num_columns(4)
                                .min_col_width(100.0)
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new("Window")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Days")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Realized Vol %")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Percentile")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.end_row();
                                    for w in &snap.windows {
                                        ui.label(
                                            egui::RichText::new(&w.label)
                                                .small()
                                                .monospace()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{}", w.trading_days))
                                                .small()
                                                .monospace(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{:.2}",
                                                w.realized_vol_pct
                                            ))
                                            .small()
                                            .monospace(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{:.0}%", w.percentile))
                                                .small()
                                                .monospace(),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });
                    }
                });
            self.show_rvol = open;
        }

        // FCFY — FCF Yield & Dividend Sustainability
        if self.show_fcfy {
            if self.fcfy_symbol.is_empty() {
                self.fcfy_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_fcfy;
            egui::Window::new("FCFY — FCF Yield & Payout")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 420.0])
                .max_size([640.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.fcfy_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.fcfy_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.fcfy_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_fcf_yield(&conn, &sym_u) {
                                        self.fcfy_snapshot = snap;
                                        self.fcfy_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.fcfy_symbol.to_uppercase();
                            self.fcfy_loading = true;
                            self.fcfy_symbol = sym.clone();
                            let (market_cap, stock_price) = if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    if let Ok(Some(fa)) = typhoon_engine::core::fundamentals::get_fundamentals(&conn, &sym) {
                                        (fa.market_cap.unwrap_or(0.0), fa.stock_price.unwrap_or(0.0))
                                    } else { (0.0, 0.0) }
                                } else { (0.0, 0.0) }
                            } else { (0.0, 0.0) };
                            let _ = self.broker_tx.send(BrokerCmd::ComputeFcfYieldSnapshot {
                                symbol: sym, market_cap, stock_price,
                            });
                        }
                        if self.fcfy_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.fcfy_snapshot;
                    if snap.symbol.is_empty() {
                        ui.label(egui::RichText::new("No data — run FA (Financials) and Fundamentals, then click Compute.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.sustainability_label.as_str() {
                            "SAFE" => UP,
                            "STRETCHED" => AXIS_TEXT,
                            "UNSUSTAINABLE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — FCF yield {:.2}% · div yield {:.2}% · payout-from-FCF {:.1}% · 5Y CAGR {:+.1}% — {} — as of {}",
                            snap.symbol, snap.ttm_fcf_yield_pct, snap.ttm_dividend_yield_pct,
                            snap.ttm_payout_from_fcf_pct, snap.fcf_cagr_5y_pct,
                            snap.sustainability_label, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            egui::Grid::new("fcfy_grid").striped(true).num_columns(6).min_col_width(80.0).show(ui, |ui| {
                                ui.label(egui::RichText::new("Period").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Date").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("FCF").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Div Paid").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Payout-FCF %").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("FCF Yield %").color(AXIS_TEXT).small().strong());
                                ui.end_row();
                                for p in &snap.periods {
                                    ui.label(egui::RichText::new(&p.period).small().monospace());
                                    ui.label(egui::RichText::new(&p.date).small().monospace());
                                    ui.label(egui::RichText::new(format!("{:.0}M", p.free_cash_flow / 1e6)).small().monospace());
                                    ui.label(egui::RichText::new(format!("{:.0}M", p.dividends_paid / 1e6)).small().monospace());
                                    ui.label(egui::RichText::new(format!("{:.1}%", p.payout_from_fcf_pct)).small().monospace());
                                    ui.label(egui::RichText::new(format!("{:.2}%", p.fcf_yield_pct)).small().monospace());
                                    ui.end_row();
                                }
                            });
                        });
                    }
                });
            self.show_fcfy = open;
        }

        // SHRT — Short Interest & Days-to-Cover
        if self.show_shrt {
            if self.shrt_symbol.is_empty() {
                self.shrt_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_shrt;
            egui::Window::new("SHRT — Short Interest & DTC")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 340.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.shrt_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.shrt_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.shrt_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_short_interest(&conn, &sym_u) {
                                        self.shrt_snapshot = snap;
                                        self.shrt_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.shrt_symbol.to_uppercase();
                            self.shrt_loading = true;
                            self.shrt_symbol = sym.clone();
                            let (shares_out, float_shares, short_pct_of_float, short_ratio_reported, bars_json) =
                                if let Some(ref cache) = self.cache {
                                    if let Ok(conn) = cache.connection() {
                                        let fa = typhoon_engine::core::fundamentals::get_fundamentals(&conn, &sym).ok().flatten();
                                        let shares_out = fa.as_ref().and_then(|f| f.shares_outstanding).unwrap_or(0.0);
                                        let short_pct = fa.as_ref().and_then(|f| f.short_percent_of_float).unwrap_or(0.0);
                                        let short_ratio = fa.as_ref().and_then(|f| f.short_ratio).unwrap_or(0.0);
                                        let float_shares = typhoon_engine::core::research::get_shares_float(&conn, &sym)
                                            .ok().flatten().map(|s| s.float_shares).unwrap_or(0.0);
                                        let mut bars: Vec<typhoon_engine::core::research::HistoricalPriceRow> =
                                            typhoon_engine::core::research::get_historical_price(&conn, &sym)
                                                .ok().flatten().unwrap_or_default();
                                        if bars.len() >= 2 && bars[0].date > bars[bars.len()-1].date {
                                            bars.reverse();
                                        }
                                        (shares_out, float_shares, short_pct, short_ratio,
                                         serde_json::to_string(&bars).unwrap_or_default())
                                    } else { (0.0, 0.0, 0.0, 0.0, String::new()) }
                                } else { (0.0, 0.0, 0.0, 0.0, String::new()) };
                            let _ = self.broker_tx.send(BrokerCmd::ComputeShortInterestSnapshot {
                                symbol: sym, shares_out, float_shares,
                                short_pct_of_float, short_ratio_reported, bars_json,
                            });
                        }
                        if self.shrt_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.shrt_snapshot;
                    if snap.symbol.is_empty() {
                        ui.label(egui::RichText::new("No data — run FA (Fundamentals/SharesFloat) + HP, then click Compute.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.squeeze_risk_label.as_str() {
                            "LOW" => UP,
                            "ELEVATED" => AXIS_TEXT,
                            "HIGH" | "EXTREME" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — as of {}", snap.symbol, snap.squeeze_risk_label, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("shrt_grid").striped(true).num_columns(2).min_col_width(180.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Short % of float").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}%", snap.short_percent_of_float)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Days to cover").small().strong());
                            ui.label(egui::RichText::new(format!("{:.1}", snap.days_to_cover)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Short shares").small().strong());
                            ui.label(egui::RichText::new(format!("{:.0}M", snap.short_shares / 1e6)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Float").small().strong());
                            ui.label(egui::RichText::new(format!("{:.0}M", snap.shares_float / 1e6)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Shares outstanding").small().strong());
                            ui.label(egui::RichText::new(format!("{:.0}M", snap.shares_outstanding / 1e6)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Avg daily vol (20d)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.0}K", snap.avg_daily_volume_20d / 1e3)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Short ratio (reported)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}", snap.short_ratio_reported)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Utilization proxy").small().strong());
                            ui.label(egui::RichText::new(format!("{:.1}%", snap.utilization_proxy_pct)).small().monospace());
                            ui.end_row();
                        });
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).color(AXIS_TEXT).small());
                        }
                    }
                });
            self.show_shrt = open;
        }
    }
}
