use super::*;

impl TyphooNApp {
    pub(super) fn render_research_fractal_tail_nonlinear_rank_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "HIGUCHI — Higuchi Fractal Dimension (1988)",
                default_size: [540.0, 300.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_higuchi,
            &mut self.higuchi_symbol,
            &mut self.higuchi_loading,
            &mut self.higuchi_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_higuchi(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeHiguchiSnapshot { symbol },
            super::render::render_higuchi_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "PICKANDS — Pickands 1975 Tail-Index Estimator",
                default_size: [540.0, 320.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_pickands,
            &mut self.pickands_symbol,
            &mut self.pickands_loading,
            &mut self.pickands_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_pickands(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputePickandsSnapshot { symbol },
            super::render::render_pickands_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "KAPPA3 — Kaplan-Knowles 2004 Kappa-3 Ratio",
                default_size: [540.0, 320.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_kappa3,
            &mut self.kappa3_symbol,
            &mut self.kappa3_loading,
            &mut self.kappa3_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_kappa3(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeKappa3Snapshot { symbol },
            super::render::render_kappa3_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "LYAPUNOV — Largest Lyapunov Exponent (Rosenstein 1993)",
                default_size: [560.0, 320.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_lyapunov,
            &mut self.lyapunov_symbol,
            &mut self.lyapunov_loading,
            &mut self.lyapunov_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_lyapunov(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeLyapunovSnapshot { symbol },
            super::render::render_lyapunov_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "RANKAC — Spearman Rank Autocorrelation (lags 1/5/10)",
                default_size: [540.0, 300.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_rankac,
            &mut self.rankac_symbol,
            &mut self.rankac_loading,
            &mut self.rankac_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_rankac(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeRankacSnapshot { symbol },
            super::render::render_rankac_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
