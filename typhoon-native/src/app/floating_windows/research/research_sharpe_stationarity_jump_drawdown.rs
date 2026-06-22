use super::*;

impl TyphooNApp {
    pub(super) fn render_research_sharpe_stationarity_jump_drawdown_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "PSR — Probabilistic Sharpe Ratio",
                default_size: [580.0, 380.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_psr,
            &mut self.psr_symbol,
            &mut self.psr_loading,
            &mut self.psr_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_psr(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputePsrSnapshot { symbol },
            super::render::render_psr_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "ADF — Dickey-Fuller Unit-Root Test",
                default_size: [580.0, 380.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_adf,
            &mut self.adf_symbol,
            &mut self.adf_loading,
            &mut self.adf_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_adf(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeAdfSnapshot { symbol },
            super::render::render_adf_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "MNKENDALL — Mann-Kendall Trend Test",
                default_size: [580.0, 380.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_mnkendall,
            &mut self.mnkendall_symbol,
            &mut self.mnkendall_loading,
            &mut self.mnkendall_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_mnkendall(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeMnkendallSnapshot { symbol },
            super::render::render_mnkendall_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "BIPOWER — Bipower Variation / Jump Ratio",
                default_size: [580.0, 380.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_bipower,
            &mut self.bipower_symbol,
            &mut self.bipower_loading,
            &mut self.bipower_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_bipower(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeBipowerSnapshot { symbol },
            super::render::render_bipower_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "DDDUR — Drawdown Duration Statistics",
                default_size: [580.0, 400.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_dddur,
            &mut self.dddur_symbol,
            &mut self.dddur_loading,
            &mut self.dddur_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_dddur(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeDddurSnapshot { symbol },
            super::render::render_dddur_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
