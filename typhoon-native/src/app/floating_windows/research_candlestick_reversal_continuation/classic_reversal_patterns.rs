use super::*;

impl TyphooNApp {
    pub(super) fn render_cdl_reversal_classic_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
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
    }
}
