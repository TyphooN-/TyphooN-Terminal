use super::*;

impl TyphooNApp {
    pub(super) fn render_research_ohlc_price_transforms_windows(&mut self, ctx: &egui::Context) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── : AVGPRICE / MEDPRICE / TYPPRICE / WCLPRICE / VARIANCE ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "AVGPRICE — OHLC average (O+H+L+C)/4",
                default_size: [520.0, 240.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_avgprice_win,
            &mut self.avgprice_win_symbol,
            &mut self.avgprice_win_loading,
            &mut self.avgprice_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_avgprice(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeAvgpriceSnapshot { symbol },
            super::render::render_avgprice_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "MEDPRICE — range median (H+L)/2",
                default_size: [520.0, 240.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_medprice_win,
            &mut self.medprice_win_symbol,
            &mut self.medprice_win_loading,
            &mut self.medprice_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_medprice(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeMedpriceSnapshot { symbol },
            super::render::render_medprice_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "TYPPRICE — typical price (H+L+C)/3",
                default_size: [520.0, 240.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_typprice_win,
            &mut self.typprice_win_symbol,
            &mut self.typprice_win_loading,
            &mut self.typprice_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_typprice(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeTypPriceSnapshot { symbol },
            super::render::render_typprice_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "WCLPRICE — weighted close (H+L+2C)/4",
                default_size: [520.0, 240.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_wclprice_win,
            &mut self.wclprice_win_symbol,
            &mut self.wclprice_win_loading,
            &mut self.wclprice_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_wclprice(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeWclPriceSnapshot { symbol },
            super::render::render_wclprice_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "VARIANCE — close variance (5-bar population, TA-Lib default)",
                default_size: [540.0, 260.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_variance_win,
            &mut self.variance_win_symbol,
            &mut self.variance_win_loading,
            &mut self.variance_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_variance(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeVarianceSnapshot { symbol },
            super::render::render_variance_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
