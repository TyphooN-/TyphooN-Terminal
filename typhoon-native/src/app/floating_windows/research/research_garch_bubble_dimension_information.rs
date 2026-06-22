use super::*;

impl TyphooNApp {
    pub(super) fn render_research_garch_bubble_dimension_information_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "GARCH11 — GARCH(1,1) Conditional Volatility Fit",
                default_size: [560.0, 340.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_garch11,
            &mut self.garch11_symbol,
            &mut self.garch11_loading,
            &mut self.garch11_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_garch11(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeGarch11Snapshot { symbol },
            super::render::render_garch11_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "SADF — Phillips-Wu-Yu Sup-ADF Bubble Test",
                default_size: [560.0, 320.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_sadf,
            &mut self.sadf_symbol,
            &mut self.sadf_loading,
            &mut self.sadf_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_sadf(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeSadfSnapshot { symbol },
            super::render::render_sadf_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "CORDIM — Grassberger-Procaccia Correlation Dimension D2",
                default_size: [540.0, 300.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_cordim,
            &mut self.cordim_symbol,
            &mut self.cordim_loading,
            &mut self.cordim_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_cordim(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeCordimSnapshot { symbol },
            super::render::render_cordim_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "SKSPEC — Rolling-Window Skewness Spectrum",
                default_size: [560.0, 320.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_skspec,
            &mut self.skspec_symbol,
            &mut self.skspec_loading,
            &mut self.skspec_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_skspec(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeSkspecSnapshot { symbol },
            super::render::render_skspec_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "AUTOMI — Auto Mutual Information (Info-Theoretic ACF)",
                default_size: [540.0, 300.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_automi,
            &mut self.automi_symbol,
            &mut self.automi_loading,
            &mut self.automi_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_automi(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeAutomiSnapshot { symbol },
            super::render::render_automi_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
