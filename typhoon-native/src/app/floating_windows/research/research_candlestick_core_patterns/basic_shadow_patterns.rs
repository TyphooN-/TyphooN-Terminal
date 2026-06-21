use super::*;

impl TyphooNApp {
    pub(super) fn render_cdl_basic_shadow_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
        if self.show_cdl_doji_win {
            if self.cdl_doji_win_symbol.is_empty() {
                self.cdl_doji_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_doji_win;
            egui::Window::new("CDLDOJI — Doji candlestick pattern")
                .open(&mut open).resizable(true).default_size([540.0, 270.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_doji_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_doji_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_doji_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_doji(&conn, &sym_u) { self.cdl_doji_win_snapshot = snap; self.cdl_doji_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_doji_win_symbol.to_uppercase(); self.cdl_doji_win_loading = true; self.cdl_doji_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlDojiSnapshot { symbol: sym });
                        }
                        if self.cdl_doji_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_doji_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_doji_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥2 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = if snap.last_bar_match { AXIS_TEXT } else { AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — body {:.1}% — upper {:.1}% — lower {:.1}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_doji_label, snap.pattern_value, snap.body_pct_range, snap.upper_shadow_pct, snap.lower_shadow_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_doji_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Body % of range").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Upper shadow %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.upper_shadow_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Lower shadow %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.lower_shadow_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_doji_win = open;
        }

        if self.show_cdl_hammer_win {
            if self.cdl_hammer_win_symbol.is_empty() {
                self.cdl_hammer_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_hammer_win;
            egui::Window::new("CDLHAMMER — Hammer candlestick (bullish)")
                .open(&mut open).resizable(true).default_size([540.0, 270.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_hammer_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_hammer_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_hammer_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_hammer(&conn, &sym_u) { self.cdl_hammer_win_snapshot = snap; self.cdl_hammer_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_hammer_win_symbol.to_uppercase(); self.cdl_hammer_win_loading = true; self.cdl_hammer_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlHammerSnapshot { symbol: sym });
                        }
                        if self.cdl_hammer_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_hammer_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_hammer_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥2 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = if snap.last_bar_match { UP } else { AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — body {:.1}% — upper {:.1}% — lower {:.1}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_hammer_label, snap.pattern_value, snap.body_pct_range, snap.upper_shadow_pct, snap.lower_shadow_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_hammer_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Body % of range").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Upper shadow %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.upper_shadow_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Lower shadow %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.lower_shadow_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_hammer_win = open;
        }

        if self.show_cdl_shooting_star_win {
            if self.cdl_shooting_star_win_symbol.is_empty() {
                self.cdl_shooting_star_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_shooting_star_win;
            egui::Window::new("CDLSHOOTINGSTAR — Shooting Star (bearish)")
                .open(&mut open).resizable(true).default_size([540.0, 270.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_shooting_star_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_shooting_star_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_shooting_star_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_shooting_star(&conn, &sym_u) { self.cdl_shooting_star_win_snapshot = snap; self.cdl_shooting_star_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_shooting_star_win_symbol.to_uppercase(); self.cdl_shooting_star_win_loading = true; self.cdl_shooting_star_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlShootingStarSnapshot { symbol: sym });
                        }
                        if self.cdl_shooting_star_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_shooting_star_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_shooting_star_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥2 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = if snap.last_bar_match { DOWN } else { AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — body {:.1}% — upper {:.1}% — lower {:.1}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_shooting_star_label, snap.pattern_value, snap.body_pct_range, snap.upper_shadow_pct, snap.lower_shadow_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_shooting_star_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Body % of range").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Upper shadow %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.upper_shadow_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Lower shadow %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.lower_shadow_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_shooting_star_win = open;
        }
    }
}
