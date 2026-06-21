use super::*;

impl TyphooNApp {
    pub(super) fn handle_insider_fundamental_commands(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── palette entries ──
            "INS" | "INSIDER_TRADES" | "FORM4" => {
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
                    self.insider_symbol = sym.clone();
                }
                self.show_insider_trades = true;
                if !self.fmp_key.is_empty() && !self.insider_symbol.is_empty() {
                    self.insider_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchInsiderTrades {
                        symbol: self.insider_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "HDS" | "INST" | "INSTITUTIONAL" | "13F" => {
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
                    self.inst_holders_symbol = sym.clone();
                }
                self.show_inst_holders = true;
                if !self.fmp_key.is_empty() && !self.inst_holders_symbol.is_empty() {
                    self.inst_holders_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchInstitutionalHolders {
                        symbol: self.inst_holders_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "FLOAT" | "SHARES" | "OUTSTANDING" => {
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
                    self.float_symbol = sym.clone();
                }
                self.show_shares_float = true;
                if !self.fmp_key.is_empty() && !self.float_symbol.is_empty() {
                    self.float_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchSharesFloat {
                        symbol: self.float_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "HP" | "HIST" | "HISTORICAL" | "PRICE_HISTORY" => {
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
                    self.hp_symbol = sym.clone();
                }
                self.show_hist_price = true;
                if !self.fmp_key.is_empty() && !self.hp_symbol.is_empty() {
                    self.hp_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchHistoricalPrice {
                        symbol: self.hp_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                        limit: self.hp_limit.max(50),
                    });
                }
            }
            "EPS" | "SURPRISE" | "EARNINGS_SURPRISE" => {
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
                    self.eps_symbol = sym.clone();
                }
                self.show_eps_surprise = true;
                if !self.fmp_key.is_empty() && !self.eps_symbol.is_empty() {
                    self.eps_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchEarningsSurprises {
                        symbol: self.eps_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            _ => return false,
        }
        true
    }
}
