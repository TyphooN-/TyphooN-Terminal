use super::*;

impl TyphooNApp {
    pub(super) fn render_oscillator_forecast_flow_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
        if self.show_qqe_win {
            if self.qqe_win_symbol.is_empty() {
                self.qqe_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_qqe_win;
            egui::Window::new(
                "QQE — Quantitative Qualitative Estimation (smoothed RSI + adaptive bands)",
            )
            .open(&mut open)
            .resizable(true)
            .default_size([580.0, 300.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.qqe_win_symbol).desired_width(100.0),
                    );
                    if ui.button("Use Chart").clicked() {
                        self.qqe_win_symbol = chart_sym_research.clone();
                    }
                    if ui.button("Load Cached").clicked() {
                        if let Some(ref cache) = self.cache {
                            if let Ok(conn) = cache.connection() {
                                let sym_u = self.qqe_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) =
                                    typhoon_engine::core::research::get_qqe(&conn, &sym_u)
                                {
                                    self.qqe_win_snapshot = snap;
                                    self.qqe_win_symbol = sym_u;
                                }
                            }
                        }
                    }
                    if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                        let sym = self.qqe_win_symbol.to_uppercase();
                        self.qqe_win_loading = true;
                        self.qqe_win_symbol = sym.clone();
                        let _ = self
                            .broker_tx
                            .send(BrokerCmd::ComputeQqeSnapshot { symbol: sym });
                    }
                    if self.qqe_win_loading {
                        ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                    }
                });
                ui.separator();
                let snap = &self.qqe_win_snapshot;
                if snap.symbol.is_empty() || snap.qqe_label == "INSUFFICIENT_DATA" {
                    ui.label(
                        egui::RichText::new("No data — HP cache needs ≥40 bars.")
                            .color(AXIS_TEXT)
                            .small(),
                    );
                } else {
                    let color = match snap.qqe_label.as_str() {
                        "STRONG_BULL" | "BULL" => UP,
                        "STRONG_BEAR" | "BEAR" => DOWN,
                        _ => AXIS_TEXT,
                    };
                    ui.label(
                        egui::RichText::new(format!(
                            "{} — {} — RSI {:.2} → smoothed {:.2} — close {:.4} — as of {}",
                            snap.symbol,
                            snap.qqe_label,
                            snap.rsi_value,
                            snap.rsi_smoothed,
                            snap.last_close,
                            snap.as_of
                        ))
                        .strong()
                        .color(color),
                    );
                    ui.separator();
                    egui::Grid::new("qqe_summary")
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
                            ui.label(egui::RichText::new("RSI / smooth lengths").small().strong());
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} / {}",
                                    snap.rsi_length, snap.smooth_length
                                ))
                                .small()
                                .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("QQE factor").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:.3}", snap.qqe_factor))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("RSI raw").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:.2}", snap.rsi_value))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("RSI smoothed").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:.2}", snap.rsi_smoothed))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("Fast ATR_RSI avg").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:.3}", snap.fast_atr_rsi_avg))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("Upper band").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:.2}", snap.upper_band))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("Lower band").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:.2}", snap.lower_band))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("Prior smoothed").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:.2}", snap.qqe_prev))
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
            self.show_qqe_win = open;
        }

        if self.show_pmo_win {
            if self.pmo_win_symbol.is_empty() {
                self.pmo_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_pmo_win;
            egui::Window::new("PMO — Pring's Price Momentum Oscillator (double-smoothed ROC + signal)")
                .open(&mut open).resizable(true).default_size([560.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.pmo_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.pmo_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.pmo_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_pmo(&conn, &sym_u) { self.pmo_win_snapshot = snap; self.pmo_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.pmo_win_symbol.to_uppercase(); self.pmo_win_loading = true; self.pmo_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputePmoSnapshot { symbol: sym });
                        }
                        if self.pmo_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.pmo_win_snapshot;
                    if snap.symbol.is_empty() || snap.pmo_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥70 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.pmo_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — PMO {:+.4} · signal {:+.4} · hist {:+.4} — close {:.4} — as of {}",
                            snap.symbol, snap.pmo_label, snap.pmo_value, snap.pmo_signal, snap.histogram, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("pmo_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Smooth1 / Smooth2 / Signal").small().strong()); ui.label(egui::RichText::new(format!("{} / {} / {}", snap.smooth1_length, snap.smooth2_length, snap.signal_length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("PMO").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.pmo_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("PMO prev").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.pmo_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Signal").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.pmo_signal)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Histogram (PMO − signal)").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.histogram)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_pmo_win = open;
        }

        if self.show_cfo_win {
            if self.cfo_win_symbol.is_empty() {
                self.cfo_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cfo_win;
            egui::Window::new(
                "CFO — Chande Forecast Oscillator (100·(close − linreg_forecast)/close)",
            )
            .open(&mut open)
            .resizable(true)
            .default_size([560.0, 260.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.cfo_win_symbol).desired_width(100.0),
                    );
                    if ui.button("Use Chart").clicked() {
                        self.cfo_win_symbol = chart_sym_research.clone();
                    }
                    if ui.button("Load Cached").clicked() {
                        if let Some(ref cache) = self.cache {
                            if let Ok(conn) = cache.connection() {
                                let sym_u = self.cfo_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) =
                                    typhoon_engine::core::research::get_cfo(&conn, &sym_u)
                                {
                                    self.cfo_win_snapshot = snap;
                                    self.cfo_win_symbol = sym_u;
                                }
                            }
                        }
                    }
                    if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                        let sym = self.cfo_win_symbol.to_uppercase();
                        self.cfo_win_loading = true;
                        self.cfo_win_symbol = sym.clone();
                        let _ = self
                            .broker_tx
                            .send(BrokerCmd::ComputeCfoSnapshot { symbol: sym });
                    }
                    if self.cfo_win_loading {
                        ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                    }
                });
                ui.separator();
                let snap = &self.cfo_win_snapshot;
                if snap.symbol.is_empty() || snap.cfo_label == "INSUFFICIENT_DATA" {
                    ui.label(
                        egui::RichText::new("No data — HP cache needs ≥15 bars.")
                            .color(AXIS_TEXT)
                            .small(),
                    );
                } else {
                    let color = match snap.cfo_label.as_str() {
                        "STRONG_ABOVE_TREND" | "ABOVE_TREND" => UP,
                        "STRONG_BELOW_TREND" | "BELOW_TREND" => DOWN,
                        _ => AXIS_TEXT,
                    };
                    ui.label(
                        egui::RichText::new(format!(
                            "{} — {} — CFO {:+.2}% (prev {:+.2}%) — close {:.4} — as of {}",
                            snap.symbol,
                            snap.cfo_label,
                            snap.cfo_value,
                            snap.cfo_prev,
                            snap.last_close,
                            snap.as_of
                        ))
                        .strong()
                        .color(color),
                    );
                    ui.separator();
                    egui::Grid::new("cfo_summary")
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
                            ui.label(egui::RichText::new("OLS slope").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:+.6}", snap.slope))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("OLS intercept").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:.4}", snap.intercept))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("One-bar forecast").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:.4}", snap.forecast))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("CFO").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:+.2}%", snap.cfo_value))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("CFO prev").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:+.2}%", snap.cfo_prev))
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
            self.show_cfo_win = open;
        }

        if self.show_tmf_win {
            if self.tmf_win_symbol.is_empty() {
                self.tmf_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_tmf_win;
            egui::Window::new("TMF — Twiggs Money Flow (EMA-smoothed true-range money flow)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.tmf_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.tmf_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.tmf_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_tmf(&conn, &sym_u)
                                    {
                                        self.tmf_win_snapshot = snap;
                                        self.tmf_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.tmf_win_symbol.to_uppercase();
                            self.tmf_win_loading = true;
                            self.tmf_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeTmfSnapshot { symbol: sym });
                        }
                        if self.tmf_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.tmf_win_snapshot;
                    if snap.symbol.is_empty() || snap.tmf_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥22 bars with volume.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.tmf_label.as_str() {
                            "STRONG_INFLOW" | "INFLOW" => UP,
                            "STRONG_OUTFLOW" | "OUTFLOW" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — TMF {:+.4} (prev {:+.4}) — close {:.4} — as of {}",
                                snap.symbol,
                                snap.tmf_label,
                                snap.tmf_value,
                                snap.tmf_prev,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("tmf_summary")
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
                                ui.label(egui::RichText::new("EMA money-flow").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.ema_money_flow))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("EMA volume").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.ema_volume))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("TMF").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.tmf_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("TMF prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.tmf_prev))
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
            self.show_tmf_win = open;
        }
    }
}
