use super::*;

mod company_events_commands;
mod dividend_estimates_ratings_commands;
mod earnings_peers_commands;
mod financials_management_cot_commands;
mod fundamental_rankings_commands;
mod fundamental_ratios_commands;
mod insider_fundamental_commands;
mod market_overview_commands;
mod sentiment_transcripts_tape_commands;
mod splits_etf_index_commands;

impl TyphooNApp {
    pub(super) fn handle_company_fundamentals_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // Company events, sentiment, transcripts, commodities, and tape research
            _ if self.handle_company_events_commands(cmd_upper) => {}
            _ if self.handle_sentiment_transcripts_tape_commands(cmd_upper) => {}
            _ if self.handle_dividend_estimates_ratings_commands(cmd_upper) => {}
            _ if self.handle_financials_management_cot_commands(cmd_upper) => {}
            _ if self.handle_splits_etf_index_commands(cmd_upper) => {}
            _ if self.handle_insider_fundamental_commands(cmd_upper) => {}
            _ if self.handle_market_overview_commands(cmd_upper) => {}
            _ if self.handle_fundamental_ratios_commands(cmd_upper) => {}
            _ if self.handle_fundamental_rankings_commands(cmd_upper) => {}
            _ => return false,
        }
        true
    }
}
