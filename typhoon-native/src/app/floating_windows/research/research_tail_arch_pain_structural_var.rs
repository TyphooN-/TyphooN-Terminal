use super::*;

impl TyphooNApp {
    pub(super) fn render_research_tail_arch_pain_structural_var_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "HILLTAIL — Hill Tail-Index Estimator",
                default_size: [580.0, 380.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_hilltail,
            &mut self.hilltail_symbol,
            &mut self.hilltail_loading,
            &mut self.hilltail_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_hilltail(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeHilltailSnapshot { symbol },
            super::render::render_hilltail_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "ARCHLM — Engle ARCH-LM Test",
                default_size: [580.0, 380.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_archlm,
            &mut self.archlm_symbol,
            &mut self.archlm_loading,
            &mut self.archlm_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_archlm(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeArchlmSnapshot { symbol },
            super::render::render_archlm_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "PAINRATIO — Pain Index + Pain Ratio",
                default_size: [580.0, 360.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_painratio,
            &mut self.painratio_symbol,
            &mut self.painratio_loading,
            &mut self.painratio_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_painratio(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputePainratioSnapshot { symbol },
            super::render::render_painratio_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "CUSUM — Brown-Durbin-Evans Structural Break Test",
                default_size: [580.0, 380.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_cusum,
            &mut self.cusum_symbol,
            &mut self.cusum_loading,
            &mut self.cusum_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_cusum(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeCusumSnapshot { symbol },
            super::render::render_cusum_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "CFVAR — Cornish-Fisher Modified VaR",
                default_size: [620.0, 420.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_cfvar,
            &mut self.cfvar_symbol,
            &mut self.cfvar_loading,
            &mut self.cfvar_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_cfvar(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeCfvarSnapshot { symbol },
            super::render::render_cfvar_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
