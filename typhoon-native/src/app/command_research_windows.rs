use super::*;

mod candlestick_quant_models;
mod company_fundamentals;
mod market_structure_models;
mod technical_indicator_models;
mod valuation_risk_models;

impl TyphooNApp {
    pub(super) fn handle_research_window_command(&mut self, cmd_upper: &String) -> bool {
        self.handle_company_fundamentals_command(cmd_upper)
            || self.handle_valuation_risk_model_command(cmd_upper)
            || self.handle_market_structure_model_command(cmd_upper)
            || self.handle_technical_indicator_model_command(cmd_upper)
            || self.handle_candlestick_quant_model_command(cmd_upper)
    }
}
