use super::*;

impl TyphooNApp {
    pub(super) fn render_research_volume_momentum_oscillators_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "MASS — Mass Index (Dorsey, 1992)",
                default_size: [520.0, 260.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_mass_win,
            &mut self.mass_win_symbol,
            &mut self.mass_win_loading,
            &mut self.mass_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_mass(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeMassSnapshot { symbol },
            super::render::render_mass_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "CHAIKOSC — Chaikin Oscillator (3/10)",
                default_size: [520.0, 280.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_chaikosc_win,
            &mut self.chaikosc_win_symbol,
            &mut self.chaikosc_win_loading,
            &mut self.chaikosc_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_chaikosc(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeChaikoscSnapshot { symbol },
            super::render::render_chaikosc_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "KLINGER — Klinger Volume Oscillator (34/55/13)",
                default_size: [520.0, 300.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_klinger_win,
            &mut self.klinger_win_symbol,
            &mut self.klinger_win_loading,
            &mut self.klinger_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_klinger(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeKlingerSnapshot { symbol },
            super::render::render_klinger_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "STOCHRSI — Stochastic RSI (14/14/3/3)",
                default_size: [520.0, 300.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_stochrsi_win,
            &mut self.stochrsi_win_symbol,
            &mut self.stochrsi_win_loading,
            &mut self.stochrsi_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_stochrsi(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeStochRsiSnapshot { symbol },
            super::render::render_stochrsi_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
