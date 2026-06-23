use super::*;

impl TyphooNApp {
    pub(super) fn render_research_robust_entropy_quantile_volatility_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "ROBVOL — Robust Volatility",
                default_size: [520.0, 300.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_robvol,
            &mut self.robvol_symbol,
            &mut self.robvol_loading,
            &mut self.robvol_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_robvol(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeRobvolSnapshot { symbol },
            super::render::render_robvol_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "RENYIENT — Rényi Entropy (α=2)",
                default_size: [520.0, 300.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_renyient,
            &mut self.renyient_symbol,
            &mut self.renyient_loading,
            &mut self.renyient_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_renyient(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeRenyientSnapshot { symbol },
            super::render::render_renyient_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "RETQUANT — Return Quantile Profile",
                default_size: [600.0, 360.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_retquant,
            &mut self.retquant_symbol,
            &mut self.retquant_loading,
            &mut self.retquant_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_retquant(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeRetquantSnapshot { symbol },
            super::render::render_retquant_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "MSENT — Multiscale Entropy",
                default_size: [520.0, 300.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_msent,
            &mut self.msent_symbol,
            &mut self.msent_loading,
            &mut self.msent_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_msent(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeMsentSnapshot { symbol },
            super::render::render_msent_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "EWMAVOL — EWMA Volatility (RiskMetrics)",
                default_size: [520.0, 300.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_ewmavol,
            &mut self.ewmavol_symbol,
            &mut self.ewmavol_loading,
            &mut self.ewmavol_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_ewmavol(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeEwmavolSnapshot { symbol },
            super::render::render_ewmavol_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
