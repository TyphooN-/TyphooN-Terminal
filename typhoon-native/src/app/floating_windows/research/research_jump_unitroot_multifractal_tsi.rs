use super::*;

impl TyphooNApp {
    pub(super) fn render_research_jump_unitroot_multifractal_tsi_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "BNSJUMP — Barndorff-Nielsen-Shephard Jump-Test Z",
                default_size: [560.0, 320.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_bnsjump,
            &mut self.bnsjump_symbol,
            &mut self.bnsjump_loading,
            &mut self.bnsjump_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_bnsjump(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeBnsjumpSnapshot { symbol },
            super::render::render_bnsjump_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "PPROOT — Phillips-Perron Unit-Root Test",
                default_size: [560.0, 320.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_pproot,
            &mut self.pproot_symbol,
            &mut self.pproot_loading,
            &mut self.pproot_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_pproot(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputePprootSnapshot { symbol },
            super::render::render_pproot_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "MFDFA — Multifractal DFA (q ∈ {-2, 0, +2})",
                default_size: [560.0, 320.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_mfdfa,
            &mut self.mfdfa_symbol,
            &mut self.mfdfa_loading,
            &mut self.mfdfa_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_mfdfa(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeMfdfaSnapshot { symbol },
            super::render::render_mfdfa_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "HILLKS — Hill-Tail KS Goodness-of-Fit",
                default_size: [540.0, 300.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_hillks,
            &mut self.hillks_symbol,
            &mut self.hillks_loading,
            &mut self.hillks_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_hillks(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeHillksSnapshot { symbol },
            super::render::render_hillks_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "TSI — True Strength Index (Blau 1991)",
                default_size: [540.0, 300.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_tsi,
            &mut self.tsi_symbol,
            &mut self.tsi_loading,
            &mut self.tsi_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_tsi(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeTsiSnapshot { symbol },
            super::render::render_tsi_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
