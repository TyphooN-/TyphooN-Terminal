use super::*;

impl TyphooNApp {
    pub(super) fn render_cdl_primary_one_bar_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
        if self.show_cdl_dragonfly_doji_win {
            if self.cdl_dragonfly_doji_win_symbol.is_empty() {
                self.cdl_dragonfly_doji_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_dragonfly_doji_win;
            egui::Window::new("CDLDRAGONFLYDOJI — Dragonfly Doji (T-shape support signal)")
                .open(&mut open).resizable(true).default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_dragonfly_doji_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_dragonfly_doji_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_dragonfly_doji_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_dragonfly_doji(&conn, &sym_u) { self.cdl_dragonfly_doji_win_snapshot = snap; self.cdl_dragonfly_doji_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_dragonfly_doji_win_symbol.to_uppercase(); self.cdl_dragonfly_doji_win_loading = true; self.cdl_dragonfly_doji_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlDragonflyDojiSnapshot { symbol: sym });
                        }
                        if self.cdl_dragonfly_doji_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_dragonfly_doji_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_dragonfly_doji_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥2 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_dragonfly_doji_label.as_str() { "BULLISH_PATTERN" => UP, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — body {:.1}% — upper {:.1}% — lower {:.1}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_dragonfly_doji_label, snap.pattern_value, snap.body_pct_range, snap.upper_shadow_pct, snap.lower_shadow_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_dragonfly_doji_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
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
            self.show_cdl_dragonfly_doji_win = open;
        }

        if self.show_cdl_gravestone_doji_win {
            if self.cdl_gravestone_doji_win_symbol.is_empty() {
                self.cdl_gravestone_doji_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_gravestone_doji_win;
            egui::Window::new("CDLGRAVESTONEDOJI — Gravestone Doji (inverted-T resistance signal)")
                .open(&mut open).resizable(true).default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_gravestone_doji_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_gravestone_doji_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_gravestone_doji_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_gravestone_doji(&conn, &sym_u) { self.cdl_gravestone_doji_win_snapshot = snap; self.cdl_gravestone_doji_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_gravestone_doji_win_symbol.to_uppercase(); self.cdl_gravestone_doji_win_loading = true; self.cdl_gravestone_doji_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlGravestoneDojiSnapshot { symbol: sym });
                        }
                        if self.cdl_gravestone_doji_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_gravestone_doji_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_gravestone_doji_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥2 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_gravestone_doji_label.as_str() { "BEARISH_PATTERN" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — body {:.1}% — upper {:.1}% — lower {:.1}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_gravestone_doji_label, snap.pattern_value, snap.body_pct_range, snap.upper_shadow_pct, snap.lower_shadow_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_gravestone_doji_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
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
            self.show_cdl_gravestone_doji_win = open;
        }

        if self.show_cdl_hanging_man_win {
            if self.cdl_hanging_man_win_symbol.is_empty() {
                self.cdl_hanging_man_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_hanging_man_win;
            egui::Window::new("CDLHANGINGMAN — Hanging Man (bearish reversal at tops)")
                .open(&mut open).resizable(true).default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_hanging_man_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_hanging_man_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_hanging_man_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_hanging_man(&conn, &sym_u) { self.cdl_hanging_man_win_snapshot = snap; self.cdl_hanging_man_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_hanging_man_win_symbol.to_uppercase(); self.cdl_hanging_man_win_loading = true; self.cdl_hanging_man_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlHangingManSnapshot { symbol: sym });
                        }
                        if self.cdl_hanging_man_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_hanging_man_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_hanging_man_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥2 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_hanging_man_label.as_str() { "BEARISH_PATTERN" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — body {:.1}% — upper {:.1}% — lower {:.1}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_hanging_man_label, snap.pattern_value, snap.body_pct_range, snap.upper_shadow_pct, snap.lower_shadow_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_hanging_man_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
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
            self.show_cdl_hanging_man_win = open;
        }

        if self.show_cdl_inverted_hammer_win {
            if self.cdl_inverted_hammer_win_symbol.is_empty() {
                self.cdl_inverted_hammer_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_inverted_hammer_win;
            egui::Window::new("CDLINVERTEDHAMMER — Inverted Hammer (bullish reversal at bottoms)")
                .open(&mut open).resizable(true).default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_inverted_hammer_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_inverted_hammer_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_inverted_hammer_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_inverted_hammer(&conn, &sym_u) { self.cdl_inverted_hammer_win_snapshot = snap; self.cdl_inverted_hammer_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_inverted_hammer_win_symbol.to_uppercase(); self.cdl_inverted_hammer_win_loading = true; self.cdl_inverted_hammer_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlInvertedHammerSnapshot { symbol: sym });
                        }
                        if self.cdl_inverted_hammer_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_inverted_hammer_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_inverted_hammer_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥2 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_inverted_hammer_label.as_str() { "BULLISH_PATTERN" => UP, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — body {:.1}% — upper {:.1}% — lower {:.1}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_inverted_hammer_label, snap.pattern_value, snap.body_pct_range, snap.upper_shadow_pct, snap.lower_shadow_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_inverted_hammer_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
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
            self.show_cdl_inverted_hammer_win = open;
        }
    }

    pub(super) fn render_cdl_secondary_one_bar_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
        if self.show_cdl_long_legged_doji_win {
            if self.cdl_long_legged_doji_win_symbol.is_empty() {
                self.cdl_long_legged_doji_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_long_legged_doji_win;
            egui::Window::new("CDLLONGLEGGEDDOJI — Long-Legged Doji (both shadows dominant)")
                .open(&mut open).resizable(true).default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_long_legged_doji_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_long_legged_doji_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_long_legged_doji_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_long_legged_doji(&conn, &sym_u) { self.cdl_long_legged_doji_win_snapshot = snap; self.cdl_long_legged_doji_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_long_legged_doji_win_symbol.to_uppercase(); self.cdl_long_legged_doji_win_loading = true; self.cdl_long_legged_doji_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlLongLeggedDojiSnapshot { symbol: sym });
                        }
                        if self.cdl_long_legged_doji_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_long_legged_doji_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_long_legged_doji_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥2 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_long_legged_doji_label.as_str() { "DOJI_PATTERN" => AXIS_TEXT, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — body {:.1}% — upper {:.1}% — lower {:.1}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_long_legged_doji_label, snap.pattern_value, snap.body_pct_range, snap.upper_shadow_pct, snap.lower_shadow_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_long_legged_doji_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
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
            self.show_cdl_long_legged_doji_win = open;
        }

        if self.show_cdl_marubozu_win {
            if self.cdl_marubozu_win_symbol.is_empty() {
                self.cdl_marubozu_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_marubozu_win;
            egui::Window::new("CDLMARUBOZU — Marubozu (full-body conviction candle)")
                .open(&mut open).resizable(true).default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_marubozu_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_marubozu_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_marubozu_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_marubozu(&conn, &sym_u) { self.cdl_marubozu_win_snapshot = snap; self.cdl_marubozu_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_marubozu_win_symbol.to_uppercase(); self.cdl_marubozu_win_loading = true; self.cdl_marubozu_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlMarubozuSnapshot { symbol: sym });
                        }
                        if self.cdl_marubozu_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_marubozu_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_marubozu_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥2 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_marubozu_label.as_str() { "BULLISH_PATTERN" => UP, "BEARISH_PATTERN" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — body {:.1}% — upper {:.1}% — lower {:.1}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_marubozu_label, snap.pattern_value, snap.body_pct_range, snap.upper_shadow_pct, snap.lower_shadow_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_marubozu_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
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
            self.show_cdl_marubozu_win = open;
        }

        if self.show_cdl_spinning_top_win {
            if self.cdl_spinning_top_win_symbol.is_empty() {
                self.cdl_spinning_top_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_spinning_top_win;
            egui::Window::new("CDLSPINNINGTOP — Spinning Top (small body, shadows dominant)")
                .open(&mut open).resizable(true).default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_spinning_top_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_spinning_top_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_spinning_top_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_spinning_top(&conn, &sym_u) { self.cdl_spinning_top_win_snapshot = snap; self.cdl_spinning_top_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_spinning_top_win_symbol.to_uppercase(); self.cdl_spinning_top_win_loading = true; self.cdl_spinning_top_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlSpinningTopSnapshot { symbol: sym });
                        }
                        if self.cdl_spinning_top_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_spinning_top_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_spinning_top_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥2 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_spinning_top_label.as_str() { "GREEN_BODY_PATTERN" => UP, "RED_BODY_PATTERN" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — body {:.1}% — upper {:.1}% — lower {:.1}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_spinning_top_label, snap.pattern_value, snap.body_pct_range, snap.upper_shadow_pct, snap.lower_shadow_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_spinning_top_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
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
            self.show_cdl_spinning_top_win = open;
        }
    }
}
