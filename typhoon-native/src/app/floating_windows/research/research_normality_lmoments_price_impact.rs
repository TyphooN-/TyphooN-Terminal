use super::*;

impl TyphooNApp {
    pub(super) fn render_research_normality_lmoments_price_impact_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "KSNORM — Kolmogorov-Smirnov Normality Test",
                default_size: [540.0, 320.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_ksnorm,
            &mut self.ksnorm_symbol,
            &mut self.ksnorm_loading,
            &mut self.ksnorm_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_ksnorm(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeKsnormSnapshot { symbol },
            super::render::render_ksnorm_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "ADTEST — Anderson-Darling Normality Test",
                default_size: [540.0, 320.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_adtest,
            &mut self.adtest_symbol,
            &mut self.adtest_loading,
            &mut self.adtest_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_adtest(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeAdtestSnapshot { symbol },
            super::render::render_adtest_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "LMOM — L-Moments (Hosking 1990)",
                default_size: [520.0, 300.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_lmom,
            &mut self.lmom_symbol,
            &mut self.lmom_loading,
            &mut self.lmom_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_lmom(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeLmomSnapshot { symbol },
            super::render::render_lmom_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "KYLELAM — Kyle's Price-Impact λ",
                default_size: [540.0, 320.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_kylelam,
            &mut self.kylelam_symbol,
            &mut self.kylelam_loading,
            &mut self.kylelam_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_kylelam(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeKylelamSnapshot { symbol },
            super::render::render_kylelam_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
