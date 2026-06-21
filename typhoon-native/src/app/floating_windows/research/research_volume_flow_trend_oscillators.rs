use super::*;

mod momentum_body_power_windows;
mod trend_cycle_average_windows;
mod volume_index_flow_windows;

impl TyphooNApp {
    pub(super) fn render_research_volume_flow_trend_oscillators_windows(
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

        self.render_volume_index_flow_windows(ctx, &chart_sym_research);

        self.render_momentum_body_power_windows(ctx, &chart_sym_research);

        self.render_trend_cycle_average_windows(ctx, &chart_sym_research);
    }
}
