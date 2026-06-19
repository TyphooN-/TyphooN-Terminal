use super::*;

impl TyphooNApp {
    pub(super) fn render_research_candlestick_marubozu_line_patterns_windows(
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

        // ── Research Round 77 popup windows ──
        if self.show_cdl_belt_hold_win {
            if self.cdl_belt_hold_win_symbol.is_empty() {
                self.cdl_belt_hold_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_belt_hold_win;
            egui::Window::new("CDLBELTHOLD — Belt Hold")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_belt_hold_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_belt_hold_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_belt_hold_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_belt_hold(&conn, &sym_u) { self.cdl_belt_hold_win_snapshot = snap; self.cdl_belt_hold_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_belt_hold_win_symbol.to_uppercase(); self.cdl_belt_hold_win_loading = true; self.cdl_belt_hold_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlBeltHoldSnapshot { symbol: sym });
                        }
                        if self.cdl_belt_hold_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_belt_hold_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_belt_hold_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥2 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_belt_hold_label.as_str() { "BULLISH_PATTERN" => UP, "BEARISH_PATTERN" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — body {:.1}% — opening_shadow {:.1}% — closing_shadow {:.1}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_belt_hold_label, snap.pattern_value, snap.body_pct_range, snap.opening_shadow_pct, snap.closing_shadow_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_belt_hold_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Body % range").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Opening shadow %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.opening_shadow_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Closing shadow %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.closing_shadow_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_belt_hold_win = open;
        }

        if self.show_cdl_closing_marubozu_win {
            if self.cdl_closing_marubozu_win_symbol.is_empty() {
                self.cdl_closing_marubozu_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_closing_marubozu_win;
            egui::Window::new("CDLCLOSINGMARUBOZU — Closing Marubozu")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_closing_marubozu_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_closing_marubozu_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_closing_marubozu_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_closing_marubozu(&conn, &sym_u) { self.cdl_closing_marubozu_win_snapshot = snap; self.cdl_closing_marubozu_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_closing_marubozu_win_symbol.to_uppercase(); self.cdl_closing_marubozu_win_loading = true; self.cdl_closing_marubozu_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlClosingMarubozuSnapshot { symbol: sym });
                        }
                        if self.cdl_closing_marubozu_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_closing_marubozu_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_closing_marubozu_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥2 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_closing_marubozu_label.as_str() { "BULLISH_PATTERN" => UP, "BEARISH_PATTERN" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — body {:.1}% — opening_shadow {:.1}% — closing_shadow {:.1}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_closing_marubozu_label, snap.pattern_value, snap.body_pct_range, snap.opening_shadow_pct, snap.closing_shadow_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_closing_marubozu_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Body % range").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Opening shadow %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.opening_shadow_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Closing shadow %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.closing_shadow_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_closing_marubozu_win = open;
        }

        if self.show_cdl_high_wave_win {
            if self.cdl_high_wave_win_symbol.is_empty() {
                self.cdl_high_wave_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_high_wave_win;
            egui::Window::new("CDLHIGHWAVE — High Wave")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_high_wave_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_high_wave_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_high_wave_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_high_wave(&conn, &sym_u) { self.cdl_high_wave_win_snapshot = snap; self.cdl_high_wave_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_high_wave_win_symbol.to_uppercase(); self.cdl_high_wave_win_loading = true; self.cdl_high_wave_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlHighWaveSnapshot { symbol: sym });
                        }
                        if self.cdl_high_wave_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_high_wave_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_high_wave_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥2 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_high_wave_label.as_str() { "GREEN_BODY_PATTERN" => UP, "RED_BODY_PATTERN" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — body {:.1}% — upper {:.1}% — lower {:.1}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_high_wave_label, snap.pattern_value, snap.body_pct_range, snap.upper_shadow_pct, snap.lower_shadow_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_high_wave_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Body % range").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Upper shadow %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.upper_shadow_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Lower shadow %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.lower_shadow_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_high_wave_win = open;
        }

        if self.show_cdl_long_line_win {
            if self.cdl_long_line_win_symbol.is_empty() {
                self.cdl_long_line_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_long_line_win;
            egui::Window::new("CDLLONGLINE — Long Line")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_long_line_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_long_line_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_long_line_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_long_line(&conn, &sym_u) { self.cdl_long_line_win_snapshot = snap; self.cdl_long_line_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_long_line_win_symbol.to_uppercase(); self.cdl_long_line_win_loading = true; self.cdl_long_line_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlLongLineSnapshot { symbol: sym });
                        }
                        if self.cdl_long_line_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_long_line_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_long_line_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥2 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_long_line_label.as_str() { "GREEN_BODY_PATTERN" => UP, "RED_BODY_PATTERN" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — body {:.1}% — upper {:.1}% — lower {:.1}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_long_line_label, snap.pattern_value, snap.body_pct_range, snap.upper_shadow_pct, snap.lower_shadow_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_long_line_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Body % range").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Upper shadow %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.upper_shadow_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Lower shadow %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.lower_shadow_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_long_line_win = open;
        }

        if self.show_cdl_short_line_win {
            if self.cdl_short_line_win_symbol.is_empty() {
                self.cdl_short_line_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_short_line_win;
            egui::Window::new("CDLSHORTLINE — Short Line")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_short_line_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_short_line_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_short_line_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_short_line(&conn, &sym_u) { self.cdl_short_line_win_snapshot = snap; self.cdl_short_line_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_short_line_win_symbol.to_uppercase(); self.cdl_short_line_win_loading = true; self.cdl_short_line_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlShortLineSnapshot { symbol: sym });
                        }
                        if self.cdl_short_line_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_short_line_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_short_line_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥2 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_short_line_label.as_str() { "GREEN_BODY_PATTERN" => UP, "RED_BODY_PATTERN" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — body {:.1}% — upper {:.1}% — lower {:.1}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_short_line_label, snap.pattern_value, snap.body_pct_range, snap.upper_shadow_pct, snap.lower_shadow_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_short_line_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Body % range").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Upper shadow %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.upper_shadow_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Lower shadow %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.lower_shadow_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_short_line_win = open;
        }
    }
}
