use super::*;

impl TyphooNApp {
    pub(super) fn render_research_directional_movement_family_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research PLUS_DI / MINUS_DI / PLUS_DM / MINUS_DM / DX ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "PLUS_DI — Wilder +DI (period 14)",
                default_size: [540.0, 260.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_plus_di_win,
            &mut self.plus_di_win_symbol,
            &mut self.plus_di_win_loading,
            &mut self.plus_di_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_plus_di(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputePlusDiSnapshot { symbol },
            super::render::render_plus_di_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "MINUS_DI — Wilder −DI (period 14)",
                default_size: [540.0, 260.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_minus_di_win,
            &mut self.minus_di_win_symbol,
            &mut self.minus_di_win_loading,
            &mut self.minus_di_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_minus_di(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeMinusDiSnapshot { symbol },
            super::render::render_minus_di_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "PLUS_DM — Wilder raw +DM (period 14)",
                default_size: [540.0, 260.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_plus_dm_win,
            &mut self.plus_dm_win_symbol,
            &mut self.plus_dm_win_loading,
            &mut self.plus_dm_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_plus_dm(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputePlusDmSnapshot { symbol },
            super::render::render_plus_dm_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "MINUS_DM — Wilder raw −DM (period 14)",
                default_size: [540.0, 260.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_minus_dm_win,
            &mut self.minus_dm_win_symbol,
            &mut self.minus_dm_win_loading,
            &mut self.minus_dm_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_minus_dm(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeMinusDmSnapshot { symbol },
            super::render::render_minus_dm_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "DX — Wilder Directional Movement Index (period 14)",
                default_size: [540.0, 260.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_dx_win,
            &mut self.dx_win_symbol,
            &mut self.dx_win_loading,
            &mut self.dx_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_dx(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeDxSnapshot { symbol },
            super::render::render_dx_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
