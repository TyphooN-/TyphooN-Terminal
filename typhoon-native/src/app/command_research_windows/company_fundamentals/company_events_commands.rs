use super::*;

impl TyphooNApp {
    pub(super) fn handle_company_events_commands(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            "DES" | "DESCRIPTION" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.desc_symbol = sym.clone();
                }
                self.show_company_desc = true;
                if !self.finnhub_key.is_empty() && !self.desc_symbol.is_empty() {
                    self.desc_loading = true;
                    let s = self.desc_symbol.to_uppercase();
                    let k = self.finnhub_key.clone();
                    let _ = self.broker_tx.send(BrokerCmd::FetchCompanyProfile {
                        symbol: s.clone(),
                        finnhub_key: k.clone(),
                    });
                    let _ = self.broker_tx.send(BrokerCmd::FetchStockPeers {
                        symbol: s.clone(),
                        finnhub_key: k.clone(),
                    });
                    let _ = self.broker_tx.send(BrokerCmd::FetchEarningsHistory {
                        symbol: s.clone(),
                        finnhub_key: k.clone(),
                    });
                    let _ = self.broker_tx.send(BrokerCmd::FetchPressReleases {
                        symbol: s,
                        finnhub_key: k,
                    });
                }
            }
            _ => return false,
        }
        true
    }
}
