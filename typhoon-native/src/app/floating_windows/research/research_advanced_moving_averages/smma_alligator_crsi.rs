use super::*;

impl TyphooNApp {
    pub(super) fn render_smma_alligator_crsi_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
        if self.show_smma_win {
            if self.smma_win_symbol.is_empty() {
                self.smma_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_smma_win;
            egui::Window::new("SMMA — Wilder Smoothed Moving Average (α = 1/N)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.smma_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.smma_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.smma_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_smma(&conn, &sym_u)
                                    {
                                        self.smma_win_snapshot = snap;
                                        self.smma_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.smma_win_symbol.to_uppercase();
                            self.smma_win_loading = true;
                            self.smma_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeSmmaSnapshot { symbol: sym });
                        }
                        if self.smma_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.smma_win_snapshot;
                    if snap.symbol.is_empty() || snap.smma_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥16 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.smma_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — SMMA {:.4} — dev {:+.2}% — close {:.4} — as of {}",
                                snap.symbol,
                                snap.smma_label,
                                snap.smma_value,
                                snap.deviation_pct,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("smma_summary")
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
                                ui.label(egui::RichText::new("SMMA").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.smma_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("SMMA prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.smma_prev))
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
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
                        }
                    }
                });
            self.show_smma_win = open;
        }

        if self.show_alligator_win {
            if self.alligator_win_symbol.is_empty() {
                self.alligator_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_alligator_win;
            egui::Window::new("ALLIGATOR — Bill Williams's Alligator (SMMA₁₃⁺⁸ / ₈⁺⁵ / ₅⁺³)")
                .open(&mut open).resizable(true).default_size([620.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.alligator_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.alligator_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.alligator_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_alligator(&conn, &sym_u) { self.alligator_win_snapshot = snap; self.alligator_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.alligator_win_symbol.to_uppercase(); self.alligator_win_loading = true; self.alligator_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeAlligatorSnapshot { symbol: sym });
                        }
                        if self.alligator_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.alligator_win_snapshot;
                    if snap.symbol.is_empty() || snap.alligator_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥23 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.alligator_label.as_str() {
                            "EATING_UP" => UP,
                            "EATING_DOWN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — jaw {:.4} · teeth {:.4} · lips {:.4} — spread {:.2}% — close {:.4} — as of {}", snap.symbol, snap.alligator_label, snap.jaw, snap.teeth, snap.lips, snap.spread_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("alligator_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Jaw (SMMA₁₃⁺⁸)").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.jaw)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Jaw prev").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.jaw_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Teeth (SMMA₈⁺⁵)").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.teeth)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Teeth prev").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.teeth_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Lips (SMMA₅⁺³)").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.lips)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Lips prev").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.lips_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Spread %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}%", snap.spread_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_alligator_win = open;
        }

        if self.show_crsi_win {
            if self.crsi_win_symbol.is_empty() {
                self.crsi_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_crsi_win;
            egui::Window::new("CRSI — Connors RSI (RSI₃ close · RSI₂ streak · pct-rank ROC₁/100)")
                .open(&mut open)
                .resizable(true)
                .default_size([600.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.crsi_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.crsi_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.crsi_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_crsi(&conn, &sym_u)
                                    {
                                        self.crsi_win_snapshot = snap;
                                        self.crsi_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.crsi_win_symbol.to_uppercase();
                            self.crsi_win_loading = true;
                            self.crsi_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCrsiSnapshot { symbol: sym });
                        }
                        if self.crsi_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.crsi_win_snapshot;
                    if snap.symbol.is_empty() || snap.crsi_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥108 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.crsi_label.as_str() {
                            "OVERBOUGHT" | "BULLISH" => UP,
                            "OVERSOLD" | "BEARISH" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — CRSI {:.2} — close {:.4} — as of {}",
                                snap.symbol,
                                snap.crsi_label,
                                snap.crsi_value,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("crsi_summary")
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
                                    egui::RichText::new("RSI length (close)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.rsi_length))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Streak RSI length").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.streak_length))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Rank lookback").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.rank_lookback))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("RSI₃(close)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.rsi_close))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("RSI₂(streak)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.rsi_streak))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Pct-rank ROC").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.percent_rank))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("CRSI").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.crsi_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("CRSI prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.crsi_prev))
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
            self.show_crsi_win = open;
        }
    }
}
