use super::*;

impl TyphooNApp {
    pub(super) fn render_research_oscillator_price_momentum_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "PPO — Percentage Price Oscillator (12/26/9)",
                default_size: [520.0, 280.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_ppo_win,
            &mut self.ppo_win_symbol,
            &mut self.ppo_win_loading,
            &mut self.ppo_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_ppo(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputePpoSnapshot { symbol },
            super::render::render_ppo_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "DPO — Detrended Price Oscillator (20)",
                default_size: [520.0, 260.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_dpo_win,
            &mut self.dpo_win_symbol,
            &mut self.dpo_win_loading,
            &mut self.dpo_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_dpo(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeDpoSnapshot { symbol },
            super::render::render_dpo_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "KST — Know Sure Thing (Pring, 1992)",
                default_size: [520.0, 300.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_kst_win,
            &mut self.kst_win_symbol,
            &mut self.kst_win_loading,
            &mut self.kst_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_kst(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeKstSnapshot { symbol },
            super::render::render_kst_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "ULTOSC — Ultimate Oscillator (7/14/28)",
                default_size: [520.0, 280.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_ultosc_win,
            &mut self.ultosc_win_symbol,
            &mut self.ultosc_win_loading,
            &mut self.ultosc_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_ultosc(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeUltoscSnapshot { symbol },
            super::render::render_ultosc_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "WILLR — Williams %R (14)",
                default_size: [520.0, 240.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_willr_win,
            &mut self.willr_win_symbol,
            &mut self.willr_win_loading,
            &mut self.willr_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_willr(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeWillrSnapshot { symbol },
            super::render::render_willr_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
