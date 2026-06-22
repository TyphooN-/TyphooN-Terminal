use super::*;

impl TyphooNApp {
    pub(super) fn render_research_entropy_tail_autocorrelation_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "ENTROPY — Shannon Return Entropy",
                default_size: [520.0, 300.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_entropy,
            &mut self.entropy_symbol,
            &mut self.entropy_loading,
            &mut self.entropy_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_entropy(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeEntropySnapshot { symbol },
            super::render::render_entropy_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "RACHEV — Conditional Tail Expectation Ratio",
                default_size: [560.0, 350.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_rachev,
            &mut self.rachev_symbol,
            &mut self.rachev_loading,
            &mut self.rachev_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_rachev(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeRachevSnapshot { symbol },
            super::render::render_rachev_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "GPR — Gain-to-Pain Ratio",
                default_size: [520.0, 350.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_gpr,
            &mut self.gpr_symbol,
            &mut self.gpr_loading,
            &mut self.gpr_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_gpr(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeGprSnapshot { symbol },
            super::render::render_gpr_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "PACF — Partial Autocorrelation",
                default_size: [560.0, 380.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_pacf,
            &mut self.pacf_symbol,
            &mut self.pacf_loading,
            &mut self.pacf_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_pacf(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputePacfSnapshot { symbol },
            super::render::render_pacf_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "APEN — Approximate Entropy",
                default_size: [520.0, 300.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_apen,
            &mut self.apen_symbol,
            &mut self.apen_loading,
            &mut self.apen_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_apen(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeApenSnapshot { symbol },
            super::render::render_apen_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
