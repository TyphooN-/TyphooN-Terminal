use super::*;

impl TyphooNApp {
    pub(super) fn render_research_directional_moneyflow_sar_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "ADX — Wilder's Directional Index (14)",
                default_size: [520.0, 280.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_adx_win,
            &mut self.adx_win_symbol,
            &mut self.adx_win_loading,
            &mut self.adx_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_adx(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeAdxSnapshot { symbol },
            super::render::render_adx_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "CCI — Commodity Channel Index (20)",
                default_size: [520.0, 260.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_cci_win,
            &mut self.cci_win_symbol,
            &mut self.cci_win_loading,
            &mut self.cci_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_cci(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeCciSnapshot { symbol },
            super::render::render_cci_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "CMF — Chaikin Money Flow (20)",
                default_size: [520.0, 260.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_cmf_win,
            &mut self.cmf_win_symbol,
            &mut self.cmf_win_loading,
            &mut self.cmf_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_cmf(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeCmfSnapshot { symbol },
            super::render::render_cmf_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "MFI — Money Flow Index (14)",
                default_size: [520.0, 260.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_mfi_win,
            &mut self.mfi_win_symbol,
            &mut self.mfi_win_loading,
            &mut self.mfi_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_mfi(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeMfiSnapshot { symbol },
            super::render::render_mfi_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "PSAR — Parabolic Stop-And-Reverse",
                default_size: [520.0, 300.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_psar_win,
            &mut self.psar_win_symbol,
            &mut self.psar_win_loading,
            &mut self.psar_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_psar(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputePsarSnapshot { symbol },
            super::render::render_psar_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "VORTEX — Vortex Indicator (14)",
                default_size: [520.0, 280.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_vortex_win,
            &mut self.vortex_win_symbol,
            &mut self.vortex_win_loading,
            &mut self.vortex_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_vortex(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeVortexSnapshot { symbol },
            super::render::render_vortex_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "CHOP — Choppiness Index (14)",
                default_size: [520.0, 260.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_chop_win,
            &mut self.chop_win_symbol,
            &mut self.chop_win_loading,
            &mut self.chop_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_chop(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeChopSnapshot { symbol },
            super::render::render_chop_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "OBV — On-Balance Volume (20-bar slope)",
                default_size: [520.0, 280.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_obv_win,
            &mut self.obv_win_symbol,
            &mut self.obv_win_loading,
            &mut self.obv_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_obv(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeObvSnapshot { symbol },
            super::render::render_obv_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "TRIX — Triple-EMA Oscillator (15/9)",
                default_size: [520.0, 260.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_trix_win,
            &mut self.trix_win_symbol,
            &mut self.trix_win_loading,
            &mut self.trix_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_trix(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeTrixSnapshot { symbol },
            super::render::render_trix_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "HMA — Hull Moving Average (20)",
                default_size: [520.0, 260.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_hma_win,
            &mut self.hma_win_symbol,
            &mut self.hma_win_loading,
            &mut self.hma_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_hma(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeHmaSnapshot { symbol },
            super::render::render_hma_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
