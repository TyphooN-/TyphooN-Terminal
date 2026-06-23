use super::*;

/// Active-chart research symbol for command handlers — empty when no chart is open —
/// derived from the chart's `source:symbol:timeframe` key. ADR-125: dedups the 52
/// identical inline derivations across this command tree. Pure over the symbol string.
pub(crate) fn command_chart_symbol(chart_symbol: Option<&str>) -> String {
    chart_symbol
        .map(|sym| {
            sym.split(':')
                .rev()
                .nth(1)
                .or_else(|| sym.split(':').last())
                .unwrap_or("")
                .to_string()
        })
        .unwrap_or_default()
}

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
