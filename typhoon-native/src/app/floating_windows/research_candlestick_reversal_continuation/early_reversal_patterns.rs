use super::*;

impl TyphooNApp {
    pub(super) fn render_cdl_reversal_early_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
        if self.show_cdl_counterattack_win {
            if self.cdl_counterattack_win_symbol.is_empty() {
                self.cdl_counterattack_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_counterattack_win;
            egui::Window::new("CDLCOUNTERATTACK — Counterattack")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_counterattack_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_counterattack_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_counterattack_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_counterattack(&conn, &sym_u) { self.cdl_counterattack_win_snapshot = snap; self.cdl_counterattack_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_counterattack_win_symbol.to_uppercase(); self.cdl_counterattack_win_loading = true; self.cdl_counterattack_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlCounterattackSnapshot { symbol: sym });
                        }
                        if self.cdl_counterattack_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_counterattack_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_counterattack_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥3 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_counterattack_label.as_str() { "BULLISH_PATTERN" => UP, "BEARISH_PATTERN" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — gap_open {:.2}% — close_diff/body {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_counterattack_label, snap.pattern_value, snap.gap_open_pct, snap.close_diff_pct_body, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_counterattack_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prior body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.prior_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Current body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.current_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Gap-open %").small().strong()); ui.label(egui::RichText::new(format!("{:+.2}", snap.gap_open_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Close diff / prior body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.close_diff_pct_body)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_counterattack_win = open;
        }

        if self.show_cdl_homing_pigeon_win {
            if self.cdl_homing_pigeon_win_symbol.is_empty() {
                self.cdl_homing_pigeon_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_homing_pigeon_win;
            egui::Window::new("CDLHOMINGPIGEON — Homing Pigeon")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_homing_pigeon_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_homing_pigeon_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_homing_pigeon_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_homing_pigeon(&conn, &sym_u) { self.cdl_homing_pigeon_win_snapshot = snap; self.cdl_homing_pigeon_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_homing_pigeon_win_symbol.to_uppercase(); self.cdl_homing_pigeon_win_loading = true; self.cdl_homing_pigeon_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlHomingPigeonSnapshot { symbol: sym });
                        }
                        if self.cdl_homing_pigeon_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_homing_pigeon_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_homing_pigeon_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥3 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_homing_pigeon_label.as_str() { "BULLISH_PATTERN" => UP, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — body_ratio {:.3} — inner_margin {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_homing_pigeon_label, snap.pattern_value, snap.body_size_ratio, snap.inner_body_margin_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_homing_pigeon_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prior body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.prior_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Current body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.current_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Body size ratio").small().strong()); ui.label(egui::RichText::new(format!("{:.3}", snap.body_size_ratio)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Inner-body margin %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.inner_body_margin_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_homing_pigeon_win = open;
        }

        if self.show_cdl_in_neck_win {
            if self.cdl_in_neck_win_symbol.is_empty() {
                self.cdl_in_neck_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_in_neck_win;
            egui::Window::new("CDLINNECK — In-Neck")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_in_neck_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_in_neck_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_in_neck_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_in_neck(&conn, &sym_u) { self.cdl_in_neck_win_snapshot = snap; self.cdl_in_neck_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_in_neck_win_symbol.to_uppercase(); self.cdl_in_neck_win_loading = true; self.cdl_in_neck_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlInNeckSnapshot { symbol: sym });
                        }
                        if self.cdl_in_neck_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_in_neck_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_in_neck_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥3 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_in_neck_label.as_str() { "BEARISH_CONTINUATION" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — gap_open {:.2}% — penetration {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_in_neck_label, snap.pattern_value, snap.gap_open_pct, snap.penetration_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_in_neck_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prior body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.prior_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Current body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.current_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Gap-open %").small().strong()); ui.label(egui::RichText::new(format!("{:+.2}", snap.gap_open_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Penetration %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.penetration_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_in_neck_win = open;
        }

        if self.show_cdl_on_neck_win {
            if self.cdl_on_neck_win_symbol.is_empty() {
                self.cdl_on_neck_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_on_neck_win;
            egui::Window::new("CDLONNECK — On-Neck")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_on_neck_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_on_neck_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_on_neck_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_on_neck(&conn, &sym_u) { self.cdl_on_neck_win_snapshot = snap; self.cdl_on_neck_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_on_neck_win_symbol.to_uppercase(); self.cdl_on_neck_win_loading = true; self.cdl_on_neck_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlOnNeckSnapshot { symbol: sym });
                        }
                        if self.cdl_on_neck_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_on_neck_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_on_neck_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥3 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_on_neck_label.as_str() { "BEARISH_CONTINUATION" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — gap_open {:.2}% — close_match {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_on_neck_label, snap.pattern_value, snap.gap_open_pct, snap.close_match_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_on_neck_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prior body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.prior_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Current body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.current_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Gap-open %").small().strong()); ui.label(egui::RichText::new(format!("{:+.2}", snap.gap_open_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Close-match %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.close_match_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_on_neck_win = open;
        }

        if self.show_cdl_thrusting_win {
            if self.cdl_thrusting_win_symbol.is_empty() {
                self.cdl_thrusting_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_thrusting_win;
            egui::Window::new("CDLTHRUSTING — Thrusting")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_thrusting_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_thrusting_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_thrusting_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_thrusting(&conn, &sym_u) { self.cdl_thrusting_win_snapshot = snap; self.cdl_thrusting_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_thrusting_win_symbol.to_uppercase(); self.cdl_thrusting_win_loading = true; self.cdl_thrusting_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlThrustingSnapshot { symbol: sym });
                        }
                        if self.cdl_thrusting_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_thrusting_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_thrusting_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥3 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_thrusting_label.as_str() { "BEARISH_CONTINUATION" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — gap_open {:.2}% — penetration {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_thrusting_label, snap.pattern_value, snap.gap_open_pct, snap.penetration_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_thrusting_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prior body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.prior_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Current body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.current_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Gap-open %").small().strong()); ui.label(egui::RichText::new(format!("{:+.2}", snap.gap_open_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Penetration %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.penetration_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_thrusting_win = open;
        }
    }
}
