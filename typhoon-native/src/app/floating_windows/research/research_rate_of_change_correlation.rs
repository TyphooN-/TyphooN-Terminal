use super::*;

mod bands_volume_linear_windows;
mod correlation_extrema_windows;
mod rate_of_change_windows;

impl TyphooNApp {
    pub(super) fn render_research_rate_of_change_correlation_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        self.render_rate_of_change_windows(ctx, &chart_sym_research);

        self.render_correlation_extrema_windows(ctx, &chart_sym_research);

        self.render_bands_volume_linear_windows(ctx, &chart_sym_research);
    }
}
