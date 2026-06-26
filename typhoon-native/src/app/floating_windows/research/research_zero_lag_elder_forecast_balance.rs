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
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        self.render_adaptive_elder_average_windows(ctx, &chart_sym_research);

        self.render_forecast_smoothing_windows(ctx, &chart_sym_research);

        self.render_trend_volume_oscillator_windows(ctx, &chart_sym_research);

        self.render_balance_calendar_windows(ctx, &chart_sym_research);
    }
}
