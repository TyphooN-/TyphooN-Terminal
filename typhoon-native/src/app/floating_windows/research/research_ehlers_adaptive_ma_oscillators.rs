use super::*;

impl TyphooNApp {
    pub(super) fn render_research_ehlers_adaptive_ma_oscillators_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research WMA / RAINBOW / MESA_SINE / FRAMA / IBS windows ──

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "WMA — Weighted Moving Average (linearly-weighted SMA, N=20)",
                default_size: [560.0, 240.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_wma_win,
            &mut self.wma_win_symbol,
            &mut self.wma_win_loading,
            &mut self.wma_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_wma(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeWmaSnapshot { symbol },
            super::render::render_wma_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "RAINBOW — Rainbow MA Oscillator (10-level recursive SMA stack)",
                default_size: [580.0, 260.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_rainbow_win,
            &mut self.rainbow_win_symbol,
            &mut self.rainbow_win_loading,
            &mut self.rainbow_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_rainbow(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeRainbowSnapshot { symbol },
            super::render::render_rainbow_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "MESA_SINE — Ehlers MESA Sine Wave (cycle phase + lead-sine)",
                default_size: [580.0, 260.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_mesa_sine_win,
            &mut self.mesa_sine_win_symbol,
            &mut self.mesa_sine_win_loading,
            &mut self.mesa_sine_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_mesa_sine(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeMesaSineSnapshot { symbol },
            super::render::render_mesa_sine_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "FRAMA — Fractal Adaptive Moving Average (Ehlers, D-driven α)",
                default_size: [560.0, 240.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_frama_win,
            &mut self.frama_win_symbol,
            &mut self.frama_win_loading,
            &mut self.frama_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_frama(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeFramaSnapshot { symbol },
            super::render::render_frama_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "IBS — Internal Bar Strength ((close−low)/(high−low) + 14-bar SMA)",
                default_size: [560.0, 240.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_ibs_win,
            &mut self.ibs_win_symbol,
            &mut self.ibs_win_loading,
            &mut self.ibs_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_ibs(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeIbsSnapshot { symbol },
            super::render::render_ibs_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
