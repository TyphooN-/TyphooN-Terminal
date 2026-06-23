use super::*;

impl TyphooNApp {
    pub(super) fn render_research_insider_dividend_earnings_momentum_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // MNGR — Insider Activity Bias
        if let Some(sym) = window_shell::render_compute_window_ext(
            ctx,
            window_shell::ComputeWindow {
                title: "MNGR — Insider Activity Bias",
                default_size: [560.0, 420.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_mngr,
            &mut self.mngr_symbol,
            &mut self.mngr_loading,
            &mut self.mngr_snapshot,
            |ui| {
                ui.label(egui::RichText::new("Window (days):").color(AXIS_TEXT));
                ui.add(egui::DragValue::new(&mut self.mngr_window_days).range(30..=365));
            },
            |conn, s| {
                typhoon_engine::core::research::get_insider_activity(conn, s)
                    .ok()
                    .flatten()
            },
            |symbol| symbol,
            super::render::render_mngr_snapshot,
        ) {
            let _ = self
                .broker_tx
                .send(BrokerCmd::ComputeInsiderActivitySnapshot {
                    symbol: sym,
                    window_days: self.mngr_window_days,
                });
        }

        // DIVG — Dividend Growth Analysis
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "DIVG — Dividend Growth Analysis",
                default_size: [600.0, 440.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_divg,
            &mut self.divg_symbol,
            &mut self.divg_loading,
            &mut self.divg_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_divg(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeDivgSnapshot { symbol },
            super::render::render_divg_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // EARM — Earnings Momentum Trend
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "EARM — Earnings Momentum Trend",
                default_size: [620.0, 460.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_earm,
            &mut self.earm_symbol,
            &mut self.earm_loading,
            &mut self.earm_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_earm(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeEarmSnapshot { symbol },
            super::render::render_earm_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // SECTR — Sector Rotation Strength
        if let Some(sym) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "SECTR — Sector Rotation Strength",
                default_size: [560.0, 420.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_sectr,
            &mut self.sectr_symbol,
            &mut self.sectr_loading,
            &mut self.sectr_snapshot,
            |conn, s| {
                typhoon_engine::core::research::get_sector_rotation(conn, s)
                    .ok()
                    .flatten()
            },
            |symbol| symbol,
            super::render::render_sectr_snapshot,
        ) {
            let symbol_sector = if let Some(ref cache) = self.cache {
                if let Ok(conn) = cache.connection() {
                    if let Ok(Some(fa)) =
                        typhoon_engine::core::fundamentals::get_fundamentals(&conn, &sym)
                    {
                        fa.sector
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            let _ = self
                .broker_tx
                .send(BrokerCmd::ComputeSectorRotationSnapshot {
                    symbol: sym,
                    symbol_sector,
                });
        }

        // UPDM — Upgrade/Downgrade Momentum
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "UPDM — Upgrade/Downgrade Momentum",
                default_size: [560.0, 420.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_updm,
            &mut self.updm_symbol,
            &mut self.updm_loading,
            &mut self.updm_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_updm(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeUpdmSnapshot { symbol },
            super::render::render_updm_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
