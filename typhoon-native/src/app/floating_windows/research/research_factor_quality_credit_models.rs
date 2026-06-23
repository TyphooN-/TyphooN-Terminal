use super::*;

impl TyphooNApp {
    pub(super) fn render_research_factor_quality_credit_models_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // MOM — 12-1 Month Momentum Score
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "MOM — 12-1 Month Momentum Score",
                default_size: [520.0, 380.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_mom,
            &mut self.mom_symbol,
            &mut self.mom_loading,
            &mut self.mom_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_momentum(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeMomentumSnapshot { symbol },
            super::render::render_momentum_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // LIQ — Liquidity Profile
        if let Some(sym) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "LIQ — Liquidity Profile",
                default_size: [540.0, 420.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_liq,
            &mut self.liq_symbol,
            &mut self.liq_loading,
            &mut self.liq_snapshot,
            |conn, s| {
                typhoon_engine::core::research::get_liquidity(conn, s)
                    .ok()
                    .flatten()
            },
            |symbol| symbol,
            super::render::render_liq_snapshot,
        ) {
            // Pre-read shares outstanding from cached Fundamentals so the
            // broker thread can stay Send-safe without reaching back into SQLite.
            let mut shares_outstanding = 0.0_f64;
            if let Some(ref cache) = self.cache {
                if let Ok(conn) = cache.connection() {
                    if let Ok(Some(prof)) = typhoon_engine::core::research::get_profile(&conn, &sym)
                    {
                        shares_outstanding = prof.shares_outstanding;
                    }
                }
            }
            let _ = self.broker_tx.send(BrokerCmd::ComputeLiquiditySnapshot {
                symbol: sym,
                window_days: self.liq_window_days,
                shares_outstanding,
            });
        }

        // BREAK — Breakout Proximity
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "BREAK — Breakout Proximity",
                default_size: [540.0, 420.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_break,
            &mut self.break_symbol,
            &mut self.break_loading,
            &mut self.break_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_breakout(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeBreakoutSnapshot { symbol },
            super::render::render_break_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // CCRL — Cash Conversion Cycle
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "CCRL — Cash Conversion Cycle",
                default_size: [620.0, 440.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_ccrl,
            &mut self.ccrl_symbol,
            &mut self.ccrl_loading,
            &mut self.ccrl_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_cash_cycle(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeCashCycleSnapshot { symbol },
            super::render::render_ccrl_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // CREDIT — Unified Credit Score
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "CREDIT — Unified Credit Score",
                default_size: [620.0, 460.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_credit,
            &mut self.credit_symbol,
            &mut self.credit_loading,
            &mut self.credit_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_credit(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeCreditSnapshot { symbol },
            super::render::render_credit_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // GROWM — Growth at a Reasonable Price (GARP) composite
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "GROWM — GARP Composite (MOM + EARM + DIVG)",
                default_size: [620.0, 420.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_growm,
            &mut self.growm_symbol,
            &mut self.growm_loading,
            &mut self.growm_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_growm(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeGrowmSnapshot { symbol },
            super::render::render_growm_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // FLOW — Insider + Institutional flow score
        if let Some(sym) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "FLOW — Insider + Institutional Flow",
                default_size: [620.0, 400.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_flow,
            &mut self.flow_symbol,
            &mut self.flow_loading,
            &mut self.flow_snapshot,
            |conn, s| {
                typhoon_engine::core::research::get_flow(conn, s)
                    .ok()
                    .flatten()
            },
            |symbol| symbol,
            super::render::render_flow_snapshot,
        ) {
            let _ = self.broker_tx.send(BrokerCmd::ComputeFlowSnapshot {
                symbol: sym,
                window_days: self.flow_window_days,
            });
        }

        // REGIME — Market regime classifier
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "REGIME — Market Regime Classifier (VOLE + TECH + HRA)",
                default_size: [600.0, 380.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_regime,
            &mut self.regime_symbol,
            &mut self.regime_loading,
            &mut self.regime_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_regime(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeRegimeSnapshot { symbol },
            super::render::render_regime_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // RELVOL — Relative volume
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "RELVOL — Relative Volume",
                default_size: [580.0, 360.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_relvol,
            &mut self.relvol_symbol,
            &mut self.relvol_loading,
            &mut self.relvol_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_relvol(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeRelvolSnapshot { symbol },
            super::render::render_relvol_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // MARGINS — Margin trajectory
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "MARGINS — Margin Trajectory (Gross / Op / Net)",
                default_size: [640.0, 460.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_margins,
            &mut self.margins_symbol,
            &mut self.margins_loading,
            &mut self.margins_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_margins(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeMarginsSnapshot { symbol },
            super::render::render_margins_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // VAL — Value-factor composite vs sector peers
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "VAL — Value-Factor Composite",
                default_size: [640.0, 460.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_val,
            &mut self.val_symbol,
            &mut self.val_loading,
            &mut self.val_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_val(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeValSnapshot { symbol },
            super::render::render_val_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // QUAL — Quality-factor composite
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "QUAL — Quality-Factor Composite",
                default_size: [640.0, 400.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_qual,
            &mut self.qual_symbol,
            &mut self.qual_loading,
            &mut self.qual_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_qual(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeQualSnapshot { symbol },
            super::render::render_qual_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // RISK — Risk-factor composite
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "RISK — Risk-Factor Composite",
                default_size: [640.0, 420.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_risk,
            &mut self.risk_symbol,
            &mut self.risk_loading,
            &mut self.risk_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_risk(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeRiskSnapshot { symbol },
            super::render::render_risk_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        // INSSTRK — Insider streak detector
        if self.show_insstrk {
            if self.insstrk_symbol.is_empty() {
                self.insstrk_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_insstrk;
            egui::Window::new("INSSTRK — Insider Streak Detector")
                .open(&mut open)
                .resizable(true)
                .default_size([680.0, 460.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.insstrk_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.insstrk_symbol = chart_sym_research.clone();
                        }
                        ui.label(egui::RichText::new("Window (days):").color(AXIS_TEXT));
                        ui.add(egui::DragValue::new(&mut self.insstrk_window_days).range(30..=720));
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.insstrk_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_insstrk(&conn, &sym_u)
                                    {
                                        self.insstrk_snapshot = snap;
                                        self.insstrk_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.insstrk_symbol.to_uppercase();
                            self.insstrk_loading = true;
                            self.insstrk_symbol = sym.clone();
                            let wd = self.insstrk_window_days;
                            let _ = self.broker_tx.send(BrokerCmd::ComputeInsstrkSnapshot {
                                symbol: sym,
                                window_days: wd,
                            });
                        }
                        if self.insstrk_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_insstrk_snapshot(ui, &self.insstrk_snapshot);
                });
            self.show_insstrk = open;
        }

        // COVG — Analyst coverage breadth + churn
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "COVG — Analyst Coverage Breadth & Churn",
                default_size: [640.0, 420.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_covg,
            &mut self.covg_symbol,
            &mut self.covg_loading,
            &mut self.covg_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_covg(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeCovgSnapshot { symbol },
            super::render::render_covg_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
