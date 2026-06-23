use super::*;

impl TyphooNApp {
    pub(super) fn render_research_solvency_scores_volatility_targets_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // Solvency, quality, volatility-estimator, EPS-beat, and price-target research

        // ALTZ — Altman Z-Score
        if let Some(sym) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "ALTZ — Altman Z-Score",
                default_size: [620.0, 420.0],
                max_size: Some([620.0, 560.0]),
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_altz,
            &mut self.altz_symbol,
            &mut self.altz_loading,
            &mut self.altz_snapshot,
            |conn, s| {
                typhoon_engine::core::research::get_altman_z(conn, s)
                    .ok()
                    .flatten()
            },
            |symbol| symbol,
            super::render::render_altz_snapshot,
        ) {
            let market_value_equity = if let Some(ref cache) = self.cache {
                if let Ok(conn) = cache.connection() {
                    if let Ok(Some(fa)) =
                        typhoon_engine::core::fundamentals::get_fundamentals(&conn, &sym)
                    {
                        fa.market_cap.unwrap_or(0.0)
                    } else {
                        0.0
                    }
                } else {
                    0.0
                }
            } else {
                0.0
            };
            let _ = self.broker_tx.send(BrokerCmd::ComputeAltmanZSnapshot {
                symbol: sym,
                market_value_equity,
            });
        }

        // PTFS — Piotroski F-Score
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "PTFS — Piotroski F-Score",
                default_size: [520.0, 480.0],
                max_size: Some([640.0, 560.0]),
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_ptfs,
            &mut self.ptfs_symbol,
            &mut self.ptfs_loading,
            &mut self.ptfs_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_piotroski(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputePiotroskiSnapshot { symbol },
            super::render::render_ptfs_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // VOLE — OHLC Volatility Estimators
        if let Some(sym) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "VOLE — OHLC Volatility Estimators",
                default_size: [580.0, 360.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_vole,
            &mut self.vole_symbol,
            &mut self.vole_loading,
            &mut self.vole_snapshot,
            |conn, s| {
                typhoon_engine::core::research::get_ohlc_vol(conn, s)
                    .ok()
                    .flatten()
            },
            |symbol| symbol,
            super::render::render_vole_snapshot,
        ) {
            let bars_json = if let Some(ref cache) = self.cache {
                if let Ok(conn) = cache.connection() {
                    let mut bars: Vec<typhoon_engine::core::research::HistoricalPriceRow> =
                        typhoon_engine::core::research::get_historical_price(&conn, &sym)
                            .ok()
                            .flatten()
                            .unwrap_or_default();
                    if bars.len() >= 2 && bars[0].date > bars[bars.len() - 1].date {
                        bars.reverse();
                    }
                    serde_json::to_string(&bars).unwrap_or_default()
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            let _ = self.broker_tx.send(BrokerCmd::ComputeOhlcVolSnapshot {
                symbol: sym,
                window_days: 60,
                bars_json,
            });
        }

        // EPSB — EPS Beat Streak & Surprise
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "EPSB — EPS Beat Streak",
                default_size: [560.0, 380.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_epsb,
            &mut self.epsb_symbol,
            &mut self.epsb_loading,
            &mut self.epsb_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_eps_beat(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeEpsBeatSnapshot { symbol },
            super::render::render_epsb_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // PTD — Price Target Dispersion & Implied Return
        if let Some(sym) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "PTD — Price Target Dispersion",
                default_size: [560.0, 380.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_ptd,
            &mut self.ptd_symbol,
            &mut self.ptd_loading,
            &mut self.ptd_snapshot,
            |conn, s| {
                typhoon_engine::core::research::get_price_target_dispersion(conn, s)
                    .ok()
                    .flatten()
            },
            |symbol| symbol,
            super::render::render_ptd_snapshot,
        ) {
            let current_price = if let Some(ref cache) = self.cache {
                if let Ok(conn) = cache.connection() {
                    if let Ok(Some(fa)) =
                        typhoon_engine::core::fundamentals::get_fundamentals(&conn, &sym)
                    {
                        fa.stock_price.unwrap_or(0.0)
                    } else {
                        0.0
                    }
                } else {
                    0.0
                }
            } else {
                0.0
            };
            let _ = self
                .broker_tx
                .send(BrokerCmd::ComputePriceTargetDispersionSnapshot {
                    symbol: sym,
                    current_price,
                });
        }
    }
}
