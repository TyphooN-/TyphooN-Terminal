use super::*;

impl TyphooNApp {
    pub(super) fn render_research_omega_fractal_burke_seasonality_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "OMEGA — Omega Ratio (τ=0)",
                default_size: [640.0, 440.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_omega,
            &mut self.omega_symbol,
            &mut self.omega_loading,
            &mut self.omega_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_omega(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeOmegaSnapshot { symbol },
            super::render::render_omega_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "DFA — Detrended Fluctuation Analysis",
                default_size: [640.0, 440.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_dfa,
            &mut self.dfa_symbol,
            &mut self.dfa_loading,
            &mut self.dfa_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_dfa(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeDfaSnapshot { symbol },
            super::render::render_dfa_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "BURKE — Burke Ratio (Σdd² adjusted)",
                default_size: [640.0, 440.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_burke,
            &mut self.burke_symbol,
            &mut self.burke_loading,
            &mut self.burke_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_burke(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeBurkeSnapshot { symbol },
            super::render::render_burke_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "MONTHSEAS — Monthly Seasonality",
                default_size: [720.0, 540.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_monthseas,
            &mut self.monthseas_symbol,
            &mut self.monthseas_loading,
            &mut self.monthseas_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_monthseas(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeMonthseasSnapshot { symbol },
            super::render::render_monthseas_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "ROLLSPRD — Roll's Implicit Bid-Ask Spread",
                default_size: [640.0, 440.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_rollsprd,
            &mut self.rollsprd_symbol,
            &mut self.rollsprd_loading,
            &mut self.rollsprd_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_rollsprd(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeRollsprdSnapshot { symbol },
            super::render::render_rollsprd_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
