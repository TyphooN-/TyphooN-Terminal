use super::*;

mod downside_efficiency_volatility_commands;
mod drawdown_seasonality_spread_commands;
mod entropy_recovery_stationarity_commands;
mod entropy_tail_reward_memory_commands;
mod event_dividend_risk_rank_commands;
mod factor_growth_quality_commands;
mod leverage_quality_liquidity_rank_commands;
mod price_path_gap_volatility_commands;
mod range_volatility_calendar_tail_commands;
mod return_distribution_tail_commands;
mod reward_risk_serial_liquidity_commands;
mod risk_adjusted_liquidity_normality_commands;
mod stationarity_jump_drawdown_commands;
mod tail_heteroskedasticity_stability_commands;
mod upside_leverage_concentration_commands;

impl TyphooNApp {
    pub(super) fn handle_valuation_risk_model_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            _ if self.handle_factor_growth_quality_command(cmd_upper) => {}
            _ if self.handle_leverage_quality_liquidity_rank_command(cmd_upper) => {}
            _ if self.handle_event_dividend_risk_rank_command(cmd_upper) => {}
            _ if self.handle_return_distribution_tail_command(cmd_upper) => {}
            _ if self.handle_price_path_gap_volatility_command(cmd_upper) => {}
            _ if self.handle_downside_efficiency_volatility_command(cmd_upper) => {}
            _ if self.handle_risk_adjusted_liquidity_normality_command(cmd_upper) => {}
            _ if self.handle_drawdown_seasonality_spread_command(cmd_upper) => {}
            _ if self.handle_range_volatility_calendar_tail_command(cmd_upper) => {}
            _ if self.handle_reward_risk_serial_liquidity_command(cmd_upper) => {}
            _ if self.handle_stationarity_jump_drawdown_command(cmd_upper) => {}
            _ if self.handle_tail_heteroskedasticity_stability_command(cmd_upper) => {}
            _ if self.handle_entropy_tail_reward_memory_command(cmd_upper) => {}
            _ if self.handle_upside_leverage_concentration_command(cmd_upper) => {}
            _ if self.handle_entropy_recovery_stationarity_command(cmd_upper) => {}
            _ => return false,
        }
        true
    }
}
