use super::*;

impl TyphooNApp {
    pub(super) fn handle_earnings_peers_commands(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            "IPO" => {
                self.show_ipo_calendar = true;
                if !self.finnhub_key.is_empty() {
                    self.ipo_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchIpoCalendar {
                        finnhub_key: self.finnhub_key.clone(),
                        days_ahead: 30,
                        days_back: 30,
                    });
                }
            }
            "ERN" | "EARNINGS_HISTORY" => {
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
                    self.earnings_history_symbol = sym;
                }
                self.show_earnings_history = true;
                if !self.finnhub_key.is_empty() && !self.earnings_history_symbol.is_empty() {
                    self.earnings_history_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchEarningsHistory {
                        symbol: self.earnings_history_symbol.to_uppercase(),
                        finnhub_key: self.finnhub_key.clone(),
                    });
                }
            }
            "PEERS" => {
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
                    self.peers_symbol = sym;
                }
                self.show_peers = true;
                if !self.finnhub_key.is_empty() && !self.peers_symbol.is_empty() {
                    self.peers_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchStockPeers {
                        symbol: self.peers_symbol.to_uppercase(),
                        finnhub_key: self.finnhub_key.clone(),
                    });
                }
            }
            "PRESS" => {
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
                    self.press_symbol = sym;
                }
                self.show_press_releases = true;
                if !self.finnhub_key.is_empty() && !self.press_symbol.is_empty() {
                    self.press_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchPressReleases {
                        symbol: self.press_symbol.to_uppercase(),
                        finnhub_key: self.finnhub_key.clone(),
                    });
                }
            }
            _ => return false,
        }
        true
    }
}
