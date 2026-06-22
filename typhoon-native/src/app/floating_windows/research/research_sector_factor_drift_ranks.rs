use super::*;

impl TyphooNApp {
    pub(super) fn render_research_sector_factor_drift_ranks_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──

        // VRK — Value Rank vs sector peers
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "VRK — Value Rank vs Sector Peers",
                default_size: [640.0, 360.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_vrk,
            &mut self.vrk_symbol,
            &mut self.vrk_loading,
            &mut self.vrk_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_vrk(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeVrkSnapshot { symbol },
            super::render::render_vrk_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // QRK — Quality Rank vs sector peers
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "QRK — Quality Rank vs Sector Peers",
                default_size: [640.0, 360.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_qrk,
            &mut self.qrk_symbol,
            &mut self.qrk_loading,
            &mut self.qrk_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_qrk(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeQrkSnapshot { symbol },
            super::render::render_qrk_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // RRK — Risk Rank vs sector peers (inverted — higher pct = SAFER)
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "RRK — Risk Rank vs Sector Peers (Higher = Safer)",
                default_size: [640.0, 360.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_rrk,
            &mut self.rrk_symbol,
            &mut self.rrk_loading,
            &mut self.rrk_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_rrk(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeRrkSnapshot { symbol },
            super::render::render_rrk_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // RELEPSGR — Relative 3y EPS CAGR vs sector median
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "RELEPSGR — Relative 3y EPS CAGR vs Sector",
                default_size: [640.0, 380.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_relepsgr,
            &mut self.relepsgr_symbol,
            &mut self.relepsgr_loading,
            &mut self.relepsgr_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_relepsgr(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeRelepsgrSnapshot { symbol },
            super::render::render_relepsgr_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // PEAD — Post-Earnings-Announcement Drift
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "PEAD — Post-Earnings-Announcement Drift",
                default_size: [720.0, 480.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_pead,
            &mut self.pead_symbol,
            &mut self.pead_loading,
            &mut self.pead_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_pead(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputePeadSnapshot { symbol },
            super::render::render_pead_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
