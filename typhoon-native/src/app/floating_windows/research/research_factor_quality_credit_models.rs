use super::*;

impl TyphooNApp {
    pub(super) fn render_research_factor_quality_credit_models_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // MOM — 12-1 Month Momentum Score
        if self.show_mom {
            if self.mom_symbol.is_empty() {
                self.mom_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_mom;
            egui::Window::new("MOM — 12-1 Month Momentum Score")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.mom_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.mom_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.mom_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_momentum(&conn, &sym_u)
                                    {
                                        self.mom_snapshot = snap;
                                        self.mom_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.mom_symbol.to_uppercase();
                            self.mom_loading = true;
                            self.mom_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMomentumSnapshot { symbol: sym });
                        }
                        if self.mom_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_momentum_snapshot(ui, &self.mom_snapshot);
                });
            self.show_mom = open;
        }

        // LIQ — Liquidity Profile
        if self.show_liq {
            if self.liq_symbol.is_empty() {
                self.liq_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_liq;
            egui::Window::new("LIQ — Liquidity Profile")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.liq_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.liq_symbol = chart_sym_research.clone();
                        }
                        ui.label(egui::RichText::new("Window days:").color(AXIS_TEXT).small());
                        ui.add(
                            egui::DragValue::new(&mut self.liq_window_days)
                                .range(10..=252)
                                .speed(1),
                        );
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.liq_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_liquidity(&conn, &sym_u)
                                    {
                                        self.liq_snapshot = snap;
                                        self.liq_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.liq_symbol.to_uppercase();
                            self.liq_loading = true;
                            self.liq_symbol = sym.clone();
                            // Pre-read shares outstanding from cached Fundamentals so the
                            // broker thread can stay Send-safe without reaching back into SQLite.
                            let mut shares_outstanding = 0.0_f64;
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    if let Ok(Some(prof)) =
                                        typhoon_engine::core::research::get_profile(&conn, &sym)
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
                        if self.liq_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_liq_snapshot(ui, &self.liq_snapshot);
                });
            self.show_liq = open;
        }

        // BREAK — Breakout Proximity
        if self.show_break {
            if self.break_symbol.is_empty() {
                self.break_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_break;
            egui::Window::new("BREAK — Breakout Proximity")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.break_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.break_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.break_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_breakout(&conn, &sym_u)
                                    {
                                        self.break_snapshot = snap;
                                        self.break_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.break_symbol.to_uppercase();
                            self.break_loading = true;
                            self.break_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeBreakoutSnapshot { symbol: sym });
                        }
                        if self.break_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_break_snapshot(ui, &self.break_snapshot);
                });
            self.show_break = open;
        }

        // CCRL — Cash Conversion Cycle
        if self.show_ccrl {
            if self.ccrl_symbol.is_empty() {
                self.ccrl_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ccrl;
            egui::Window::new("CCRL — Cash Conversion Cycle")
                .open(&mut open)
                .resizable(true)
                .default_size([620.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ccrl_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ccrl_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ccrl_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_cash_cycle(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.ccrl_snapshot = snap;
                                        self.ccrl_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ccrl_symbol.to_uppercase();
                            self.ccrl_loading = true;
                            self.ccrl_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCashCycleSnapshot { symbol: sym });
                        }
                        if self.ccrl_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_ccrl_snapshot(ui, &self.ccrl_snapshot);
                });
            self.show_ccrl = open;
        }

        // CREDIT — Unified Credit Score
        if self.show_credit {
            if self.credit_symbol.is_empty() {
                self.credit_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_credit;
            egui::Window::new("CREDIT — Unified Credit Score")
                .open(&mut open)
                .resizable(true)
                .default_size([620.0, 460.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.credit_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.credit_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.credit_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_credit(&conn, &sym_u)
                                    {
                                        self.credit_snapshot = snap;
                                        self.credit_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.credit_symbol.to_uppercase();
                            self.credit_loading = true;
                            self.credit_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCreditSnapshot { symbol: sym });
                        }
                        if self.credit_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_credit_snapshot(ui, &self.credit_snapshot);
                });
            self.show_credit = open;
        }

        // GROWM — Growth at a Reasonable Price (GARP) composite
        if self.show_growm {
            if self.growm_symbol.is_empty() {
                self.growm_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_growm;
            egui::Window::new("GROWM — GARP Composite (MOM + EARM + DIVG)")
                .open(&mut open)
                .resizable(true)
                .default_size([620.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.growm_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.growm_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.growm_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_growm(&conn, &sym_u)
                                    {
                                        self.growm_snapshot = snap;
                                        self.growm_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.growm_symbol.to_uppercase();
                            self.growm_loading = true;
                            self.growm_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeGrowmSnapshot { symbol: sym });
                        }
                        if self.growm_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_growm_snapshot(ui, &self.growm_snapshot);
                });
            self.show_growm = open;
        }

        // FLOW — Insider + Institutional flow score
        if self.show_flow {
            if self.flow_symbol.is_empty() {
                self.flow_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_flow;
            egui::Window::new("FLOW — Insider + Institutional Flow")
                .open(&mut open)
                .resizable(true)
                .default_size([620.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.flow_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.flow_symbol = chart_sym_research.clone();
                        }
                        ui.label(egui::RichText::new("Window (days):").color(AXIS_TEXT));
                        ui.add(
                            egui::DragValue::new(&mut self.flow_window_days)
                                .range(7..=365)
                                .speed(1),
                        );
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.flow_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_flow(&conn, &sym_u)
                                    {
                                        self.flow_snapshot = snap;
                                        self.flow_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.flow_symbol.to_uppercase();
                            self.flow_loading = true;
                            self.flow_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeFlowSnapshot {
                                symbol: sym,
                                window_days: self.flow_window_days,
                            });
                        }
                        if self.flow_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_flow_snapshot(ui, &self.flow_snapshot);
                });
            self.show_flow = open;
        }

        // REGIME — Market regime classifier
        if self.show_regime {
            if self.regime_symbol.is_empty() {
                self.regime_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_regime;
            egui::Window::new("REGIME — Market Regime Classifier (VOLE + TECH + HRA)")
                .open(&mut open)
                .resizable(true)
                .default_size([600.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.regime_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.regime_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.regime_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_regime(&conn, &sym_u)
                                    {
                                        self.regime_snapshot = snap;
                                        self.regime_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.regime_symbol.to_uppercase();
                            self.regime_loading = true;
                            self.regime_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRegimeSnapshot { symbol: sym });
                        }
                        if self.regime_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_regime_snapshot(ui, &self.regime_snapshot);
                });
            self.show_regime = open;
        }

        // RELVOL — Relative volume
        if self.show_relvol {
            if self.relvol_symbol.is_empty() {
                self.relvol_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_relvol;
            egui::Window::new("RELVOL — Relative Volume")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 360.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.relvol_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.relvol_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.relvol_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_relvol(&conn, &sym_u)
                                    {
                                        self.relvol_snapshot = snap;
                                        self.relvol_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.relvol_symbol.to_uppercase();
                            self.relvol_loading = true;
                            self.relvol_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRelvolSnapshot { symbol: sym });
                        }
                        if self.relvol_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_relvol_snapshot(ui, &self.relvol_snapshot);
                });
            self.show_relvol = open;
        }

        // MARGINS — Margin trajectory
        if self.show_margins {
            if self.margins_symbol.is_empty() {
                self.margins_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_margins;
            egui::Window::new("MARGINS — Margin Trajectory (Gross / Op / Net)")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 460.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.margins_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.margins_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.margins_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_margins(&conn, &sym_u)
                                    {
                                        self.margins_snapshot = snap;
                                        self.margins_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.margins_symbol.to_uppercase();
                            self.margins_loading = true;
                            self.margins_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMarginsSnapshot { symbol: sym });
                        }
                        if self.margins_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_margins_snapshot(ui, &self.margins_snapshot);
                });
            self.show_margins = open;
        }

        // VAL — Value-factor composite vs sector peers
        if self.show_val {
            if self.val_symbol.is_empty() {
                self.val_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_val;
            egui::Window::new("VAL — Value-Factor Composite")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 460.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.val_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.val_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.val_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_val(&conn, &sym_u)
                                    {
                                        self.val_snapshot = snap;
                                        self.val_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.val_symbol.to_uppercase();
                            self.val_loading = true;
                            self.val_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeValSnapshot { symbol: sym });
                        }
                        if self.val_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_val_snapshot(ui, &self.val_snapshot);
                });
            self.show_val = open;
        }

        // QUAL — Quality-factor composite
        if self.show_qual {
            if self.qual_symbol.is_empty() {
                self.qual_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_qual;
            egui::Window::new("QUAL — Quality-Factor Composite")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.qual_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.qual_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.qual_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_qual(&conn, &sym_u)
                                    {
                                        self.qual_snapshot = snap;
                                        self.qual_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.qual_symbol.to_uppercase();
                            self.qual_loading = true;
                            self.qual_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeQualSnapshot { symbol: sym });
                        }
                        if self.qual_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_qual_snapshot(ui, &self.qual_snapshot);
                });
            self.show_qual = open;
        }

        // RISK — Risk-factor composite
        if self.show_risk {
            if self.risk_symbol.is_empty() {
                self.risk_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_risk;
            egui::Window::new("RISK — Risk-Factor Composite")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.risk_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.risk_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.risk_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_risk(&conn, &sym_u)
                                    {
                                        self.risk_snapshot = snap;
                                        self.risk_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.risk_symbol.to_uppercase();
                            self.risk_loading = true;
                            self.risk_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRiskSnapshot { symbol: sym });
                        }
                        if self.risk_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_risk_snapshot(ui, &self.risk_snapshot);
                });
            self.show_risk = open;
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
        if self.show_covg {
            if self.covg_symbol.is_empty() {
                self.covg_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_covg;
            egui::Window::new("COVG — Analyst Coverage Breadth & Churn")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.covg_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.covg_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.covg_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_covg(&conn, &sym_u)
                                    {
                                        self.covg_snapshot = snap;
                                        self.covg_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.covg_symbol.to_uppercase();
                            self.covg_loading = true;
                            self.covg_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCovgSnapshot { symbol: sym });
                        }
                        if self.covg_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_covg_snapshot(ui, &self.covg_snapshot);
                });
            self.show_covg = open;
        }
    }
}
