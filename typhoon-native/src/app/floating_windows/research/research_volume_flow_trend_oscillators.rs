use super::*;

mod momentum_body_power_windows;
mod trend_cycle_average_windows;
mod volume_index_flow_windows;

impl TyphooNApp {
    pub(super) fn render_research_volume_flow_trend_oscillators_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research = research_chart_symbol(
            self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
        );

        self.render_volume_index_flow_windows(ctx, &chart_sym_research);

        self.render_momentum_body_power_windows(ctx, &chart_sym_research);

        self.render_trend_cycle_average_windows(ctx, &chart_sym_research);
    }
}
