use super::*;

mod classic_reversal_patterns;
mod early_reversal_patterns;
mod final_continuation_patterns;
mod gap_continuation_patterns;
mod shadow_kicking_patterns;

impl TyphooNApp {
    pub(super) fn render_research_candlestick_reversal_continuation_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research = research_chart_symbol(
            self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
        );

        // ── popup windows ──
        self.render_cdl_reversal_early_windows(ctx, &chart_sym_research);

        self.render_cdl_reversal_classic_windows(ctx, &chart_sym_research);

        self.render_cdl_shadow_kicking_windows(ctx, &chart_sym_research);

        self.render_cdl_gap_continuation_windows(ctx, &chart_sym_research);

        self.render_cdl_final_continuation_windows(ctx, &chart_sym_research);
    }
}
