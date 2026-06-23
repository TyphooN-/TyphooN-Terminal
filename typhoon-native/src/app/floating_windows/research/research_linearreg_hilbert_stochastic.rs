use super::*;

impl TyphooNApp {
    pub(super) fn render_research_linearreg_hilbert_stochastic_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── egui windows ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "LINEARREG_SLOPE — Least-squares slope on close (TA-Lib parity)",
                default_size: [560.0, 260.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_linearreg_slope_win,
            &mut self.linearreg_slope_win_symbol,
            &mut self.linearreg_slope_win_loading,
            &mut self.linearreg_slope_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_linearreg_slope(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeLinearregSlopeSnapshot { symbol },
            super::render::render_linearreg_slope_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "HT_DCPERIOD — Hilbert Dominant Cycle Period (Ehlers homodyne)",
                default_size: [560.0, 260.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_ht_dcperiod_win,
            &mut self.ht_dcperiod_win_symbol,
            &mut self.ht_dcperiod_win_loading,
            &mut self.ht_dcperiod_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_ht_dcperiod(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeHtDcperiodSnapshot { symbol },
            super::render::render_ht_dcperiod_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "HT_TRENDMODE — Hilbert Trend vs Cycle Regime (Ehlers CV classifier)",
                default_size: [560.0, 260.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_ht_trendmode_win,
            &mut self.ht_trendmode_win_symbol,
            &mut self.ht_trendmode_win_loading,
            &mut self.ht_trendmode_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_ht_trendmode(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeHtTrendmodeSnapshot { symbol },
            super::render::render_ht_trendmode_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "ACCBANDS — Headley Acceleration Bands (SMA-20 of H×(1+4·(H-L)/(H+L)))",
                default_size: [580.0, 280.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_accbands_win,
            &mut self.accbands_win_symbol,
            &mut self.accbands_win_loading,
            &mut self.accbands_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_accbands(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeAccbandsSnapshot { symbol },
            super::render::render_accbands_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "STOCHF — Fast Stochastic (TA-Lib, unsmoothed %K + SMA-3 %D)",
                default_size: [560.0, 280.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_stochf_win,
            &mut self.stochf_win_symbol,
            &mut self.stochf_win_loading,
            &mut self.stochf_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_stochf(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeStochfSnapshot { symbol },
            super::render::render_stochf_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
