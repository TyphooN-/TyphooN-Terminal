use super::*;

impl TyphooNApp {
    pub(super) fn render_cdl_three_soldiers_crows_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
        if self.show_cdl_three_black_crows_win {
            if self.cdl_three_black_crows_win_symbol.is_empty() {
                self.cdl_three_black_crows_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_three_black_crows_win;
            egui::Window::new("CDL3BLACKCROWS — Three Black Crows (bearish continuation)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cdl_three_black_crows_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.cdl_three_black_crows_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u =
                                        self.cdl_three_black_crows_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_cdl_three_black_crows(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.cdl_three_black_crows_win_snapshot = snap;
                                        self.cdl_three_black_crows_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_three_black_crows_win_symbol.to_uppercase();
                            self.cdl_three_black_crows_win_loading = true;
                            self.cdl_three_black_crows_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCdlThreeBlackCrowsSnapshot { symbol: sym });
                        }
                        if self.cdl_three_black_crows_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.cdl_three_black_crows_win_snapshot;
                    if snap.symbol.is_empty()
                        || snap.cdl_three_black_crows_label == "INSUFFICIENT_DATA"
                    {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥4 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.cdl_three_black_crows_label.as_str() {
                            "BEARISH_PATTERN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — value {} — decline {:.2}% — close {:.4} — as of {}",
                                snap.symbol,
                                snap.cdl_three_black_crows_label,
                                snap.pattern_value,
                                snap.total_close_decline_pct,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("cdl_three_black_crows_summary")
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
                                ui.label(
                                    egui::RichText::new("Avg body % of range").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.avg_body_pct_range))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Total decline %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2}",
                                        snap.total_close_decline_pct
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
            self.show_cdl_three_black_crows_win = open;
        }

        if self.show_cdl_three_white_soldiers_win {
            if self.cdl_three_white_soldiers_win_symbol.is_empty() {
                self.cdl_three_white_soldiers_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cdl_three_white_soldiers_win;
            egui::Window::new("CDL3WHITESOLDIERS — Three White Soldiers (bullish continuation)")
                .open(&mut open).resizable(true).default_size([560.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cdl_three_white_soldiers_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cdl_three_white_soldiers_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.cdl_three_white_soldiers_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_three_white_soldiers(&conn, &sym_u) { self.cdl_three_white_soldiers_win_snapshot = snap; self.cdl_three_white_soldiers_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cdl_three_white_soldiers_win_symbol.to_uppercase(); self.cdl_three_white_soldiers_win_loading = true; self.cdl_three_white_soldiers_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCdlThreeWhiteSoldiersSnapshot { symbol: sym });
                        }
                        if self.cdl_three_white_soldiers_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cdl_three_white_soldiers_win_snapshot;
                    if snap.symbol.is_empty() || snap.cdl_three_white_soldiers_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥4 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.cdl_three_white_soldiers_label.as_str() { "BULLISH_PATTERN" => UP, _ => AXIS_TEXT };
                        ui.label(egui::RichText::new(format!("{} — {} — value {} — advance {:.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.cdl_three_white_soldiers_label, snap.pattern_value, snap.total_close_advance_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cdl_three_white_soldiers_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Pattern value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev value").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.pattern_value_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Avg body % of range").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.avg_body_pct_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Total advance %").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.total_close_advance_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last bar match").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_match)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days since pattern").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.days_since_pattern)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_cdl_three_white_soldiers_win = open;
        }
    }
}
