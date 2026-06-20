use super::*;

mod basic_shadow_patterns;
mod cloud_piercing_patterns;
mod engulfing_harami_patterns;
mod final_star_patterns;
mod morning_evening_star_patterns;
mod simple_one_bar_patterns;
mod three_soldiers_crows_patterns;

impl TyphooNApp {
    pub(super) fn render_research_candlestick_core_patterns_windows(
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

        // ── Research Round 72 CDL* windows ─────────────────────────────────
        self.render_cdl_basic_shadow_windows(ctx, &chart_sym_research);

        self.render_cdl_engulfing_harami_windows(ctx, &chart_sym_research);

        self.render_cdl_morning_evening_star_windows(ctx, &chart_sym_research);

        self.render_cdl_three_soldiers_crows_windows(ctx, &chart_sym_research);

        self.render_cdl_cloud_piercing_windows(ctx, &chart_sym_research);

        self.render_cdl_primary_one_bar_windows(ctx, &chart_sym_research);

        self.render_cdl_final_star_windows(ctx, &chart_sym_research);
    }
}
