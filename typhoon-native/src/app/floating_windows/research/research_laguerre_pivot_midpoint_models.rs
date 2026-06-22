use super::*;

impl TyphooNApp {
    pub(super) fn render_research_laguerre_pivot_midpoint_models_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research = research_chart_symbol(
            self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
        );

        // ── Research LAGUERRE_RSI / ZIGZAG / PGO / HT_TRENDLINE / MIDPOINT windows ──

        if self.show_laguerre_rsi_win {
            if self.laguerre_rsi_win_symbol.is_empty() {
                self.laguerre_rsi_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_laguerre_rsi_win;
            egui::Window::new("LAGUERRE_RSI — Ehlers 4-stage Laguerre Filter RSI (γ=0.5)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.laguerre_rsi_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.laguerre_rsi_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.laguerre_rsi_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_laguerre_rsi(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.laguerre_rsi_win_snapshot = snap;
                                        self.laguerre_rsi_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.laguerre_rsi_win_symbol.to_uppercase();
                            self.laguerre_rsi_win_loading = true;
                            self.laguerre_rsi_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeLaguerreRsiSnapshot { symbol: sym });
                        }
                        if self.laguerre_rsi_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.laguerre_rsi_win_snapshot;
                    if snap.symbol.is_empty() || snap.lrsi_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥20 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.lrsi_label.as_str() {
                            "OVERBOUGHT" => DOWN,
                            "OVERSOLD" => UP,
                            "BULL" => UP,
                            "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — LRSI {:.4} (prev {:.4}) — close {:.4} — as of {}",
                                snap.symbol,
                                snap.lrsi_label,
                                snap.laguerre_rsi,
                                snap.laguerre_rsi_prev,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("laguerre_rsi_summary")
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
                                ui.label(egui::RichText::new("γ (gamma)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.gamma))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("L0").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.l0))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("L1").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.l1))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("L2").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.l2))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("L3").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.l3))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Laguerre RSI").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.laguerre_rsi))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Laguerre RSI prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.laguerre_rsi_prev))
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
            self.show_laguerre_rsi_win = open;
        }

        if self.show_zigzag_win {
            if self.zigzag_win_symbol.is_empty() {
                self.zigzag_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_zigzag_win;
            egui::Window::new("ZIGZAG — Percent-Threshold Pivot Reversal Detector (5% default)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.zigzag_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.zigzag_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.zigzag_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_zigzag(&conn, &sym_u)
                                    {
                                        self.zigzag_win_snapshot = snap;
                                        self.zigzag_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.zigzag_win_symbol.to_uppercase();
                            self.zigzag_win_loading = true;
                            self.zigzag_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeZigzagSnapshot { symbol: sym });
                        }
                        if self.zigzag_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.zigzag_win_snapshot;
                    if snap.symbol.is_empty() || snap.zigzag_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥10 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.zigzag_label.as_str() {
                            "UP_LEG" => UP,
                            "DOWN_LEG" => DOWN,
                            "AT_REVERSAL" => AXIS_TEXT,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — leg {} — reversal {:.4} — close {:.4} — as of {}",
                                snap.symbol,
                                snap.zigzag_label,
                                snap.current_leg,
                                snap.reversal_level,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("zigzag_summary")
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
                                ui.label(egui::RichText::new("Threshold %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}%", snap.threshold_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Current leg").small().strong());
                                ui.label(
                                    egui::RichText::new(&snap.current_leg).small().monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Last high value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.last_high_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Last high bars ago").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.last_high_bars_ago))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Last low value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.last_low_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Last low bars ago").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.last_low_bars_ago))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Reversal level").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.reversal_level))
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
            self.show_zigzag_win = open;
        }

        if self.show_pgo_win {
            if self.pgo_win_symbol.is_empty() {
                self.pgo_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_pgo_win;
            egui::Window::new("PGO — Pretty Good Oscillator (Mark Johnson, (close−SMA)/EMA(TR), N=14)")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.pgo_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.pgo_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.pgo_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_pgo(&conn, &sym_u) { self.pgo_win_snapshot = snap; self.pgo_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.pgo_win_symbol.to_uppercase(); self.pgo_win_loading = true; self.pgo_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputePgoSnapshot { symbol: sym });
                        }
                        if self.pgo_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.pgo_win_snapshot;
                    if snap.symbol.is_empty() || snap.pgo_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥16 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.pgo_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — PGO {:.4} (prev {:.4}) — SMA {:.4} · ATR {:.4} — close {:.4} — as of {}",
                            snap.symbol, snap.pgo_label, snap.pgo_value, snap.pgo_prev,
                            snap.sma_value, snap.atr_value, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("pgo_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("SMA value").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.sma_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("ATR (EMA of TR)").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.atr_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("PGO value").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.pgo_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("PGO prev").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.pgo_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_pgo_win = open;
        }

        if self.show_ht_trendline_win {
            if self.ht_trendline_win_symbol.is_empty() {
                self.ht_trendline_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ht_trendline_win;
            egui::Window::new("HT_TRENDLINE — Hilbert Instantaneous Trendline (Ehlers, period-adaptive WMA)")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.ht_trendline_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.ht_trendline_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.ht_trendline_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_ht_trendline(&conn, &sym_u) { self.ht_trendline_win_snapshot = snap; self.ht_trendline_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ht_trendline_win_symbol.to_uppercase(); self.ht_trendline_win_loading = true; self.ht_trendline_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeHtTrendlineSnapshot { symbol: sym });
                        }
                        if self.ht_trendline_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.ht_trendline_win_snapshot;
                    if snap.symbol.is_empty() || snap.ht_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥64 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.ht_label.as_str() {
                            "BULL" | "WEAK_BULL" => UP,
                            "BEAR" | "WEAK_BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — period {:.2} — trendline {:.4} (prev {:.4}) — spread {:+.4} ({:+.3}%) — close {:.4} — as of {}",
                            snap.symbol, snap.ht_label, snap.period,
                            snap.trendline_value, snap.trendline_prev,
                            snap.spread, snap.spread_pct * 100.0, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("ht_trendline_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Detected period").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.period)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Trendline").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.trendline_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Trendline prev").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.trendline_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Spread (close − trendline)").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.spread)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Spread %").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}%", snap.spread_pct * 100.0)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_ht_trendline_win = open;
        }

        if self.show_midpoint_win {
            if self.midpoint_win_symbol.is_empty() {
                self.midpoint_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_midpoint_win;
            egui::Window::new("MIDPOINT — (HHV(N) + LLV(N)) / 2 with Close Position (N=14)")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.midpoint_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.midpoint_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.midpoint_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_midpoint(&conn, &sym_u) { self.midpoint_win_snapshot = snap; self.midpoint_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.midpoint_win_symbol.to_uppercase(); self.midpoint_win_loading = true; self.midpoint_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeMidpointSnapshot { symbol: sym });
                        }
                        if self.midpoint_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.midpoint_win_snapshot;
                    if snap.symbol.is_empty() || snap.midpoint_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥15 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.midpoint_label.as_str() {
                            "UPPER" | "NEAR_UPPER" => UP,
                            "LOWER" | "NEAR_LOWER" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — midpoint {:.4} (prev {:.4}) — close pos {:.4} — close {:.4} — as of {}",
                            snap.symbol, snap.midpoint_label, snap.midpoint, snap.midpoint_prev,
                            snap.close_position, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("midpoint_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("HHV").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.hhv)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("LLV").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.llv)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Midpoint").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.midpoint)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Midpoint prev").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.midpoint_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Close position [0-1]").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.close_position)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_midpoint_win = open;
        }
    }
}
