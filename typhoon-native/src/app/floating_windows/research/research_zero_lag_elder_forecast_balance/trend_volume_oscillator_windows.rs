use super::*;

impl TyphooNApp {
    pub(super) fn render_trend_volume_oscillator_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
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
    }
}
