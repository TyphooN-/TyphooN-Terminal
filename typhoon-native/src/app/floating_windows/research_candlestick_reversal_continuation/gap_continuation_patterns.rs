use super::*;

impl TyphooNApp {
    pub(super) fn render_cdl_gap_continuation_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
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
    }
}
