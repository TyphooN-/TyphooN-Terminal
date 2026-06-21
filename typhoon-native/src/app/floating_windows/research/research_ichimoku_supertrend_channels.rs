use super::*;

impl TyphooNApp {
    pub(super) fn render_research_ichimoku_supertrend_channels_windows(
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
        if self.show_ichimoku_win {
            if self.ichimoku_win_symbol.is_empty() {
                self.ichimoku_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ichimoku_win;
            egui::Window::new("ICHIMOKU — Kinko Hyo Cloud")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 340.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ichimoku_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ichimoku_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ichimoku_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ichimoku(&conn, &sym_u)
                                    {
                                        self.ichimoku_win_snapshot = snap;
                                        self.ichimoku_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ichimoku_win_symbol.to_uppercase();
                            self.ichimoku_win_loading = true;
                            self.ichimoku_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeIchimokuSnapshot { symbol: sym });
                        }
                        if self.ichimoku_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.ichimoku_win_snapshot;
                    if snap.symbol.is_empty() || snap.ichimoku_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥78 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.ichimoku_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — close vs cloud {:+.2}% — as of {}",
                                snap.symbol,
                                snap.ichimoku_label,
                                snap.close_vs_cloud_pct,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("ichi_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(180.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Bars used").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.bars_used))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Tenkan (9)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.tenkan_sen))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Kijun (26)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.kijun_sen))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Senkou A").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.senkou_span_a))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Senkou B (52)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.senkou_span_b))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Cloud top").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.cloud_top))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Cloud bottom").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.cloud_bottom))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Chikou").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.chikou_span))
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
            self.show_ichimoku_win = open;
        }

        if self.show_supertrend_win {
            if self.supertrend_win_symbol.is_empty() {
                self.supertrend_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_supertrend_win;
            egui::Window::new("SUPERTREND — ATR Trailing Stop")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.supertrend_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.supertrend_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.supertrend_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_supertrend(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.supertrend_win_snapshot = snap;
                                        self.supertrend_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.supertrend_win_symbol.to_uppercase();
                            self.supertrend_win_loading = true;
                            self.supertrend_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeSupertrendSnapshot { symbol: sym });
                        }
                        if self.supertrend_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.supertrend_win_snapshot;
                    if snap.symbol.is_empty() || snap.supertrend_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥12 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.supertrend_label.as_str() {
                            "STRONG_UP" | "UP" => UP,
                            "STRONG_DOWN" | "DOWN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — ST {:.4} — dist {:+.2}% — bars {} — as of {}",
                                snap.symbol,
                                snap.supertrend_label,
                                snap.supertrend_value,
                                snap.distance_pct,
                                snap.bars_in_trend,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("st_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(180.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Bars used").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.bars_used))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Period / multiplier").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {:.1}",
                                        snap.period, snap.multiplier
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ATR").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.atr))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Upper band").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.upper_band))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Lower band").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.lower_band))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Active ST").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.supertrend_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Trend direction").small().strong());
                                ui.label(
                                    egui::RichText::new(if snap.trend_is_up {
                                        "UP"
                                    } else {
                                        "DOWN"
                                    })
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Bars in trend").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.bars_in_trend))
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
            self.show_supertrend_win = open;
        }

        if self.show_keltner_win {
            if self.keltner_win_symbol.is_empty() {
                self.keltner_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_keltner_win;
            egui::Window::new("KELTNER — Channels (EMA 20 ± 2·ATR 10)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.keltner_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.keltner_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.keltner_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_keltner(&conn, &sym_u)
                                    {
                                        self.keltner_win_snapshot = snap;
                                        self.keltner_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.keltner_win_symbol.to_uppercase();
                            self.keltner_win_loading = true;
                            self.keltner_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeKeltnerSnapshot { symbol: sym });
                        }
                        if self.keltner_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.keltner_win_snapshot;
                    if snap.symbol.is_empty() || snap.keltner_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥22 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.keltner_label.as_str() {
                            "BREAKOUT_UP" => UP,
                            "BREAKOUT_DOWN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        let ttm = if snap.ttm_squeeze_on {
                            " • TTM SQUEEZE ON"
                        } else {
                            ""
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — pos {:.1}%{} — as of {}",
                                snap.symbol,
                                snap.keltner_label,
                                snap.channel_position_pct,
                                ttm,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("kelt_summary")
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
                                ui.label(egui::RichText::new("EMA / ATR periods").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {}",
                                        snap.ema_period, snap.atr_period
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Multiplier").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.1}", snap.multiplier))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("EMA midline").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.ema_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ATR").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.atr))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Upper").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.upper_channel))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Lower").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.lower_channel))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Width").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.channel_width))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Width % of mid").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.width_pct_of_mid))
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
                                ui.label(egui::RichText::new("TTM squeeze").small().strong());
                                ui.label(
                                    egui::RichText::new(if snap.ttm_squeeze_on {
                                        "YES"
                                    } else {
                                        "no"
                                    })
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_keltner_win = open;
        }

        if self.show_fisher_win {
            if self.fisher_win_symbol.is_empty() {
                self.fisher_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_fisher_win;
            egui::Window::new("FISHER — Ehlers Fisher Transform")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.fisher_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.fisher_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.fisher_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_fisher(&conn, &sym_u)
                                    {
                                        self.fisher_win_snapshot = snap;
                                        self.fisher_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.fisher_win_symbol.to_uppercase();
                            self.fisher_win_loading = true;
                            self.fisher_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeFisherSnapshot { symbol: sym });
                        }
                        if self.fisher_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.fisher_win_snapshot;
                    if snap.symbol.is_empty() || snap.fisher_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥22 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.fisher_label.as_str() {
                            "STRONG_POS" | "POS" => UP,
                            "STRONG_NEG" | "NEG" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — fisher {:+.3} — signal {:+.3} — as of {}",
                                snap.symbol,
                                snap.fisher_label,
                                snap.fisher_value,
                                snap.fisher_signal,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("fisher_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(180.0)
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
                                ui.label(egui::RichText::new("Fisher value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.fisher_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Signal (prev)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.fisher_signal))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Peak |fisher| 10").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.peak_abs_10))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("±2 cross last 3").small().strong());
                                ui.label(
                                    egui::RichText::new(if snap.extreme_2_cross {
                                        "YES"
                                    } else {
                                        "no"
                                    })
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
            self.show_fisher_win = open;
        }

        if self.show_aroon_win {
            if self.aroon_win_symbol.is_empty() {
                self.aroon_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_aroon_win;
            egui::Window::new("AROON — Up / Down / Oscillator (25)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.aroon_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.aroon_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.aroon_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_aroon(&conn, &sym_u)
                                    {
                                        self.aroon_win_snapshot = snap;
                                        self.aroon_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.aroon_win_symbol.to_uppercase();
                            self.aroon_win_loading = true;
                            self.aroon_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeAroonSnapshot { symbol: sym });
                        }
                        if self.aroon_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.aroon_win_snapshot;
                    if snap.symbol.is_empty() || snap.aroon_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥26 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.aroon_label.as_str() {
                            "STRONG_UP" | "WEAK_UP" => UP,
                            "STRONG_DOWN" | "WEAK_DOWN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — osc {:+.1} — up {:.1} — down {:.1} — as of {}",
                                snap.symbol,
                                snap.aroon_label,
                                snap.aroon_oscillator,
                                snap.aroon_up,
                                snap.aroon_down,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("aroon_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(180.0)
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
                                ui.label(egui::RichText::new("Aroon Up").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.aroon_up))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Aroon Down").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.aroon_down))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Oscillator").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}", snap.aroon_oscillator))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Bars since high").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.bars_since_high))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Bars since low").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.bars_since_low))
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
            self.show_aroon_win = open;
        }
    }
}
