use super::*;

impl TyphooNApp {
    pub(super) fn render_research_aroon_macd_variable_ma_windows(&mut self, ctx: &egui::Context) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "AROONOSC — Aroon Oscillator (period 14)",
                default_size: [540.0, 260.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_aroonosc_win,
            &mut self.aroonosc_win_symbol,
            &mut self.aroonosc_win_loading,
            &mut self.aroonosc_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_aroonosc(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeAroonoscSnapshot { symbol },
            super::render::render_aroonosc_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "MINMAXINDEX — combined min+max recency (period 30)",
                default_size: [540.0, 260.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_minmaxindex_win,
            &mut self.minmaxindex_win_symbol,
            &mut self.minmaxindex_win_loading,
            &mut self.minmaxindex_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_minmaxindex(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeMinMaxIndexSnapshot { symbol },
            super::render::render_minmaxindex_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "MACDEXT — MACD with SMA (12/26/9)",
                default_size: [540.0, 290.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_macdext_win,
            &mut self.macdext_win_symbol,
            &mut self.macdext_win_loading,
            &mut self.macdext_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_macdext(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeMacdextSnapshot { symbol },
            super::render::render_macdext_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "MACDFIX — MACD with hardcoded EMA 12/26 + signal 9",
                default_size: [540.0, 280.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_macdfix_win,
            &mut self.macdfix_win_symbol,
            &mut self.macdfix_win_loading,
            &mut self.macdfix_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_macdfix(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeMacdfixSnapshot { symbol },
            super::render::render_macdfix_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "MAVP — Moving Average with Variable Period (5..30 ramp)",
                default_size: [540.0, 260.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_mavp_win,
            &mut self.mavp_win_symbol,
            &mut self.mavp_win_loading,
            &mut self.mavp_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_mavp(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeMavpSnapshot { symbol },
            super::render::render_mavp_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
