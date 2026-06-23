use super::*;

impl TyphooNApp {
    pub(super) fn render_research_factor_ranking_extensions_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        // SIZEF — Size Factor Rank vs Sector Peers
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "SIZEF — Size Factor Rank vs Sector Peers",
                default_size: [640.0, 360.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_sizef,
            &mut self.sizef_symbol,
            &mut self.sizef_loading,
            &mut self.sizef_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_sizef(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeSizefSnapshot { symbol },
            super::render::render_sizef_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // MOMF — Momentum Factor Rank vs Sector Peers
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "MOMF — Momentum Factor Rank vs Sector Peers",
                default_size: [640.0, 360.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_momf,
            &mut self.momf_symbol,
            &mut self.momf_loading,
            &mut self.momf_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_momf(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeMomfSnapshot { symbol },
            super::render::render_momf_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // PEADRANK — Post-Earnings Drift Rank vs Sector Peers
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "PEADRANK — PEAD Drift Rank vs Sector Peers",
                default_size: [640.0, 360.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_peadrank,
            &mut self.peadrank_symbol,
            &mut self.peadrank_loading,
            &mut self.peadrank_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_peadrank(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputePeadrankSnapshot { symbol },
            super::render::render_peadrank_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // FQM — Fundamental Quality Meter
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "FQM — Fundamental Quality Meter",
                default_size: [640.0, 380.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_fqm,
            &mut self.fqm_symbol,
            &mut self.fqm_loading,
            &mut self.fqm_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_fqm(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeFqmSnapshot { symbol },
            super::render::render_fqm_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // REVRANK — Relative 3y Revenue CAGR vs Sector Median
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "REVRANK — Relative 3y Revenue CAGR vs Sector",
                default_size: [640.0, 380.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_revrank,
            &mut self.revrank_symbol,
            &mut self.revrank_loading,
            &mut self.revrank_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_revrank(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeRevrankSnapshot { symbol },
            super::render::render_revrank_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // LEVRANK — Leverage Rank vs Sector Peers
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "LEVRANK — Leverage Rank vs Sector",
                default_size: [640.0, 380.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_levrank,
            &mut self.levrank_symbol,
            &mut self.levrank_loading,
            &mut self.levrank_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_levrank(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeLevrankSnapshot { symbol },
            super::render::render_levrank_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // OPERANK — Operating Quality Rank vs Sector Peers
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "OPERANK — Operating Quality Rank vs Sector",
                default_size: [640.0, 380.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_operank,
            &mut self.operank_symbol,
            &mut self.operank_loading,
            &mut self.operank_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_operank(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeOperankSnapshot { symbol },
            super::render::render_operank_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // FQMRANK — Fundamental Quality Meter Rank vs Sector Peers
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "FQMRANK — Fundamental Quality Rank vs Sector",
                default_size: [640.0, 380.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_fqmrank,
            &mut self.fqmrank_symbol,
            &mut self.fqmrank_loading,
            &mut self.fqmrank_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_fqmrank(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeFqmrankSnapshot { symbol },
            super::render::render_fqmrank_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // LIQRANK — Liquidity Rank vs Sector Peers
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "LIQRANK — Liquidity Rank vs Sector",
                default_size: [640.0, 380.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_liqrank,
            &mut self.liqrank_symbol,
            &mut self.liqrank_loading,
            &mut self.liqrank_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_liqrank(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeLiqrankSnapshot { symbol },
            super::render::render_liqrank_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // TLRANK — 30-day Liquidity Rank vs Sector Peers
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "TLRANK — 30-Day Liquidity Rank",
                default_size: [660.0, 400.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_tlrank,
            &mut self.tlrank_symbol,
            &mut self.tlrank_loading,
            &mut self.tlrank_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_tlrank(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeTlrankSnapshot { symbol },
            super::render::render_tlrank_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
