use super::*;

impl TyphooNApp {
    pub(super) fn handle_splits_etf_index_commands(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── palette entries ──
            "SPLT" | "SPLIT" | "SPLITS" | "STOCK_SPLIT" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.splits_symbol = sym.clone();
                }
                self.show_splits = true;
                if !self.splits_symbol.is_empty() {
                    self.splits_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchStockSplits {
                        symbol: self.splits_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "ETF" | "HOLDINGS" | "ETF_HOLDINGS" | "FUND" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.etf_symbol = sym.clone();
                }
                self.show_etf_holdings = true;
                if !self.fmp_key.is_empty() && !self.etf_symbol.is_empty() {
                    self.etf_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchEtfHoldings {
                        symbol: self.etf_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "ANR" | "ANALYST_RECS" | "RECOMMENDATIONS" | "PRICE_TARGET" | "PT" | "TARGET" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.anr_symbol = sym.clone();
                }
                self.show_analyst_recs = true;
                if !self.finnhub_key.is_empty() && !self.anr_symbol.is_empty() {
                    self.anr_loading = true;
                    let sym_u = self.anr_symbol.to_uppercase();
                    let _ = self.broker_tx.send(BrokerCmd::FetchAnalystRecs {
                        symbol: sym_u.clone(),
                        finnhub_key: self.finnhub_key.clone(),
                    });
                    let _ = self.broker_tx.send(BrokerCmd::FetchPriceTarget {
                        symbol: sym_u,
                        finnhub_key: self.finnhub_key.clone(),
                    });
                }
            }
            "ESG" | "SUSTAINABILITY" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.esg_symbol = sym.clone();
                }
                self.show_esg = true;
                if !self.fmp_key.is_empty() && !self.esg_symbol.is_empty() {
                    self.esg_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchEsgScores {
                        symbol: self.esg_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "MEMB" | "MEMBERS" | "CONSTITUENTS" | "SP500" | "NDX" | "DJIA" => {
                let requested = match cmd_upper.as_str() {
                    "NDX" => "NDX",
                    "DJIA" => "DJIA",
                    "SP500" => "SP500",
                    _ => {
                        if self.index_code.is_empty() {
                            "SP500"
                        } else {
                            self.index_code.as_str()
                        }
                    }
                };
                self.index_code = requested.to_string();
                self.show_index_members = true;
                if !self.fmp_key.is_empty() {
                    self.memb_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchIndexMembers {
                        index_code: self.index_code.clone(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            _ => return false,
        }
        true
    }
}
