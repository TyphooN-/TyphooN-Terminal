use super::*;

impl TyphooNApp {
    pub(super) fn render_research_massindex_atr_squeeze_force_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "MASSINDEX — Dorsey Mass Index (EMA/EMA ratio, reversal bulge)",
                default_size: [580.0, 280.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_mass_index_win,
            &mut self.mass_index_win_symbol,
            &mut self.mass_index_win_loading,
            &mut self.mass_index_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_mass_index(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeMassIndexSnapshot { symbol },
            super::render::render_mass_index_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "NATR — Normalized ATR (TA-Lib, 100 × ATR / close)",
                default_size: [540.0, 240.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_natr_win,
            &mut self.natr_win_symbol,
            &mut self.natr_win_loading,
            &mut self.natr_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_natr(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeNatrSnapshot { symbol },
            super::render::render_natr_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "TTM_SQUEEZE — Carter's BB ⊂ KC Regime + Momentum (20)",
                default_size: [600.0, 280.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_ttm_squeeze_win,
            &mut self.ttm_squeeze_win_symbol,
            &mut self.ttm_squeeze_win_loading,
            &mut self.ttm_squeeze_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_ttm_squeeze(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeTtmSqueezeSnapshot { symbol },
            super::render::render_ttm_squeeze_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "FORCE_INDEX — Elder Force Index (EMA of volume × Δclose, 13)",
                default_size: [580.0, 260.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_force_index_win,
            &mut self.force_index_win_symbol,
            &mut self.force_index_win_loading,
            &mut self.force_index_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_force_index(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeForceIndexSnapshot { symbol },
            super::render::render_force_index_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "TRANGE — True Range (raw, single-bar, gap-aware)",
                default_size: [580.0, 280.0],
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_trange_win,
            &mut self.trange_win_symbol,
            &mut self.trange_win_loading,
            &mut self.trange_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_trange(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeTrangeSnapshot { symbol },
            super::render::render_trange_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
