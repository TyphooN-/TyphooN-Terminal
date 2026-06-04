use super::*;

impl TyphooNApp {
    pub(super) fn render_research_round67_windows(&mut self, ctx: &egui::Context) {
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

        // ── Research Round 67: PLUS_DI / MINUS_DI / PLUS_DM / MINUS_DM / DX ──
        if self.show_plus_di_win {
            if self.plus_di_win_symbol.is_empty() {
                self.plus_di_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_plus_di_win;
            egui::Window::new("PLUS_DI — Wilder +DI (period 14)")
                .open(&mut open).resizable(true).default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.plus_di_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.plus_di_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.plus_di_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_plus_di(&conn, &sym_u) { self.plus_di_win_snapshot = snap; self.plus_di_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.plus_di_win_symbol.to_uppercase(); self.plus_di_win_loading = true; self.plus_di_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputePlusDiSnapshot { symbol: sym });
                        }
                        if self.plus_di_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.plus_di_win_snapshot;
                    if snap.symbol.is_empty() || snap.plus_di_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥16 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.plus_di_label.as_str() {
                            "BULL_DOMINANT" | "BULL_LEAN" => UP,
                            "BEAR_LEAN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — +DI {:.3} — -DI {:.3} — ATR {:.4} — close {:.4} — as of {}",
                            snap.symbol, snap.plus_di_label, snap.plus_di, snap.minus_di, snap.atr, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("plus_di_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Period").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.period)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("+DI").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.plus_di)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("+DI prev").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.plus_di_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("−DI (ref)").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.minus_di)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("ATR").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.atr)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_plus_di_win = open;
        }

        if self.show_minus_di_win {
            if self.minus_di_win_symbol.is_empty() {
                self.minus_di_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_minus_di_win;
            egui::Window::new("MINUS_DI — Wilder −DI (period 14)")
                .open(&mut open).resizable(true).default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.minus_di_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.minus_di_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.minus_di_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_minus_di(&conn, &sym_u) { self.minus_di_win_snapshot = snap; self.minus_di_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.minus_di_win_symbol.to_uppercase(); self.minus_di_win_loading = true; self.minus_di_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeMinusDiSnapshot { symbol: sym });
                        }
                        if self.minus_di_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.minus_di_win_snapshot;
                    if snap.symbol.is_empty() || snap.minus_di_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥16 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.minus_di_label.as_str() {
                            "BEAR_DOMINANT" | "BEAR_LEAN" => DOWN,
                            "BULL_LEAN" => UP,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — -DI {:.3} — +DI {:.3} — ATR {:.4} — close {:.4} — as of {}",
                            snap.symbol, snap.minus_di_label, snap.minus_di, snap.plus_di, snap.atr, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("minus_di_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Period").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.period)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("−DI").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.minus_di)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("−DI prev").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.minus_di_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("+DI (ref)").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.plus_di)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("ATR").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.atr)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_minus_di_win = open;
        }

        if self.show_plus_dm_win {
            if self.plus_dm_win_symbol.is_empty() {
                self.plus_dm_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_plus_dm_win;
            egui::Window::new("PLUS_DM — Wilder raw +DM (period 14)")
                .open(&mut open).resizable(true).default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.plus_dm_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.plus_dm_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.plus_dm_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_plus_dm(&conn, &sym_u) { self.plus_dm_win_snapshot = snap; self.plus_dm_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.plus_dm_win_symbol.to_uppercase(); self.plus_dm_win_loading = true; self.plus_dm_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputePlusDmSnapshot { symbol: sym });
                        }
                        if self.plus_dm_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.plus_dm_win_snapshot;
                    if snap.symbol.is_empty() || snap.plus_dm_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥16 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.plus_dm_label.as_str() {
                            "BULL_PRESSURE" | "BULL_SOFT" => UP,
                            "BEAR_PRESSURE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — +DM raw {:.4} — +DM smoothed {:.4} — close {:.4} — as of {}",
                            snap.symbol, snap.plus_dm_label, snap.plus_dm_raw, snap.plus_dm_smoothed, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("plus_dm_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Period").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.period)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("+DM raw").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.plus_dm_raw)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("+DM smoothed").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.plus_dm_smoothed)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("+DM smoothed prev").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.plus_dm_smoothed_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Up-move (H − H_prev)").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.up_move)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Down-move (L_prev − L)").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.down_move)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_plus_dm_win = open;
        }

        if self.show_minus_dm_win {
            if self.minus_dm_win_symbol.is_empty() {
                self.minus_dm_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_minus_dm_win;
            egui::Window::new("MINUS_DM — Wilder raw −DM (period 14)")
                .open(&mut open).resizable(true).default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.minus_dm_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.minus_dm_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.minus_dm_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_minus_dm(&conn, &sym_u) { self.minus_dm_win_snapshot = snap; self.minus_dm_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.minus_dm_win_symbol.to_uppercase(); self.minus_dm_win_loading = true; self.minus_dm_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeMinusDmSnapshot { symbol: sym });
                        }
                        if self.minus_dm_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.minus_dm_win_snapshot;
                    if snap.symbol.is_empty() || snap.minus_dm_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥16 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.minus_dm_label.as_str() {
                            "BEAR_PRESSURE" | "BEAR_SOFT" => DOWN,
                            "BULL_PRESSURE" => UP,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — −DM raw {:.4} — −DM smoothed {:.4} — close {:.4} — as of {}",
                            snap.symbol, snap.minus_dm_label, snap.minus_dm_raw, snap.minus_dm_smoothed, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("minus_dm_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Period").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.period)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("−DM raw").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.minus_dm_raw)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("−DM smoothed").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.minus_dm_smoothed)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("−DM smoothed prev").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.minus_dm_smoothed_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Up-move (H − H_prev)").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.up_move)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Down-move (L_prev − L)").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.down_move)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_minus_dm_win = open;
        }

        if self.show_dx_win {
            if self.dx_win_symbol.is_empty() {
                self.dx_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_dx_win;
            egui::Window::new("DX — Wilder Directional Movement Index (period 14)")
                .open(&mut open).resizable(true).default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.dx_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.dx_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.dx_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_dx(&conn, &sym_u) { self.dx_win_snapshot = snap; self.dx_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.dx_win_symbol.to_uppercase(); self.dx_win_loading = true; self.dx_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeDxSnapshot { symbol: sym });
                        }
                        if self.dx_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.dx_win_snapshot;
                    if snap.symbol.is_empty() || snap.dx_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥16 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.dx_label.as_str() {
                            "STRONG_DIR" | "DIR" => if snap.plus_di >= snap.minus_di { UP } else { DOWN },
                            "NO_DIR" => AXIS_TEXT,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — DX {:.3} — +DI {:.3} — -DI {:.3} — close {:.4} — as of {}",
                            snap.symbol, snap.dx_label, snap.dx, snap.plus_di, snap.minus_di, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("dx_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Period").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.period)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("DX").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.dx)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("DX prev").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.dx_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("+DI").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.plus_di)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("−DI").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.minus_di)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_dx_win = open;
        }
    }
}
