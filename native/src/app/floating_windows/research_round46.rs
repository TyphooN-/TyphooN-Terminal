use super::*;

impl TyphooNApp {
    pub(super) fn render_research_round46_windows(&mut self, ctx: &egui::Context) {
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

        // ── Research Round 46 windows ──
        if self.show_ppo_win {
            if self.ppo_win_symbol.is_empty() {
                self.ppo_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ppo_win;
            egui::Window::new("PPO — Percentage Price Oscillator (12/26/9)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ppo_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ppo_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ppo_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ppo(&conn, &sym_u)
                                    {
                                        self.ppo_win_snapshot = snap;
                                        self.ppo_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ppo_win_symbol.to_uppercase();
                            self.ppo_win_loading = true;
                            self.ppo_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputePpoSnapshot { symbol: sym });
                        }
                        if self.ppo_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.ppo_win_snapshot;
                    if snap.symbol.is_empty() || snap.ppo_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥37 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.ppo_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — PPO {:+.3} — signal {:+.3} — hist {:+.3} — as of {}",
                                snap.symbol,
                                snap.ppo_label,
                                snap.ppo_value,
                                snap.signal_value,
                                snap.histogram,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("ppo_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(200.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Bars used").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.bars_used))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Fast / slow / signal").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {} / {}",
                                        snap.fast_period, snap.slow_period, snap.signal_period
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("EMA fast").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.ema_fast))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("EMA slow").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.ema_slow))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("PPO").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.ppo_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Signal").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.signal_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Histogram").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.histogram))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Last close").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.last_close))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_ppo_win = open;
        }

        if self.show_dpo_win {
            if self.dpo_win_symbol.is_empty() {
                self.dpo_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_dpo_win;
            egui::Window::new("DPO — Detrended Price Oscillator (20)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.dpo_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.dpo_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.dpo_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_dpo(&conn, &sym_u)
                                    {
                                        self.dpo_win_snapshot = snap;
                                        self.dpo_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.dpo_win_symbol.to_uppercase();
                            self.dpo_win_loading = true;
                            self.dpo_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeDpoSnapshot { symbol: sym });
                        }
                        if self.dpo_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.dpo_win_snapshot;
                    if snap.symbol.is_empty() || snap.dpo_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥32 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.dpo_label.as_str() {
                            "PEAK_HIGH" | "BULL" => UP,
                            "PEAK_LOW" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — DPO {:+.4} ({:+.2}%) — as of {}",
                                snap.symbol,
                                snap.dpo_label,
                                snap.dpo_value,
                                snap.dpo_pct,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("dpo_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(200.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Bars used").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.bars_used))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Period / shift").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {}",
                                        snap.period, snap.shift
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("SMA value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sma_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("DPO").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.dpo_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("DPO %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}%", snap.dpo_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Last close").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.last_close))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_dpo_win = open;
        }

        if self.show_kst_win {
            if self.kst_win_symbol.is_empty() {
                self.kst_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_kst_win;
            egui::Window::new("KST — Know Sure Thing (Pring, 1992)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.kst_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.kst_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.kst_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_kst(&conn, &sym_u)
                                    {
                                        self.kst_win_snapshot = snap;
                                        self.kst_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.kst_win_symbol.to_uppercase();
                            self.kst_win_loading = true;
                            self.kst_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeKstSnapshot { symbol: sym });
                        }
                        if self.kst_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.kst_win_snapshot;
                    if snap.symbol.is_empty() || snap.kst_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥56 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.kst_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — KST {:+.3} — signal {:+.3} — hist {:+.3} — as of {}",
                                snap.symbol,
                                snap.kst_label,
                                snap.kst_value,
                                snap.signal_value,
                                snap.histogram,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("kst_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(200.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Bars used").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.bars_used))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("RCMA1 (ROC10/SMA10)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.rcma1))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("RCMA2 (ROC15/SMA10)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.rcma2))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("RCMA3 (ROC20/SMA10)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.rcma3))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("RCMA4 (ROC30/SMA15)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.rcma4))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("KST").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.kst_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Signal (SMA-9)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.signal_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Histogram").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.histogram))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Last close").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.last_close))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_kst_win = open;
        }

        if self.show_ultosc_win {
            if self.ultosc_win_symbol.is_empty() {
                self.ultosc_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ultosc_win;
            egui::Window::new("ULTOSC — Ultimate Oscillator (7/14/28)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ultosc_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ultosc_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ultosc_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ultosc(&conn, &sym_u)
                                    {
                                        self.ultosc_win_snapshot = snap;
                                        self.ultosc_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ultosc_win_symbol.to_uppercase();
                            self.ultosc_win_loading = true;
                            self.ultosc_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeUltoscSnapshot { symbol: sym });
                        }
                        if self.ultosc_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.ultosc_win_snapshot;
                    if snap.symbol.is_empty() || snap.ultosc_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.ultosc_label.as_str() {
                            "OVERBOUGHT" | "BULL" => UP,
                            "OVERSOLD" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — UO {:.2} — as of {}",
                                snap.symbol, snap.ultosc_label, snap.ultosc_value, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("ultosc_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(200.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Bars used").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.bars_used))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Periods").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {} / {}",
                                        snap.period_short, snap.period_mid, snap.period_long
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Avg short (7)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.avg_short))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Avg mid (14)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.avg_mid))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Avg long (28)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.avg_long))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Ultimate Osc").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.ultosc_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Last close").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.last_close))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_ultosc_win = open;
        }

        if self.show_willr_win {
            if self.willr_win_symbol.is_empty() {
                self.willr_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_willr_win;
            egui::Window::new("WILLR — Williams %R (14)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.willr_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.willr_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.willr_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_willr(&conn, &sym_u)
                                    {
                                        self.willr_win_snapshot = snap;
                                        self.willr_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.willr_win_symbol.to_uppercase();
                            self.willr_win_loading = true;
                            self.willr_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeWillrSnapshot { symbol: sym });
                        }
                        if self.willr_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.willr_win_snapshot;
                    if snap.symbol.is_empty() || snap.willr_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥15 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.willr_label.as_str() {
                            "OVERBOUGHT" | "BULL" => UP,
                            "OVERSOLD" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — %R {:.2} — as of {}",
                                snap.symbol, snap.willr_label, snap.willr_value, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("willr_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(200.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Bars used").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.bars_used))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Period").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.period))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Highest high").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.highest_high))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Lowest low").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.lowest_low))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("%R").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.willr_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Last close").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.last_close))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_willr_win = open;
        }
    }
}
