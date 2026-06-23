use super::*;

impl TyphooNApp {
    pub(super) fn handle_market_overview_commands(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── palette entries ──
            // (intentionally omits "INDICES" to preserve the legacy ETF dashboard below)
            "WEI" | "GLOBAL_INDICES" => {
                self.show_wei = true;
                // Load cached snapshot first for instant display.
                if self.wei_indices.is_empty() {
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
                // Then kick a fresh fetch (Yahoo, no API key).
                self.wei_loading = true;
                let _ = self.broker_tx.send(BrokerCmd::FetchWorldIndices);
            }
            // (intentionally omits "MOVERS" to preserve the legacy broker top-movers arm below)
            "MOV" | "GAINERS" | "LOSERS" | "ACTIVES" => {
                self.show_market_movers = true;
                if self.market_movers.gainers.is_empty()
                    && self.market_movers.losers.is_empty()
                    && self.market_movers.actives.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(mov)) =
                                typhoon_engine::core::research::get_market_movers(&conn)
                            {
                                self.market_movers = mov;
                            }
                        }
                    }
                }
                if !self.fmp_key.is_empty() {
                    self.mov_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchMarketMovers {
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "INDU" | "SECTOR" | "SECTORS" | "SECTOR_PERFORMANCE" => {
                self.show_sector_perf = true;
                if self.sector_perf.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(rows)) =
                                typhoon_engine::core::research::get_sector_performance(&conn)
                            {
                                self.sector_perf = rows;
                            }
                        }
                    }
                }
                if !self.fmp_key.is_empty() {
                    self.indu_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchSectorPerformance {
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "CACS" | "CORP_ACTIONS" | "CORPORATE_ACTIONS" | "ACTIONS" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cacs_symbol = sym;
                }
                self.show_cacs = true;
                // No fetcher — the window aggregates cached splits/dividends/earnings/IPOs.
            }
            "WACC" | "COST_OF_CAPITAL" | "CAPM" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.wacc_symbol = sym;
                }
                self.show_wacc = true;
                if self.wacc_snapshot.symbol.is_empty() && !self.wacc_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_wacc(&conn, &self.wacc_symbol)
                            {
                                self.wacc_snapshot = snap;
                            }
                        }
                    }
                }
                if !self.fmp_key.is_empty() && !self.wacc_symbol.is_empty() {
                    self.wacc_loading = true;
                    // Pass the latest 10Y yield we know about (in-memory);
                    // fall back to 4.5 % if the GY window hasn't been fetched.
                    let rf = self
                        .treasury_yields
                        .iter()
                        .find(|y| y.tenor == "10Y")
                        .map(|y| y.yield_pct)
                        .unwrap_or(4.5);
                    let _ = self.broker_tx.send(BrokerCmd::FetchWaccSnapshot {
                        symbol: self.wacc_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                        risk_free_pct: rf,
                    });
                }
            }
            _ => return false,
        }
        true
    }
}
