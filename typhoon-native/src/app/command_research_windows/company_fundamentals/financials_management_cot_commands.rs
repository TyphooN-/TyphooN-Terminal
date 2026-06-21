use super::*;

impl TyphooNApp {
    pub(super) fn handle_financials_management_cot_commands(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // Financial statements, management, and COT research
            "FA" | "FINANCIALS" | "INCOME" | "BALANCE" | "CASHFLOW" => {
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
                    self.financials_symbol = sym.clone();
                }
                self.show_financials = true;
                self.financials_view = match cmd_upper.as_str() {
                    "BALANCE" => FinancialsView::Balance,
                    "CASHFLOW" => FinancialsView::CashFlow,
                    _ => FinancialsView::Income,
                };
                if !self.fmp_key.is_empty() && !self.financials_symbol.is_empty() {
                    self.financials_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchFinancialStatements {
                        symbol: self.financials_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "MGMT" | "MANAGEMENT" | "OFFICERS" | "EXECUTIVES" => {
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
                    self.executives_symbol = sym.clone();
                }
                self.show_executives = true;
                if !self.finnhub_key.is_empty() && !self.executives_symbol.is_empty() {
                    self.executives_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchExecutives {
                        symbol: self.executives_symbol.to_uppercase(),
                        finnhub_key: self.finnhub_key.clone(),
                    });
                }
            }
            "COT" | "COMMITMENTS" | "POSITIONING" => {
                self.show_cot = true;
                self.cot_loading = true;
                let _ = self.broker_tx.send(BrokerCmd::FetchCotReports);
            }
            _ => return false,
        }
        true
    }
}
