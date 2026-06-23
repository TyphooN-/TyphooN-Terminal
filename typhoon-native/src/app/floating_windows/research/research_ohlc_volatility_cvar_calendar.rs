use super::*;

impl TyphooNApp {
    pub(super) fn render_research_ohlc_volatility_cvar_calendar_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "PARKINSON — H-L Range Volatility",
                default_size: [560.0, 360.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_parkinson,
            &mut self.parkinson_symbol,
            &mut self.parkinson_loading,
            &mut self.parkinson_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_parkinson(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeParkinsonSnapshot { symbol },
            super::render::render_parkinson_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "GKVOL — Garman-Klass OHLC Volatility",
                default_size: [560.0, 380.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_gkvol,
            &mut self.gkvol_symbol,
            &mut self.gkvol_loading,
            &mut self.gkvol_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_gkvol(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeGkvolSnapshot { symbol },
            super::render::render_gkvol_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "RSVOL — Rogers-Satchell OHLC Volatility",
                default_size: [560.0, 340.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_rsvol,
            &mut self.rsvol_symbol,
            &mut self.rsvol_loading,
            &mut self.rsvol_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_rsvol(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeRsvolSnapshot { symbol },
            super::render::render_rsvol_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "CVAR — Conditional VaR / Expected Shortfall",
                default_size: [600.0, 420.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_cvar,
            &mut self.cvar_symbol,
            &mut self.cvar_loading,
            &mut self.cvar_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_cvar(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeCvarSnapshot { symbol },
            super::render::render_cvar_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "DOWEFFECT — Day-of-Week Intraday Seasonality",
                default_size: [640.0, 460.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_doweffect,
            &mut self.doweffect_symbol,
            &mut self.doweffect_loading,
            &mut self.doweffect_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_doweffect(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeDoweffectSnapshot { symbol },
            super::render::render_doweffect_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
