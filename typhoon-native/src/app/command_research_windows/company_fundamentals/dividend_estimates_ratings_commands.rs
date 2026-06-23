use super::*;

impl TyphooNApp {
    pub(super) fn handle_dividend_estimates_ratings_commands(
        &mut self,
        cmd_upper: &String,
    ) -> bool {
        match cmd_upper.as_str() {
            // Dividend, earnings-estimate, rating, and treasury research
            "DVD" | "DIV_HISTORY" | "DIVIDEND_HISTORY" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.dividend_history_symbol = sym.clone();
                }
                self.show_dividend_history = true;
                if !self.fmp_key.is_empty() && !self.dividend_history_symbol.is_empty() {
                    self.dividend_history_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchDividendHistory {
                        symbol: self.dividend_history_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "EEB" | "ESTIMATES" | "FORWARD_EARNINGS" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.earnings_estimates_symbol = sym.clone();
                }
                self.show_earnings_estimates = true;
                if !self.fmp_key.is_empty() && !self.earnings_estimates_symbol.is_empty() {
                    self.earnings_estimates_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchEarningsEstimates {
                        symbol: self.earnings_estimates_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "UPDG" | "UPGRADES" | "DOWNGRADES" | "RATING_CHANGES" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.rating_changes_symbol = sym.clone();
                }
                self.show_rating_changes = true;
                if !self.fmp_key.is_empty() && !self.rating_changes_symbol.is_empty() {
                    self.rating_changes_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchRatingChanges {
                        symbol: self.rating_changes_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "GY" | "TREASURY" | "YIELD_CURVE" | "YIELDS" => {
                self.show_treasury_curve = true;
                self.treasury_yields_loading = true;
                let _ = self.broker_tx.send(BrokerCmd::FetchTreasuryYields);
            }
            _ => return false,
        }
        true
    }
}
