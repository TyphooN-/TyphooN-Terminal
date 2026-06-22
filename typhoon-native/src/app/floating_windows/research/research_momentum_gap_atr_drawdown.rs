use super::*;

impl TyphooNApp {
    pub(super) fn render_research_momentum_gap_atr_drawdown_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // SURPSTK — Earnings Surprise Streak
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "SURPSTK — Earnings Surprise Streak",
                default_size: [640.0, 380.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_surpstk,
            &mut self.surpstk_symbol,
            &mut self.surpstk_loading,
            &mut self.surpstk_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_surpstk(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeSurpstkSnapshot { symbol },
            super::render::render_surpstk_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // DVDRANK — Dividend Growth Rank vs sector peers
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "DVDRANK — Dividend Growth Rank",
                default_size: [640.0, 360.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_dvdrank,
            &mut self.dvdrank_symbol,
            &mut self.dvdrank_loading,
            &mut self.dvdrank_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_dvdrank(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeDvdrankSnapshot { symbol },
            super::render::render_dvdrank_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // EARMRANK — Earnings Momentum Rank vs sector peers
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "EARMRANK — Earnings Momentum Rank",
                default_size: [640.0, 360.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_earmrank,
            &mut self.earmrank_symbol,
            &mut self.earmrank_loading,
            &mut self.earmrank_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_earmrank(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeEarmrankSnapshot { symbol },
            super::render::render_earmrank_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // UPDGRANK — Upgrade/Downgrade Rank vs sector peers
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "UPDGRANK — Upgrade/Downgrade Rank",
                default_size: [640.0, 360.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_updgrank,
            &mut self.updgrank_symbol,
            &mut self.updgrank_loading,
            &mut self.updgrank_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_updgrank(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeUpdgrankSnapshot { symbol },
            super::render::render_updgrank_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // GY — Gap Yearly (253-bar gap census)
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "GY — Gap Yearly (253d census)",
                default_size: [640.0, 400.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_gy,
            &mut self.gy_symbol,
            &mut self.gy_loading,
            &mut self.gy_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_gy(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeGySnapshot { symbol },
            super::render::render_gy_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // DES — Daily Event Streak
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "DES — Daily Event Streak",
                default_size: [640.0, 400.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_des,
            &mut self.des_symbol,
            &mut self.des_loading,
            &mut self.des_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_des(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeDesSnapshot { symbol },
            super::render::render_des_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // DVDYIELDRANK — Dividend Yield Rank vs Sector Peers
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "DVDYIELDRANK — Dividend Yield Rank",
                default_size: [640.0, 400.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_dvdyieldrank,
            &mut self.dvdyieldrank_symbol,
            &mut self.dvdyieldrank_loading,
            &mut self.dvdyieldrank_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_dvdyieldrank(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeDvdyieldrankSnapshot { symbol },
            super::render::render_dvdyieldrank_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // SHRANK — Short Interest Rank (risk-inverted)
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "SHRANK — Short Interest Rank",
                default_size: [640.0, 400.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_shrank,
            &mut self.shrank_symbol,
            &mut self.shrank_loading,
            &mut self.shrank_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_shrank(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeShrankSnapshot { symbol },
            super::render::render_shrank_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // SHORTRANK_DELTA — Short Interest Trend Rank (risk-inverted)
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "SHORTRANK_DELTA — Short Interest Trend Rank",
                default_size: [700.0, 420.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_shortrank_delta,
            &mut self.shortrank_delta_symbol,
            &mut self.shortrank_delta_loading,
            &mut self.shortrank_delta_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_shortrank_delta(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeShortrankDeltaSnapshot { symbol },
            super::render::render_shortrank_delta_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // INSIDERCONC — Insider ownership concentration vs sector peers
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "INSIDERCONC — Insider Ownership Concentration",
                default_size: [720.0, 440.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_insiderconc,
            &mut self.insiderconc_symbol,
            &mut self.insiderconc_loading,
            &mut self.insiderconc_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_insiderconc(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeInsiderconcSnapshot { symbol },
            super::render::render_insiderconc_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
