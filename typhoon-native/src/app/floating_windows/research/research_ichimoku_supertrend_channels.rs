use super::*;

impl TyphooNApp {
    pub(super) fn render_research_ichimoku_supertrend_channels_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "ICHIMOKU — Kinko Hyo Cloud",
                default_size: [560.0, 340.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_ichimoku_win,
            &mut self.ichimoku_win_symbol,
            &mut self.ichimoku_win_loading,
            &mut self.ichimoku_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_ichimoku(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeIchimokuSnapshot { symbol },
            super::render::render_ichimoku_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "SUPERTREND — ATR Trailing Stop",
                default_size: [520.0, 320.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_supertrend_win,
            &mut self.supertrend_win_symbol,
            &mut self.supertrend_win_loading,
            &mut self.supertrend_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_supertrend(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeSupertrendSnapshot { symbol },
            super::render::render_supertrend_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "KELTNER — Channels (EMA 20 ± 2·ATR 10)",
                default_size: [520.0, 320.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_keltner_win,
            &mut self.keltner_win_symbol,
            &mut self.keltner_win_loading,
            &mut self.keltner_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_keltner(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeKeltnerSnapshot { symbol },
            super::render::render_keltner_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "FISHER — Ehlers Fisher Transform",
                default_size: [520.0, 280.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_fisher_win,
            &mut self.fisher_win_symbol,
            &mut self.fisher_win_loading,
            &mut self.fisher_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_fisher(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeFisherSnapshot { symbol },
            super::render::render_fisher_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "AROON — Up / Down / Oscillator (25)",
                default_size: [520.0, 280.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_aroon_win,
            &mut self.aroon_win_symbol,
            &mut self.aroon_win_loading,
            &mut self.aroon_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_aroon(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeAroonSnapshot { symbol },
            super::render::render_aroon_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
