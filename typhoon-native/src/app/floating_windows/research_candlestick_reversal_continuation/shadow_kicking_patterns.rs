use super::*;

impl TyphooNApp {
    pub(super) fn render_cdl_shadow_kicking_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
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
    }
}
