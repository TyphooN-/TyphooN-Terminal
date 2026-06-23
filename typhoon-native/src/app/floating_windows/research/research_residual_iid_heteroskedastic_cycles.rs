use super::*;

impl TyphooNApp {
    pub(super) fn render_research_residual_iid_heteroskedastic_cycles_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "DURBINWATSON — Durbin-Watson Residual Autocorrelation",
                default_size: [540.0, 280.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_durbinwatson,
            &mut self.durbinwatson_symbol,
            &mut self.durbinwatson_loading,
            &mut self.durbinwatson_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_durbinwatson(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeDurbinWatsonSnapshot { symbol },
            super::render::render_durbinwatson_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "BDSTEST — Brock-Dechert-Scheinkman iid Test",
                default_size: [560.0, 320.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_bdstest,
            &mut self.bdstest_symbol,
            &mut self.bdstest_loading,
            &mut self.bdstest_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_bdstest(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeBdsTestSnapshot { symbol },
            super::render::render_bdstest_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "BREUSCHPAGAN — Breusch-Pagan Heteroskedasticity LM Test",
                default_size: [560.0, 300.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_breuschpagan,
            &mut self.breuschpagan_symbol,
            &mut self.breuschpagan_loading,
            &mut self.breuschpagan_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_breuschpagan(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeBreuschPaganSnapshot { symbol },
            super::render::render_breuschpagan_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "TURNPTS — Bartels Turning-Points Test",
                default_size: [560.0, 300.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_turnpts,
            &mut self.turnpts_symbol,
            &mut self.turnpts_loading,
            &mut self.turnpts_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_turnpts(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeTurnPtsSnapshot { symbol },
            super::render::render_turnpts_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "PERIODOGRAM — Direct-DFT Dominant-Cycle Detection",
                default_size: [560.0, 320.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_periodogram,
            &mut self.periodogram_symbol,
            &mut self.periodogram_loading,
            &mut self.periodogram_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_periodogram(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputePeriodogramSnapshot { symbol },
            super::render::render_periodogram_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
