use super::*;

impl TyphooNApp {
    pub(super) fn render_research_round78_windows(&mut self, ctx: &egui::Context) {
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

        // ── Research Round 78 popup windows ──
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

        if self.show_cdl_two_crows_win {
            if self.cdl_two_crows_win_symbol.is_empty() {
                self.cdl_two_crows_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_two_crows_win;
            egui::Window::new("CDL2CROWS — Two Crows")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_two_crows_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_two_crows_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_two_crows_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_two_crows(&conn, &sym_u) { self.cdl_two_crows_win_snapshot = snap; self.cdl_two_crows_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_two_crows_win_symbol.to_uppercase(); self.cdl_two_crows_win_loading = true; self.cdl_two_crows_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlTwoCrowsSnapshot { symbol: sym });
                        }
                        if self.cdl_two_crows_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_two_crows_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_two_crows_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥4 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_two_crows_label.as_str() { "BEARISH_PATTERN" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — gap {:.2}% — penetration {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_two_crows_label, snap.pattern_value, snap.second_gap_pct, snap.third_penetration_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_two_crows_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("First body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.first_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Second gap %").small().strong()); ui.label(egui::RichText::new(format!("{:+.2}", snap.second_gap_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Third penetration %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.third_penetration_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_two_crows_win = open;
        }

        if self.show_cdl_three_line_strike_win {
            if self.cdl_three_line_strike_win_symbol.is_empty() {
                self.cdl_three_line_strike_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_three_line_strike_win;
            egui::Window::new("CDL3LINESTRIKE — Three Line Strike")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_three_line_strike_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_three_line_strike_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_three_line_strike_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_three_line_strike(&conn, &sym_u) { self.cdl_three_line_strike_win_snapshot = snap; self.cdl_three_line_strike_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_three_line_strike_win_symbol.to_uppercase(); self.cdl_three_line_strike_win_loading = true; self.cdl_three_line_strike_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlThreeLineStrikeSnapshot { symbol: sym });
                        }
                        if self.cdl_three_line_strike_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_three_line_strike_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_three_line_strike_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥5 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_three_line_strike_label.as_str() { "BULLISH_PATTERN" => UP, "BEARISH_PATTERN" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — strike_body {:.2}% — strike_vs_first_open {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_three_line_strike_label, snap.pattern_value, snap.strike_body_pct_range, snap.strike_close_vs_first_open_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_three_line_strike_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Avg body 1-3 %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.avg_first_three_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Strike body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.strike_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Strike vs first open %").small().strong()); ui.label(egui::RichText::new(format!("{:+.2}", snap.strike_close_vs_first_open_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_three_line_strike_win = open;
        }

        if self.show_cdl_three_outside_win {
            if self.cdl_three_outside_win_symbol.is_empty() {
                self.cdl_three_outside_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_three_outside_win;
            egui::Window::new("CDL3OUTSIDE — Three Outside")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_three_outside_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_three_outside_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_three_outside_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_three_outside(&conn, &sym_u) { self.cdl_three_outside_win_snapshot = snap; self.cdl_three_outside_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_three_outside_win_symbol.to_uppercase(); self.cdl_three_outside_win_loading = true; self.cdl_three_outside_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlThreeOutsideSnapshot { symbol: sym });
                        }
                        if self.cdl_three_outside_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_three_outside_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_three_outside_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥4 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_three_outside_label.as_str() { "BULLISH_PATTERN" => UP, "BEARISH_PATTERN" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — engulf_ratio {:.3} — confirm {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_three_outside_label, snap.pattern_value, snap.engulf_body_ratio, snap.confirmation_pct_body2, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_three_outside_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("First body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.first_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Engulf ratio").small().strong()); ui.label(egui::RichText::new(format!("{:.3}", snap.engulf_body_ratio)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Confirmation %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.confirmation_pct_body2)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_three_outside_win = open;
        }

        if self.show_cdl_matching_low_win {
            if self.cdl_matching_low_win_symbol.is_empty() {
                self.cdl_matching_low_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_matching_low_win;
            egui::Window::new("CDLMATCHINGLOW — Matching Low")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cdl_matching_low_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.cdl_matching_low_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.cdl_matching_low_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_cdl_matching_low(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.cdl_matching_low_win_snapshot = snap;
                                        self.cdl_matching_low_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_matching_low_win_symbol.to_uppercase();
                            self.cdl_matching_low_win_loading = true;
                            self.cdl_matching_low_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCdlMatchingLowSnapshot { symbol: sym });
                        }
                        if self.cdl_matching_low_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.cdl_matching_low_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_matching_low_label == "INSUFFICIENT_DATA"
                    {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥3 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.cdl_matching_low_label.as_str() {
                            "BULLISH_PATTERN" => UP,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — value {} — close_match {:.2}% — close {:.4} — as of {}",
                                snap.symbol,
                                snap.cdl_matching_low_label,
                                snap.pattern_value,
                                snap.close_match_pct_body,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("cdl_matching_low_summary")
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
                                ui.label(egui::RichText::new("Pattern value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.pattern_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Prev value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.pattern_value_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Prior body %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2}",
                                        snap.prior_body_pct_range
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Current body %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2}",
                                        snap.current_body_pct_range
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Close-match %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2}",
                                        snap.close_match_pct_body
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Last bar match").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.last_bar_match))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Days since pattern").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.days_since_pattern))
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
            self.show_cdl_matching_low_win = open;
        }

        if self.show_cdl_separating_lines_win {
            if self.cdl_separating_lines_win_symbol.is_empty() {
                self.cdl_separating_lines_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_separating_lines_win;
            egui::Window::new("CDLSEPARATINGLINES — Separating Lines")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_separating_lines_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_separating_lines_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_separating_lines_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_separating_lines(&conn, &sym_u) { self.cdl_separating_lines_win_snapshot = snap; self.cdl_separating_lines_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_separating_lines_win_symbol.to_uppercase(); self.cdl_separating_lines_win_loading = true; self.cdl_separating_lines_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlSeparatingLinesSnapshot { symbol: sym });
                        }
                        if self.cdl_separating_lines_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_separating_lines_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_separating_lines_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥3 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_separating_lines_label.as_str() { "BULLISH_CONTINUATION" => UP, "BEARISH_CONTINUATION" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — open_match {:.2}% — continuation {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_separating_lines_label, snap.pattern_value, snap.open_match_pct_body, snap.continuation_pct_body, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_separating_lines_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prior body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.prior_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Current body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.current_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Open-match %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.open_match_pct_body)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Continuation %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.continuation_pct_body)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_separating_lines_win = open;
        }

        if self.show_cdl_stick_sandwich_win {
            if self.cdl_stick_sandwich_win_symbol.is_empty() {
                self.cdl_stick_sandwich_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_stick_sandwich_win;
            egui::Window::new("CDLSTICKSANDWICH — Stick Sandwich")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_stick_sandwich_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_stick_sandwich_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_stick_sandwich_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_stick_sandwich(&conn, &sym_u) { self.cdl_stick_sandwich_win_snapshot = snap; self.cdl_stick_sandwich_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_stick_sandwich_win_symbol.to_uppercase(); self.cdl_stick_sandwich_win_loading = true; self.cdl_stick_sandwich_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlStickSandwichSnapshot { symbol: sym });
                        }
                        if self.cdl_stick_sandwich_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_stick_sandwich_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_stick_sandwich_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥4 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_stick_sandwich_label.as_str() { "BULLISH_PATTERN" => UP, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — close_match {:.2}% — rebound {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_stick_sandwich_label, snap.pattern_value, snap.close_match_pct_body, snap.middle_rebound_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_stick_sandwich_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("First body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.first_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Third body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.third_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Close-match %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.close_match_pct_body)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Middle rebound %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.middle_rebound_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_stick_sandwich_win = open;
        }

        if self.show_cdl_rickshaw_man_win {
            if self.cdl_rickshaw_man_win_symbol.is_empty() {
                self.cdl_rickshaw_man_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_rickshaw_man_win;
            egui::Window::new("CDLRICKSHAWMAN — Rickshaw Man")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_rickshaw_man_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_rickshaw_man_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_rickshaw_man_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_rickshaw_man(&conn, &sym_u) { self.cdl_rickshaw_man_win_snapshot = snap; self.cdl_rickshaw_man_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_rickshaw_man_win_symbol.to_uppercase(); self.cdl_rickshaw_man_win_loading = true; self.cdl_rickshaw_man_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlRickshawManSnapshot { symbol: sym });
                        }
                        if self.cdl_rickshaw_man_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_rickshaw_man_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_rickshaw_man_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥2 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = AXIS_TEXT;
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — body {:.2}% — upper {:.2}% — lower {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_rickshaw_man_label, snap.pattern_value, snap.body_pct_range, snap.upper_shadow_pct, snap.lower_shadow_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_rickshaw_man_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Upper shadow %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.upper_shadow_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Lower shadow %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.lower_shadow_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Midpoint offset %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.body_midpoint_offset_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_rickshaw_man_win = open;
        }

        if self.show_cdl_takuri_win {
            if self.cdl_takuri_win_symbol.is_empty() {
                self.cdl_takuri_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_takuri_win;
            egui::Window::new("CDLTAKURI — Takuri")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_takuri_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_takuri_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_takuri_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_takuri(&conn, &sym_u) { self.cdl_takuri_win_snapshot = snap; self.cdl_takuri_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_takuri_win_symbol.to_uppercase(); self.cdl_takuri_win_loading = true; self.cdl_takuri_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlTakuriSnapshot { symbol: sym });
                        }
                        if self.cdl_takuri_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_takuri_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_takuri_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥2 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_takuri_label.as_str() { "BULLISH_PATTERN" => UP, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — upper {:.2}% — lower {:.2}% — ratio {:.2}x — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_takuri_label, snap.pattern_value, snap.upper_shadow_pct, snap.lower_shadow_pct, snap.lower_to_upper_ratio, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_takuri_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Upper shadow %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.upper_shadow_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Lower shadow %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.lower_shadow_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Lower/upper ratio").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.lower_to_upper_ratio)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_takuri_win = open;
        }

        if self.show_cdl_three_stars_in_south_win {
            if self.cdl_three_stars_in_south_win_symbol.is_empty() {
                self.cdl_three_stars_in_south_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_three_stars_in_south_win;
            egui::Window::new("CDL3STARSINSOUTH — Three Stars In The South")
                .open(&mut open).resizable(true).default_size([580.0, 270.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_three_stars_in_south_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_three_stars_in_south_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_three_stars_in_south_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_three_stars_in_south(&conn, &sym_u) {
                                    self.cdl_three_stars_in_south_win_snapshot = snap;
                                    self.cdl_three_stars_in_south_win_symbol = sym_u;
                                }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_three_stars_in_south_win_symbol.to_uppercase();
                            self.cdl_three_stars_in_south_win_loading = true;
                            self.cdl_three_stars_in_south_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlThreeStarsInSouthSnapshot { symbol: sym });
                        }
                        if self.cdl_three_stars_in_south_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_three_stars_in_south_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_three_stars_in_south_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥4 bars.").color(AXIS_TEXT).small());
                    } else {
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — value {} — body1 {:.2}% — lower1 {:.2}% — body3 {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_three_stars_in_south_label, snap.pattern_value,
                            snap.first_body_pct_range, snap.first_lower_shadow_pct,
                            snap.third_body_pct_range, snap.last_close, snap.as_of
                        )).strong().color(UP));
                        ui.separator();
                        egui::Grid::new("cdl_three_stars_in_south_summary").striped(true).num_columns(2).min_col_width(210.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("First body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.first_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("First lower shadow %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.first_lower_shadow_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Second body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.second_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Third body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.third_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Third inside %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.third_inside_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_three_stars_in_south_win = open;
        }

        if self.show_cdl_identical_three_crows_win {
            if self.cdl_identical_three_crows_win_symbol.is_empty() {
                self.cdl_identical_three_crows_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_identical_three_crows_win;
            egui::Window::new("CDLIDENTICAL3CROWS — Identical Three Crows")
                .open(&mut open).resizable(true).default_size([580.0, 270.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_identical_three_crows_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_identical_three_crows_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_identical_three_crows_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_identical_three_crows(&conn, &sym_u) {
                                    self.cdl_identical_three_crows_win_snapshot = snap;
                                    self.cdl_identical_three_crows_win_symbol = sym_u;
                                }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_identical_three_crows_win_symbol.to_uppercase();
                            self.cdl_identical_three_crows_win_loading = true;
                            self.cdl_identical_three_crows_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlIdenticalThreeCrowsSnapshot { symbol: sym });
                        }
                        if self.cdl_identical_three_crows_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_identical_three_crows_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_identical_three_crows_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥4 bars.").color(AXIS_TEXT).small());
                    } else {
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — value {} — avg body {:.2}% — open-match {:.2}%/{:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_identical_three_crows_label, snap.pattern_value,
                            snap.avg_body_pct_range, snap.open1_vs_close0_pct_body,
                            snap.open2_vs_close1_pct_body, snap.last_close, snap.as_of
                        )).strong().color(DOWN));
                        ui.separator();
                        egui::Grid::new("cdl_identical_three_crows_summary").striped(true).num_columns(2).min_col_width(210.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Average body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.avg_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Open1 vs close0 %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.open1_vs_close0_pct_body)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Open2 vs close1 %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.open2_vs_close1_pct_body)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Total close decline %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.total_close_decline_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_identical_three_crows_win = open;
        }

        if self.show_cdl_kicking_win {
            if self.cdl_kicking_win_symbol.is_empty() {
                self.cdl_kicking_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_kicking_win;
            egui::Window::new("CDLKICKING — Kicking")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_kicking_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_kicking_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_kicking_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_kicking(&conn, &sym_u) {
                                    self.cdl_kicking_win_snapshot = snap;
                                    self.cdl_kicking_win_symbol = sym_u;
                                }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_kicking_win_symbol.to_uppercase();
                            self.cdl_kicking_win_loading = true;
                            self.cdl_kicking_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlKickingSnapshot { symbol: sym });
                        }
                        if self.cdl_kicking_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_kicking_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_kicking_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥3 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_kicking_label.as_str() { "BULLISH_PATTERN" => UP, "BEARISH_PATTERN" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — value {} — gap {:.2}% — body ratio {:.2}x — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_kicking_label, snap.pattern_value,
                            snap.gap_pct_range, snap.second_to_first_body_ratio, snap.last_close, snap.as_of
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_kicking_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("First body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.first_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Second body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.second_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Gap %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.gap_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Body ratio").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.second_to_first_body_ratio)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_kicking_win = open;
        }

        if self.show_cdl_kicking_by_length_win {
            if self.cdl_kicking_by_length_win_symbol.is_empty() {
                self.cdl_kicking_by_length_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_kicking_by_length_win;
            egui::Window::new("CDLKICKINGBYLENGTH — Kicking By Length")
                .open(&mut open).resizable(true).default_size([570.0, 265.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_kicking_by_length_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_kicking_by_length_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_kicking_by_length_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_kicking_by_length(&conn, &sym_u) {
                                    self.cdl_kicking_by_length_win_snapshot = snap;
                                    self.cdl_kicking_by_length_win_symbol = sym_u;
                                }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_kicking_by_length_win_symbol.to_uppercase();
                            self.cdl_kicking_by_length_win_loading = true;
                            self.cdl_kicking_by_length_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlKickingByLengthSnapshot { symbol: sym });
                        }
                        if self.cdl_kicking_by_length_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_kicking_by_length_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_kicking_by_length_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥3 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_kicking_by_length_label.as_str() { "BULLISH_PATTERN" => UP, "BEARISH_PATTERN" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — value {} — gap {:.2}% — dominant {:.2}x ({}) — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_kicking_by_length_label, snap.pattern_value,
                            snap.gap_pct_range, snap.dominant_body_ratio, snap.dominant_side,
                            snap.last_close, snap.as_of
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_kicking_by_length_summary").striped(true).num_columns(2).min_col_width(205.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("First body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.first_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Second body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.second_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Gap %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.gap_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Dominant ratio").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.dominant_body_ratio)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Dominant side").small().strong()); ui.label(egui::RichText::new(&snap.dominant_side).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_kicking_by_length_win = open;
        }

        if self.show_cdl_ladder_bottom_win {
            if self.cdl_ladder_bottom_win_symbol.is_empty() {
                self.cdl_ladder_bottom_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_ladder_bottom_win;
            egui::Window::new("CDLLADDERBOTTOM — Ladder Bottom")
                .open(&mut open).resizable(true).default_size([580.0, 270.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_ladder_bottom_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_ladder_bottom_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_ladder_bottom_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_ladder_bottom(&conn, &sym_u) {
                                    self.cdl_ladder_bottom_win_snapshot = snap;
                                    self.cdl_ladder_bottom_win_symbol = sym_u;
                                }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_ladder_bottom_win_symbol.to_uppercase();
                            self.cdl_ladder_bottom_win_loading = true;
                            self.cdl_ladder_bottom_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlLadderBottomSnapshot { symbol: sym });
                        }
                        if self.cdl_ladder_bottom_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_ladder_bottom_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_ladder_bottom_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥6 bars.").color(AXIS_TEXT).small());
                    } else {
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — value {} — upper4 {:.2}% — breakout {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_ladder_bottom_label, snap.pattern_value,
                            snap.fourth_upper_shadow_pct, snap.breakout_pct_vs_fourth_high,
                            snap.last_close, snap.as_of
                        )).strong().color(UP));
                        ui.separator();
                        egui::Grid::new("cdl_ladder_bottom_summary").striped(true).num_columns(2).min_col_width(210.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Avg first 3 body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.avg_first_three_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Fourth body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.fourth_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Fourth upper shadow %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.fourth_upper_shadow_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Fifth body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.fifth_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Breakout vs fourth high %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.breakout_pct_vs_fourth_high)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_ladder_bottom_win = open;
        }

        if self.show_cdl_unique_three_river_win {
            if self.cdl_unique_three_river_win_symbol.is_empty() {
                self.cdl_unique_three_river_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_unique_three_river_win;
            egui::Window::new("CDLUNIQUE3RIVER — Unique 3 River")
                .open(&mut open).resizable(true).default_size([580.0, 270.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_unique_three_river_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_unique_three_river_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_unique_three_river_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_unique_three_river(&conn, &sym_u) {
                                    self.cdl_unique_three_river_win_snapshot = snap;
                                    self.cdl_unique_three_river_win_symbol = sym_u;
                                }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_unique_three_river_win_symbol.to_uppercase();
                            self.cdl_unique_three_river_win_loading = true;
                            self.cdl_unique_three_river_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlUniqueThreeRiverSnapshot { symbol: sym });
                        }
                        if self.cdl_unique_three_river_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_unique_three_river_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_unique_three_river_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥4 bars.").color(AXIS_TEXT).small());
                    } else {
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — value {} — lower2 {:.2}% — close3-vs-close2 {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_unique_three_river_label, snap.pattern_value,
                            snap.second_lower_shadow_pct, snap.third_close_vs_second_close_pct,
                            snap.last_close, snap.as_of
                        )).strong().color(UP));
                        ui.separator();
                        egui::Grid::new("cdl_unique_three_river_summary").striped(true).num_columns(2).min_col_width(210.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("First body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.first_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Second body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.second_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Second lower shadow %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.second_lower_shadow_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Third body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.third_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Third close vs second close %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.third_close_vs_second_close_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_unique_three_river_win = open;
        }

        if self.show_cdl_advance_block_win {
            if self.cdl_advance_block_win_symbol.is_empty() {
                self.cdl_advance_block_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_advance_block_win;
            egui::Window::new("CDLADVANCEBLOCK — Advance Block")
                .open(&mut open).resizable(true).default_size([580.0, 270.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_advance_block_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_advance_block_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_advance_block_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_advance_block(&conn, &sym_u) {
                                    self.cdl_advance_block_win_snapshot = snap;
                                    self.cdl_advance_block_win_symbol = sym_u;
                                }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_advance_block_win_symbol.to_uppercase();
                            self.cdl_advance_block_win_loading = true;
                            self.cdl_advance_block_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlAdvanceBlockSnapshot { symbol: sym });
                        }
                        if self.cdl_advance_block_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_advance_block_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_advance_block_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥4 bars.").color(AXIS_TEXT).small());
                    } else {
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — value {} — upper3 {:.2}% — close_gain {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_advance_block_label, snap.pattern_value,
                            snap.third_upper_shadow_pct, snap.total_close_gain_pct, snap.last_close, snap.as_of
                        )).strong().color(DOWN));
                        ui.separator();
                        egui::Grid::new("cdl_advance_block_summary").striped(true).num_columns(2).min_col_width(210.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Body1 %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.first_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Body2 %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.second_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Body3 %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.third_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Upper3 %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.third_upper_shadow_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Total close gain %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.total_close_gain_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_advance_block_win = open;
        }

        if self.show_cdl_breakaway_win {
            if self.cdl_breakaway_win_symbol.is_empty() {
                self.cdl_breakaway_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_breakaway_win;
            egui::Window::new("CDLBREAKAWAY — Breakaway")
                .open(&mut open).resizable(true).default_size([580.0, 270.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_breakaway_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_breakaway_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_breakaway_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_breakaway(&conn, &sym_u) {
                                    self.cdl_breakaway_win_snapshot = snap;
                                    self.cdl_breakaway_win_symbol = sym_u;
                                }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_breakaway_win_symbol.to_uppercase();
                            self.cdl_breakaway_win_loading = true;
                            self.cdl_breakaway_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlBreakawaySnapshot { symbol: sym });
                        }
                        if self.cdl_breakaway_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_breakaway_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_breakaway_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥6 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_breakaway_label.as_str() { "BULLISH_PATTERN" => UP, "BEARISH_PATTERN" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — value {} — gap {:.2}% — retrace {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_breakaway_label, snap.pattern_value,
                            snap.initial_gap_pct_range, snap.gap_retracement_pct, snap.last_close, snap.as_of
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_breakaway_summary").striped(true).num_columns(2).min_col_width(210.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Body1 %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.first_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Initial gap %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.initial_gap_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Body5 %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.fifth_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Gap retrace %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.gap_retracement_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_breakaway_win = open;
        }

        if self.show_cdl_gap_side_side_white_win {
            if self.cdl_gap_side_side_white_win_symbol.is_empty() {
                self.cdl_gap_side_side_white_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_gap_side_side_white_win;
            egui::Window::new("CDLGAPSIDESIDEWHITE — Gap Side Side White")
                .open(&mut open).resizable(true).default_size([590.0, 270.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_gap_side_side_white_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_gap_side_side_white_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_gap_side_side_white_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_gap_side_side_white(&conn, &sym_u) {
                                    self.cdl_gap_side_side_white_win_snapshot = snap;
                                    self.cdl_gap_side_side_white_win_symbol = sym_u;
                                }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_gap_side_side_white_win_symbol.to_uppercase();
                            self.cdl_gap_side_side_white_win_loading = true;
                            self.cdl_gap_side_side_white_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlGapSideSideWhiteSnapshot { symbol: sym });
                        }
                        if self.cdl_gap_side_side_white_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_gap_side_side_white_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_gap_side_side_white_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥4 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_gap_side_side_white_label.as_str() { "BULLISH_CONTINUATION" => UP, "BEARISH_CONTINUATION" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — value {} — gap {:.2}% — open_sim {:.2}% — close_sim {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_gap_side_side_white_label, snap.pattern_value,
                            snap.gap_pct_range, snap.open_similarity_pct_body, snap.close_similarity_pct_body,
                            snap.last_close, snap.as_of
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_gap_side_side_white_summary").striped(true).num_columns(2).min_col_width(210.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Gap %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.gap_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Body2 %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.second_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Body3 %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.third_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Open similarity %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.open_similarity_pct_body)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Close similarity %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.close_similarity_pct_body)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_gap_side_side_white_win = open;
        }

        if self.show_cdl_upside_gap_two_crows_win {
            if self.cdl_upside_gap_two_crows_win_symbol.is_empty() {
                self.cdl_upside_gap_two_crows_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_upside_gap_two_crows_win;
            egui::Window::new("CDLUPSIDEGAP2CROWS — Upside Gap Two Crows")
                .open(&mut open).resizable(true).default_size([590.0, 270.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_upside_gap_two_crows_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_upside_gap_two_crows_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_upside_gap_two_crows_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_upside_gap_two_crows(&conn, &sym_u) {
                                    self.cdl_upside_gap_two_crows_win_snapshot = snap;
                                    self.cdl_upside_gap_two_crows_win_symbol = sym_u;
                                }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_upside_gap_two_crows_win_symbol.to_uppercase();
                            self.cdl_upside_gap_two_crows_win_loading = true;
                            self.cdl_upside_gap_two_crows_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlUpsideGapTwoCrowsSnapshot { symbol: sym });
                        }
                        if self.cdl_upside_gap_two_crows_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_upside_gap_two_crows_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_upside_gap_two_crows_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥4 bars.").color(AXIS_TEXT).small());
                    } else {
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — value {} — gap {:.2}% — open3>{} {:.2}% — into_gap {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_upside_gap_two_crows_label, snap.pattern_value,
                            snap.upside_gap_pct_range, "open2", snap.third_open_above_second_pct_body,
                            snap.third_close_into_gap_pct, snap.last_close, snap.as_of
                        )).strong().color(DOWN));
                        ui.separator();
                        egui::Grid::new("cdl_upside_gap_two_crows_summary").striped(true).num_columns(2).min_col_width(210.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Body1 %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.first_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Upside gap %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.upside_gap_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Open3 above open2 %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.third_open_above_second_pct_body)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Close3 into gap %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.third_close_into_gap_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_upside_gap_two_crows_win = open;
        }

        if self.show_cdl_xside_gap_three_methods_win {
            if self.cdl_xside_gap_three_methods_win_symbol.is_empty() {
                self.cdl_xside_gap_three_methods_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_xside_gap_three_methods_win;
            egui::Window::new("CDLXSIDEGAP3METHODS — X-Side Gap Three Methods")
                .open(&mut open).resizable(true).default_size([590.0, 270.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_xside_gap_three_methods_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_xside_gap_three_methods_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_xside_gap_three_methods_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_xside_gap_three_methods(&conn, &sym_u) {
                                    self.cdl_xside_gap_three_methods_win_snapshot = snap;
                                    self.cdl_xside_gap_three_methods_win_symbol = sym_u;
                                }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_xside_gap_three_methods_win_symbol.to_uppercase();
                            self.cdl_xside_gap_three_methods_win_loading = true;
                            self.cdl_xside_gap_three_methods_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlXSideGapThreeMethodsSnapshot { symbol: sym });
                        }
                        if self.cdl_xside_gap_three_methods_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_xside_gap_three_methods_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_xside_gap_three_methods_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥4 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_xside_gap_three_methods_label.as_str() { "BULLISH_CONTINUATION" => UP, "BEARISH_CONTINUATION" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — value {} — gap {:.2}% — fill {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_xside_gap_three_methods_label, snap.pattern_value,
                            snap.gap_pct_range, snap.gap_fill_pct, snap.last_close, snap.as_of
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_xside_gap_three_methods_summary").striped(true).num_columns(2).min_col_width(210.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Gap %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.gap_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Body2 %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.second_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Body3 %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.third_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Gap fill %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.gap_fill_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_xside_gap_three_methods_win = open;
        }

        if self.show_cdl_conceal_baby_swallow_win {
            if self.cdl_conceal_baby_swallow_win_symbol.is_empty() {
                self.cdl_conceal_baby_swallow_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_conceal_baby_swallow_win;
            egui::Window::new("CDLCONCEALBABYSWALL — Conceal Baby Swallow")
                .open(&mut open).resizable(true).default_size([590.0, 270.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_conceal_baby_swallow_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_conceal_baby_swallow_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_conceal_baby_swallow_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_conceal_baby_swallow(&conn, &sym_u) {
                                    self.cdl_conceal_baby_swallow_win_snapshot = snap;
                                    self.cdl_conceal_baby_swallow_win_symbol = sym_u;
                                }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_conceal_baby_swallow_win_symbol.to_uppercase();
                            self.cdl_conceal_baby_swallow_win_loading = true;
                            self.cdl_conceal_baby_swallow_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlConcealBabySwallowSnapshot { symbol: sym });
                        }
                        if self.cdl_conceal_baby_swallow_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_conceal_baby_swallow_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_conceal_baby_swallow_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥5 bars.").color(AXIS_TEXT).small());
                    } else {
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — value {} — upper3 {:.2}% — engulf4 {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_conceal_baby_swallow_label, snap.pattern_value,
                            snap.third_upper_shadow_pct, snap.fourth_range_engulf_pct,
                            snap.last_close, snap.as_of
                        )).strong().color(UP));
                        ui.separator();
                        egui::Grid::new("cdl_conceal_baby_swallow_summary").striped(true).num_columns(2).min_col_width(210.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Body1 %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.first_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Body2 %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.second_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Upper3 %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.third_upper_shadow_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Engulf4 %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.fourth_range_engulf_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_conceal_baby_swallow_win = open;
        }

        if self.show_cdl_hikkake_win {
            if self.cdl_hikkake_win_symbol.is_empty() {
                self.cdl_hikkake_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_hikkake_win;
            egui::Window::new("CDLHIKKAKE — Hikkake")
                .open(&mut open).resizable(true).default_size([590.0, 265.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_hikkake_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_hikkake_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_hikkake_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_hikkake(&conn, &sym_u) {
                                    self.cdl_hikkake_win_snapshot = snap;
                                    self.cdl_hikkake_win_symbol = sym_u;
                                }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_hikkake_win_symbol.to_uppercase();
                            self.cdl_hikkake_win_loading = true;
                            self.cdl_hikkake_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlHikkakeSnapshot { symbol: sym });
                        }
                        if self.cdl_hikkake_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_hikkake_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_hikkake_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥4 bars.").color(AXIS_TEXT).small());
                    } else {
                        let tone = if snap.pattern_value < 0 { DOWN } else { UP };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — value {} — inside {:.2}% — false break {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_hikkake_label, snap.pattern_value,
                            snap.inside_width_pct_mother, snap.false_break_extension_pct,
                            snap.last_close, snap.as_of
                        )).strong().color(tone));
                        ui.separator();
                        egui::Grid::new("cdl_hikkake_summary").striped(true).num_columns(2).min_col_width(210.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Inside width %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.inside_width_pct_mother)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("False break %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.false_break_extension_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Trigger body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.trigger_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_hikkake_win = open;
        }

        if self.show_cdl_hikkake_mod_win {
            if self.cdl_hikkake_mod_win_symbol.is_empty() {
                self.cdl_hikkake_mod_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_hikkake_mod_win;
            egui::Window::new("CDLHIKKAKEMOD — Modified Hikkake")
                .open(&mut open).resizable(true).default_size([600.0, 270.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_hikkake_mod_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_hikkake_mod_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_hikkake_mod_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_hikkake_mod(&conn, &sym_u) {
                                    self.cdl_hikkake_mod_win_snapshot = snap;
                                    self.cdl_hikkake_mod_win_symbol = sym_u;
                                }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_hikkake_mod_win_symbol.to_uppercase();
                            self.cdl_hikkake_mod_win_loading = true;
                            self.cdl_hikkake_mod_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlHikkakeModSnapshot { symbol: sym });
                        }
                        if self.cdl_hikkake_mod_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_hikkake_mod_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_hikkake_mod_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥5 bars.").color(AXIS_TEXT).small());
                    } else {
                        let tone = if snap.pattern_value < 0 { DOWN } else { UP };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — value {} — false break {:.2}% — confirm {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_hikkake_mod_label, snap.pattern_value,
                            snap.false_break_extension_pct, snap.confirmation_extension_pct,
                            snap.last_close, snap.as_of
                        )).strong().color(tone));
                        ui.separator();
                        egui::Grid::new("cdl_hikkake_mod_summary").striped(true).num_columns(2).min_col_width(210.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Inside width %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.inside_width_pct_mother)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("False break %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.false_break_extension_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Confirmation %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.confirmation_extension_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_hikkake_mod_win = open;
        }

        if self.show_cdl_mat_hold_win {
            if self.cdl_mat_hold_win_symbol.is_empty() {
                self.cdl_mat_hold_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_mat_hold_win;
            egui::Window::new("CDLMATHOLD — Mat Hold")
                .open(&mut open).resizable(true).default_size([605.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_mat_hold_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_mat_hold_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_mat_hold_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_mat_hold(&conn, &sym_u) {
                                    self.cdl_mat_hold_win_snapshot = snap;
                                    self.cdl_mat_hold_win_symbol = sym_u;
                                }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_mat_hold_win_symbol.to_uppercase();
                            self.cdl_mat_hold_win_loading = true;
                            self.cdl_mat_hold_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlMatHoldSnapshot { symbol: sym });
                        }
                        if self.cdl_mat_hold_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_mat_hold_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_mat_hold_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥5 bars.").color(AXIS_TEXT).small());
                    } else {
                        let tone = if snap.pattern_value < 0 { DOWN } else { UP };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — value {} — gap {:.2}% — hold depth {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_mat_hold_label, snap.pattern_value,
                            snap.initial_gap_pct_range, snap.hold_depth_pct_body,
                            snap.last_close, snap.as_of
                        )).strong().color(tone));
                        ui.separator();
                        egui::Grid::new("cdl_mat_hold_summary").striped(true).num_columns(2).min_col_width(210.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("First body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.first_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Middle avg body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.middle_avg_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Initial gap %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.initial_gap_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Hold depth %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.hold_depth_pct_body)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Final body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.final_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_mat_hold_win = open;
        }

        if self.show_cdl_rise_fall_three_methods_win {
            if self.cdl_rise_fall_three_methods_win_symbol.is_empty() {
                self.cdl_rise_fall_three_methods_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_rise_fall_three_methods_win;
            egui::Window::new("CDLRISEFALL3METHODS — Rising/Falling Three Methods")
                .open(&mut open).resizable(true).default_size([620.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_rise_fall_three_methods_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_rise_fall_three_methods_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_rise_fall_three_methods_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_rise_fall_three_methods(&conn, &sym_u) {
                                    self.cdl_rise_fall_three_methods_win_snapshot = snap;
                                    self.cdl_rise_fall_three_methods_win_symbol = sym_u;
                                }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_rise_fall_three_methods_win_symbol.to_uppercase();
                            self.cdl_rise_fall_three_methods_win_loading = true;
                            self.cdl_rise_fall_three_methods_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlRiseFallThreeMethodsSnapshot { symbol: sym });
                        }
                        if self.cdl_rise_fall_three_methods_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_rise_fall_three_methods_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_rise_fall_three_methods_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥5 bars.").color(AXIS_TEXT).small());
                    } else {
                        let tone = if snap.pattern_value < 0 { DOWN } else { UP };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — value {} — contain {:.2}% — body5 {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_rise_fall_three_methods_label, snap.pattern_value,
                            snap.containment_pct_body, snap.final_body_pct_range,
                            snap.last_close, snap.as_of
                        )).strong().color(tone));
                        ui.separator();
                        egui::Grid::new("cdl_rise_fall_three_methods_summary").striped(true).num_columns(2).min_col_width(210.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("First body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.first_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Middle avg body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.middle_avg_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Containment %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.containment_pct_body)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Final body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.final_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_rise_fall_three_methods_win = open;
        }

        if self.show_cdl_stalled_pattern_win {
            if self.cdl_stalled_pattern_win_symbol.is_empty() {
                self.cdl_stalled_pattern_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_stalled_pattern_win;
            egui::Window::new("CDLSTALLEDPATTERN — Stalled Pattern")
                .open(&mut open).resizable(true).default_size([615.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_stalled_pattern_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_stalled_pattern_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_stalled_pattern_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_stalled_pattern(&conn, &sym_u) {
                                    self.cdl_stalled_pattern_win_snapshot = snap;
                                    self.cdl_stalled_pattern_win_symbol = sym_u;
                                }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_stalled_pattern_win_symbol.to_uppercase();
                            self.cdl_stalled_pattern_win_loading = true;
                            self.cdl_stalled_pattern_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlStalledPatternSnapshot { symbol: sym });
                        }
                        if self.cdl_stalled_pattern_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_stalled_pattern_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_stalled_pattern_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥3 bars.").color(AXIS_TEXT).small());
                    } else {
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — value {} — gap3 {:.2}% — upper3 {:.2}% — progress {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_stalled_pattern_label, snap.pattern_value,
                            snap.third_open_gap_pct_range, snap.third_upper_shadow_pct,
                            snap.close_progress_pct_prev_leg, snap.last_close, snap.as_of
                        )).strong().color(DOWN));
                        ui.separator();
                        egui::Grid::new("cdl_stalled_pattern_summary").striped(true).num_columns(2).min_col_width(210.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("First body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.first_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Second body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.second_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Third body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.third_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Third gap %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.third_open_gap_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Upper shadow 3 %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.third_upper_shadow_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Progress vs prev leg %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.close_progress_pct_prev_leg)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_stalled_pattern_win = open;
        }

        if self.show_cdl_tasuki_gap_win {
            if self.cdl_tasuki_gap_win_symbol.is_empty() {
                self.cdl_tasuki_gap_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_tasuki_gap_win;
            egui::Window::new("CDLTASUKIGAP — Tasuki Gap")
                .open(&mut open).resizable(true).default_size([610.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_tasuki_gap_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_tasuki_gap_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_tasuki_gap_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_tasuki_gap(&conn, &sym_u) {
                                    self.cdl_tasuki_gap_win_snapshot = snap;
                                    self.cdl_tasuki_gap_win_symbol = sym_u;
                                }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_tasuki_gap_win_symbol.to_uppercase();
                            self.cdl_tasuki_gap_win_loading = true;
                            self.cdl_tasuki_gap_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlTasukiGapSnapshot { symbol: sym });
                        }
                        if self.cdl_tasuki_gap_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_tasuki_gap_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_tasuki_gap_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥3 bars.").color(AXIS_TEXT).small());
                    } else {
                        let tone = if snap.pattern_value < 0 { DOWN } else { UP };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — value {} — gap {:.2}% — gap fill {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_tasuki_gap_label, snap.pattern_value,
                            snap.gap_pct_range, snap.gap_fill_pct, snap.last_close, snap.as_of
                        )).strong().color(tone));
                        ui.separator();
                        egui::Grid::new("cdl_tasuki_gap_summary").striped(true).num_columns(2).min_col_width(210.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("First body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.first_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Second body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.second_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Third body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.third_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Gap %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.gap_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Gap fill %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.gap_fill_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Open3 % in body2").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.third_open_pct_second_body)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_tasuki_gap_win = open;
        }
    }
}
