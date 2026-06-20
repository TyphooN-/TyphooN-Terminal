use super::*;

impl TyphooNApp {
    pub(super) fn render_research_volume_momentum_oscillators_windows(
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

        // ── Research section ──
        if self.show_mass_win {
            if self.mass_win_symbol.is_empty() {
                self.mass_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_mass_win;
            egui::Window::new("MASS — Mass Index (Dorsey, 1992)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.mass_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.mass_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.mass_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_mass(&conn, &sym_u)
                                    {
                                        self.mass_win_snapshot = snap;
                                        self.mass_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.mass_win_symbol.to_uppercase();
                            self.mass_win_loading = true;
                            self.mass_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMassSnapshot { symbol: sym });
                        }
                        if self.mass_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.mass_win_snapshot;
                    if snap.symbol.is_empty() || snap.mass_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥45 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.mass_label.as_str() {
                            "REVERSAL_BULGE" => DOWN,
                            "WATCH" => AXIS_TEXT,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — Mass {:.3} — ratio {:.3} — as of {}",
                                snap.symbol,
                                snap.mass_label,
                                snap.mass_value,
                                snap.single_ratio,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("mass_summary")
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
                                ui.label(egui::RichText::new("EMA / sum").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {}",
                                        snap.ema_period, snap.sum_period
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Single ratio").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.single_ratio))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Mass value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.mass_value))
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
            self.show_mass_win = open;
        }

        if self.show_chaikosc_win {
            if self.chaikosc_win_symbol.is_empty() {
                self.chaikosc_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_chaikosc_win;
            egui::Window::new("CHAIKOSC — Chaikin Oscillator (3/10)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.chaikosc_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.chaikosc_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.chaikosc_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_chaikosc(&conn, &sym_u)
                                    {
                                        self.chaikosc_win_snapshot = snap;
                                        self.chaikosc_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.chaikosc_win_symbol.to_uppercase();
                            self.chaikosc_win_loading = true;
                            self.chaikosc_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeChaikoscSnapshot { symbol: sym });
                        }
                        if self.chaikosc_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.chaikosc_win_snapshot;
                    if snap.symbol.is_empty() || snap.chaikosc_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥12 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.chaikosc_label.as_str() {
                            "STRONG_ACCUM" | "ACCUM" => UP,
                            "STRONG_DIST" | "DIST" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — CO {:+.1} — as of {}",
                                snap.symbol, snap.chaikosc_label, snap.chaikosc_value, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("chaikosc_summary")
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
                                ui.label(egui::RichText::new("Fast / slow").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {}",
                                        snap.fast_period, snap.slow_period
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("A/D last").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.ad_last))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("EMA(3) A/D").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.ema_fast_ad))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("EMA(10) A/D").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.ema_slow_ad))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Oscillator").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}", snap.chaikosc_value))
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
            self.show_chaikosc_win = open;
        }

        if self.show_klinger_win {
            if self.klinger_win_symbol.is_empty() {
                self.klinger_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_klinger_win;
            egui::Window::new("KLINGER — Klinger Volume Oscillator (34/55/13)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.klinger_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.klinger_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.klinger_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_klinger(&conn, &sym_u)
                                    {
                                        self.klinger_win_snapshot = snap;
                                        self.klinger_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.klinger_win_symbol.to_uppercase();
                            self.klinger_win_loading = true;
                            self.klinger_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeKlingerSnapshot { symbol: sym });
                        }
                        if self.klinger_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.klinger_win_snapshot;
                    if snap.symbol.is_empty() || snap.klinger_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥71 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.klinger_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — KVO {:+.1} — signal {:+.1} — hist {:+.1} — as of {}",
                                snap.symbol,
                                snap.klinger_label,
                                snap.kvo_value,
                                snap.signal_value,
                                snap.histogram,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("klinger_summary")
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
                                ui.label(egui::RichText::new("EMA fast VF").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.ema_fast_vf))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("EMA slow VF").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.ema_slow_vf))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("KVO").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}", snap.kvo_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Signal (EMA-13)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}", snap.signal_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Histogram").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}", snap.histogram))
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
            self.show_klinger_win = open;
        }

        if self.show_stochrsi_win {
            if self.stochrsi_win_symbol.is_empty() {
                self.stochrsi_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_stochrsi_win;
            egui::Window::new("STOCHRSI — Stochastic RSI (14/14/3/3)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.stochrsi_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.stochrsi_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.stochrsi_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_stochrsi(&conn, &sym_u)
                                    {
                                        self.stochrsi_win_snapshot = snap;
                                        self.stochrsi_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.stochrsi_win_symbol.to_uppercase();
                            self.stochrsi_win_loading = true;
                            self.stochrsi_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeStochRsiSnapshot { symbol: sym });
                        }
                        if self.stochrsi_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.stochrsi_win_snapshot;
                    if snap.symbol.is_empty() || snap.stochrsi_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥36 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.stochrsi_label.as_str() {
                            "OVERBOUGHT" | "BULL" => UP,
                            "OVERSOLD" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — %K {:.2} — %D {:.2} — RSI {:.2} — as of {}",
                                snap.symbol,
                                snap.stochrsi_label,
                                snap.k_value,
                                snap.d_value,
                                snap.rsi_value,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("stochrsi_summary")
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
                                    egui::RichText::new("Periods (RSI/stoch/%K/%D)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {} / {} / {}",
                                        snap.rsi_period,
                                        snap.stoch_period,
                                        snap.k_period,
                                        snap.d_period
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("RSI value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.rsi_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("RSI min").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.rsi_min))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("RSI max").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.rsi_max))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("StochRSI raw").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.stoch_rsi_raw))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("%K").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.k_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("%D").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.d_value))
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
            self.show_stochrsi_win = open;
        }
    }
}
