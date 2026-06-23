use super::*;

impl TyphooNApp {
    pub(super) fn render_research_calmar_ulcer_liquidity_normality_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "CALMAR — Calmar Ratio (Return / Max Drawdown)",
                default_size: [640.0, 440.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_calmar,
            &mut self.calmar_symbol,
            &mut self.calmar_loading,
            &mut self.calmar_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_calmar(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeCalmarSnapshot { symbol },
            super::render::render_calmar_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "ULCER — Ulcer Index + Martin Ratio (UPI)",
                default_size: [640.0, 440.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_ulcer,
            &mut self.ulcer_symbol,
            &mut self.ulcer_loading,
            &mut self.ulcer_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_ulcer(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeUlcerSnapshot { symbol },
            super::render::render_ulcer_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "VARRATIO — Lo-MacKinlay Variance Ratio",
                default_size: [640.0, 440.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_varratio,
            &mut self.varratio_symbol,
            &mut self.varratio_loading,
            &mut self.varratio_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_varratio(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeVarratioSnapshot { symbol },
            super::render::render_varratio_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "AMIHUD — Amihud Illiquidity Ratio",
                default_size: [640.0, 440.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_amihud,
            &mut self.amihud_symbol,
            &mut self.amihud_loading,
            &mut self.amihud_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_amihud(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeAmihudSnapshot { symbol },
            super::render::render_amihud_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "JBNORM — Jarque-Bera Normality Test",
                default_size: [640.0, 440.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_jbnorm,
            &mut self.jbnorm_symbol,
            &mut self.jbnorm_loading,
            &mut self.jbnorm_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_jbnorm(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeJbnormSnapshot { symbol },
            super::render::render_jbnorm_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
