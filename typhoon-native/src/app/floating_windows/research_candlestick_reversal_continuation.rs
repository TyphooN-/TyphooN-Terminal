use super::*;

mod classic_reversal_patterns;
mod early_reversal_patterns;
mod gap_continuation_patterns;
mod shadow_kicking_patterns;

impl TyphooNApp {
    pub(super) fn render_research_candlestick_reversal_continuation_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
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
        self.render_cdl_reversal_early_windows(ctx, &chart_sym_research);

        self.render_cdl_reversal_classic_windows(ctx, &chart_sym_research);

        self.render_cdl_shadow_kicking_windows(ctx, &chart_sym_research);

        self.render_cdl_gap_continuation_windows(ctx, &chart_sym_research);

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
