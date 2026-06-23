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
        if let Some(sym) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "LEV — Debt Leverage & Coverage",
                default_size: [620.0, 440.0],
                max_size: Some([620.0, 560.0]),
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_lev,
            &mut self.lev_symbol,
            &mut self.lev_loading,
            &mut self.lev_snapshot,
            |conn, s| {
                typhoon_engine::core::research::get_leverage(conn, s)
                    .ok()
                    .flatten()
            },
            |symbol| symbol,
            super::render::render_lev_snapshot,
        ) {
            let (total_debt_fund, cash_fund) = if let Some(ref cache) = self.cache {
                if let Ok(conn) = cache.connection() {
                    if let Ok(Some(fa)) =
                        typhoon_engine::core::fundamentals::get_fundamentals(&conn, &sym)
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

        // ACRL — Earnings Quality (NI vs FCF)
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "ACRL — Earnings Quality",
                default_size: [620.0, 420.0],
                max_size: Some([620.0, 560.0]),
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_acrl,
            &mut self.acrl_symbol,
            &mut self.acrl_loading,
            &mut self.acrl_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_accruals(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeAccrualsSnapshot { symbol },
            super::render::render_acrl_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // RVOL — Realized Volatility Cone
        if let Some(sym) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "RVOL — Realized Volatility Cone",
                default_size: [620.0, 400.0],
                max_size: None,
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
        if let Some(sym) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "FCFY — FCF Yield & Payout",
                default_size: [640.0, 420.0],
                max_size: Some([640.0, 560.0]),
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_fcfy,
            &mut self.fcfy_symbol,
            &mut self.fcfy_loading,
            &mut self.fcfy_snapshot,
            |conn, s| {
                typhoon_engine::core::research::get_fcf_yield(conn, s)
                    .ok()
                    .flatten()
            },
            |symbol| symbol,
            super::render::render_fcfy_snapshot,
        ) {
            let (market_cap, stock_price) = if let Some(ref cache) = self.cache {
                if let Ok(conn) = cache.connection() {
                    if let Ok(Some(fa)) =
                        typhoon_engine::core::fundamentals::get_fundamentals(&conn, &sym)
                    {
                        (fa.market_cap.unwrap_or(0.0), fa.stock_price.unwrap_or(0.0))
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

        // SHRT — Short Interest & Days-to-Cover
        if let Some(sym) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "SHRT — Short Interest & DTC",
                default_size: [560.0, 340.0],
                max_size: None,
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
