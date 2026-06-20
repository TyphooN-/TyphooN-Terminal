use super::*;

mod basic_shadow_patterns;
mod cloud_piercing_patterns;
mod engulfing_harami_patterns;
mod morning_evening_star_patterns;
mod three_soldiers_crows_patterns;

impl TyphooNApp {
    pub(super) fn render_research_candlestick_core_patterns_windows(
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

        // ── Research Round 72 CDL* windows ─────────────────────────────────
        self.render_cdl_basic_shadow_windows(ctx, &chart_sym_research);

        self.render_cdl_engulfing_harami_windows(ctx, &chart_sym_research);

        self.render_cdl_morning_evening_star_windows(ctx, &chart_sym_research);

        self.render_cdl_three_soldiers_crows_windows(ctx, &chart_sym_research);

        self.render_cdl_cloud_piercing_windows(ctx, &chart_sym_research);

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

        if self.show_cdl_harami_cross_win {
            if self.cdl_harami_cross_win_symbol.is_empty() {
                self.cdl_harami_cross_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_harami_cross_win;
            egui::Window::new("CDLHARAMICROSS — Harami Cross (2-bar reversal with inside doji)")
                .open(&mut open).resizable(true).default_size([560.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_harami_cross_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_harami_cross_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_harami_cross_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_harami_cross(&conn, &sym_u) { self.cdl_harami_cross_win_snapshot = snap; self.cdl_harami_cross_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_harami_cross_win_symbol.to_uppercase(); self.cdl_harami_cross_win_loading = true; self.cdl_harami_cross_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlHaramiCrossSnapshot { symbol: sym });
                        }
                        if self.cdl_harami_cross_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_harami_cross_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_harami_cross_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥3 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_harami_cross_label.as_str() { "BULLISH_PATTERN" => UP, "BEARISH_PATTERN" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — prior {:.1}% — cur {:.1}% — ratio {:.3} — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_harami_cross_label, snap.pattern_value, snap.prior_body_pct_range, snap.current_body_pct_range, snap.body_size_ratio, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_harami_cross_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prior body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.prior_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Current body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.current_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Body size ratio").small().strong()); ui.label(egui::RichText::new(format!("{:.3}", snap.body_size_ratio)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_harami_cross_win = open;
        }

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

        if self.show_cdl_tristar_win {
            if self.cdl_tristar_win_symbol.is_empty() {
                self.cdl_tristar_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_tristar_win;
            egui::Window::new("CDLTRISTAR — Tri-Star (3-bar triple-doji reversal)")
                .open(&mut open).resizable(true).default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_tristar_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_tristar_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_tristar_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_tristar(&conn, &sym_u) { self.cdl_tristar_win_snapshot = snap; self.cdl_tristar_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_tristar_win_symbol.to_uppercase(); self.cdl_tristar_win_loading = true; self.cdl_tristar_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlTristarSnapshot { symbol: sym });
                        }
                        if self.cdl_tristar_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_tristar_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_tristar_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥4 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_tristar_label.as_str() { "BULLISH_PATTERN" => UP, "BEARISH_PATTERN" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — avg_body {:.1}% — mid_gap {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_tristar_label, snap.pattern_value, snap.avg_body_pct_range, snap.middle_gap_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_tristar_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Avg body % of range").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.avg_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Middle gap %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.middle_gap_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_tristar_win = open;
        }

        if self.show_cdl_doji_star_win {
            if self.cdl_doji_star_win_symbol.is_empty() {
                self.cdl_doji_star_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_doji_star_win;
            egui::Window::new("CDLDOJISTAR — Doji Star (2-bar reversal precursor)")
                .open(&mut open).resizable(true).default_size([560.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_doji_star_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_doji_star_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_doji_star_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_doji_star(&conn, &sym_u) { self.cdl_doji_star_win_snapshot = snap; self.cdl_doji_star_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_doji_star_win_symbol.to_uppercase(); self.cdl_doji_star_win_loading = true; self.cdl_doji_star_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlDojiStarSnapshot { symbol: sym });
                        }
                        if self.cdl_doji_star_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_doji_star_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_doji_star_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥3 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_doji_star_label.as_str() { "BULLISH_PATTERN" => UP, "BEARISH_PATTERN" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — prior {:.1}% — cur {:.1}% — gap {:+.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_doji_star_label, snap.pattern_value, snap.prior_body_pct_range, snap.current_body_pct_range, snap.gap_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_doji_star_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prior body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.prior_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Current body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.current_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Gap %").small().strong()); ui.label(egui::RichText::new(format!("{:+.2}", snap.gap_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_doji_star_win = open;
        }

        if self.show_cdl_morning_doji_star_win {
            if self.cdl_morning_doji_star_win_symbol.is_empty() {
                self.cdl_morning_doji_star_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_morning_doji_star_win;
            egui::Window::new("CDLMORNINGDOJISTAR — Morning Doji Star (3-bar bullish reversal)")
                .open(&mut open).resizable(true).default_size([560.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_morning_doji_star_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_morning_doji_star_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_morning_doji_star_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_morning_doji_star(&conn, &sym_u) { self.cdl_morning_doji_star_win_snapshot = snap; self.cdl_morning_doji_star_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_morning_doji_star_win_symbol.to_uppercase(); self.cdl_morning_doji_star_win_loading = true; self.cdl_morning_doji_star_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlMorningDojiStarSnapshot { symbol: sym });
                        }
                        if self.cdl_morning_doji_star_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_morning_doji_star_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_morning_doji_star_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥4 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_morning_doji_star_label.as_str() { "BULLISH_PATTERN" => UP, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — b1 {:.1}% — b2 {:.1}% — b3_vs_mid {:+.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_morning_doji_star_label, snap.pattern_value, snap.bar1_body_pct_range, snap.bar2_body_pct_range, snap.bar3_close_vs_bar1_mid_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_morning_doji_star_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Bar 1 body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.bar1_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Bar 2 body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.bar2_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Bar 3 vs bar-1 mid %").small().strong()); ui.label(egui::RichText::new(format!("{:+.2}", snap.bar3_close_vs_bar1_mid_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_morning_doji_star_win = open;
        }

        if self.show_cdl_evening_doji_star_win {
            if self.cdl_evening_doji_star_win_symbol.is_empty() {
                self.cdl_evening_doji_star_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_evening_doji_star_win;
            egui::Window::new("CDLEVENINGDOJISTAR — Evening Doji Star (3-bar bearish reversal)")
                .open(&mut open).resizable(true).default_size([560.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_evening_doji_star_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_evening_doji_star_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_evening_doji_star_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_evening_doji_star(&conn, &sym_u) { self.cdl_evening_doji_star_win_snapshot = snap; self.cdl_evening_doji_star_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_evening_doji_star_win_symbol.to_uppercase(); self.cdl_evening_doji_star_win_loading = true; self.cdl_evening_doji_star_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlEveningDojiStarSnapshot { symbol: sym });
                        }
                        if self.cdl_evening_doji_star_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_evening_doji_star_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_evening_doji_star_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥4 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_evening_doji_star_label.as_str() { "BEARISH_PATTERN" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — b1 {:.1}% — b2 {:.1}% — b3_vs_mid {:+.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_evening_doji_star_label, snap.pattern_value, snap.bar1_body_pct_range, snap.bar2_body_pct_range, snap.bar3_close_vs_bar1_mid_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_evening_doji_star_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Bar 1 body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.bar1_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Bar 2 body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.bar2_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Bar 3 vs bar-1 mid %").small().strong()); ui.label(egui::RichText::new(format!("{:+.2}", snap.bar3_close_vs_bar1_mid_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_evening_doji_star_win = open;
        }

        if self.show_cdl_abandoned_baby_win {
            if self.cdl_abandoned_baby_win_symbol.is_empty() {
                self.cdl_abandoned_baby_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_abandoned_baby_win;
            egui::Window::new("CDLABANDONEDBABY — Abandoned Baby (strongest 3-bar star variant)")
                .open(&mut open).resizable(true).default_size([560.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_abandoned_baby_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_abandoned_baby_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_abandoned_baby_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_abandoned_baby(&conn, &sym_u) { self.cdl_abandoned_baby_win_snapshot = snap; self.cdl_abandoned_baby_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_abandoned_baby_win_symbol.to_uppercase(); self.cdl_abandoned_baby_win_loading = true; self.cdl_abandoned_baby_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlAbandonedBabySnapshot { symbol: sym });
                        }
                        if self.cdl_abandoned_baby_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_abandoned_baby_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_abandoned_baby_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥4 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_abandoned_baby_label.as_str() { "BULLISH_PATTERN" => UP, "BEARISH_PATTERN" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — b1 {:.1}% — b2 {:.1}% — gap_down {:+.2}% — gap_up {:+.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_abandoned_baby_label, snap.pattern_value, snap.bar1_body_pct_range, snap.bar2_body_pct_range, snap.gap_down_pct, snap.gap_up_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_abandoned_baby_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Bar 1 body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.bar1_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Bar 2 body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.bar2_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Gap down %").small().strong()); ui.label(egui::RichText::new(format!("{:+.2}", snap.gap_down_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Gap up %").small().strong()); ui.label(egui::RichText::new(format!("{:+.2}", snap.gap_up_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_abandoned_baby_win = open;
        }

        if self.show_cdl_three_inside_win {
            if self.cdl_three_inside_win_symbol.is_empty() {
                self.cdl_three_inside_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_three_inside_win;
            egui::Window::new("CDL3INSIDE — Three Inside Up/Down (confirmed Harami)")
                .open(&mut open).resizable(true).default_size([560.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_three_inside_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_three_inside_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_three_inside_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_three_inside(&conn, &sym_u) { self.cdl_three_inside_win_snapshot = snap; self.cdl_three_inside_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_three_inside_win_symbol.to_uppercase(); self.cdl_three_inside_win_loading = true; self.cdl_three_inside_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlThreeInsideSnapshot { symbol: sym });
                        }
                        if self.cdl_three_inside_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_three_inside_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_three_inside_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥4 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_three_inside_label.as_str() { "BULLISH_PATTERN" => UP, "BEARISH_PATTERN" => DOWN, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — b1 {:.1}% — body_ratio {:.3} — b3_vs_b1_open {:+.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_three_inside_label, snap.pattern_value, snap.bar1_body_pct_range, snap.body_size_ratio, snap.bar3_close_vs_bar1_open_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_three_inside_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Bar 1 body %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.bar1_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Body size ratio").small().strong()); ui.label(egui::RichText::new(format!("{:.3}", snap.body_size_ratio)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Bar 3 vs bar-1 open %").small().strong()); ui.label(egui::RichText::new(format!("{:+.2}", snap.bar3_close_vs_bar1_open_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_three_inside_win = open;
        }
    }
}
