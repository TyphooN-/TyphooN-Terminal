use super::*;

mod directional_moneyflow_sar_commands;
mod distribution_entropy_commands;
mod fractal_tail_dependence_commands;
mod jump_stationarity_tail_commands;
mod momentum_oscillator_commands;
mod momentum_volatility_adaptive_commands;
mod moving_average_transform_commands;
mod price_transform_extrema_commands;
mod residual_cycle_memory_commands;
mod squeeze_channel_adaptive_commands;
mod trend_channel_transform_commands;
mod trend_cycle_average_commands;
mod volatility_bubble_nonlinearity_commands;
mod volume_choppiness_moving_average_commands;
mod volume_momentum_trend_cycle_commands;

impl TyphooNApp {
    pub(super) fn handle_market_structure_model_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            _ if self.handle_distribution_entropy_command(cmd_upper) => {}
            _ if self.handle_fractal_tail_dependence_command(cmd_upper) => {}
            _ if self.handle_jump_stationarity_tail_command(cmd_upper) => {}
            _ if self.handle_volatility_bubble_nonlinearity_command(cmd_upper) => {}
            _ if self.handle_residual_cycle_memory_command(cmd_upper) => {}
            _ if self.handle_squeeze_channel_adaptive_command(cmd_upper) => {}
            _ if self.handle_trend_channel_transform_command(cmd_upper) => {}
            _ if self.handle_directional_moneyflow_sar_command(cmd_upper) => {}
            _ if self.handle_volume_choppiness_moving_average_command(cmd_upper) => {}
            _ if self.handle_momentum_oscillator_command(cmd_upper) => {}
            _ if self.handle_price_transform_extrema_command(cmd_upper) => {}
            _ if self.handle_volume_momentum_trend_cycle_command(cmd_upper) => {}
            _ if self.handle_trend_cycle_average_command(cmd_upper) => {}
            _ if self.handle_moving_average_transform_command(cmd_upper) => {}
            _ if self.handle_momentum_volatility_adaptive_command(cmd_upper) => {}
            _ => return false,
        }
        true
    }
}
