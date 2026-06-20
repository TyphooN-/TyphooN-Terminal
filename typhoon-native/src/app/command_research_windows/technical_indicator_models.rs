use super::*;

mod adaptive_momentum_commands;
mod dmi_adx_commands;
mod linearreg_commands;
mod linearreg_slope_commands;
mod midprice_commands;
mod momentum_flow_commands;
mod research_stats_commands;
mod volatility_force_commands;
mod wma_rainbow_mesa_frama_commands;

impl TyphooNApp {
    pub(super) fn handle_technical_indicator_model_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // (delegated early groups)
            _ if self.handle_adaptive_momentum_commands(cmd_upper) => {}
            _ if self.handle_momentum_flow_commands(cmd_upper) => {}
            _ if self.handle_wma_rainbow_mesa_frama_commands(cmd_upper) => {}
            _ if self.handle_volatility_force_commands(cmd_upper) => {}
            _ if self.handle_linearreg_slope_commands(cmd_upper) => {}
            _ if self.handle_linearreg_commands(cmd_upper) => {}
            _ if self.handle_midprice_commands(cmd_upper) => {}
            _ if self.handle_dmi_adx_commands(cmd_upper) => {}
            _ if self.handle_research_stats_commands(cmd_upper) => {}
            _ => return false,
        }
        true
    }
}
