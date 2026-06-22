use super::*;

impl TyphooNApp {
    pub(super) fn render_research_leverage_quality_volatility_shorts_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // Leverage, accruals, realized-volatility, cash-flow, and short-interest research
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
                        ui.add(
                            egui::TextEdit::singleline(&mut self.lev_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.lev_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.lev_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_leverage(&conn, &sym_u)
                                    {
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
                                    if let Ok(Some(fa)) =
                                        typhoon_engine::core::fundamentals::get_fundamentals(
                                            &conn, &sym,
                                        )
                                    {
                                        (
                                            fa.total_debt.unwrap_or(0.0),
                                            fa.cash_and_equivalents.unwrap_or(0.0),
                                        )
                                    } else {
                                        (0.0, 0.0)
                                    }
                                } else {
                                    (0.0, 0.0)
                                }
                            } else {
                                (0.0, 0.0)
                            };
                            let _ = self.broker_tx.send(BrokerCmd::ComputeLeverageSnapshot {
                                symbol: sym,
                                total_debt_fund,
                                cash_fund,
                            });
                        }
                        if self.lev_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_lev_snapshot(ui, &self.lev_snapshot);
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
                        ui.add(
                            egui::TextEdit::singleline(&mut self.acrl_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.acrl_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.acrl_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_accruals(&conn, &sym_u)
                                    {
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
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeAccrualsSnapshot { symbol: sym });
                        }
                        if self.acrl_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_acrl_snapshot(ui, &self.acrl_snapshot);
                });
            self.show_acrl = open;
        }

        // RVOL — Realized Volatility Cone
        if let Some(sym) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "RVOL — Realized Volatility Cone",
                default_size: [620.0, 400.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_rvol,
            &mut self.rvol_symbol,
            &mut self.rvol_loading,
            &mut self.rvol_snapshot,
            |conn, s| {
                typhoon_engine::core::research::get_realized_vol(conn, s)
                    .ok()
                    .flatten()
            },
            |symbol| symbol,
            super::render::render_rvol_snapshot,
        ) {
            let (bars_json, current_atm_iv_pct) = if let Some(ref cache) = self.cache {
                if let Ok(conn) = cache.connection() {
                    let mut bars: Vec<typhoon_engine::core::research::HistoricalPriceRow> =
                        typhoon_engine::core::research::get_historical_price(&conn, &sym)
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
                        ui.add(
                            egui::TextEdit::singleline(&mut self.fcfy_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.fcfy_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.fcfy_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_fcf_yield(&conn, &sym_u)
                                    {
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
                                    if let Ok(Some(fa)) =
                                        typhoon_engine::core::fundamentals::get_fundamentals(
                                            &conn, &sym,
                                        )
                                    {
                                        (
                                            fa.market_cap.unwrap_or(0.0),
                                            fa.stock_price.unwrap_or(0.0),
                                        )
                                    } else {
                                        (0.0, 0.0)
                                    }
                                } else {
                                    (0.0, 0.0)
                                }
                            } else {
                                (0.0, 0.0)
                            };
                            let _ = self.broker_tx.send(BrokerCmd::ComputeFcfYieldSnapshot {
                                symbol: sym,
                                market_cap,
                                stock_price,
                            });
                        }
                        if self.fcfy_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_fcfy_snapshot(ui, &self.fcfy_snapshot);
                });
            self.show_fcfy = open;
        }

        // SHRT — Short Interest & Days-to-Cover
        if let Some(sym) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "SHRT — Short Interest & DTC",
                default_size: [560.0, 340.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_shrt,
            &mut self.shrt_symbol,
            &mut self.shrt_loading,
            &mut self.shrt_snapshot,
            |conn, s| {
                typhoon_engine::core::research::get_short_interest(conn, s)
                    .ok()
                    .flatten()
            },
            |symbol| symbol,
            super::render::render_shrt_snapshot,
        ) {
            let (shares_out, float_shares, short_pct_of_float, short_ratio_reported, bars_json) =
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let fa = typhoon_engine::core::fundamentals::get_fundamentals(&conn, &sym)
                            .ok()
                            .flatten();
                        let shares_out = fa
                            .as_ref()
                            .and_then(|f| f.shares_outstanding)
                            .unwrap_or(0.0);
                        let short_pct = fa
                            .as_ref()
                            .and_then(|f| f.short_percent_of_float)
                            .unwrap_or(0.0);
                        let short_ratio = fa.as_ref().and_then(|f| f.short_ratio).unwrap_or(0.0);
                        let float_shares =
                            typhoon_engine::core::research::get_shares_float(&conn, &sym)
                                .ok()
                                .flatten()
                                .map(|s| s.float_shares)
                                .unwrap_or(0.0);
                        let mut bars: Vec<typhoon_engine::core::research::HistoricalPriceRow> =
                            typhoon_engine::core::research::get_historical_price(&conn, &sym)
                                .ok()
                                .flatten()
                                .unwrap_or_default();
                        if bars.len() >= 2 && bars[0].date > bars[bars.len() - 1].date {
                            bars.reverse();
                        }
                        (
                            shares_out,
                            float_shares,
                            short_pct,
                            short_ratio,
                            serde_json::to_string(&bars).unwrap_or_default(),
                        )
                    } else {
                        (0.0, 0.0, 0.0, 0.0, String::new())
                    }
                } else {
                    (0.0, 0.0, 0.0, 0.0, String::new())
                };
            let _ = self
                .broker_tx
                .send(BrokerCmd::ComputeShortInterestSnapshot {
                    symbol: sym,
                    shares_out,
                    float_shares,
                    short_pct_of_float,
                    short_ratio_reported,
                    bars_json,
                });
        }
    }
}
