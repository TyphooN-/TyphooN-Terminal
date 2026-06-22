use super::*;

mod bands_intraday_guppy;
mod cycle_volume_stat_tail;
mod oscillator_forecast_flow;
mod smma_alligator_crsi;
mod volume_trend_kdj;

impl TyphooNApp {
    pub(super) fn render_research_advanced_moving_averages_windows(&mut self, ctx: &egui::Context) {
        let chart_sym_research = research_chart_symbol(
            self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
        );

        // ── Research SMMA / ALLIGATOR / CRSI / SEB / IMI ──
        self.render_smma_alligator_crsi_windows(ctx, &chart_sym_research);

        self.render_bands_intraday_guppy_windows(ctx, &chart_sym_research);

        self.render_volume_trend_kdj_windows(ctx, &chart_sym_research);

        self.render_oscillator_forecast_flow_windows(ctx, &chart_sym_research);

        self.render_cycle_volume_stat_tail_windows(ctx, &chart_sym_research);
    }
}
