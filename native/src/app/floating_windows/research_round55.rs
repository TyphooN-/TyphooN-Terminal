use super::*;

impl TyphooNApp {
    pub(super) fn render_research_round55_windows(&mut self, ctx: &egui::Context) {
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

        if self.show_seb_win {
            if self.seb_win_symbol.is_empty() {
                self.seb_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_seb_win;
            egui::Window::new("SEB — Standard Error Bands (linreg endpoint ± k·SE)")
                .open(&mut open).resizable(true).default_size([580.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.seb_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.seb_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.seb_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_seb(&conn, &sym_u) { self.seb_win_snapshot = snap; self.seb_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.seb_win_symbol.to_uppercase(); self.seb_win_loading = true; self.seb_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeSebSnapshot { symbol: sym });
                        }
                        if self.seb_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.seb_win_snapshot;
                    if snap.symbol.is_empty() || snap.seb_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥22 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.seb_label.as_str() {
                            "ABOVE_BAND" | "UPPER_HALF" => UP,
                            "BELOW_BAND" | "LOWER_HALF" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — mid {:.4} [ {:.4} … {:.4} ] — pos {:.1}% — close {:.4} — as of {}", snap.symbol, snap.seb_label, snap.middle, snap.lower, snap.upper, snap.position_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("seb_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("k·SE").small().strong()); ui.label(egui::RichText::new(format!("{:.1}", snap.num_se)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Upper").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.upper)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Middle (linreg)").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.middle)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Lower").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.lower)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Bandwidth").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.bandwidth)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Position %").small().strong()); ui.label(egui::RichText::new(format!("{:.1}%", snap.position_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_seb_win = open;
        }

        if self.show_imi_win {
            if self.imi_win_symbol.is_empty() {
                self.imi_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_imi_win;
            egui::Window::new("IMI — Chande's Intraday Momentum Index (RSI-style on close − open)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.imi_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.imi_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.imi_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_imi(&conn, &sym_u)
                                    {
                                        self.imi_win_snapshot = snap;
                                        self.imi_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.imi_win_symbol.to_uppercase();
                            self.imi_win_loading = true;
                            self.imi_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeImiSnapshot { symbol: sym });
                        }
                        if self.imi_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.imi_win_snapshot;
                    if snap.symbol.is_empty() || snap.imi_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥16 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.imi_label.as_str() {
                            "OVERBOUGHT" | "BULL" => UP,
                            "OVERSOLD" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — IMI {:.2} — close {:.4} — as of {}",
                                snap.symbol,
                                snap.imi_label,
                                snap.imi_value,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("imi_summary")
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
                                ui.label(egui::RichText::new("ΣUp (c−o > 0)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sum_gains))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ΣDown (c−o < 0)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sum_losses))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("IMI").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.imi_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("IMI prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.imi_prev))
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
            self.show_imi_win = open;
        }

        if self.show_gmma_win {
            if self.gmma_win_symbol.is_empty() {
                self.gmma_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_gmma_win;
            egui::Window::new("GMMA — Guppy Multiple Moving Average (6 short + 6 long EMA groups)")
                .open(&mut open).resizable(true).default_size([620.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.gmma_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.gmma_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.gmma_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_gmma(&conn, &sym_u) { self.gmma_win_snapshot = snap; self.gmma_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.gmma_win_symbol.to_uppercase(); self.gmma_win_loading = true; self.gmma_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeGmmaSnapshot { symbol: sym });
                        }
                        if self.gmma_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.gmma_win_snapshot;
                    if snap.symbol.is_empty() || snap.gmma_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥62 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.gmma_label.as_str() {
                            "STRONG_UPTREND" | "UPTREND" => UP,
                            "STRONG_DOWNTREND" | "DOWNTREND" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — short-avg {:.4} · long-avg {:.4} · gap {:+.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.gmma_label, snap.short_ema_avg, snap.long_ema_avg, snap.group_gap_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("gmma_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Short group (3,5,8,10,12,15)").small().strong()); ui.label(egui::RichText::new(format!("min {:.4} · avg {:.4} · max {:.4}", snap.short_min, snap.short_ema_avg, snap.short_max)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Long group (30,35,40,45,50,60)").small().strong()); ui.label(egui::RichText::new(format!("min {:.4} · avg {:.4} · max {:.4}", snap.long_min, snap.long_ema_avg, snap.long_max)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Short compression").small().strong()); ui.label(egui::RichText::new(format!("{:.2}%", snap.short_compression_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Long compression").small().strong()); ui.label(egui::RichText::new(format!("{:.2}%", snap.long_compression_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Group gap").small().strong()); ui.label(egui::RichText::new(format!("{:+.2}%", snap.group_gap_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_gmma_win = open;
        }

        if self.show_maenv_win {
            if self.maenv_win_symbol.is_empty() {
                self.maenv_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_maenv_win;
            egui::Window::new("MAENV — Moving Average Envelope (SMA₂₀ ± 2.5%)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.maenv_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.maenv_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.maenv_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_maenv(&conn, &sym_u)
                                    {
                                        self.maenv_win_snapshot = snap;
                                        self.maenv_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.maenv_win_symbol.to_uppercase();
                            self.maenv_win_loading = true;
                            self.maenv_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMaenvSnapshot { symbol: sym });
                        }
                        if self.maenv_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.maenv_win_snapshot;
                    if snap.symbol.is_empty() || snap.maenv_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥21 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.maenv_label.as_str() {
                            "ABOVE_BAND" | "UPPER_HALF" => UP,
                            "BELOW_BAND" | "LOWER_HALF" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — mid {:.4} · pos {:.1}% — close {:.4} — as of {}",
                                snap.symbol,
                                snap.maenv_label,
                                snap.middle,
                                snap.position_pct,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("maenv_summary")
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
                                ui.label(egui::RichText::new("Length / band%").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / ±{:.2}%",
                                        snap.length, snap.pct_band
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Upper").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.upper))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Middle (SMA₂₀)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.middle))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Lower").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.lower))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Bandwidth").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}%", snap.bandwidth_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Position within band").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.1}%", snap.position_pct))
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
            self.show_maenv_win = open;
        }

        if self.show_adl_win {
            if self.adl_win_symbol.is_empty() {
                self.adl_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_adl_win;
            egui::Window::new(
                "ADL — Chaikin Accumulation/Distribution Line (cumulative MFM · volume)",
            )
            .open(&mut open)
            .resizable(true)
            .default_size([560.0, 260.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.adl_win_symbol).desired_width(100.0),
                    );
                    if ui.button("Use Chart").clicked() {
                        self.adl_win_symbol = chart_sym_research.clone();
                    }
                    if ui.button("Load Cached").clicked() {
                        if let Some(ref cache) = self.cache {
                            if let Ok(conn) = cache.connection() {
                                let sym_u = self.adl_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) =
                                    typhoon_engine::core::research::get_adl(&conn, &sym_u)
                                {
                                    self.adl_win_snapshot = snap;
                                    self.adl_win_symbol = sym_u;
                                }
                            }
                        }
                    }
                    if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                        let sym = self.adl_win_symbol.to_uppercase();
                        self.adl_win_loading = true;
                        self.adl_win_symbol = sym.clone();
                        let _ = self
                            .broker_tx
                            .send(BrokerCmd::ComputeAdlSnapshot { symbol: sym });
                    }
                    if self.adl_win_loading {
                        ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                    }
                });
                ui.separator();
                let snap = &self.adl_win_snapshot;
                if snap.symbol.is_empty() || snap.adl_label == "INSUFFICIENT_DATA" {
                    ui.label(
                        egui::RichText::new("No data — HP cache needs ≥22 bars.")
                            .color(AXIS_TEXT)
                            .small(),
                    );
                } else {
                    let color = match snap.adl_label.as_str() {
                        "STRONG_ACCUMULATION" | "ACCUMULATION" => UP,
                        "STRONG_DISTRIBUTION" | "DISTRIBUTION" => DOWN,
                        _ => AXIS_TEXT,
                    };
                    ui.label(
                        egui::RichText::new(format!(
                            "{} — {} — ADL {:.2} · slope/bar {:+.2} — close {:.4} — as of {}",
                            snap.symbol,
                            snap.adl_label,
                            snap.adl_value,
                            snap.slope_per_bar,
                            snap.last_close,
                            snap.as_of
                        ))
                        .strong()
                        .color(color),
                    );
                    ui.separator();
                    egui::Grid::new("adl_summary")
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
                            ui.label(egui::RichText::new("ADL value").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:.2}", snap.adl_value))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("ADL prev").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:.2}", snap.adl_prev))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(
                                egui::RichText::new(format!("ADL SMA({})", snap.adl_sma_length))
                                    .small()
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:.2}", snap.adl_sma))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("Slope per bar").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:+.2}", snap.slope_per_bar))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("Price Δ (20-bar)").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:+.2}%", snap.price_delta_pct))
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
            self.show_adl_win = open;
        }

        if self.show_vhf_win {
            if self.vhf_win_symbol.is_empty() {
                self.vhf_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_vhf_win;
            egui::Window::new("VHF — Vertical Horizontal Filter (trending vs ranging, 28-bar)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.vhf_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.vhf_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.vhf_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_vhf(&conn, &sym_u)
                                    {
                                        self.vhf_win_snapshot = snap;
                                        self.vhf_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.vhf_win_symbol.to_uppercase();
                            self.vhf_win_loading = true;
                            self.vhf_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeVhfSnapshot { symbol: sym });
                        }
                        if self.vhf_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.vhf_win_snapshot;
                    if snap.symbol.is_empty() || snap.vhf_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.vhf_label.as_str() {
                            "STRONG_TREND" | "TREND" => UP,
                            "STRONG_RANGING" | "RANGING" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — VHF {:.4} (prev {:.4}) — close {:.4} — as of {}",
                                snap.symbol,
                                snap.vhf_label,
                                snap.vhf_value,
                                snap.vhf_prev,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("vhf_summary")
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
                                ui.label(egui::RichText::new("Σ|Δclose|").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sum_abs_delta))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("VHF").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.vhf_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("VHF prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.vhf_prev))
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
            self.show_vhf_win = open;
        }

        if self.show_vroc_win {
            if self.vroc_win_symbol.is_empty() {
                self.vroc_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_vroc_win;
            egui::Window::new("VROC — Volume Rate of Change (14-bar ROC of volume)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.vroc_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.vroc_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.vroc_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_vroc(&conn, &sym_u)
                                    {
                                        self.vroc_win_snapshot = snap;
                                        self.vroc_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.vroc_win_symbol.to_uppercase();
                            self.vroc_win_loading = true;
                            self.vroc_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeVrocSnapshot { symbol: sym });
                        }
                        if self.vroc_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.vroc_win_snapshot;
                    if snap.symbol.is_empty() || snap.vroc_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥16 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.vroc_label.as_str() {
                            "SURGE" | "ELEVATED" => UP,
                            "COLLAPSE" | "QUIET" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — VROC {:+.2}% (prev {:+.2}%) — close {:.4} — as of {}",
                                snap.symbol,
                                snap.vroc_label,
                                snap.vroc_value,
                                snap.vroc_prev,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("vroc_summary")
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
                                ui.label(egui::RichText::new("Volume now").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.0}", snap.volume_now))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Volume 14 bars ago").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.0}", snap.volume_then))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("VROC").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}%", snap.vroc_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("VROC prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}%", snap.vroc_prev))
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
            self.show_vroc_win = open;
        }

        if self.show_kdj_win {
            if self.kdj_win_symbol.is_empty() {
                self.kdj_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_kdj_win;
            egui::Window::new("KDJ — Chinese Stochastic Variant (%K, %D, J = 3K − 2D)")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.kdj_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.kdj_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.kdj_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_kdj(&conn, &sym_u)
                                    {
                                        self.kdj_win_snapshot = snap;
                                        self.kdj_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.kdj_win_symbol.to_uppercase();
                            self.kdj_win_loading = true;
                            self.kdj_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeKdjSnapshot { symbol: sym });
                        }
                        if self.kdj_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.kdj_win_snapshot;
                    if snap.symbol.is_empty() || snap.kdj_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥14 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.kdj_label.as_str() {
                            "OVERBOUGHT" | "BULL" => UP,
                            "OVERSOLD" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — K {:.2} · D {:.2} · J {:.2} — close {:.4} — as of {}",
                                snap.symbol,
                                snap.kdj_label,
                                snap.k_value,
                                snap.d_value,
                                snap.j_value,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("kdj_summary")
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
                                    egui::RichText::new("Stoch length / smoothing")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {}",
                                        snap.stoch_length, snap.k_smooth
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("RSV (raw stochastic %)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.rsv))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("K (EMA₁/₃ of RSV)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.k_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("D (EMA₁/₃ of K)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.d_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("J (3·K − 2·D)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.j_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("J prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.j_prev))
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
            self.show_kdj_win = open;
        }

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
