use super::*;

impl TyphooNApp {
    pub(super) fn render_research_upside_leverage_drawdown_var_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "UPR — Upside Potential Ratio",
                default_size: [520.0, 300.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_upr,
            &mut self.upr_symbol,
            &mut self.upr_loading,
            &mut self.upr_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_upr(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeUprSnapshot { symbol },
            super::render::render_upr_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "LEVEREFF — Leverage Effect",
                default_size: [560.0, 320.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_levereff,
            &mut self.levereff_symbol,
            &mut self.levereff_loading,
            &mut self.levereff_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_levereff(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeLevereffSnapshot { symbol },
            super::render::render_levereff_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "DRAWDAR — Drawdown-at-Risk",
                default_size: [560.0, 350.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_drawdar,
            &mut self.drawdar_symbol,
            &mut self.drawdar_loading,
            &mut self.drawdar_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_drawdar(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeDrawdarSnapshot { symbol },
            super::render::render_drawdar_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "VARHALF — Volatility Half-Life",
                default_size: [520.0, 300.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_varhalf,
            &mut self.varhalf_symbol,
            &mut self.varhalf_loading,
            &mut self.varhalf_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_varhalf(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeVarhalfSnapshot { symbol },
            super::render::render_varhalf_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "GINI — Return Concentration",
                default_size: [520.0, 280.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_gini,
            &mut self.gini_symbol,
            &mut self.gini_loading,
            &mut self.gini_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_gini(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeGiniSnapshot { symbol },
            super::render::render_gini_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
