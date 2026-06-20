use super::*;

mod bands_intraday_guppy;
mod smma_alligator_crsi;
mod volume_trend_kdj;

impl TyphooNApp {
    pub(super) fn render_research_advanced_moving_averages_windows(&mut self, ctx: &egui::Context) {
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

        // ── Research Round 55: SMMA / ALLIGATOR / CRSI / SEB / IMI ──
        self.render_smma_alligator_crsi_windows(ctx, &chart_sym_research);

        self.render_bands_intraday_guppy_windows(ctx, &chart_sym_research);

        self.render_volume_trend_kdj_windows(ctx, &chart_sym_research);

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

        if self.show_fractals_win {
            if self.fractals_win_symbol.is_empty() {
                self.fractals_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_fractals_win;
            egui::Window::new("FRACTALS — Bill Williams 5-bar peak/trough pivots")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.fractals_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.fractals_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.fractals_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_fractals(&conn, &sym_u) { self.fractals_win_snapshot = snap; self.fractals_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.fractals_win_symbol.to_uppercase(); self.fractals_win_loading = true; self.fractals_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeFractalsSnapshot { symbol: sym });
                        }
                        if self.fractals_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.fractals_win_snapshot;
                    if snap.symbol.is_empty() || snap.fractals_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥5 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.fractals_label.as_str() {
                            "UP_RECENT" | "BOTH_RECENT" => UP,
                            "DOWN_RECENT" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — last up {:.4} ({} bars) · last down {:.4} ({} bars) — close {:.4} — as of {}",
                            snap.symbol, snap.fractals_label, snap.last_up_high, snap.last_up_bars_ago,
                            snap.last_down_low, snap.last_down_bars_ago, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("fractals_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Window").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.window)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last up fractal high").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_up_high)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last up bars ago").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_up_bars_ago)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last down fractal low").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_down_low)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last down bars ago").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_down_bars_ago)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Up fractal count").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.up_fractal_count)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Down fractal count").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.down_fractal_count)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_fractals_win = open;
        }

        if self.show_ift_rsi_win {
            if self.ift_rsi_win_symbol.is_empty() {
                self.ift_rsi_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ift_rsi_win;
            egui::Window::new("IFT_RSI — Ehlers Inverse Fisher Transform of RSI")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.ift_rsi_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.ift_rsi_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.ift_rsi_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_ift_rsi(&conn, &sym_u) { self.ift_rsi_win_snapshot = snap; self.ift_rsi_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ift_rsi_win_symbol.to_uppercase(); self.ift_rsi_win_loading = true; self.ift_rsi_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeIftRsiSnapshot { symbol: sym });
                        }
                        if self.ift_rsi_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.ift_rsi_win_snapshot;
                    if snap.symbol.is_empty() || snap.ift_rsi_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥25 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.ift_rsi_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — IFT {:+.4} (prev {:+.4}) · RSI {:.2} — close {:.4} — as of {}",
                            snap.symbol, snap.ift_rsi_label, snap.ift_value, snap.ift_prev, snap.rsi_value, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("ift_rsi_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("RSI length / WMA length").small().strong()); ui.label(egui::RichText::new(format!("{} / {}", snap.rsi_length, snap.wma_length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("RSI").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.rsi_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("v (WMA of 0.1·(RSI − 50))").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.v_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("IFT value").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.ift_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("IFT prev").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.ift_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_ift_rsi_win = open;
        }

        if self.show_mama_win {
            if self.mama_win_symbol.is_empty() {
                self.mama_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_mama_win;
            egui::Window::new("MAMA — MESA Adaptive MA (Ehlers, Hilbert-phase adaptive α)")
                .open(&mut open).resizable(true).default_size([560.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.mama_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.mama_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.mama_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_mama(&conn, &sym_u) { self.mama_win_snapshot = snap; self.mama_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.mama_win_symbol.to_uppercase(); self.mama_win_loading = true; self.mama_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeMamaSnapshot { symbol: sym });
                        }
                        if self.mama_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.mama_win_snapshot;
                    if snap.symbol.is_empty() || snap.mama_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥32 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.mama_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — MAMA {:.4} · FAMA {:.4} · α {:.4} · period {:.2} — close {:.4} — as of {}",
                            snap.symbol, snap.mama_label, snap.mama_value, snap.fama_value, snap.alpha, snap.period, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("mama_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Fast / Slow limit").small().strong()); ui.label(egui::RichText::new(format!("{:.2} / {:.2}", snap.fast_limit, snap.slow_limit)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("MAMA").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.mama_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("MAMA prev").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.mama_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("FAMA").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.fama_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("FAMA prev").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.fama_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Adaptive α").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.alpha)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Dominant cycle period").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.period)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_mama_win = open;
        }

        if self.show_cog_win {
            if self.cog_win_symbol.is_empty() {
                self.cog_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cog_win;
            egui::Window::new("COG — Ehlers Center of Gravity (zero-lag recency-weighted centroid)")
                .open(&mut open).resizable(true).default_size([520.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cog_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cog_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cog_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cog(&conn, &sym_u) { self.cog_win_snapshot = snap; self.cog_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cog_win_symbol.to_uppercase(); self.cog_win_loading = true; self.cog_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCogSnapshot { symbol: sym });
                        }
                        if self.cog_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cog_win_snapshot;
                    if snap.symbol.is_empty() || snap.cog_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥14 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cog_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — COG {:+.4} · signal {:+.4} · prev {:+.4} — close {:.4} — as of {}",
                            snap.symbol, snap.cog_label, snap.cog_value, snap.cog_signal, snap.cog_prev, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cog_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("COG").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.cog_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("COG signal (3-bar lag)").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.cog_signal)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("COG prev").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.cog_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cog_win = open;
        }

        if self.show_didi_win {
            if self.didi_win_symbol.is_empty() {
                self.didi_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_didi_win;
            egui::Window::new("DIDI — Didi Aguiar 3-SMA Brazilian Needles (3/8/20 normalized)")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.didi_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.didi_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.didi_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_didi(&conn, &sym_u) { self.didi_win_snapshot = snap; self.didi_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.didi_win_symbol.to_uppercase(); self.didi_win_loading = true; self.didi_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeDidiSnapshot { symbol: sym });
                        }
                        if self.didi_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.didi_win_snapshot;
                    if snap.symbol.is_empty() || snap.didi_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥22 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.didi_label.as_str() {
                            "BULL_NEEDLES" | "BULL" => UP,
                            "BEAR_NEEDLES" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — short/medium/long {}/{}/{} · short ratio {:+.4} · long ratio {:+.4} — close {:.4} — as of {}",
                            snap.symbol, snap.didi_label, snap.short_length, snap.medium_length, snap.long_length,
                            snap.short_ratio, snap.long_ratio, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("didi_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Short / Medium / Long").small().strong()); ui.label(egui::RichText::new(format!("{} / {} / {}", snap.short_length, snap.medium_length, snap.long_length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Short ratio (short/medium − 1)").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.short_ratio)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Short ratio prev").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.short_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Long ratio (long/medium − 1)").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.long_ratio)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Long ratio prev").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.long_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_didi_win = open;
        }

        if self.show_demarker_win {
            if self.demarker_win_symbol.is_empty() {
                self.demarker_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_demarker_win;
            egui::Window::new("DEMARKER — Tom DeMark DeMarker (14-bar high/low-range oscillator, bounded [0,1])")
                .open(&mut open).resizable(true).default_size([560.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.demarker_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.demarker_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.demarker_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_demarker(&conn, &sym_u) { self.demarker_win_snapshot = snap; self.demarker_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.demarker_win_symbol.to_uppercase(); self.demarker_win_loading = true; self.demarker_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeDemarkerSnapshot { symbol: sym });
                        }
                        if self.demarker_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.demarker_win_snapshot;
                    if snap.symbol.is_empty() || snap.demarker_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥16 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.demarker_label.as_str() {
                            "BULL" => UP,
                            "BEAR" => DOWN,
                            "OVERBOUGHT" => DOWN,
                            "OVERSOLD" => UP,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — DeM {:.4} (prev {:.4}) · ΣDeMax {:.4} · ΣDeMin {:.4} — close {:.4} — as of {}",
                            snap.symbol, snap.demarker_label, snap.demarker_value, snap.demarker_prev,
                            snap.demax_sum, snap.demin_sum, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("demarker_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("ΣDeMax").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.demax_sum)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("ΣDeMin").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.demin_sum)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("DeM").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.demarker_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("DeM prev").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.demarker_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_demarker_win = open;
        }

        if self.show_gator_win {
            if self.gator_win_symbol.is_empty() {
                self.gator_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_gator_win;
            egui::Window::new(
                "GATOR — Bill Williams Gator Oscillator (jaw/teeth/lips SMMA-spread life-cycle)",
            )
            .open(&mut open)
            .resizable(true)
            .default_size([580.0, 260.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.gator_win_symbol).desired_width(100.0),
                    );
                    if ui.button("Use Chart").clicked() {
                        self.gator_win_symbol = chart_sym_research.clone();
                    }
                    if ui.button("Load Cached").clicked() {
                        if let Some(ref cache) = self.cache {
                            if let Ok(conn) = cache.connection() {
                                let sym_u = self.gator_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) =
                                    typhoon_engine::core::research::get_gator(&conn, &sym_u)
                                {
                                    self.gator_win_snapshot = snap;
                                    self.gator_win_symbol = sym_u;
                                }
                            }
                        }
                    }
                    if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                        let sym = self.gator_win_symbol.to_uppercase();
                        self.gator_win_loading = true;
                        self.gator_win_symbol = sym.clone();
                        let _ = self
                            .broker_tx
                            .send(BrokerCmd::ComputeGatorSnapshot { symbol: sym });
                    }
                    if self.gator_win_loading {
                        ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                    }
                });
                ui.separator();
                let snap = &self.gator_win_snapshot;
                if snap.symbol.is_empty() || snap.gator_label == "INSUFFICIENT_DATA" {
                    ui.label(
                        egui::RichText::new("No data — HP cache needs ≥23 bars.")
                            .color(AXIS_TEXT)
                            .small(),
                    );
                } else {
                    let color = match snap.gator_label.as_str() {
                        "EATING" => UP,
                        "SATED" => DOWN,
                        _ => AXIS_TEXT,
                    };
                    ui.label(
                        egui::RichText::new(format!(
                            "{} — {} — upper {:+.4} · lower {:+.4} — close {:.4} — as of {}",
                            snap.symbol,
                            snap.gator_label,
                            snap.upper_bar,
                            snap.lower_bar,
                            snap.last_close,
                            snap.as_of
                        ))
                        .strong()
                        .color(color),
                    );
                    ui.separator();
                    egui::Grid::new("gator_summary")
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
                            ui.label(egui::RichText::new("Jaw / Teeth / Lips").small().strong());
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} / {} / {}",
                                    snap.jaw_length, snap.teeth_length, snap.lips_length
                                ))
                                .small()
                                .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("Upper bar").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:+.4}", snap.upper_bar))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("Upper prev").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:+.4}", snap.upper_prev))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("Lower bar").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:+.4}", snap.lower_bar))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("Lower prev").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:+.4}", snap.lower_prev))
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
            self.show_gator_win = open;
        }

        if self.show_bw_mfi_win {
            if self.bw_mfi_win_symbol.is_empty() {
                self.bw_mfi_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_bw_mfi_win;
            egui::Window::new("BW_MFI — Bill Williams Market Facilitation Index (range-per-volume 4-color)")
                .open(&mut open).resizable(true).default_size([580.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.bw_mfi_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.bw_mfi_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.bw_mfi_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_bw_mfi(&conn, &sym_u) { self.bw_mfi_win_snapshot = snap; self.bw_mfi_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.bw_mfi_win_symbol.to_uppercase(); self.bw_mfi_win_loading = true; self.bw_mfi_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeBwMfiSnapshot { symbol: sym });
                        }
                        if self.bw_mfi_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.bw_mfi_win_snapshot;
                    if snap.symbol.is_empty() || snap.bwmfi_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥2 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.bwmfi_color.as_str() {
                            "GREEN" => UP,
                            "FADE"  => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — MFI {:.4} (prev {:.4}) · vol {:.0} (prev {:.0}) — close {:.4} — as of {}",
                            snap.symbol, snap.bwmfi_color, snap.mfi_value, snap.mfi_prev,
                            snap.volume, snap.volume_prev, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("bw_mfi_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("MFI value").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.mfi_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("MFI prev").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.mfi_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Volume").small().strong()); ui.label(egui::RichText::new(format!("{:.0}", snap.volume)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Volume prev").small().strong()); ui.label(egui::RichText::new(format!("{:.0}", snap.volume_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Color").small().strong()); ui.label(egui::RichText::new(&snap.bwmfi_color).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_bw_mfi_win = open;
        }

        if self.show_vwma_win {
            if self.vwma_win_symbol.is_empty() {
                self.vwma_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_vwma_win;
            egui::Window::new("VWMA — Volume Weighted Moving Average (N=20) vs SMA")
                .open(&mut open).resizable(true).default_size([560.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.vwma_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.vwma_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.vwma_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_vwma(&conn, &sym_u) { self.vwma_win_snapshot = snap; self.vwma_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.vwma_win_symbol.to_uppercase(); self.vwma_win_loading = true; self.vwma_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeVwmaSnapshot { symbol: sym });
                        }
                        if self.vwma_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.vwma_win_snapshot;
                    if snap.symbol.is_empty() || snap.vwma_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥21 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.vwma_label.as_str() {
                            "BULL" | "WEAK_BULL" => UP,
                            "BEAR" | "WEAK_BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — VWMA {:.4} · SMA {:.4} · spread {:+.4} ({:+.3}%) — close {:.4} — as of {}",
                            snap.symbol, snap.vwma_label, snap.vwma_value, snap.sma_value, snap.spread, snap.spread_ratio * 100.0,
                            snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("vwma_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("VWMA").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.vwma_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("VWMA prev").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.vwma_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("SMA").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.sma_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Spread (VWMA − SMA)").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.spread)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Spread ratio").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.spread_ratio)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_vwma_win = open;
        }

        if self.show_stddev_win {
            if self.stddev_win_symbol.is_empty() {
                self.stddev_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_stddev_win;
            egui::Window::new("STDDEV — Rolling Standard Deviation (N=20 + 60-bar regime classifier)")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.stddev_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.stddev_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.stddev_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_stddev(&conn, &sym_u) { self.stddev_win_snapshot = snap; self.stddev_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.stddev_win_symbol.to_uppercase(); self.stddev_win_loading = true; self.stddev_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeStddevSnapshot { symbol: sym });
                        }
                        if self.stddev_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.stddev_win_snapshot;
                    if snap.symbol.is_empty() || snap.regime_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥60 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.regime_label.as_str() {
                            "HIGH_VOL" => DOWN,
                            "LOW_VOL"  => UP,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — σ {:.4} · σ_long {:.4} · annualized {:.4} · cv {:.4} — close {:.4} — as of {}",
                            snap.symbol, snap.regime_label, snap.stddev, snap.stddev_long, snap.annualized, snap.cv,
                            snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("stddev_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Length / Long").small().strong()); ui.label(egui::RichText::new(format!("{} / {}", snap.length, snap.long_length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Mean").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.mean)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Variance").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.variance)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Stddev (N=20)").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.stddev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Stddev (N=60)").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.stddev_long)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Coefficient of variation").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.cv)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Annualized (×√252)").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.annualized)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_stddev_win = open;
        }
    }
}
