use super::*;

impl TyphooNApp {
    pub(super) fn render_research_candlestick_marubozu_line_patterns_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── popup windows ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "CDLBELTHOLD — Belt Hold",
                default_size: [560.0, 260.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_cdl_belt_hold_win,
            &mut self.cdl_belt_hold_win_symbol,
            &mut self.cdl_belt_hold_win_loading,
            &mut self.cdl_belt_hold_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_cdl_belt_hold(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeCdlBeltHoldSnapshot { symbol },
            super::render::render_cdl_belt_hold_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "CDLCLOSINGMARUBOZU — Closing Marubozu",
                default_size: [560.0, 260.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_cdl_closing_marubozu_win,
            &mut self.cdl_closing_marubozu_win_symbol,
            &mut self.cdl_closing_marubozu_win_loading,
            &mut self.cdl_closing_marubozu_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_cdl_closing_marubozu(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeCdlClosingMarubozuSnapshot { symbol },
            super::render::render_cdl_closing_marubozu_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "CDLHIGHWAVE — High Wave",
                default_size: [560.0, 260.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_cdl_high_wave_win,
            &mut self.cdl_high_wave_win_symbol,
            &mut self.cdl_high_wave_win_loading,
            &mut self.cdl_high_wave_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_cdl_high_wave(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeCdlHighWaveSnapshot { symbol },
            super::render::render_cdl_high_wave_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "CDLLONGLINE — Long Line",
                default_size: [560.0, 260.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_cdl_long_line_win,
            &mut self.cdl_long_line_win_symbol,
            &mut self.cdl_long_line_win_loading,
            &mut self.cdl_long_line_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_cdl_long_line(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeCdlLongLineSnapshot { symbol },
            super::render::render_cdl_long_line_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "CDLSHORTLINE — Short Line",
                default_size: [560.0, 260.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_cdl_short_line_win,
            &mut self.cdl_short_line_win_symbol,
            &mut self.cdl_short_line_win_loading,
            &mut self.cdl_short_line_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_cdl_short_line(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeCdlShortLineSnapshot { symbol },
            super::render::render_cdl_short_line_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
