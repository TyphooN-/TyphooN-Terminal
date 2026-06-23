use super::*;

impl TyphooNApp {
    pub(super) fn render_research_moving_average_regression_pivots_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "DEMA — Double Exponential Moving Average (length 20)",
                default_size: [540.0, 260.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_dema_win,
            &mut self.dema_win_symbol,
            &mut self.dema_win_loading,
            &mut self.dema_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_dema(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeDemaSnapshot { symbol },
            super::render::render_dema_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "TEMA — Triple Exponential Moving Average (length 20)",
                default_size: [540.0, 260.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_tema_win,
            &mut self.tema_win_symbol,
            &mut self.tema_win_loading,
            &mut self.tema_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_tema(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeTemaSnapshot { symbol },
            super::render::render_tema_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "LINREG — Linear Regression Channel (length 20, ±2σ)",
                default_size: [560.0, 300.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_linreg_win,
            &mut self.linreg_win_symbol,
            &mut self.linreg_win_loading,
            &mut self.linreg_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_linreg(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeLinregSnapshot { symbol },
            super::render::render_linreg_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "PIVOTS — Classic Floor-Trader Pivot Points (prior bar)",
                default_size: [560.0, 300.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_pivots_win,
            &mut self.pivots_win_symbol,
            &mut self.pivots_win_loading,
            &mut self.pivots_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_pivots(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputePivotsSnapshot { symbol },
            super::render::render_pivots_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "HEIKIN — Heikin-Ashi Candle Sentiment Tracker",
                default_size: [560.0, 300.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_heikin_win,
            &mut self.heikin_win_symbol,
            &mut self.heikin_win_loading,
            &mut self.heikin_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_heikin(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeHeikinSnapshot { symbol },
            super::render::render_heikin_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
