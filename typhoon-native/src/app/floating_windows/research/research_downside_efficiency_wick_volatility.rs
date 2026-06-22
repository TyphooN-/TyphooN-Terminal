use super::*;

impl TyphooNApp {
    pub(super) fn render_research_downside_efficiency_wick_volatility_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "DOWNVOL — Downside Deviation / Sortino",
                default_size: [640.0, 440.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_downvol,
            &mut self.downvol_symbol,
            &mut self.downvol_loading,
            &mut self.downvol_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_downvol(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeDownvolSnapshot { symbol },
            super::render::render_downvol_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "SHARPR — Sharpe Ratio (rf=0)",
                default_size: [640.0, 440.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_sharpr,
            &mut self.sharpr_symbol,
            &mut self.sharpr_loading,
            &mut self.sharpr_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_sharpr(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeSharprSnapshot { symbol },
            super::render::render_sharpr_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "EFFRATIO — Kaufman Efficiency Ratio",
                default_size: [640.0, 440.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_effratio,
            &mut self.effratio_symbol,
            &mut self.effratio_loading,
            &mut self.effratio_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_effratio(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeEffratioSnapshot { symbol },
            super::render::render_effratio_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "WICKBIAS — Upper vs Lower Wick Asymmetry",
                default_size: [640.0, 440.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_wickbias,
            &mut self.wickbias_symbol,
            &mut self.wickbias_loading,
            &mut self.wickbias_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_wickbias(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeWickbiasSnapshot { symbol },
            super::render::render_wickbias_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "VOLOFVOL — Stdev of Rolling 20d Realized Vol",
                default_size: [640.0, 440.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_volofvol,
            &mut self.volofvol_symbol,
            &mut self.volofvol_loading,
            &mut self.volofvol_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_volofvol(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeVolofvolSnapshot { symbol },
            super::render::render_volofvol_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
