use super::*;

impl TyphooNApp {
    pub(super) fn handle_fundamental_ratios_commands(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── palette entries ──
            "WCR" | "CURRENCY" | "CURRENCIES" | "FX_RATES" => {
                self.show_wcr = true;
                if self.wcr_rates.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(rows)) =
                                typhoon_engine::core::research::get_currency_rates(&conn)
                            {
                                self.wcr_rates = rows;
                            }
                        }
                    }
                }
                self.wcr_loading = true;
                let _ = self.broker_tx.send(BrokerCmd::FetchCurrencyRates);
            }
            "BETA" | "ROLLING_BETA" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.beta_symbol = sym;
                }
                self.show_beta = true;
                if self.beta_snapshot.symbol.is_empty() && !self.beta_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_beta(&conn, &self.beta_symbol)
                            {
                                self.beta_snapshot = snap;
                            }
                        }
                    }
                }
                if !self.fmp_key.is_empty() && !self.beta_symbol.is_empty() {
                    self.beta_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchBetaSnapshot {
                        symbol: self.beta_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "DDM" | "GORDON_GROWTH" | "DIVIDEND_DISCOUNT" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.ddm_symbol = sym;
                }
                self.show_ddm = true;
                if self.ddm_snapshot.symbol.is_empty() && !self.ddm_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_ddm(&conn, &self.ddm_symbol)
                            {
                                self.ddm_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RV" | "RELATIVE_VALUATION" | "PEER_VALUATION" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.rv_symbol = sym;
                }
                self.show_rv = true;
                if self.rv_snapshot.symbol.is_empty() && !self.rv_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(rv)) =
                                typhoon_engine::core::research::get_relative_valuation(
                                    &conn,
                                    &self.rv_symbol,
                                )
                            {
                                self.rv_snapshot = rv;
                            }
                        }
                    }
                }
            }
            "FIGI" | "OPENFIGI" | "IDENTIFIERS" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.figi_symbol = sym;
                }
                self.show_figi = true;
                if self.figi_snapshot.identifiers.is_empty() && !self.figi_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_figi(&conn, &self.figi_symbol)
                            {
                                self.figi_snapshot = snap;
                            }
                        }
                    }
                }
                if !self.figi_symbol.is_empty() {
                    self.figi_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchFigiIdentifiers {
                        symbol: self.figi_symbol.to_uppercase(),
                    });
                }
            }
            _ => return false,
        }
        true
    }
}
