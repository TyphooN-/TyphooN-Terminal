use super::*;

impl TyphooNApp {
    pub(super) fn render_research_factor_quality_credit_models_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research = research_chart_symbol(
            self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
        );

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
                        ui.add(egui::TextEdit::singleline(&mut self.mom_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.mom_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.mom_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_momentum(&conn, &sym_u) {
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
                            let _ = self.broker_tx.send(BrokerCmd::ComputeMomentumSnapshot { symbol: sym });
                        }
                        if self.mom_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.mom_snapshot;
                    if snap.symbol.is_empty() || snap.bars_used == 0 {
                        ui.label(egui::RichText::new("No data — ensure HP bars are cached for this symbol, then click Compute.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.regime_label.as_str() {
                            "STRONG" => UP,
                            "CRASH" | "WEAK" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — trend: {} — bars: {} — as of {}",
                            snap.symbol, snap.regime_label, snap.trend_label, snap.bars_used, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("mom_grid").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            let pct_row = |ui: &mut egui::Ui, label: &str, val: f64| {
                                ui.label(egui::RichText::new(label).small().strong());
                                let c = if val > 0.0 { UP } else if val < 0.0 { DOWN } else { AXIS_TEXT };
                                ui.label(egui::RichText::new(format!("{:+.2}%", val)).small().monospace().color(c));
                                ui.end_row();
                            };
                            pct_row(ui, "Return 1m",    snap.return_1m_pct);
                            pct_row(ui, "Return 3m",    snap.return_3m_pct);
                            pct_row(ui, "Return 6m",    snap.return_6m_pct);
                            pct_row(ui, "Return 12m",   snap.return_12m_pct);
                            pct_row(ui, "Return 12-1",  snap.return_12_1_pct);
                            ui.label(egui::RichText::new("Annualized vol").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}%", snap.vol_annualized_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Vol-adjusted score").small().strong());
                            let cv = if snap.vol_adjusted_score > 0.0 { UP } else if snap.vol_adjusted_score < 0.0 { DOWN } else { AXIS_TEXT };
                            ui.label(egui::RichText::new(format!("{:+.3}", snap.vol_adjusted_score)).small().monospace().color(cv));
                            ui.end_row();
                            ui.label(egui::RichText::new("Composite score").small().strong());
                            ui.label(egui::RichText::new(format!("{:.1} / 100", snap.composite_score)).small().monospace().color(color));
                            ui.end_row();
                        });
                    }
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
                        ui.add(egui::TextEdit::singleline(&mut self.liq_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.liq_symbol = chart_sym_research.clone(); }
                        ui.label(egui::RichText::new("Window days:").color(AXIS_TEXT).small());
                        ui.add(egui::DragValue::new(&mut self.liq_window_days).range(10..=252).speed(1));
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.liq_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_liquidity(&conn, &sym_u) {
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
                                    if let Ok(Some(prof)) = typhoon_engine::core::research::get_profile(&conn, &sym) {
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
                    ui.separator();
                    let snap = &self.liq_snapshot;
                    if snap.symbol.is_empty() || snap.window_days == 0 {
                        ui.label(egui::RichText::new("No data — ensure HP bars are cached for this symbol, then click Compute.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.liquidity_tier.as_str() {
                            "DEEP" | "LIQUID" => UP,
                            "THIN" | "ILLIQUID" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — window: {}d — as of {}",
                            snap.symbol, snap.liquidity_tier, snap.window_days, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("liq_grid").striped(true).num_columns(2).min_col_width(240.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Avg daily share volume").small().strong());
                            ui.label(egui::RichText::new(format!("{:>15.0}", snap.avg_daily_share_volume)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Median daily share volume").small().strong());
                            ui.label(egui::RichText::new(format!("{:>15.0}", snap.median_daily_share_volume)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Avg daily dollar volume").small().strong());
                            ui.label(egui::RichText::new(format!("${:>14.0}", snap.avg_daily_dollar_volume)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Median daily dollar volume").small().strong());
                            ui.label(egui::RichText::new(format!("${:>14.0}", snap.median_daily_dollar_volume)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Shares outstanding").small().strong());
                            ui.label(egui::RichText::new(format!("{:>15.0}", snap.shares_outstanding)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Daily turnover").small().strong());
                            ui.label(egui::RichText::new(format!("{:.3}%", snap.daily_turnover_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Amihud illiquidity ×1e6").small().strong());
                            ui.label(egui::RichText::new(format!("{:.4}", snap.amihud_illiquidity)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Avg true range").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}%", snap.avg_true_range_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Spread proxy (Corwin-Schultz)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.3}%", snap.spread_proxy_pct)).small().monospace());
                            ui.end_row();
                        });
                    }
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
                        ui.add(egui::TextEdit::singleline(&mut self.break_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.break_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.break_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_breakout(&conn, &sym_u) {
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
                            let _ = self.broker_tx.send(BrokerCmd::ComputeBreakoutSnapshot { symbol: sym });
                        }
                        if self.break_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.break_snapshot;
                    if snap.symbol.is_empty() || snap.current_price <= 0.0 {
                        ui.label(egui::RichText::new("No data — ensure HP bars are cached for this symbol, then click Compute.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.breakout_label.as_str() {
                            "NEW_HIGH" => UP,
                            "NEW_LOW" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — setup: {} — last: {:.2} — as of {}",
                            snap.symbol, snap.breakout_label, snap.setup_label, snap.current_price, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("break_grid").striped(true).num_columns(4).spacing([14.0, 3.0]).show(ui, |ui| {
                            ui.label(egui::RichText::new("Window").strong().small());
                            ui.label(egui::RichText::new("High").strong().small().color(UP));
                            ui.label(egui::RichText::new("Low").strong().small().color(DOWN));
                            ui.label(egui::RichText::new("Pos in range").strong().small());
                            ui.end_row();
                            ui.label(egui::RichText::new("20d").monospace().small());
                            ui.label(egui::RichText::new(format!("{:.2}", snap.high_20d)).monospace().small());
                            ui.label(egui::RichText::new(format!("{:.2}", snap.low_20d)).monospace().small());
                            ui.label(egui::RichText::new(format!("{:.0}%", snap.position_in_20d_range_pct)).monospace().small());
                            ui.end_row();
                            ui.label(egui::RichText::new("60d").monospace().small());
                            ui.label(egui::RichText::new(format!("{:.2}", snap.high_60d)).monospace().small());
                            ui.label(egui::RichText::new(format!("{:.2}", snap.low_60d)).monospace().small());
                            ui.label(egui::RichText::new("").small());
                            ui.end_row();
                            ui.label(egui::RichText::new("52w").monospace().small());
                            ui.label(egui::RichText::new(format!("{:.2}", snap.high_52w)).monospace().small());
                            ui.label(egui::RichText::new(format!("{:.2}", snap.low_52w)).monospace().small());
                            ui.label(egui::RichText::new(format!("{:.0}%", snap.position_in_52w_range_pct)).monospace().small());
                            ui.end_row();
                        });
                        ui.separator();
                        egui::Grid::new("break_sub").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            let dist_row = |ui: &mut egui::Ui, label: &str, val: f64| {
                                ui.label(egui::RichText::new(label).small().strong());
                                let c = if val >= 0.0 { UP } else { DOWN };
                                ui.label(egui::RichText::new(format!("{:+.2}%", val)).small().monospace().color(c));
                                ui.end_row();
                            };
                            dist_row(ui, "Distance from 52w high",  snap.dist_from_52w_high_pct);
                            dist_row(ui, "Distance from 52w low",   snap.dist_from_52w_low_pct);
                            dist_row(ui, "Distance from 20d high",  snap.dist_from_20d_high_pct);
                            dist_row(ui, "Distance from 60d high",  snap.dist_from_60d_high_pct);
                            ui.label(egui::RichText::new("Consolidation (20d range/mean)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}%", snap.consolidation_pct)).small().monospace());
                            ui.end_row();
                        });
                    }
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
                    ui.separator();
                    let snap = &self.ccrl_snapshot;
                    if snap.symbol.is_empty() || snap.periods_used == 0 {
                        ui.label(
                            egui::RichText::new(
                                "No data — run FA for this symbol, then click Compute.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.efficiency_label.as_str() {
                            "EFFICIENT" => UP,
                            "INEFFICIENT" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — trend: {} — latest: {} — CCC {:.1}d — as of {}",
                                snap.symbol,
                                snap.efficiency_label,
                                snap.trend_label,
                                snap.latest_period,
                                snap.ccc_days,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("ccrl_sub")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("DSO (days sales outstanding)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.1} days", snap.dso_days))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("DIO (days inventory outstanding)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.1} days", snap.dio_days))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("DPO (days payables outstanding)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.1} days", snap.dpo_days))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Prior CCC").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.1} days", snap.prior_ccc_days))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("CCC change vs prior").small().strong(),
                                );
                                let cc = if snap.ccc_change_days < 0.0 {
                                    UP
                                } else if snap.ccc_change_days > 0.0 {
                                    DOWN
                                } else {
                                    AXIS_TEXT
                                };
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.1} days",
                                        snap.ccc_change_days
                                    ))
                                    .small()
                                    .monospace()
                                    .color(cc),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("3y avg CCC").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.1} days",
                                        snap.ccc_3y_avg_days
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                            });
                        if !snap.periods.is_empty() {
                            ui.separator();
                            ui.label(
                                egui::RichText::new("Per-period history")
                                    .strong()
                                    .small()
                                    .color(AXIS_TEXT),
                            );
                            egui::Grid::new("ccrl_grid")
                                .striped(true)
                                .num_columns(5)
                                .spacing([14.0, 3.0])
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new("Period").strong().small());
                                    ui.label(egui::RichText::new("DSO").strong().small());
                                    ui.label(egui::RichText::new("DIO").strong().small());
                                    ui.label(egui::RichText::new("DPO").strong().small());
                                    ui.label(egui::RichText::new("CCC").strong().small());
                                    ui.end_row();
                                    for row in &snap.periods {
                                        ui.label(
                                            egui::RichText::new(&row.period).monospace().small(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{:.0}", row.dso_days))
                                                .monospace()
                                                .small(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{:.0}", row.dio_days))
                                                .monospace()
                                                .small(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{:.0}", row.dpo_days))
                                                .monospace()
                                                .small(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{:.0}", row.ccc_days))
                                                .monospace()
                                                .small(),
                                        );
                                        ui.end_row();
                                    }
                                });
                        }
                    }
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
                        ui.add(egui::TextEdit::singleline(&mut self.credit_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.credit_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.credit_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_credit(&conn, &sym_u) {
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
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCreditSnapshot { symbol: sym });
                        }
                        if self.credit_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.credit_snapshot;
                    if snap.symbol.is_empty() || snap.inputs_available == 0 {
                        ui.label(egui::RichText::new("No data — run ALTZ, PTFS, LEV and ACRL for this symbol, then click Compute.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.letter_grade.as_str() {
                            "AAA" | "AA" | "A" | "BBB" => UP,
                            "CCC" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — {} — composite: {:.1} / 100 — as of {}",
                            snap.symbol, snap.letter_grade, snap.credit_label,
                            snap.composite_score, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("credit_sub").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Altman Z").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2} ({})", snap.altman_z, snap.altman_zone)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Piotroski score").small().strong());
                            ui.label(egui::RichText::new(format!("{}/9 ({})", snap.piotroski_score, snap.piotroski_label)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Leverage summary").small().strong());
                            ui.label(egui::RichText::new(&snap.leverage_summary).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Accruals trend").small().strong());
                            ui.label(egui::RichText::new(&snap.accruals_trend).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("TTM cash conversion").small().strong());
                            ui.label(egui::RichText::new(format!("{:.1}%", snap.accruals_ttm_cash_conversion_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Inputs available").small().strong());
                            ui.label(egui::RichText::new(format!("{} / 4", snap.inputs_available)).small().monospace());
                            ui.end_row();
                        });
                        if !snap.components.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new("Component contributions").strong().small().color(AXIS_TEXT));
                            egui::Grid::new("credit_grid").striped(true).num_columns(5).spacing([14.0, 3.0]).show(ui, |ui| {
                                ui.label(egui::RichText::new("Component").strong().small());
                                ui.label(egui::RichText::new("Value").strong().small());
                                ui.label(egui::RichText::new("Score").strong().small());
                                ui.label(egui::RichText::new("Weight").strong().small());
                                ui.label(egui::RichText::new("Contribution").strong().small());
                                ui.end_row();
                                for c in &snap.components {
                                    ui.label(egui::RichText::new(&c.name).monospace().small());
                                    ui.label(egui::RichText::new(&c.value).monospace().small());
                                    ui.label(egui::RichText::new(format!("{:.1}", c.score)).monospace().small());
                                    ui.label(egui::RichText::new(format!("{:.0}%", c.weight)).monospace().small());
                                    ui.label(egui::RichText::new(format!("{:.1}", c.contribution)).monospace().small());
                                    ui.end_row();
                                }
                            });
                        }
                    }
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
                        ui.add(egui::TextEdit::singleline(&mut self.growm_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.growm_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.growm_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_growm(&conn, &sym_u) {
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
                            let _ = self.broker_tx.send(BrokerCmd::ComputeGrowmSnapshot { symbol: sym });
                        }
                        if self.growm_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.growm_snapshot;
                    if snap.symbol.is_empty() || snap.inputs_available == 0 {
                        ui.label(egui::RichText::new("No data — run MOM, EARM and/or DIVG for this symbol first, then click Compute.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.garp_label.as_str() {
                            "GARP" | "GROWTH" => UP,
                            "SPECULATIVE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — composite: {:.1} / 100 — as of {}",
                            snap.symbol, snap.garp_label, snap.composite_score, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("growm_sub").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Momentum regime").small().strong());
                            ui.label(egui::RichText::new(format!("{} ({:.1})", snap.momentum_regime, snap.momentum_score)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Earnings trend").small().strong());
                            ui.label(egui::RichText::new(format!("{} ({:.1})", snap.earnings_label, snap.earnings_momentum_score)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Dividend CAGR 3y").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}% ({})", snap.dividend_cagr_3y_pct, snap.dividend_trend)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Inputs available").small().strong());
                            ui.label(egui::RichText::new(format!("{} / 3", snap.inputs_available)).small().monospace());
                            ui.end_row();
                        });
                        if !snap.components.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new("Component contributions").strong().small().color(AXIS_TEXT));
                            egui::Grid::new("growm_grid").striped(true).num_columns(5).spacing([14.0, 3.0]).show(ui, |ui| {
                                ui.label(egui::RichText::new("Component").strong().small());
                                ui.label(egui::RichText::new("Value").strong().small());
                                ui.label(egui::RichText::new("Score").strong().small());
                                ui.label(egui::RichText::new("Weight").strong().small());
                                ui.label(egui::RichText::new("Contribution").strong().small());
                                ui.end_row();
                                for c in &snap.components {
                                    ui.label(egui::RichText::new(&c.name).monospace().small());
                                    ui.label(egui::RichText::new(&c.value).monospace().small());
                                    ui.label(egui::RichText::new(format!("{:.1}", c.score)).monospace().small());
                                    ui.label(egui::RichText::new(format!("{:.0}%", c.weight)).monospace().small());
                                    ui.label(egui::RichText::new(format!("{:.1}", c.contribution)).monospace().small());
                                    ui.end_row();
                                }
                            });
                        }
                    }
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
                        ui.add(egui::TextEdit::singleline(&mut self.flow_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.flow_symbol = chart_sym_research.clone(); }
                        ui.label(egui::RichText::new("Window (days):").color(AXIS_TEXT));
                        ui.add(egui::DragValue::new(&mut self.flow_window_days).range(7..=365).speed(1));
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.flow_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_flow(&conn, &sym_u) {
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
                                symbol: sym, window_days: self.flow_window_days,
                            });
                        }
                        if self.flow_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.flow_snapshot;
                    if snap.symbol.is_empty() || (snap.insider_trade_count == 0 && snap.institutional_holders_tracked == 0) {
                        ui.label(egui::RichText::new("No data — run INS (insider trades) and/or HDS (institutional holders) for this symbol, then click Compute.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.flow_label.as_str() {
                            "STRONG_BUY" | "BUY" => UP,
                            "SELL" | "STRONG_SELL" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — composite: {:.1} / 100 — {}d window — as of {}",
                            snap.symbol, snap.flow_label, snap.composite_score, snap.window_days, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("flow_sub").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Insider score").small().strong());
                            ui.label(egui::RichText::new(format!("{:.1}", snap.insider_score)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Insider buys (USD)").small().strong());
                            ui.label(egui::RichText::new(format!("${:.0}", snap.insider_buy_value_usd)).small().monospace().color(UP));
                            ui.end_row();
                            ui.label(egui::RichText::new("Insider sells (USD)").small().strong());
                            ui.label(egui::RichText::new(format!("${:.0}", snap.insider_sell_value_usd)).small().monospace().color(DOWN));
                            ui.end_row();
                            ui.label(egui::RichText::new("Insider net (USD)").small().strong());
                            let nc = if snap.insider_net_value_usd > 0.0 { UP } else if snap.insider_net_value_usd < 0.0 { DOWN } else { AXIS_TEXT };
                            ui.label(egui::RichText::new(format!("${:+.0}", snap.insider_net_value_usd)).small().monospace().color(nc));
                            ui.end_row();
                            ui.label(egui::RichText::new("Insider trades / unique insiders").small().strong());
                            ui.label(egui::RichText::new(format!("{} / {}", snap.insider_trade_count, snap.unique_insiders)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Institutional score").small().strong());
                            ui.label(egui::RichText::new(format!("{:.1}", snap.institutional_score)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Institutional buyers / sellers").small().strong());
                            ui.label(egui::RichText::new(format!("{} / {}", snap.institutional_buyers, snap.institutional_sellers)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Holders tracked").small().strong());
                            ui.label(egui::RichText::new(format!("{}", snap.institutional_holders_tracked)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Inst. net ratio").small().strong());
                            ui.label(egui::RichText::new(format!("{:+.2}", snap.institutional_net_ratio)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Inst. share delta (net)").small().strong());
                            let nc2 = if snap.institutional_share_delta > 0.0 { UP } else if snap.institutional_share_delta < 0.0 { DOWN } else { AXIS_TEXT };
                            ui.label(egui::RichText::new(format!("{:+.0}", snap.institutional_share_delta)).small().monospace().color(nc2));
                            ui.end_row();
                        });
                    }
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
                        ui.add(egui::TextEdit::singleline(&mut self.regime_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.regime_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.regime_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_regime(&conn, &sym_u) {
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
                            let _ = self.broker_tx.send(BrokerCmd::ComputeRegimeSnapshot { symbol: sym });
                        }
                        if self.regime_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.regime_snapshot;
                    if snap.symbol.is_empty() || snap.inputs_available == 0 {
                        ui.label(egui::RichText::new("No data — run VOLE, TECH and/or HRA for this symbol, then click Compute.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.regime_label.as_str() {
                            "TRENDING" => UP,
                            "VOLATILE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — composite: {:.1} / 100 — as of {}",
                            snap.symbol, snap.regime_label, snap.composite_score, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("regime_sub").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Realized vol").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}% ({})", snap.realized_vol_pct, snap.vol_source)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("ADX").small().strong());
                            ui.label(egui::RichText::new(format!("{:.1} — {}", snap.adx_value, snap.trend_summary)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("1Y return").small().strong());
                            let rc = if snap.return_1y_pct > 0.0 { UP } else if snap.return_1y_pct < 0.0 { DOWN } else { AXIS_TEXT };
                            ui.label(egui::RichText::new(format!("{:+.2}%", snap.return_1y_pct)).small().monospace().color(rc));
                            ui.end_row();
                            ui.label(egui::RichText::new("Sharpe").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}", snap.sharpe_ratio)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Trend strength score").small().strong());
                            ui.label(egui::RichText::new(format!("{:.1} / 100", snap.trend_strength_score)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Volatility score").small().strong());
                            ui.label(egui::RichText::new(format!("{:.1} / 100", snap.volatility_score)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Return score").small().strong());
                            ui.label(egui::RichText::new(format!("{:.1} / 100", snap.return_score)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Inputs available").small().strong());
                            ui.label(egui::RichText::new(format!("{} / 3", snap.inputs_available)).small().monospace());
                            ui.end_row();
                        });
                    }
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
                        ui.add(egui::TextEdit::singleline(&mut self.relvol_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.relvol_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.relvol_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_relvol(&conn, &sym_u) {
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
                            let _ = self.broker_tx.send(BrokerCmd::ComputeRelvolSnapshot { symbol: sym });
                        }
                        if self.relvol_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.relvol_snapshot;
                    if snap.symbol.is_empty() || snap.activity_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — run HP (historical prices, ≥20 bars) for this symbol, then click Compute.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.activity_label.as_str() {
                            "EXTREME" | "HIGH" => UP,
                            "LOW" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — {} — {:.2}× (20d) — as of {}",
                            snap.symbol, snap.activity_label, snap.direction_label,
                            snap.rel_volume_20d, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("relvol_sub").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Current volume").small().strong());
                            ui.label(egui::RichText::new(format!("{:.0}", snap.current_volume)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Avg volume (5d / 20d / 60d)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.0} / {:.0} / {:.0}",
                                snap.avg_volume_5d, snap.avg_volume_20d, snap.avg_volume_60d)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Rel volume (5d / 20d / 60d)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}× / {:.2}× / {:.2}×",
                                snap.rel_volume_5d, snap.rel_volume_20d, snap.rel_volume_60d)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Vol trend (5d vs 20d)").small().strong());
                            let tc = if snap.volume_trend_5d_pct > 0.0 { UP } else if snap.volume_trend_5d_pct < 0.0 { DOWN } else { AXIS_TEXT };
                            ui.label(egui::RichText::new(format!("{:+.2}%", snap.volume_trend_5d_pct)).small().monospace().color(tc));
                            ui.end_row();
                            ui.label(egui::RichText::new("60d percentile").small().strong());
                            ui.label(egui::RichText::new(format!("{:.0}", snap.volume_percentile_60d)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Bars used").small().strong());
                            ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace());
                            ui.end_row();
                        });
                    }
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
                        ui.add(egui::TextEdit::singleline(&mut self.margins_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.margins_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.margins_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_margins(&conn, &sym_u) {
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
                            let _ = self.broker_tx.send(BrokerCmd::ComputeMarginsSnapshot { symbol: sym });
                        }
                        if self.margins_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.margins_snapshot;
                    if snap.symbol.is_empty() || snap.periods_used == 0 {
                        ui.label(egui::RichText::new("No data — run FA (financial statements) for this symbol, then click Compute.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.overall_trend_label.as_str() {
                            "EXPANDING" => UP,
                            "CONTRACTING" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — overall: {} — quality: {} — basis: {} — latest: {} — as of {}",
                            snap.symbol, snap.overall_trend_label, snap.quality_label,
                            snap.basis, snap.latest_period, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("margins_sub").striped(true).num_columns(4).min_col_width(140.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Metric").strong().small());
                            ui.label(egui::RichText::new("Latest").strong().small());
                            ui.label(egui::RichText::new("Prior").strong().small());
                            ui.label(egui::RichText::new("Change / Trend").strong().small());
                            ui.end_row();
                            let cc_g = if snap.gross_margin_change_pct > 0.0 { UP } else if snap.gross_margin_change_pct < 0.0 { DOWN } else { AXIS_TEXT };
                            ui.label(egui::RichText::new("Gross margin").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}%", snap.latest_gross_margin_pct)).small().monospace());
                            ui.label(egui::RichText::new(format!("{:.2}%", snap.prior_gross_margin_pct)).small().monospace());
                            ui.label(egui::RichText::new(format!("{:+.2}pp — {}",
                                snap.gross_margin_change_pct, snap.gross_trend_label)).small().monospace().color(cc_g));
                            ui.end_row();
                            let cc_o = if snap.operating_margin_change_pct > 0.0 { UP } else if snap.operating_margin_change_pct < 0.0 { DOWN } else { AXIS_TEXT };
                            ui.label(egui::RichText::new("Operating margin").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}%", snap.latest_operating_margin_pct)).small().monospace());
                            ui.label(egui::RichText::new(format!("{:.2}%", snap.prior_operating_margin_pct)).small().monospace());
                            ui.label(egui::RichText::new(format!("{:+.2}pp — {}",
                                snap.operating_margin_change_pct, snap.operating_trend_label)).small().monospace().color(cc_o));
                            ui.end_row();
                            let cc_n = if snap.net_margin_change_pct > 0.0 { UP } else if snap.net_margin_change_pct < 0.0 { DOWN } else { AXIS_TEXT };
                            ui.label(egui::RichText::new("Net margin").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}%", snap.latest_net_margin_pct)).small().monospace());
                            ui.label(egui::RichText::new(format!("{:.2}%", snap.prior_net_margin_pct)).small().monospace());
                            ui.label(egui::RichText::new(format!("{:+.2}pp — {}",
                                snap.net_margin_change_pct, snap.net_trend_label)).small().monospace().color(cc_n));
                            ui.end_row();
                        });
                        ui.separator();
                        egui::Grid::new("margins_avg").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Average gross (periods)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}%", snap.avg_gross_margin_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Average operating").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}%", snap.avg_operating_margin_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Average net").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}%", snap.avg_net_margin_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Periods used").small().strong());
                            ui.label(egui::RichText::new(format!("{}", snap.periods_used)).small().monospace());
                            ui.end_row();
                        });
                        if !snap.periods.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new("Per-period history").strong().small().color(AXIS_TEXT));
                            egui::Grid::new("margins_grid").striped(true).num_columns(4).spacing([14.0, 3.0]).show(ui, |ui| {
                                ui.label(egui::RichText::new("Period").strong().small());
                                ui.label(egui::RichText::new("Gross %").strong().small());
                                ui.label(egui::RichText::new("Op %").strong().small());
                                ui.label(egui::RichText::new("Net %").strong().small());
                                ui.end_row();
                                for row in &snap.periods {
                                    ui.label(egui::RichText::new(&row.period).monospace().small());
                                    ui.label(egui::RichText::new(format!("{:.2}", row.gross_margin_pct)).monospace().small());
                                    ui.label(egui::RichText::new(format!("{:.2}", row.operating_margin_pct)).monospace().small());
                                    ui.label(egui::RichText::new(format!("{:.2}", row.net_margin_pct)).monospace().small());
                                    ui.end_row();
                                }
                            });
                        }
                    }
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
                        ui.add(egui::TextEdit::singleline(&mut self.val_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.val_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.val_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_val(&conn, &sym_u) {
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
                            let _ = self.broker_tx.send(BrokerCmd::ComputeValSnapshot { symbol: sym });
                        }
                        if self.val_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.val_snapshot;
                    if snap.symbol.is_empty() || snap.value_label == "NO_DATA" {
                        ui.label(egui::RichText::new("No data — needs FUNDAMENTALS cached for this symbol + sector peers, ideally FCFY too.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.value_label.as_str() {
                            "DEEP_VALUE" | "VALUE" => UP,
                            "EXPENSIVE" | "PREMIUM" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — composite {:.1} — sector {} — peers {} — as of {}",
                            snap.symbol, snap.value_label, snap.composite_score,
                            if snap.sector.is_empty() { "?".to_string() } else { snap.sector.clone() },
                            snap.peers_considered, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("val_metrics").striped(true).num_columns(3).min_col_width(150.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Metric").strong().small());
                            ui.label(egui::RichText::new("Symbol").strong().small());
                            ui.label(egui::RichText::new("Sector Median").strong().small());
                            ui.end_row();
                            ui.label(egui::RichText::new("P/E").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}", snap.pe_ratio)).small().monospace());
                            ui.label(egui::RichText::new(format!("{:.2}", snap.pe_sector_median)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Forward P/E").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}", snap.forward_pe)).small().monospace());
                            ui.label(egui::RichText::new(format!("{:.2}", snap.forward_pe_sector_median)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("P/B").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}", snap.price_to_book)).small().monospace());
                            ui.label(egui::RichText::new(format!("{:.2}", snap.price_to_book_sector_median)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("P/S").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}", snap.price_to_sales)).small().monospace());
                            ui.label(egui::RichText::new(format!("{:.2}", snap.price_to_sales_sector_median)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("EV/EBITDA").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}", snap.ev_to_ebitda)).small().monospace());
                            ui.label(egui::RichText::new(format!("{:.2}", snap.ev_to_ebitda_sector_median)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("FCF Yield").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}%", snap.fcf_yield_pct)).small().monospace());
                            ui.label(egui::RichText::new(format!("{:.2}%", snap.fcf_yield_sector_median_pct)).small().monospace());
                            ui.end_row();
                        });
                        if !snap.components.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new("Component contributions").strong().small().color(AXIS_TEXT));
                            egui::Grid::new("val_comps").striped(true).num_columns(4).min_col_width(130.0).show(ui, |ui| {
                                ui.label(egui::RichText::new("Component").strong().small());
                                ui.label(egui::RichText::new("Value").strong().small());
                                ui.label(egui::RichText::new("Score").strong().small());
                                ui.label(egui::RichText::new("Weight").strong().small());
                                ui.end_row();
                                for c in &snap.components {
                                    ui.label(egui::RichText::new(&c.name).small().strong());
                                    ui.label(egui::RichText::new(&c.value).small().monospace());
                                    ui.label(egui::RichText::new(format!("{:.1}", c.score)).small().monospace());
                                    ui.label(egui::RichText::new(format!("{:.0}%", c.weight)).small().monospace());
                                    ui.end_row();
                                }
                            });
                        }
                    }
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
                        ui.add(egui::TextEdit::singleline(&mut self.qual_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.qual_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.qual_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_qual(&conn, &sym_u) {
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
                            let _ = self.broker_tx.send(BrokerCmd::ComputeQualSnapshot { symbol: sym });
                        }
                        if self.qual_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.qual_snapshot;
                    if snap.symbol.is_empty() || snap.quality_label == "NO_DATA" {
                        ui.label(egui::RichText::new("No data — needs at least one of PTFS / MARGINS / ACRL / LEV cached.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.quality_label.as_str() {
                            "HIGH_QUALITY" | "QUALITY" => UP,
                            "POOR" | "WEAK" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — composite {:.1} — as of {}",
                            snap.symbol, snap.quality_label, snap.composite_score, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("qual_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Piotroski F").small().strong());
                            ui.label(egui::RichText::new(format!("{}/9 ({})", snap.piotroski_score, snap.piotroski_label)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Operating margin").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}% ({})", snap.operating_margin_pct, snap.margin_trend_label)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Cash conversion").small().strong());
                            ui.label(egui::RichText::new(format!("{:.0}% ({})", snap.cash_conversion_pct, snap.accruals_trend_label)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Leverage").small().strong());
                            ui.label(egui::RichText::new(format!("{} — D/EBITDA {:.2}", snap.leverage_summary, snap.debt_to_ebitda)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Inputs used").small().strong());
                            ui.label(egui::RichText::new(format!("{}/4", snap.inputs_available)).small().monospace());
                            ui.end_row();
                        });
                        if !snap.components.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new("Component contributions").strong().small().color(AXIS_TEXT));
                            egui::Grid::new("qual_comps").striped(true).num_columns(4).min_col_width(130.0).show(ui, |ui| {
                                ui.label(egui::RichText::new("Component").strong().small());
                                ui.label(egui::RichText::new("Value").strong().small());
                                ui.label(egui::RichText::new("Score").strong().small());
                                ui.label(egui::RichText::new("Weight").strong().small());
                                ui.end_row();
                                for c in &snap.components {
                                    ui.label(egui::RichText::new(&c.name).small().strong());
                                    ui.label(egui::RichText::new(&c.value).small().monospace());
                                    ui.label(egui::RichText::new(format!("{:.1}", c.score)).small().monospace());
                                    ui.label(egui::RichText::new(format!("{:.0}%", c.weight)).small().monospace());
                                    ui.end_row();
                                }
                            });
                        }
                    }
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
                        ui.add(egui::TextEdit::singleline(&mut self.risk_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.risk_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.risk_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_risk(&conn, &sym_u) {
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
                            let _ = self.broker_tx.send(BrokerCmd::ComputeRiskSnapshot { symbol: sym });
                        }
                        if self.risk_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.risk_snapshot;
                    if snap.symbol.is_empty() || snap.risk_label == "NO_DATA" {
                        ui.label(egui::RichText::new("No data — needs at least one of VOLE / BETA / LIQ / SHRT / ALTZ cached.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.risk_label.as_str() {
                            "LOW_RISK" => UP,
                            "DISTRESSED" | "HIGH_RISK" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — composite {:.1} (higher = riskier) — as of {}",
                            snap.symbol, snap.risk_label, snap.composite_score, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("risk_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Realized vol").small().strong());
                            ui.label(egui::RichText::new(format!("{:.1}%", snap.realized_vol_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Beta 1Y").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}", snap.beta_1y)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Liquidity").small().strong());
                            ui.label(egui::RichText::new(&snap.liquidity_tier).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Short % float / DTC").small().strong());
                            ui.label(egui::RichText::new(format!("{:.1}% / {:.1}", snap.short_percent_of_float, snap.days_to_cover)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Altman Z").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2} ({})", snap.altman_z, snap.altman_zone)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Inputs used").small().strong());
                            ui.label(egui::RichText::new(format!("{}/5", snap.inputs_available)).small().monospace());
                            ui.end_row();
                        });
                        if !snap.components.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new("Component contributions").strong().small().color(AXIS_TEXT));
                            egui::Grid::new("risk_comps").striped(true).num_columns(4).min_col_width(130.0).show(ui, |ui| {
                                ui.label(egui::RichText::new("Component").strong().small());
                                ui.label(egui::RichText::new("Value").strong().small());
                                ui.label(egui::RichText::new("Score").strong().small());
                                ui.label(egui::RichText::new("Weight").strong().small());
                                ui.end_row();
                                for c in &snap.components {
                                    ui.label(egui::RichText::new(&c.name).small().strong());
                                    ui.label(egui::RichText::new(&c.value).small().monospace());
                                    ui.label(egui::RichText::new(format!("{:.1}", c.score)).small().monospace());
                                    ui.label(egui::RichText::new(format!("{:.0}%", c.weight)).small().monospace());
                                    ui.end_row();
                                }
                            });
                        }
                    }
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
                        ui.add(egui::TextEdit::singleline(&mut self.insstrk_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.insstrk_symbol = chart_sym_research.clone(); }
                        ui.label(egui::RichText::new("Window (days):").color(AXIS_TEXT));
                        ui.add(egui::DragValue::new(&mut self.insstrk_window_days).range(30..=720));
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.insstrk_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_insstrk(&conn, &sym_u) {
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
                            let _ = self.broker_tx.send(BrokerCmd::ComputeInsstrkSnapshot { symbol: sym, window_days: wd });
                        }
                        if self.insstrk_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.insstrk_snapshot;
                    if snap.symbol.is_empty() || snap.streak_label == "NONE" {
                        ui.label(egui::RichText::new("No insider trades in window — run INS for this symbol, then click Compute.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.streak_label.as_str() {
                            "STRONG_ACCUMULATION" | "ACCUMULATION" => UP,
                            "STRONG_DISTRIBUTION" | "DISTRIBUTION" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — {} insiders — window {}d — as of {}",
                            snap.symbol, snap.streak_label, snap.unique_insiders,
                            snap.window_days, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("insstrk_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Buy streaks / Sell streaks").small().strong());
                            ui.label(egui::RichText::new(format!("{} / {}", snap.buy_streak_count, snap.sell_streak_count)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Longest buy / sell").small().strong());
                            ui.label(egui::RichText::new(format!("{} / {}", snap.longest_buy_streak, snap.longest_sell_streak)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Net buy / sell value").small().strong());
                            ui.label(egui::RichText::new(format!("${:.0} / ${:.0}", snap.net_buy_value_usd, snap.net_sell_value_usd)).small().monospace());
                            ui.end_row();
                        });
                        if !snap.rows.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new("Per-insider streaks").strong().small().color(AXIS_TEXT));
                            egui::Grid::new("insstrk_rows").striped(true).num_columns(5).min_col_width(130.0).show(ui, |ui| {
                                ui.label(egui::RichText::new("Insider").strong().small());
                                ui.label(egui::RichText::new("Dir").strong().small());
                                ui.label(egui::RichText::new("Events").strong().small());
                                ui.label(egui::RichText::new("Net $").strong().small());
                                ui.label(egui::RichText::new("Latest").strong().small());
                                ui.end_row();
                                for r in &snap.rows {
                                    let rc = match r.streak_direction.as_str() { "BUY" => UP, "SELL" => DOWN, _ => AXIS_TEXT };
                                    ui.label(egui::RichText::new(&r.insider_name).small());
                                    ui.label(egui::RichText::new(&r.streak_direction).small().color(rc));
                                    ui.label(egui::RichText::new(format!("{}", r.consecutive_events)).small().monospace());
                                    ui.label(egui::RichText::new(format!("${:.0}", r.net_value_usd)).small().monospace());
                                    ui.label(egui::RichText::new(&r.latest_date).small().monospace());
                                    ui.end_row();
                                }
                            });
                        }
                    }
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
                        ui.add(egui::TextEdit::singleline(&mut self.covg_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.covg_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.covg_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_covg(&conn, &sym_u) {
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
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCovgSnapshot { symbol: sym });
                        }
                        if self.covg_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.covg_snapshot;
                    if snap.symbol.is_empty() || snap.coverage_label == "NONE" {
                        ui.label(egui::RichText::new("No data — needs ANR (price targets / consensus) and/or UPDG (rating changes) cached.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.coverage_label.as_str() {
                            "EXPANDING" => UP,
                            "CONTRACTING" | "THIN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — composite {:.1} — {} analysts — as of {}",
                            snap.symbol, snap.coverage_label, snap.composite_score,
                            snap.num_analysts, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("covg_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Target mean (low / high)").small().strong());
                            ui.label(egui::RichText::new(format!("${:.2} (${:.2} / ${:.2})", snap.target_mean, snap.target_low, snap.target_high)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Consensus SB/B/H/S/SS").small().strong());
                            ui.label(egui::RichText::new(format!("{}/{}/{}/{}/{}",
                                snap.consensus_strong_buy, snap.consensus_buy, snap.consensus_hold,
                                snap.consensus_sell, snap.consensus_strong_sell)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Bullish ratio").small().strong());
                            ui.label(egui::RichText::new(format!("{:.0}%", snap.consensus_bull_ratio * 100.0)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Upgrades / Downgrades 90d").small().strong());
                            let nc = if snap.net_90d > 0 { UP } else if snap.net_90d < 0 { DOWN } else { AXIS_TEXT };
                            ui.label(egui::RichText::new(format!("{} / {} (net {:+})", snap.upgrades_90d, snap.downgrades_90d, snap.net_90d)).small().monospace().color(nc));
                            ui.end_row();
                            ui.label(egui::RichText::new("Breadth / Consensus / Churn score").small().strong());
                            ui.label(egui::RichText::new(format!("{:.0} / {:.0} / {:.0}", snap.breadth_score, snap.consensus_score, snap.churn_score)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Inputs used").small().strong());
                            ui.label(egui::RichText::new(format!("{}/3", snap.inputs_available)).small().monospace());
                            ui.end_row();
                        });
                    }
                });
            self.show_covg = open;
        }
    }
}
