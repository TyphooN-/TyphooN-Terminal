use super::*;

impl TyphooNApp {
    pub(super) fn render_research_linearreg_hilbert_phase_windows(&mut self, ctx: &egui::Context) {
        let chart_sym_research = research_chart_symbol(
            self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
        );

        // ── egui windows ──
        if self.show_linearreg_win {
            if self.linearreg_win_symbol.is_empty() {
                self.linearreg_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_linearreg_win;
            egui::Window::new("LINEARREG — TA-Lib fitted endpoint of 14-bar least-squares close")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.linearreg_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.linearreg_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.linearreg_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_linearreg(&conn, &sym_u) { self.linearreg_win_snapshot = snap; self.linearreg_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.linearreg_win_symbol.to_uppercase(); self.linearreg_win_loading = true; self.linearreg_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeLinearregSnapshot { symbol: sym });
                        }
                        if self.linearreg_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.linearreg_win_snapshot;
                    if snap.symbol.is_empty() || snap.linearreg_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥15 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.linearreg_label.as_str() {
                            "ABOVE_TREND" => UP, "BELOW_TREND" => DOWN, _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — fitted {:.4} (prev {:.4}) — residual {:+.4} ({:+.3}%) — close {:.4} — as of {}",
                            snap.symbol, snap.linearreg_label, snap.fitted, snap.fitted_prev, snap.residual, snap.residual_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("linearreg_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Fitted").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.fitted)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Fitted prev").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.fitted_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Residual").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.residual)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Residual %").small().strong()); ui.label(egui::RichText::new(format!("{:+.3}%", snap.residual_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_linearreg_win = open;
        }

        if self.show_linearreg_angle_win {
            if self.linearreg_angle_win_symbol.is_empty() {
                self.linearreg_angle_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_linearreg_angle_win;
            egui::Window::new("LINEARREG_ANGLE — atan(slope)·180/π of 14-bar fit")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.linearreg_angle_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.linearreg_angle_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.linearreg_angle_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_linearreg_angle(&conn, &sym_u) { self.linearreg_angle_win_snapshot = snap; self.linearreg_angle_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.linearreg_angle_win_symbol.to_uppercase(); self.linearreg_angle_win_loading = true; self.linearreg_angle_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeLinearregAngleSnapshot { symbol: sym });
                        }
                        if self.linearreg_angle_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.linearreg_angle_win_snapshot;
                    if snap.symbol.is_empty() || snap.angle_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥15 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.angle_label.as_str() {
                            "STRONG_UP" | "UP" => UP, "STRONG_DOWN" | "DOWN" => DOWN, _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — slope {:+.6} — angle {:+.3}° (prev {:+.3}°) — close {:.4} — as of {}",
                            snap.symbol, snap.angle_label, snap.slope, snap.angle_deg, snap.angle_deg_prev, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("linearreg_angle_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Slope").small().strong()); ui.label(egui::RichText::new(format!("{:+.6}", snap.slope)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Angle (deg)").small().strong()); ui.label(egui::RichText::new(format!("{:+.3}°", snap.angle_deg)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Angle prev").small().strong()); ui.label(egui::RichText::new(format!("{:+.3}°", snap.angle_deg_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_linearreg_angle_win = open;
        }

        if self.show_ht_dcphase_win {
            if self.ht_dcphase_win_symbol.is_empty() {
                self.ht_dcphase_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ht_dcphase_win;
            egui::Window::new("HT_DCPHASE — Ehlers Hilbert Dominant Cycle Phase (degrees)")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.ht_dcphase_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.ht_dcphase_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.ht_dcphase_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_ht_dcphase(&conn, &sym_u) { self.ht_dcphase_win_snapshot = snap; self.ht_dcphase_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ht_dcphase_win_symbol.to_uppercase(); self.ht_dcphase_win_loading = true; self.ht_dcphase_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeHtDcphaseSnapshot { symbol: sym });
                        }
                        if self.ht_dcphase_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.ht_dcphase_win_snapshot;
                    if snap.symbol.is_empty() || snap.phase_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥64 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.phase_label.as_str() {
                            "CYCLE_BOTTOM" => UP, "CYCLE_TOP" => DOWN, _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — phase {:.2}° (prev {:.2}°) — Δ {:+.2}° — period {:.2} — close {:.4} — as of {}",
                            snap.symbol, snap.phase_label, snap.phase_deg, snap.phase_deg_prev, snap.phase_delta, snap.period, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("ht_dcphase_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Phase (deg)").small().strong()); ui.label(egui::RichText::new(format!("{:.2}°", snap.phase_deg)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Phase prev").small().strong()); ui.label(egui::RichText::new(format!("{:.2}°", snap.phase_deg_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Phase Δ").small().strong()); ui.label(egui::RichText::new(format!("{:+.2}°", snap.phase_delta)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Period").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.period)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_ht_dcphase_win = open;
        }

        if self.show_ht_sine_win {
            if self.ht_sine_win_symbol.is_empty() {
                self.ht_sine_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ht_sine_win;
            egui::Window::new("HT_SINE — Ehlers Sine + Leadsine cycle-turn detector")
                .open(&mut open).resizable(true).default_size([560.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.ht_sine_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.ht_sine_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.ht_sine_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_ht_sine(&conn, &sym_u) { self.ht_sine_win_snapshot = snap; self.ht_sine_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ht_sine_win_symbol.to_uppercase(); self.ht_sine_win_loading = true; self.ht_sine_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeHtSineSnapshot { symbol: sym });
                        }
                        if self.ht_sine_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.ht_sine_win_snapshot;
                    if snap.symbol.is_empty() || snap.sine_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥64 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.sine_label.as_str() {
                            "CYCLE_TURN_UP" | "BULL" => UP, "CYCLE_TURN_DOWN" | "BEAR" => DOWN, _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — sine {:+.3} (prev {:+.3}) — leadsine {:+.3} (prev {:+.3}) — crossover {} — period {:.2} — close {:.4} — as of {}",
                            snap.symbol, snap.sine_label, snap.sine, snap.sine_prev, snap.leadsine, snap.leadsine_prev, snap.crossover, snap.period, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("ht_sine_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Sine").small().strong()); ui.label(egui::RichText::new(format!("{:+.3}", snap.sine)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Sine prev").small().strong()); ui.label(egui::RichText::new(format!("{:+.3}", snap.sine_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Leadsine").small().strong()); ui.label(egui::RichText::new(format!("{:+.3}", snap.leadsine)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Leadsine prev").small().strong()); ui.label(egui::RichText::new(format!("{:+.3}", snap.leadsine_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Crossover").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.crossover)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Period").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.period)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_ht_sine_win = open;
        }

        if self.show_ht_phasor_win {
            if self.ht_phasor_win_symbol.is_empty() {
                self.ht_phasor_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ht_phasor_win;
            egui::Window::new("HT_PHASOR — Ehlers raw I/Q + magnitude + phase")
                .open(&mut open).resizable(true).default_size([560.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.ht_phasor_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.ht_phasor_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.ht_phasor_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_ht_phasor(&conn, &sym_u) { self.ht_phasor_win_snapshot = snap; self.ht_phasor_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ht_phasor_win_symbol.to_uppercase(); self.ht_phasor_win_loading = true; self.ht_phasor_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeHtPhasorSnapshot { symbol: sym });
                        }
                        if self.ht_phasor_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.ht_phasor_win_snapshot;
                    if snap.symbol.is_empty() || snap.phasor_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥64 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.phasor_label.as_str() {
                            "STRONG_CYCLE" => UP, "WEAK_CYCLE" => DOWN, _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — I {:+.4} — Q {:+.4} — magnitude {:.4} — phase {:+.2}° — close {:.4} — as of {}",
                            snap.symbol, snap.phasor_label, snap.i_comp, snap.q_comp, snap.magnitude, snap.phase_deg, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("ht_phasor_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("I (in-phase)").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.i_comp)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("I prev").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.i_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Q (quadrature)").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.q_comp)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Q prev").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.q_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Magnitude").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.magnitude)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Phase (deg)").small().strong()); ui.label(egui::RichText::new(format!("{:+.2}°", snap.phase_deg)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_ht_phasor_win = open;
        }

        if self.show_midprice_win {
            if self.midprice_win_symbol.is_empty() {
                self.midprice_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_midprice_win;
            egui::Window::new("MIDPRICE — (HHV + LLV) / 2 range midpoint (14-bar)")
                .open(&mut open).resizable(true).default_size([560.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.midprice_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.midprice_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.midprice_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_midprice(&conn, &sym_u) { self.midprice_win_snapshot = snap; self.midprice_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.midprice_win_symbol.to_uppercase(); self.midprice_win_loading = true; self.midprice_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeMidpriceSnapshot { symbol: sym });
                        }
                        if self.midprice_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.midprice_win_snapshot;
                    if snap.symbol.is_empty() || snap.midprice_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥15 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.midprice_label.as_str() {
                            "NEAR_HIGH" | "ABOVE_MID" => UP, "NEAR_LOW" | "BELOW_MID" => DOWN, _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — midprice {:.4} — HHV {:.4} — LLV {:.4} — position {:.3} — close {:.4} — as of {}",
                            snap.symbol, snap.midprice_label, snap.midprice, snap.hhv, snap.llv, snap.position, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("midprice_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Midprice").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.midprice)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Midprice prev").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.midprice_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("HHV").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.hhv)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("LLV").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.llv)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Position").small().strong()); ui.label(egui::RichText::new(format!("{:.3}", snap.position)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_midprice_win = open;
        }

        if self.show_apo_win {
            if self.apo_win_symbol.is_empty() {
                self.apo_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_apo_win;
            egui::Window::new("APO — Absolute Price Oscillator (EMA12 − EMA26)")
                .open(&mut open).resizable(true).default_size([560.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.apo_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.apo_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.apo_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_apo(&conn, &sym_u) { self.apo_win_snapshot = snap; self.apo_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.apo_win_symbol.to_uppercase(); self.apo_win_loading = true; self.apo_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeApoSnapshot { symbol: sym });
                        }
                        if self.apo_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.apo_win_snapshot;
                    if snap.symbol.is_empty() || snap.apo_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥27 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.apo_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP, "STRONG_BEAR" | "BEAR" => DOWN, _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — APO {:+.4} — fast_EMA {:.4} — slow_EMA {:.4} — close {:.4} — as of {}",
                            snap.symbol, snap.apo_label, snap.apo, snap.fast_ema, snap.slow_ema, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("apo_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Fast period").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.fast_period)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Slow period").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.slow_period)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("APO").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.apo)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("APO prev").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.apo_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Fast EMA").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.fast_ema)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Slow EMA").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.slow_ema)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_apo_win = open;
        }

        if self.show_mom_win {
            if self.mom_win_symbol.is_empty() {
                self.mom_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_mom_win;
            egui::Window::new("MOM — raw close − close[n−10] momentum")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.mom_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.mom_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.mom_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_mom(&conn, &sym_u)
                                    {
                                        self.mom_win_snapshot = snap;
                                        self.mom_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.mom_win_symbol.to_uppercase();
                            self.mom_win_loading = true;
                            self.mom_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMomSnapshot { symbol: sym });
                        }
                        if self.mom_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.mom_win_snapshot;
                    if snap.symbol.is_empty() || snap.mom_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥12 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.mom_label.as_str() {
                            "STRONG_UP" | "UP" => UP,
                            "STRONG_DOWN" | "DOWN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — MOM {:+.4} — MOM% {:+.3} — close {:.4} — as of {}",
                                snap.symbol,
                                snap.mom_label,
                                snap.mom,
                                snap.mom_pct,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("mom_summary")
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
                                ui.label(egui::RichText::new("Period").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.period))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("MOM").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.mom))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("MOM prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.mom_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("MOM %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.mom_pct))
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
            self.show_mom_win = open;
        }

        if self.show_sarext_win {
            if self.sarext_win_symbol.is_empty() {
                self.sarext_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_sarext_win;
            egui::Window::new("SAREXT — Extended Parabolic SAR (asymmetric long/short AF)")
                .open(&mut open).resizable(true).default_size([620.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.sarext_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.sarext_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.sarext_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_sarext(&conn, &sym_u) { self.sarext_win_snapshot = snap; self.sarext_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.sarext_win_symbol.to_uppercase(); self.sarext_win_loading = true; self.sarext_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeSarextSnapshot { symbol: sym });
                        }
                        if self.sarext_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.sarext_win_snapshot;
                    if snap.symbol.is_empty() || snap.sarext_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥4 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.sarext_label.as_str() {
                            "STRONG_UP" | "UP" => UP, "STRONG_DOWN" | "DOWN" => DOWN, _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — SAR {:.4} — EP {:.4} — AF {:.3} — trend {} — in-trend {} — distance {:+.3}% — close {:.4} — as of {}",
                            snap.symbol, snap.sarext_label, snap.sar_value, snap.extreme_point, snap.acceleration_factor,
                            if snap.trend_is_up { "UP" } else { "DOWN" }, snap.bars_in_trend, snap.distance_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("sarext_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("AF long init").small().strong()); ui.label(egui::RichText::new(format!("{:.3}", snap.af_init_long)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("AF long step").small().strong()); ui.label(egui::RichText::new(format!("{:.3}", snap.af_step_long)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("AF long max").small().strong()); ui.label(egui::RichText::new(format!("{:.3}", snap.af_max_long)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("AF short init").small().strong()); ui.label(egui::RichText::new(format!("{:.3}", snap.af_init_short)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("AF short step").small().strong()); ui.label(egui::RichText::new(format!("{:.3}", snap.af_step_short)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("AF short max").small().strong()); ui.label(egui::RichText::new(format!("{:.3}", snap.af_max_short)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("SAR value").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.sar_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Extreme point").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.extreme_point)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("AF current").small().strong()); ui.label(egui::RichText::new(format!("{:.3}", snap.acceleration_factor)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Trend").small().strong()); ui.label(egui::RichText::new(if snap.trend_is_up { "UP" } else { "DOWN" }).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Bars in trend").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_in_trend)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Distance %").small().strong()); ui.label(egui::RichText::new(format!("{:+.3}", snap.distance_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_sarext_win = open;
        }

        if self.show_adxr_win {
            if self.adxr_win_symbol.is_empty() {
                self.adxr_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_adxr_win;
            egui::Window::new("ADXR — Average Directional Movement Rating (14-bar)")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.adxr_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.adxr_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.adxr_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_adxr(&conn, &sym_u) { self.adxr_win_snapshot = snap; self.adxr_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.adxr_win_symbol.to_uppercase(); self.adxr_win_loading = true; self.adxr_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeAdxrSnapshot { symbol: sym });
                        }
                        if self.adxr_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.adxr_win_snapshot;
                    if snap.symbol.is_empty() || snap.adxr_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥43 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.adxr_label.as_str() {
                            "STRONG_TREND" | "TREND" => UP, "NO_TREND" => DOWN, _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — ADXR {:.3} — ADX now {:.3} — ADX prior {:.3} — close {:.4} — as of {}",
                            snap.symbol, snap.adxr_label, snap.adxr, snap.adx_now, snap.adx_prior, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("adxr_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Period").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.period)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("ADX now").small().strong()); ui.label(egui::RichText::new(format!("{:.3}", snap.adx_now)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("ADX prior").small().strong()); ui.label(egui::RichText::new(format!("{:.3}", snap.adx_prior)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("ADXR").small().strong()); ui.label(egui::RichText::new(format!("{:.3}", snap.adxr)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("ADXR prev").small().strong()); ui.label(egui::RichText::new(format!("{:.3}", snap.adxr_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_adxr_win = open;
        }
    }
}
