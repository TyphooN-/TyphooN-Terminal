use super::*;

impl TyphooNApp {
    pub(super) fn render_research_gap_volatility_mean_reversion_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "DRAWUP — Rally History",
                default_size: [640.0, 440.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_drawup,
            &mut self.drawup_symbol,
            &mut self.drawup_loading,
            &mut self.drawup_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_drawup(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeDrawupSnapshot { symbol },
            super::render::render_drawup_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "GAPSTATS — Overnight Gap Statistics",
                default_size: [640.0, 440.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_gapstats,
            &mut self.gapstats_symbol,
            &mut self.gapstats_loading,
            &mut self.gapstats_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_gapstats(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeGapstatsSnapshot { symbol },
            super::render::render_gapstats_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "VOLCLUSTER — Volatility Clustering ACF",
                default_size: [640.0, 440.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_volcluster,
            &mut self.volcluster_symbol,
            &mut self.volcluster_loading,
            &mut self.volcluster_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_volcluster(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeVolclusterSnapshot { symbol },
            super::render::render_volcluster_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "CLOSEPLC — Close Placement in Daily Range",
                default_size: [640.0, 440.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_closeplc,
            &mut self.closeplc_symbol,
            &mut self.closeplc_loading,
            &mut self.closeplc_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_closeplc(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeCloseplcSnapshot { symbol },
            super::render::render_closeplc_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "MRHL — Mean-Reversion Half-Life (AR1)",
                default_size: [640.0, 440.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_mrhl,
            &mut self.mrhl_symbol,
            &mut self.mrhl_loading,
            &mut self.mrhl_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_mrhl(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeMrhlSnapshot { symbol },
            super::render::render_mrhl_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
