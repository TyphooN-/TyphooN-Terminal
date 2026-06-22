use super::*;

impl TyphooNApp {
    pub(super) fn render_research_entropy_stationarity_recovery_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "SAMPEN — Sample Entropy",
                default_size: [520.0, 300.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_sampen,
            &mut self.sampen_symbol,
            &mut self.sampen_loading,
            &mut self.sampen_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_sampen(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeSampenSnapshot { symbol },
            super::render::render_sampen_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "PERMEN — Permutation Entropy",
                default_size: [520.0, 300.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_permen,
            &mut self.permen_symbol,
            &mut self.permen_loading,
            &mut self.permen_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_permen(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputePermenSnapshot { symbol },
            super::render::render_permen_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "RECFACT — Recovery Factor",
                default_size: [520.0, 300.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_recfact,
            &mut self.recfact_symbol,
            &mut self.recfact_loading,
            &mut self.recfact_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_recfact(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeRecfactSnapshot { symbol },
            super::render::render_recfact_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "KPSS — Stationarity Test",
                default_size: [520.0, 300.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_kpss,
            &mut self.kpss_symbol,
            &mut self.kpss_loading,
            &mut self.kpss_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_kpss(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeKpssSnapshot { symbol },
            super::render::render_kpss_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "SPECENT — Spectral Entropy",
                default_size: [520.0, 300.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_specent,
            &mut self.specent_symbol,
            &mut self.specent_loading,
            &mut self.specent_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_specent(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeSpecentSnapshot { symbol },
            super::render::render_specent_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
