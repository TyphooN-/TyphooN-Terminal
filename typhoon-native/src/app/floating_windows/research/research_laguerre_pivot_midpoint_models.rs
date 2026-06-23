use super::*;

impl TyphooNApp {
    pub(super) fn render_research_laguerre_pivot_midpoint_models_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research LAGUERRE_RSI / ZIGZAG / PGO / HT_TRENDLINE / MIDPOINT windows ──

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "LAGUERRE_RSI — Ehlers 4-stage Laguerre Filter RSI (γ=0.5)",
                default_size: [560.0, 260.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_laguerre_rsi_win,
            &mut self.laguerre_rsi_win_symbol,
            &mut self.laguerre_rsi_win_loading,
            &mut self.laguerre_rsi_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_laguerre_rsi(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeLaguerreRsiSnapshot { symbol },
            super::render::render_laguerre_rsi_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "ZIGZAG — Percent-Threshold Pivot Reversal Detector (5% default)",
                default_size: [560.0, 260.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_zigzag_win,
            &mut self.zigzag_win_symbol,
            &mut self.zigzag_win_loading,
            &mut self.zigzag_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_zigzag(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeZigzagSnapshot { symbol },
            super::render::render_zigzag_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "PGO — Pretty Good Oscillator (Mark Johnson, (close−SMA)/EMA(TR), N=14)",
                default_size: [560.0, 260.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_pgo_win,
            &mut self.pgo_win_symbol,
            &mut self.pgo_win_loading,
            &mut self.pgo_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_pgo(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputePgoSnapshot { symbol },
            super::render::render_pgo_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title:
                    "HT_TRENDLINE — Hilbert Instantaneous Trendline (Ehlers, period-adaptive WMA)",
                default_size: [560.0, 260.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_ht_trendline_win,
            &mut self.ht_trendline_win_symbol,
            &mut self.ht_trendline_win_loading,
            &mut self.ht_trendline_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_ht_trendline(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeHtTrendlineSnapshot { symbol },
            super::render::render_ht_trendline_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "MIDPOINT — (HHV(N) + LLV(N)) / 2 with Close Position (N=14)",
                default_size: [560.0, 260.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_midpoint_win,
            &mut self.midpoint_win_symbol,
            &mut self.midpoint_win_loading,
            &mut self.midpoint_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_midpoint(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeMidpointSnapshot { symbol },
            super::render::render_midpoint_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
