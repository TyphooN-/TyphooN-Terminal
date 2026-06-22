use super::*;

mod fundamental_acceleration_windows;
mod rank_highlow_windows;
mod tail_risk_distribution_windows;
mod volatility_correlation_windows;
mod volatility_drawdown_momentum;

impl TyphooNApp {
    pub(super) fn render_research_behavior_distribution_stats_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research = research_chart_symbol(
            self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
        );

        self.render_volatility_drawdown_momentum_windows(ctx, &chart_sym_research);

        self.render_rank_highlow_windows(ctx, &chart_sym_research);

        self.render_volatility_correlation_windows(ctx, &chart_sym_research);

        self.render_fundamental_acceleration_windows(ctx, &chart_sym_research);

        self.render_tail_risk_distribution_windows(ctx, &chart_sym_research);
    }
}
