use super::*;

impl TyphooNApp {
    pub(super) fn render_research_sterling_kelly_stat_tests_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "STERLING — Sterling Ratio",
                default_size: [560.0, 360.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_sterling,
            &mut self.sterling_symbol,
            &mut self.sterling_loading,
            &mut self.sterling_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_sterling(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeSterlingSnapshot { symbol },
            super::render::render_sterling_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "KELLYF — Kelly Fraction / Optimal Leverage",
                default_size: [580.0, 400.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_kellyf,
            &mut self.kellyf_symbol,
            &mut self.kellyf_loading,
            &mut self.kellyf_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_kellyf(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeKellyfSnapshot { symbol },
            super::render::render_kellyf_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "LJUNGB — Ljung-Box Q-Statistic (h=10)",
                default_size: [560.0, 340.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_ljungb,
            &mut self.ljungb_symbol,
            &mut self.ljungb_loading,
            &mut self.ljungb_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_ljungb(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeLjungbSnapshot { symbol },
            super::render::render_ljungb_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "RUNSTEST — Wald-Wolfowitz Runs Test",
                default_size: [580.0, 400.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_runstest,
            &mut self.runstest_symbol,
            &mut self.runstest_loading,
            &mut self.runstest_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_runstest(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeRunstestSnapshot { symbol },
            super::render::render_runstest_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "ZERORET — Zero-Return-Day Fraction (Lesmond-Ogden-Trzcinka)",
                default_size: [580.0, 340.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_zeroret,
            &mut self.zeroret_symbol,
            &mut self.zeroret_loading,
            &mut self.zeroret_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_zeroret(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeZeroretSnapshot { symbol },
            super::render::render_zeroret_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
