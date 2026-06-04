use super::*;

impl TyphooNApp {
    pub(super) fn render_research_zero_lag_elder_forecast_balance_windows(
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

        // ── Research Round 52 windows ──
        if self.show_alma_win {
            if self.alma_win_symbol.is_empty() {
                self.alma_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_alma_win;
            egui::Window::new(
                "ALMA — Arnaud Legoux Moving Average (Gaussian, length 20, σ=6, offset 0.85)",
            )
            .open(&mut open)
            .resizable(true)
            .default_size([580.0, 280.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.alma_win_symbol).desired_width(100.0),
                    );
                    if ui.button("Use Chart").clicked() {
                        self.alma_win_symbol = chart_sym_research.clone();
                    }
                    if ui.button("Load Cached").clicked() {
                        if let Some(ref cache) = self.cache {
                            if let Ok(conn) = cache.connection() {
                                let sym_u = self.alma_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) =
                                    typhoon_engine::core::research::get_alma(&conn, &sym_u)
                                {
                                    self.alma_win_snapshot = snap;
                                    self.alma_win_symbol = sym_u;
                                }
                            }
                        }
                    }
                    if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                        let sym = self.alma_win_symbol.to_uppercase();
                        self.alma_win_loading = true;
                        self.alma_win_symbol = sym.clone();
                        let _ = self
                            .broker_tx
                            .send(BrokerCmd::ComputeAlmaSnapshot { symbol: sym });
                    }
                    if self.alma_win_loading {
                        ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                    }
                });
                ui.separator();
                let snap = &self.alma_win_snapshot;
                if snap.symbol.is_empty() || snap.alma_label == "INSUFFICIENT_DATA" {
                    ui.label(
                        egui::RichText::new("No data — HP cache needs ≥21 bars with OHLC.")
                            .color(AXIS_TEXT)
                            .small(),
                    );
                } else {
                    let color = match snap.alma_label.as_str() {
                        "STRONG_BULL" | "BULL" => UP,
                        "STRONG_BEAR" | "BEAR" => DOWN,
                        _ => AXIS_TEXT,
                    };
                    ui.label(
                        egui::RichText::new(format!(
                            "{} — {} — ALMA {:.4} — close {:.4} — dev {:+.2}% — as of {}",
                            snap.symbol,
                            snap.alma_label,
                            snap.alma_value,
                            snap.last_close,
                            snap.deviation_pct,
                            snap.as_of
                        ))
                        .strong()
                        .color(color),
                    );
                    ui.separator();
                    egui::Grid::new("alma_summary")
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
                            ui.label(egui::RichText::new("Length").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{}", snap.length))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("Offset").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:.3}", snap.offset))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("Sigma").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:.2}", snap.sigma))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("ALMA").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:.4}", snap.alma_value))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("ALMA prev").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:.4}", snap.alma_prev))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("Deviation %").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:+.2}%", snap.deviation_pct))
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
            self.show_alma_win = open;
        }

        if self.show_zlema_win {
            if self.zlema_win_symbol.is_empty() {
                self.zlema_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_zlema_win;
            egui::Window::new("ZLEMA — Zero-Lag Exponential Moving Average (length 20, lag 9)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 270.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.zlema_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.zlema_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.zlema_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_zlema(&conn, &sym_u)
                                    {
                                        self.zlema_win_snapshot = snap;
                                        self.zlema_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.zlema_win_symbol.to_uppercase();
                            self.zlema_win_loading = true;
                            self.zlema_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeZlemaSnapshot { symbol: sym });
                        }
                        if self.zlema_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.zlema_win_snapshot;
                    if snap.symbol.is_empty() || snap.zlema_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥31 bars with OHLC.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.zlema_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — ZLEMA {:.4} — close {:.4} — dev {:+.2}% — as of {}",
                                snap.symbol,
                                snap.zlema_label,
                                snap.zlema_value,
                                snap.last_close,
                                snap.deviation_pct,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("zlema_summary")
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
                                ui.label(egui::RichText::new("Length").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.length))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Lag shift").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.lag_shift))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ZLEMA").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.zlema_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ZLEMA prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.zlema_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Deviation %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}%", snap.deviation_pct))
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
            self.show_zlema_win = open;
        }

        if self.show_elderray_win {
            if self.elderray_win_symbol.is_empty() {
                self.elderray_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_elderray_win;
            egui::Window::new("ELDERRAY — Elder Bull/Bear Power (EMA13)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 290.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.elderray_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.elderray_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.elderray_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_elderray(&conn, &sym_u)
                                    {
                                        self.elderray_win_snapshot = snap;
                                        self.elderray_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.elderray_win_symbol.to_uppercase();
                            self.elderray_win_loading = true;
                            self.elderray_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeElderRaySnapshot { symbol: sym });
                        }
                        if self.elderray_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.elderray_win_snapshot;
                    if snap.symbol.is_empty() || snap.elder_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥15 bars with OHLC.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.elder_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — Bull {:+.4} — Bear {:+.4} — EMA13 {:.4} — as of {}",
                                snap.symbol,
                                snap.elder_label,
                                snap.bull_power,
                                snap.bear_power,
                                snap.ema13,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("elderray_summary")
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
                                ui.label(egui::RichText::new("EMA length").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.ema_length))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("EMA13").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.ema13))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("EMA13 prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.ema13_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Bull Power (H − EMA13)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.bull_power))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Bull Power prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.bull_power_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Bear Power (L − EMA13)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.bear_power))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Bear Power prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.bear_power_prev))
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
            self.show_elderray_win = open;
        }

        if self.show_tsf_win {
            if self.tsf_win_symbol.is_empty() {
                self.tsf_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_tsf_win;
            egui::Window::new("TSF — Time Series Forecast (OLS slope projected one bar forward)")
                .open(&mut open).resizable(true).default_size([580.0, 290.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.tsf_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.tsf_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.tsf_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_tsf(&conn, &sym_u) { self.tsf_win_snapshot = snap; self.tsf_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.tsf_win_symbol.to_uppercase(); self.tsf_win_loading = true; self.tsf_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeTsfSnapshot { symbol: sym });
                        }
                        if self.tsf_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.tsf_win_snapshot;
                    if snap.symbol.is_empty() || snap.tsf_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥20 bars with OHLC.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.tsf_label.as_str() {
                            "LEADING_UP" | "LAGGING_UP" => UP,
                            "LEADING_DOWN" | "LAGGING_DOWN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — forecast {:.4} — close {:.4} — slope {:+.5} — R² {:.3} — as of {}", snap.symbol, snap.tsf_label, snap.forecast_value, snap.last_close, snap.slope, snap.r_squared, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("tsf_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Slope (per bar)").small().strong()); ui.label(egui::RichText::new(format!("{:+.5}", snap.slope)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Intercept").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.intercept)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Forecast (t+1)").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.forecast_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Forecast dev %").small().strong()); ui.label(egui::RichText::new(format!("{:+.2}%", snap.forecast_deviation_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("R²").small().strong()); ui.label(egui::RichText::new(format!("{:.3}", snap.r_squared)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                    }
                });
            self.show_tsf_win = open;
        }

        if self.show_rvi_win {
            if self.rvi_win_symbol.is_empty() {
                self.rvi_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_rvi_win;
            egui::Window::new(
                "RVI — Relative Vigor Index (Ehlers, length 10, 4-bar triangular weighting)",
            )
            .open(&mut open)
            .resizable(true)
            .default_size([580.0, 280.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.rvi_win_symbol).desired_width(100.0),
                    );
                    if ui.button("Use Chart").clicked() {
                        self.rvi_win_symbol = chart_sym_research.clone();
                    }
                    if ui.button("Load Cached").clicked() {
                        if let Some(ref cache) = self.cache {
                            if let Ok(conn) = cache.connection() {
                                let sym_u = self.rvi_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) =
                                    typhoon_engine::core::research::get_rvi(&conn, &sym_u)
                                {
                                    self.rvi_win_snapshot = snap;
                                    self.rvi_win_symbol = sym_u;
                                }
                            }
                        }
                    }
                    if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                        let sym = self.rvi_win_symbol.to_uppercase();
                        self.rvi_win_loading = true;
                        self.rvi_win_symbol = sym.clone();
                        let _ = self
                            .broker_tx
                            .send(BrokerCmd::ComputeRviSnapshot { symbol: sym });
                    }
                    if self.rvi_win_loading {
                        ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                    }
                });
                ui.separator();
                let snap = &self.rvi_win_snapshot;
                if snap.symbol.is_empty() || snap.rvi_label == "INSUFFICIENT_DATA" {
                    ui.label(
                        egui::RichText::new("No data — HP cache needs ≥17 bars with OHLC.")
                            .color(AXIS_TEXT)
                            .small(),
                    );
                } else {
                    let color = match snap.rvi_label.as_str() {
                        "BULL_CROSS" | "BULL" => UP,
                        "BEAR_CROSS" | "BEAR" => DOWN,
                        _ => AXIS_TEXT,
                    };
                    ui.label(
                        egui::RichText::new(format!(
                            "{} — {} — RVI {:+.4} — signal {:+.4} — close {:.4} — as of {}",
                            snap.symbol,
                            snap.rvi_label,
                            snap.rvi_value,
                            snap.signal_value,
                            snap.last_close,
                            snap.as_of
                        ))
                        .strong()
                        .color(color),
                    );
                    ui.separator();
                    egui::Grid::new("rvi_summary")
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
                            ui.label(egui::RichText::new("Length").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{}", snap.length))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("RVI").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:+.4}", snap.rvi_value))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("RVI prev").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:+.4}", snap.rvi_prev))
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
                            ui.label(egui::RichText::new("Signal prev").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:+.4}", snap.signal_prev))
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
            self.show_rvi_win = open;
        }

        if self.show_trima_win {
            if self.trima_win_symbol.is_empty() {
                self.trima_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_trima_win;
            egui::Window::new("TRIMA — Triangular MA (SMA-of-SMA, length 20)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.trima_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.trima_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.trima_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_trima(&conn, &sym_u)
                                    {
                                        self.trima_win_snapshot = snap;
                                        self.trima_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.trima_win_symbol.to_uppercase();
                            self.trima_win_loading = true;
                            self.trima_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeTrimaSnapshot { symbol: sym });
                        }
                        if self.trima_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.trima_win_snapshot;
                    if snap.symbol.is_empty() || snap.trima_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥31 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.trima_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — TRIMA {:.4} — close {:.4} — dev {:+.2}% — as of {}",
                                snap.symbol,
                                snap.trima_label,
                                snap.trima_value,
                                snap.last_close,
                                snap.deviation_pct,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("trima_summary")
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
                                ui.label(egui::RichText::new("Length").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.length))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("TRIMA").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.trima_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("TRIMA prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.trima_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Deviation %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}%", snap.deviation_pct))
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
            self.show_trima_win = open;
        }

        if self.show_t3_win {
            if self.t3_win_symbol.is_empty() {
                self.t3_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_t3_win;
            egui::Window::new("T3 — Tillson T3 MA (six-EMA chain, v=0.7)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.t3_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.t3_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.t3_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_t3(&conn, &sym_u)
                                    {
                                        self.t3_win_snapshot = snap;
                                        self.t3_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.t3_win_symbol.to_uppercase();
                            self.t3_win_loading = true;
                            self.t3_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeT3Snapshot { symbol: sym });
                        }
                        if self.t3_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.t3_win_snapshot;
                    if snap.symbol.is_empty() || snap.t3_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new(
                                "No data — HP cache needs ≥24 bars (6-chain warmup).",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                    } else {
                        let color = match snap.t3_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — T3 {:.4} — close {:.4} — dev {:+.2}% — as of {}",
                                snap.symbol,
                                snap.t3_label,
                                snap.t3_value,
                                snap.last_close,
                                snap.deviation_pct,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("t3_summary")
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
                                ui.label(egui::RichText::new("Length").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.length))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("v (volume factor)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.v_factor))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("T3").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.t3_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("T3 prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.t3_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Deviation %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}%", snap.deviation_pct))
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
            self.show_t3_win = open;
        }

        if self.show_vidya_win {
            if self.vidya_win_symbol.is_empty() {
                self.vidya_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_vidya_win;
            egui::Window::new("VIDYA — Chande Variable Index Dynamic Average (CMO-adaptive α)")
                .open(&mut open).resizable(true).default_size([600.0, 290.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.vidya_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.vidya_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.vidya_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_vidya(&conn, &sym_u) { self.vidya_win_snapshot = snap; self.vidya_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.vidya_win_symbol.to_uppercase(); self.vidya_win_loading = true; self.vidya_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeVidyaSnapshot { symbol: sym });
                        }
                        if self.vidya_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.vidya_win_snapshot;
                    if snap.symbol.is_empty() || snap.vidya_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥31 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.vidya_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — VIDYA {:.4} — α {:.4} — |CMO| {:.2} — close {:.4} — as of {}", snap.symbol, snap.vidya_label, snap.vidya_value, snap.current_alpha, snap.cmo_magnitude, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("vidya_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Length (EMA)").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("CMO length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.cmo_length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("VIDYA").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.vidya_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("VIDYA prev").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.vidya_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Current α").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.current_alpha)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("|CMO|").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.cmo_magnitude)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Deviation %").small().strong()); ui.label(egui::RichText::new(format!("{:+.2}%", snap.deviation_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                    }
                });
            self.show_vidya_win = open;
        }

        if self.show_smi_win {
            if self.smi_win_symbol.is_empty() {
                self.smi_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_smi_win;
            egui::Window::new("SMI — Stochastic Momentum Index (Blau, 10/3/3)")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 290.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.smi_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.smi_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.smi_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_smi(&conn, &sym_u)
                                    {
                                        self.smi_win_snapshot = snap;
                                        self.smi_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.smi_win_symbol.to_uppercase();
                            self.smi_win_loading = true;
                            self.smi_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeSmiSnapshot { symbol: sym });
                        }
                        if self.smi_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.smi_win_snapshot;
                    if snap.symbol.is_empty() || snap.smi_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥21 bars with OHLC.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.smi_label.as_str() {
                            "OVERBOUGHT" | "BULL_CROSS" | "BULL" => UP,
                            "OVERSOLD" | "BEAR_CROSS" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — SMI {:+.2} — signal {:+.2} — close {:.4} — as of {}",
                                snap.symbol,
                                snap.smi_label,
                                snap.smi_value,
                                snap.signal_value,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("smi_summary")
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
                                ui.label(egui::RichText::new("Length (lookback)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.length))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Smooth length").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.smooth_length))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Signal length").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.signal_length))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("SMI").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}", snap.smi_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("SMI prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}", snap.smi_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Signal").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}", snap.signal_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Signal prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}", snap.signal_prev))
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
            self.show_smi_win = open;
        }

        if self.show_pvt_win {
            if self.pvt_win_symbol.is_empty() {
                self.pvt_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_pvt_win;
            egui::Window::new("PVT — Price Volume Trend (Dysart/Lowry cumulative volume-weighted %Δ)")
                .open(&mut open).resizable(true).default_size([600.0, 270.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.pvt_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.pvt_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.pvt_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_pvt(&conn, &sym_u) { self.pvt_win_snapshot = snap; self.pvt_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.pvt_win_symbol.to_uppercase(); self.pvt_win_loading = true; self.pvt_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputePvtSnapshot { symbol: sym });
                        }
                        if self.pvt_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.pvt_win_snapshot;
                    if snap.symbol.is_empty() || snap.pvt_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥42 bars with volume.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.pvt_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — PVT {:.2} — EMA20 {:.2} — 20-bar slope {:+.2} — close {:.4} — as of {}", snap.symbol, snap.pvt_label, snap.pvt_value, snap.pvt_ema, snap.pvt_slope, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("pvt_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("PVT").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.pvt_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("PVT prev").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.pvt_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("PVT EMA (20)").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.pvt_ema)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("20-bar slope").small().strong()); ui.label(egui::RichText::new(format!("{:+.2}", snap.pvt_slope)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                    }
                });
            self.show_pvt_win = open;
        }

        if self.show_ac_win {
            if self.ac_win_symbol.is_empty() {
                self.ac_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ac_win;
            egui::Window::new("AC — Accelerator Oscillator (Bill Williams; AO − SMA₅(AO))")
                .open(&mut open).resizable(true).default_size([600.0, 270.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.ac_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.ac_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.ac_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_ac(&conn, &sym_u) { self.ac_win_snapshot = snap; self.ac_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ac_win_symbol.to_uppercase(); self.ac_win_loading = true; self.ac_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeAcSnapshot { symbol: sym });
                        }
                        if self.ac_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.ac_win_snapshot;
                    if snap.symbol.is_empty() || snap.ac_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥40 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.ac_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — AC {:+.4} — AO {:+.4} — AO SMA5 {:+.4} — close {:.4} — as of {}", snap.symbol, snap.ac_label, snap.ac_value, snap.ao_value, snap.ao_sma5, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("ac_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("AC").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.ac_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("AC prev").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.ac_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("AO").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.ao_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("AO SMA5").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.ao_sma5)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_ac_win = open;
        }

        if self.show_chvol_win {
            if self.chvol_win_symbol.is_empty() {
                self.chvol_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_chvol_win;
            egui::Window::new("CHVOL — Chaikin Volatility (10-EMA of H−L, 10-bar ROC)")
                .open(&mut open)
                .resizable(true)
                .default_size([600.0, 270.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.chvol_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.chvol_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.chvol_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_chvol(&conn, &sym_u)
                                    {
                                        self.chvol_win_snapshot = snap;
                                        self.chvol_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.chvol_win_symbol.to_uppercase();
                            self.chvol_win_loading = true;
                            self.chvol_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeChvolSnapshot { symbol: sym });
                        }
                        if self.chvol_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.chvol_win_snapshot;
                    if snap.symbol.is_empty() || snap.chvol_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥25 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.chvol_label.as_str() {
                            "EXPANDING" => UP,
                            "CONTRACTING" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — CHVOL {:+.2}% — EMA(H−L) {:.4} — close {:.4} — as of {}",
                                snap.symbol,
                                snap.chvol_label,
                                snap.chvol_value,
                                snap.ema_range,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("chvol_summary")
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
                                ui.label(egui::RichText::new("EMA length").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.ema_length))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ROC length").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.roc_length))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("CHVOL %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}", snap.chvol_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("CHVOL prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}", snap.chvol_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("EMA(H−L)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.ema_range))
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
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
                        }
                    }
                });
            self.show_chvol_win = open;
        }

        if self.show_bbwidth_win {
            if self.bbwidth_win_symbol.is_empty() {
                self.bbwidth_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_bbwidth_win;
            egui::Window::new("BBWIDTH — Bollinger Bandwidth (SMA₂₀ ± 2σ, 125-bar percentile)")
                .open(&mut open).resizable(true).default_size([640.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.bbwidth_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.bbwidth_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.bbwidth_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_bbwidth(&conn, &sym_u) { self.bbwidth_win_snapshot = snap; self.bbwidth_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.bbwidth_win_symbol.to_uppercase(); self.bbwidth_win_loading = true; self.bbwidth_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeBbwidthSnapshot { symbol: sym });
                        }
                        if self.bbwidth_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.bbwidth_win_snapshot;
                    if snap.symbol.is_empty() || snap.bbw_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥20 bars (125 for percentile).").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.bbw_label.as_str() {
                            "SQUEEZE" => DOWN,
                            "EXPANDED" => UP,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — BBW {:.4} — pct {:.1} — mid {:.4} — close {:.4} — as of {}", snap.symbol, snap.bbw_label, snap.bbw_value, snap.bbw_percentile, snap.middle, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("bbwidth_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Stdev width").small().strong()); ui.label(egui::RichText::new(format!("±{:.1}σ", snap.num_stdev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("BBW").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.bbw_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("BBW prev").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.bbw_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("125-bar percentile").small().strong()); ui.label(egui::RichText::new(format!("{:.1}", snap.bbw_percentile)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Upper").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.upper)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Middle").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.middle)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Lower").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.lower)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_bbwidth_win = open;
        }

        if self.show_elderimp_win {
            if self.elderimp_win_symbol.is_empty() {
                self.elderimp_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_elderimp_win;
            egui::Window::new("ELDERIMP — Elder Impulse System (13-EMA slope + MACD hist slope)")
                .open(&mut open).resizable(true).default_size([620.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.elderimp_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.elderimp_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.elderimp_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_elderimp(&conn, &sym_u) { self.elderimp_win_snapshot = snap; self.elderimp_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.elderimp_win_symbol.to_uppercase(); self.elderimp_win_loading = true; self.elderimp_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeElderImpSnapshot { symbol: sym });
                        }
                        if self.elderimp_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.elderimp_win_snapshot;
                    if snap.symbol.is_empty() || snap.impulse_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥35 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.impulse_label.as_str() {
                            "GREEN" => UP,
                            "RED" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — EMA {:.4} (slope {:+.4}) — hist {:+.4} (slope {:+.4}) — close {:.4} — as of {}", snap.symbol, snap.impulse_label, snap.ema_value, snap.ema_slope, snap.macd_hist, snap.macd_hist_slope, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("elderimp_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("EMA length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.ema_length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("EMA").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.ema_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("EMA slope").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.ema_slope)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("MACD hist").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.macd_hist)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("MACD hist prev").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.macd_hist_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("MACD hist slope").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.macd_hist_slope)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_elderimp_win = open;
        }

        if self.show_rmi_win {
            if self.rmi_win_symbol.is_empty() {
                self.rmi_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_rmi_win;
            egui::Window::new("RMI — Relative Momentum Index (Altman; RSI on 5-bar momentum)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.rmi_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.rmi_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.rmi_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_rmi(&conn, &sym_u)
                                    {
                                        self.rmi_win_snapshot = snap;
                                        self.rmi_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.rmi_win_symbol.to_uppercase();
                            self.rmi_win_loading = true;
                            self.rmi_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRmiSnapshot { symbol: sym });
                        }
                        if self.rmi_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.rmi_win_snapshot;
                    if snap.symbol.is_empty() || snap.rmi_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥25 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.rmi_label.as_str() {
                            "OVERBOUGHT" | "BULL" => UP,
                            "OVERSOLD" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — RMI {:.2} — close {:.4} — as of {}",
                                snap.symbol,
                                snap.rmi_label,
                                snap.rmi_value,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("rmi_summary")
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
                                ui.label(egui::RichText::new("Length").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.length))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Momentum length").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.momentum_length))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("RMI").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.rmi_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("RMI prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.rmi_prev))
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
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
                        }
                    }
                });
            self.show_rmi_win = open;
        }

        if self.show_expcal_win {
            if self.expcal_win_symbol.is_empty() {
                self.expcal_win_symbol = chart_sym_research.clone();
            }
            if self.expcal_win_calendar.is_empty() {
                let today = chrono::Local::now().date_naive();
                self.expcal_win_calendar = typhoon_engine::core::research::compute_market_calendar(
                    today,
                    self.expcal_win_horizon_days,
                );
            }
            let mut open = self.show_expcal_win;
            egui::Window::new(
                "EXPCAL — Options Expiration Calendar (Tier 1 market · Tier 2 per-symbol)",
            )
            .open(&mut open)
            .resizable(true)
            .default_size([780.0, 480.0])
            .max_size([780.0, 560.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.expcal_win_tab, 0, "Market calendar");
                    ui.selectable_value(&mut self.expcal_win_tab, 1, "Symbol chain");
                });
                ui.separator();
                if self.expcal_win_tab == 0 {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Horizon days:").color(AXIS_TEXT));
                        let mut h = self.expcal_win_horizon_days as i32;
                        if ui
                            .add(egui::DragValue::new(&mut h).range(7..=730))
                            .changed()
                        {
                            self.expcal_win_horizon_days = h.max(7) as u32;
                            let today = chrono::Local::now().date_naive();
                            self.expcal_win_calendar =
                                typhoon_engine::core::research::compute_market_calendar(
                                    today,
                                    self.expcal_win_horizon_days,
                                );
                        }
                        if ui.button("Regenerate").clicked() {
                            let today = chrono::Local::now().date_naive();
                            self.expcal_win_calendar =
                                typhoon_engine::core::research::compute_market_calendar(
                                    today,
                                    self.expcal_win_horizon_days,
                                );
                        }
                    });
                    ui.separator();
                    if self.expcal_win_calendar.is_empty() {
                        ui.label(
                            egui::RichText::new("No upcoming Fridays in horizon.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        egui::ScrollArea::vertical()
                            .id_salt("expcal_tier1_scroll")
                            .show(ui, |ui| {
                                egui::Grid::new("expcal_tier1_grid")
                                    .striped(true)
                                    .num_columns(4)
                                    .min_col_width(90.0)
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("Date").small().strong());
                                        ui.label(egui::RichText::new("Weekday").small().strong());
                                        ui.label(egui::RichText::new("DTE").small().strong());
                                        ui.label(egui::RichText::new("Type").small().strong());
                                        ui.end_row();
                                        for e in &self.expcal_win_calendar {
                                            let color = match e.expiry_type.as_str() {
                                                "TRIPLE_WITCHING" => DOWN,
                                                "QUARTERLY" => UP,
                                                "LEAPS" => AXIS_TEXT,
                                                "MONTHLY" => UP,
                                                _ => AXIS_TEXT,
                                            };
                                            ui.label(
                                                egui::RichText::new(&e.date).small().monospace(),
                                            );
                                            ui.label(egui::RichText::new(&e.weekday).small());
                                            ui.label(
                                                egui::RichText::new(format!("{}", e.days_from_now))
                                                    .small()
                                                    .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(&e.expiry_type)
                                                    .small()
                                                    .color(color)
                                                    .strong(),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
                } else {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.expcal_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.expcal_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.expcal_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_symbol_expirations(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.expcal_win_snapshot = snap;
                                        self.expcal_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.expcal_win_symbol.to_uppercase();
                            self.expcal_win_loading = true;
                            self.expcal_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeSymbolExpirations { symbol: sym });
                        }
                        if self.expcal_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.expcal_win_snapshot;
                    if snap.symbol.is_empty() {
                        ui.label(
                            egui::RichText::new(
                                "No data — run OPTIONS first to cache the chain, then Compute.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                    } else if snap.expirations.is_empty() {
                        ui.label(
                            egui::RichText::new(if snap.note.is_empty() {
                                "Chain present but no expirations parsed."
                            } else {
                                snap.note.as_str()
                            })
                            .color(AXIS_TEXT)
                            .small(),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} expirations — underlying {:.4} — as of {}",
                                snap.symbol,
                                snap.expirations.len(),
                                snap.underlying_price,
                                snap.as_of
                            ))
                            .strong(),
                        );
                        if !snap.next_triple_witching.is_empty() {
                            ui.label(
                                egui::RichText::new(format!(
                                    "Next triple witching: {}",
                                    snap.next_triple_witching
                                ))
                                .color(DOWN)
                                .strong(),
                            );
                        }
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .id_salt("expcal_tier2_scroll")
                            .show(ui, |ui| {
                                egui::Grid::new("expcal_tier2_grid")
                                    .striped(true)
                                    .num_columns(9)
                                    .min_col_width(68.0)
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("Date").small().strong());
                                        ui.label(egui::RichText::new("DTE").small().strong());
                                        ui.label(egui::RichText::new("Type").small().strong());
                                        ui.label(egui::RichText::new("Calls").small().strong());
                                        ui.label(egui::RichText::new("Puts").small().strong());
                                        ui.label(egui::RichText::new("Call Vol").small().strong());
                                        ui.label(egui::RichText::new("Put Vol").small().strong());
                                        ui.label(egui::RichText::new("Call OI").small().strong());
                                        ui.label(
                                            egui::RichText::new("Put OI / PCR").small().strong(),
                                        );
                                        ui.end_row();
                                        for ex in &snap.expirations {
                                            let color = match ex.expiry_type.as_str() {
                                                "TRIPLE_WITCHING" => DOWN,
                                                "QUARTERLY" | "MONTHLY" => UP,
                                                _ => AXIS_TEXT,
                                            };
                                            ui.label(
                                                egui::RichText::new(&ex.date).small().monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{}",
                                                    ex.days_to_expiry
                                                ))
                                                .small()
                                                .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(&ex.expiry_type)
                                                    .small()
                                                    .color(color)
                                                    .strong(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("{}", ex.call_count))
                                                    .small()
                                                    .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("{}", ex.put_count))
                                                    .small()
                                                    .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:.0}",
                                                    ex.total_call_volume
                                                ))
                                                .small()
                                                .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:.0}",
                                                    ex.total_put_volume
                                                ))
                                                .small()
                                                .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:.0}",
                                                    ex.total_call_oi
                                                ))
                                                .small()
                                                .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:.0} / {:.2}",
                                                    ex.total_put_oi, ex.put_call_ratio
                                                ))
                                                .small()
                                                .monospace(),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            });
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
                        }
                    }
                }
            });
            self.show_expcal_win = open;
        }
    }
}
