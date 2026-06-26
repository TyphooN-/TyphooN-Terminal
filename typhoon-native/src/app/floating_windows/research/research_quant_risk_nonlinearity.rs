use super::*;

mod distribution_normality_windows;
mod quant_break_test_windows;
mod structural_risk_tail_windows;
mod volatility_range_windows;

impl TyphooNApp {
    pub(super) fn render_research_quant_risk_nonlinearity_windows(&mut self, ctx: &egui::Context) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        self.render_quant_break_test_windows(ctx, &chart_sym_research);

        self.render_volatility_range_windows(ctx, &chart_sym_research);

        self.render_distribution_normality_windows(ctx, &chart_sym_research);

        self.render_structural_risk_tail_windows(ctx, &chart_sym_research);
    }
}
