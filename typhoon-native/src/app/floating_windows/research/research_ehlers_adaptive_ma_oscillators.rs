use super::*;

impl TyphooNApp {
    pub(super) fn render_research_ehlers_adaptive_ma_oscillators_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research = research_chart_symbol(
            self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
        );

        // ── Research WMA / RAINBOW / MESA_SINE / FRAMA / IBS windows ──

        if self.show_wma_win {
            if self.wma_win_symbol.is_empty() {
                self.wma_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_wma_win;
            egui::Window::new("WMA — Weighted Moving Average (linearly-weighted SMA, N=20)")
                .open(&mut open).resizable(true).default_size([560.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.wma_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.wma_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.wma_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_wma(&conn, &sym_u) { self.wma_win_snapshot = snap; self.wma_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.wma_win_symbol.to_uppercase(); self.wma_win_loading = true; self.wma_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeWmaSnapshot { symbol: sym });
                        }
                        if self.wma_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.wma_win_snapshot;
                    if snap.symbol.is_empty() || snap.wma_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥21 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.wma_label.as_str() {
                            "BULL" | "WEAK_BULL" => UP,
                            "BEAR" | "WEAK_BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — WMA {:.4} · SMA {:.4} · spread {:+.4} ({:+.3}%) — close {:.4} — as of {}",
                            snap.symbol, snap.wma_label, snap.wma_value, snap.sma_value, snap.spread, snap.spread_pct * 100.0,
                            snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("wma_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("WMA value").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.wma_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("WMA prev").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.wma_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("SMA value").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.sma_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Spread (close − WMA)").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.spread)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Spread %").small().strong()); ui.label(egui::RichText::new(format!("{:+.3}%", snap.spread_pct * 100.0)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_wma_win = open;
        }

        if self.show_rainbow_win {
            if self.rainbow_win_symbol.is_empty() {
                self.rainbow_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_rainbow_win;
            egui::Window::new("RAINBOW — Rainbow MA Oscillator (10-level recursive SMA stack)")
                .open(&mut open).resizable(true).default_size([580.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.rainbow_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.rainbow_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.rainbow_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_rainbow(&conn, &sym_u) { self.rainbow_win_snapshot = snap; self.rainbow_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.rainbow_win_symbol.to_uppercase(); self.rainbow_win_loading = true; self.rainbow_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeRainbowSnapshot { symbol: sym });
                        }
                        if self.rainbow_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.rainbow_win_snapshot;
                    if snap.symbol.is_empty() || snap.rainbow_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥22 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.rainbow_label.as_str() {
                            "STRONG_TREND" => UP,
                            "CONSOLIDATING" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — width {:.4} ({:.3}%) · center {:.4} — close {:.4} — as of {}",
                            snap.symbol, snap.rainbow_label, snap.rainbow_width, snap.rainbow_width_pct * 100.0,
                            snap.center_value, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("rainbow_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Levels").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.levels)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Highest level").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.highest_level)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Lowest level").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.lowest_level)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Rainbow width").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.rainbow_width)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Width %").small().strong()); ui.label(egui::RichText::new(format!("{:.3}%", snap.rainbow_width_pct * 100.0)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Center (mean of levels)").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.center_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("r1 / r5 / r10").small().strong()); ui.label(egui::RichText::new(format!("{:.4} / {:.4} / {:.4}", snap.r1, snap.r5, snap.r10)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_rainbow_win = open;
        }

        if self.show_mesa_sine_win {
            if self.mesa_sine_win_symbol.is_empty() {
                self.mesa_sine_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_mesa_sine_win;
            egui::Window::new("MESA_SINE — Ehlers MESA Sine Wave (cycle phase + lead-sine)")
                .open(&mut open).resizable(true).default_size([580.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.mesa_sine_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.mesa_sine_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.mesa_sine_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_mesa_sine(&conn, &sym_u) { self.mesa_sine_win_snapshot = snap; self.mesa_sine_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.mesa_sine_win_symbol.to_uppercase(); self.mesa_sine_win_loading = true; self.mesa_sine_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeMesaSineSnapshot { symbol: sym });
                        }
                        if self.mesa_sine_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.mesa_sine_win_snapshot;
                    if snap.symbol.is_empty() || snap.mesa_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥32 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.mesa_label.as_str() {
                            "CYCLE_BUY" => UP,
                            "CYCLE_SELL" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — sine {:+.4} · lead {:+.4} · phase {:+.4} rad — close {:.4} — as of {}",
                            snap.symbol, snap.mesa_label, snap.sine_value, snap.lead_sine, snap.phase_rad,
                            snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("mesa_sine_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Period (bars)").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.period)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Phase (rad)").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.phase_rad)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Sine value").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.sine_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Sine prev").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.sine_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Lead sine").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.lead_sine)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Lead prev").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.lead_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_mesa_sine_win = open;
        }

        if self.show_frama_win {
            if self.frama_win_symbol.is_empty() {
                self.frama_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_frama_win;
            egui::Window::new("FRAMA — Fractal Adaptive Moving Average (Ehlers, D-driven α)")
                .open(&mut open).resizable(true).default_size([560.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.frama_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.frama_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.frama_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_frama(&conn, &sym_u) { self.frama_win_snapshot = snap; self.frama_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.frama_win_symbol.to_uppercase(); self.frama_win_loading = true; self.frama_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeFramaSnapshot { symbol: sym });
                        }
                        if self.frama_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.frama_win_snapshot;
                    if snap.symbol.is_empty() || snap.frama_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥32 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.frama_label.as_str() {
                            "STRONG_TREND" => UP,
                            "CHOP" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — D {:.4} · α {:.4} · FRAMA {:.4} · spread {:+.4} — close {:.4} — as of {}",
                            snap.symbol, snap.frama_label, snap.fractal_dim, snap.alpha, snap.frama_value, snap.spread,
                            snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("frama_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Fractal dim D").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.fractal_dim)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Alpha α").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.alpha)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("FRAMA value").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.frama_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("FRAMA prev").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.frama_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Spread (close − FRAMA)").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.spread)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_frama_win = open;
        }

        if self.show_ibs_win {
            if self.ibs_win_symbol.is_empty() {
                self.ibs_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ibs_win;
            egui::Window::new("IBS — Internal Bar Strength ((close−low)/(high−low) + 14-bar SMA)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ibs_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ibs_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ibs_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ibs(&conn, &sym_u)
                                    {
                                        self.ibs_win_snapshot = snap;
                                        self.ibs_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ibs_win_symbol.to_uppercase();
                            self.ibs_win_loading = true;
                            self.ibs_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeIbsSnapshot { symbol: sym });
                        }
                        if self.ibs_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.ibs_win_snapshot;
                    if snap.symbol.is_empty() || snap.ibs_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥15 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.ibs_label.as_str() {
                            "OVERBOUGHT" => DOWN,
                            "OVERSOLD" => UP,
                            "BULL" => UP,
                            "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — raw {:.4} · smoothed {:.4} — close {:.4} — as of {}",
                                snap.symbol,
                                snap.ibs_label,
                                snap.ibs_raw,
                                snap.ibs_smoothed,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("ibs_summary")
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
                                ui.label(egui::RichText::new("IBS (raw)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.ibs_raw))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("IBS (smoothed)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.ibs_smoothed))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("IBS prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.ibs_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Last high").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.last_high))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Last low").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.last_low))
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
            self.show_ibs_win = open;
        }
    }
}
