use super::*;

impl TyphooNApp {
    pub(super) fn render_research_portmanteau_ou_long_memory_spectrum_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "MCLEODLI — McLeod-Li Squared-Returns Portmanteau",
                default_size: [560.0, 320.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_mcleodli,
            &mut self.mcleodli_symbol,
            &mut self.mcleodli_loading,
            &mut self.mcleodli_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_mcleodli(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeMcLeodLiSnapshot { symbol },
            super::render::render_mcleodli_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "OUFIT — Ornstein-Uhlenbeck Mean-Reversion Fit",
                default_size: [560.0, 340.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_oufit,
            &mut self.oufit_symbol,
            &mut self.oufit_loading,
            &mut self.oufit_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_oufit(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeOuFitSnapshot { symbol },
            super::render::render_oufit_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "GPH — Geweke-Porter-Hudak Long-Memory d̂",
                default_size: [560.0, 320.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_gph,
            &mut self.gph_symbol,
            &mut self.gph_loading,
            &mut self.gph_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_gph(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeGphSnapshot { symbol },
            super::render::render_gph_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "BURGSPEC — Burg Maximum-Entropy AR Spectrum",
                default_size: [560.0, 340.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_burgspec,
            &mut self.burgspec_symbol,
            &mut self.burgspec_loading,
            &mut self.burgspec_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_burgspec(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeBurgSpecSnapshot { symbol },
            super::render::render_burgspec_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "KENDALLTAU — Kendall's Tau Lag-1 Rank Autocorrelation",
                default_size: [560.0, 320.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_kendalltau,
            &mut self.kendalltau_symbol,
            &mut self.kendalltau_loading,
            &mut self.kendalltau_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_kendalltau(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeKendallTauSnapshot { symbol },
            super::render::render_kendalltau_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
