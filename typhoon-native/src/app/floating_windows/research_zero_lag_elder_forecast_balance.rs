use super::*;

mod adaptive_elder_average_windows;
mod balance_calendar_windows;
mod forecast_smoothing_windows;
mod trend_volume_oscillator_windows;

impl TyphooNApp {
    pub(super) fn render_research_zero_lag_elder_forecast_balance_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research: String = self
            .charts
            .get(self.active_tab)
            .map(|c| {
                c.symbol
                    .split(':')
                    .rev()
                    .nth(1)
                    .or_else(|| c.symbol.split(':').last())
                    .unwrap_or("AAPL")
                    .to_string()
            })
            .unwrap_or_else(|| "AAPL".to_string());

        self.render_adaptive_elder_average_windows(ctx, &chart_sym_research);

        self.render_forecast_smoothing_windows(ctx, &chart_sym_research);

        self.render_trend_volume_oscillator_windows(ctx, &chart_sym_research);

        self.render_balance_calendar_windows(ctx, &chart_sym_research);
    }
}
