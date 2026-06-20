use super::*;

mod candlestick_pattern_commands;
mod macd_oscillator_extrema_commands;
mod statistical_test_commands;

impl TyphooNApp {
    pub(super) fn handle_candlestick_quant_model_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            _ if self.handle_macd_oscillator_extrema_command(cmd_upper) => {}
            _ if self.handle_candlestick_pattern_command(cmd_upper) => {}
            _ if self.handle_statistical_test_command(cmd_upper) => {}
            _ => return false,
        }
        true
    }
}
