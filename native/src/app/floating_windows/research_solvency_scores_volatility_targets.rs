use super::*;

impl TyphooNApp {
    pub(super) fn render_research_solvency_scores_volatility_targets_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research: String = self
            .charts
            .get(self.active_tab)
            .map(|c| {
                c.symbol
                    .split(':')
                    .rev()
                    .nth(1)
                    .or_else(|| c.symbol.split(':').last())
                    .unwrap_or("AAPL")
                    .to_string()
            })
            .unwrap_or_else(|| "AAPL".to_string());

        // ── Research Godel Parity Round 11 windows ─────────────────────────────

        // ALTZ — Altman Z-Score
        if self.show_altz {
            if self.altz_symbol.is_empty() {
                self.altz_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_altz;
            egui::Window::new("ALTZ — Altman Z-Score")
                .open(&mut open)
                .resizable(true)
                .default_size([620.0, 420.0])
                .max_size([620.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.altz_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.altz_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.altz_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_altman_z(&conn, &sym_u) {
                                        self.altz_snapshot = snap;
                                        self.altz_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.altz_symbol.to_uppercase();
                            self.altz_loading = true;
                            self.altz_symbol = sym.clone();
                            let market_value_equity = if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    if let Ok(Some(fa)) = typhoon_engine::core::fundamentals::get_fundamentals(&conn, &sym) {
                                        fa.market_cap.unwrap_or(0.0)
                                    } else { 0.0 }
                                } else { 0.0 }
                            } else { 0.0 };
                            let _ = self.broker_tx.send(BrokerCmd::ComputeAltmanZSnapshot {
                                symbol: sym, market_value_equity,
                            });
                        }
                        if self.altz_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.altz_snapshot;
                    if snap.symbol.is_empty() || snap.components.is_empty() {
                        ui.label(egui::RichText::new("No data — run FA (Financials) + Fundamentals, then click Compute.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.zone.as_str() {
                            "SAFE" => UP,
                            "GRAY" => AXIS_TEXT,
                            "DISTRESS" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — Z = {:.2} — {} — as of {}",
                            snap.symbol, snap.z_score, snap.zone, snap.as_of,
                        )).strong().color(color));
                        ui.label(egui::RichText::new(format!(
                            "WC ${:.0}M · RE ${:.0}M · EBIT ${:.0}M · MVE ${:.0}M · Sales ${:.0}M · TA ${:.0}M · TL ${:.0}M",
                            snap.working_capital / 1e6, snap.retained_earnings / 1e6, snap.ebit / 1e6,
                            snap.market_value_equity / 1e6, snap.sales / 1e6,
                            snap.total_assets / 1e6, snap.total_liabilities / 1e6,
                        )).small().color(AXIS_TEXT));
                        ui.separator();
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            egui::Grid::new("altz_grid").striped(true).num_columns(5).min_col_width(80.0).show(ui, |ui| {
                                ui.label(egui::RichText::new("Component").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Ratio").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Coeff").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Contribution").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Note").color(AXIS_TEXT).small().strong());
                                ui.end_row();
                                for c in &snap.components {
                                    ui.label(egui::RichText::new(&c.name).small().monospace().strong());
                                    ui.label(egui::RichText::new(format!("{:.3}", c.ratio)).small().monospace());
                                    ui.label(egui::RichText::new(format!("{:.1}", c.coefficient)).small().monospace());
                                    ui.label(egui::RichText::new(format!("{:.3}", c.contribution)).small().monospace());
                                    ui.label(egui::RichText::new(&c.note).color(AXIS_TEXT).small().monospace());
                                    ui.end_row();
                                }
                            });
                        });
                    }
                });
            self.show_altz = open;
        }

        // PTFS — Piotroski F-Score
        if self.show_ptfs {
            if self.ptfs_symbol.is_empty() {
                self.ptfs_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ptfs;
            egui::Window::new("PTFS — Piotroski F-Score")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 480.0])
                .max_size([640.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.ptfs_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.ptfs_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ptfs_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_piotroski(&conn, &sym_u) {
                                        self.ptfs_snapshot = snap;
                                        self.ptfs_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ptfs_symbol.to_uppercase();
                            self.ptfs_loading = true;
                            self.ptfs_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputePiotroskiSnapshot { symbol: sym });
                        }
                        if self.ptfs_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.ptfs_snapshot;
                    if snap.symbol.is_empty() || snap.checks.is_empty() {
                        ui.label(egui::RichText::new("No data — run FA (Financials) with 2+ annual periods, then click Compute.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.strength_label.as_str() {
                            "STRONG" => UP,
                            "MIXED" => AXIS_TEXT,
                            "WEAK" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — F-Score {}/9 — {} — {} vs {} — as of {}",
                            snap.symbol, snap.f_score, snap.strength_label,
                            snap.current_period, snap.prior_period, snap.as_of,
                        )).strong().color(color));
                        ui.label(egui::RichText::new(format!(
                            "Profitability {}/4 · Leverage/Liquidity {}/3 · Efficiency {}/2",
                            snap.profitability_score, snap.leverage_score, snap.efficiency_score,
                        )).small().color(AXIS_TEXT));
                        ui.separator();
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            egui::Grid::new("ptfs_grid").striped(true).num_columns(5).min_col_width(80.0).show(ui, |ui| {
                                ui.label(egui::RichText::new("Category").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Check").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Passed").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Current").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Prior").color(AXIS_TEXT).small().strong());
                                ui.end_row();
                                for c in &snap.checks {
                                    let check_color = if c.passed { UP } else { DOWN };
                                    let check_text = if c.passed { "PASS" } else { "FAIL" };
                                    ui.label(egui::RichText::new(&c.category).small().monospace());
                                    ui.label(egui::RichText::new(&c.name).small().monospace().strong());
                                    ui.label(egui::RichText::new(check_text).color(check_color).small().monospace().strong());
                                    ui.label(egui::RichText::new(format!("{:.2}", c.value_current)).small().monospace());
                                    ui.label(egui::RichText::new(format!("{:.2}", c.value_prior)).small().monospace());
                                    ui.end_row();
                                }
                            });
                        });
                    }
                });
            self.show_ptfs = open;
        }

        // VOLE — OHLC Volatility Estimators
        if self.show_vole {
            if self.vole_symbol.is_empty() {
                self.vole_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_vole;
            egui::Window::new("VOLE — OHLC Volatility Estimators")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 360.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.vole_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.vole_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.vole_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ohlc_vol(&conn, &sym_u)
                                    {
                                        self.vole_snapshot = snap;
                                        self.vole_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.vole_symbol.to_uppercase();
                            self.vole_loading = true;
                            self.vole_symbol = sym.clone();
                            let bars_json = if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let mut bars: Vec<
                                        typhoon_engine::core::research::HistoricalPriceRow,
                                    > = typhoon_engine::core::research::get_historical_price(
                                        &conn, &sym,
                                    )
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default();
                                    if bars.len() >= 2 && bars[0].date > bars[bars.len() - 1].date {
                                        bars.reverse();
                                    }
                                    serde_json::to_string(&bars).unwrap_or_default()
                                } else {
                                    String::new()
                                }
                            } else {
                                String::new()
                            };
                            let _ = self.broker_tx.send(BrokerCmd::ComputeOhlcVolSnapshot {
                                symbol: sym,
                                window_days: 60,
                                bars_json,
                            });
                        }
                        if self.vole_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.vole_snapshot;
                    if snap.symbol.is_empty() || snap.estimators.is_empty() {
                        ui.label(
                            egui::RichText::new(
                                "No data — run HP for this symbol, then click Compute.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — preferred {} = {:.2}% · {} trading days · as of {}",
                                snap.symbol,
                                snap.preferred_label,
                                snap.preferred_estimate_pct,
                                snap.trading_days,
                                snap.as_of,
                            ))
                            .strong()
                            .color(AXIS_TEXT),
                        );
                        ui.separator();
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            egui::Grid::new("vole_grid")
                                .striped(true)
                                .num_columns(4)
                                .min_col_width(100.0)
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new("Estimator")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Annualized %")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Efficiency vs CtC")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Note")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.end_row();
                                    for e in &snap.estimators {
                                        ui.label(
                                            egui::RichText::new(&e.name)
                                                .small()
                                                .monospace()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{:.2}",
                                                e.annualized_vol_pct
                                            ))
                                            .small()
                                            .monospace(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{:.2}x",
                                                e.efficiency_vs_close
                                            ))
                                            .small()
                                            .monospace(),
                                        );
                                        ui.label(
                                            egui::RichText::new(&e.note)
                                                .color(AXIS_TEXT)
                                                .small()
                                                .monospace(),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });
                    }
                });
            self.show_vole = open;
        }

        // EPSB — EPS Beat Streak & Surprise
        if self.show_epsb {
            if self.epsb_symbol.is_empty() {
                self.epsb_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_epsb;
            egui::Window::new("EPSB — EPS Beat Streak")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.epsb_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.epsb_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.epsb_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_eps_beat(&conn, &sym_u) {
                                        self.epsb_snapshot = snap;
                                        self.epsb_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.epsb_symbol.to_uppercase();
                            self.epsb_loading = true;
                            self.epsb_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeEpsBeatSnapshot { symbol: sym });
                        }
                        if self.epsb_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.epsb_snapshot;
                    if snap.symbol.is_empty() || snap.total_reports == 0 {
                        ui.label(egui::RichText::new("No data — run earnings surprise fetch (ERN) for this symbol, then click Compute.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.bias_label.as_str() {
                            "POSITIVE" => UP,
                            "NEUTRAL" => AXIS_TEXT,
                            "NEGATIVE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} · {} · beat rate {:.0}% · streak {:+} — as of {}",
                            snap.symbol, snap.bias_label, snap.trend_label,
                            snap.beat_rate_pct, snap.current_streak, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("epsb_grid").striped(true).num_columns(2).min_col_width(160.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Total reports").small().strong());
                            ui.label(egui::RichText::new(format!("{}", snap.total_reports)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Beats / Misses / Inlines").small().strong());
                            ui.label(egui::RichText::new(format!("{} / {} / {}", snap.beats, snap.misses, snap.inlines)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Longest beat streak").small().strong());
                            ui.label(egui::RichText::new(format!("{}", snap.longest_beat_streak)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Longest miss streak").small().strong());
                            ui.label(egui::RichText::new(format!("{}", snap.longest_miss_streak)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Avg surprise %").small().strong());
                            ui.label(egui::RichText::new(format!("{:+.2}%", snap.avg_surprise_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Median surprise %").small().strong());
                            ui.label(egui::RichText::new(format!("{:+.2}%", snap.median_surprise_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Recent-4 avg %").small().strong());
                            ui.label(egui::RichText::new(format!("{:+.2}%", snap.recent_avg_surprise_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Latest report").small().strong());
                            ui.label(egui::RichText::new(format!("{} ({:+.2}%)", snap.latest_date, snap.latest_surprise_pct)).small().monospace());
                            ui.end_row();
                        });
                    }
                });
            self.show_epsb = open;
        }

        // PTD — Price Target Dispersion & Implied Return
        if self.show_ptd {
            if self.ptd_symbol.is_empty() {
                self.ptd_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ptd;
            egui::Window::new("PTD — Price Target Dispersion")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ptd_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ptd_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ptd_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_price_target_dispersion(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.ptd_snapshot = snap;
                                        self.ptd_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ptd_symbol.to_uppercase();
                            self.ptd_loading = true;
                            self.ptd_symbol = sym.clone();
                            let current_price = if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    if let Ok(Some(fa)) =
                                        typhoon_engine::core::fundamentals::get_fundamentals(
                                            &conn, &sym,
                                        )
                                    {
                                        fa.stock_price.unwrap_or(0.0)
                                    } else {
                                        0.0
                                    }
                                } else {
                                    0.0
                                }
                            } else {
                                0.0
                            };
                            let _ = self.broker_tx.send(
                                BrokerCmd::ComputePriceTargetDispersionSnapshot {
                                    symbol: sym,
                                    current_price,
                                },
                            );
                        }
                        if self.ptd_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.ptd_snapshot;
                    if snap.symbol.is_empty() || snap.num_analysts <= 0 {
                        ui.label(
                            egui::RichText::new(
                                "No data — run UPDG / PT for this symbol, then click Compute.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.consensus_label.as_str() {
                            "BULLISH" => UP,
                            "NEUTRAL" => AXIS_TEXT,
                            "BEARISH" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — {} analysts — as of {}",
                                snap.symbol, snap.consensus_label, snap.num_analysts, snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("ptd_grid")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(180.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Current price").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("${:.2}", snap.current_price))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Target high / low").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "${:.2} / ${:.2}",
                                        snap.target_high, snap.target_low
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Target mean / median").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "${:.2} / ${:.2}",
                                        snap.target_mean, snap.target_median
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Dispersion %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.1}%", snap.dispersion_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Spread % (vs current)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.1}%", snap.spread_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Implied return (median)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.1}%",
                                        snap.implied_return_median_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Implied return (mean)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.1}%",
                                        snap.implied_return_mean_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Upside to high / Downside to low")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.1}% / {:+.1}%",
                                        snap.upside_to_high_pct, snap.downside_to_low_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_ptd = open;
        }
    }
}
