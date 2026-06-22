use super::*;

impl TyphooNApp {
    pub(super) fn render_research_aroon_macd_variable_ma_windows(&mut self, ctx: &egui::Context) {
        let chart_sym_research = research_chart_symbol(
            self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
        );

        // ── Research section ──
        if self.show_aroonosc_win {
            if self.aroonosc_win_symbol.is_empty() {
                self.aroonosc_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_aroonosc_win;
            egui::Window::new("AROONOSC — Aroon Oscillator (period 14)")
                .open(&mut open).resizable(true).default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.aroonosc_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.aroonosc_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.aroonosc_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_aroonosc(&conn, &sym_u) { self.aroonosc_win_snapshot = snap; self.aroonosc_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.aroonosc_win_symbol.to_uppercase(); self.aroonosc_win_loading = true; self.aroonosc_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeAroonoscSnapshot { symbol: sym });
                        }
                        if self.aroonosc_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.aroonosc_win_snapshot;
                    if snap.symbol.is_empty() || snap.aroonosc_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥16 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.aroonosc_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "BEAR" | "STRONG_BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — osc {:+.1} — up {:.1} / down {:.1} — close {:.4} — as of {}",
                            snap.symbol, snap.aroonosc_label, snap.aroonosc, snap.aroon_up, snap.aroon_down, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("aroonosc_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Period").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.period)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("AROONOSC").small().strong()); ui.label(egui::RichText::new(format!("{:+.2}", snap.aroonosc)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("AROONOSC prev").small().strong()); ui.label(egui::RichText::new(format!("{:+.2}", snap.aroonosc_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("AROON_UP").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.aroon_up)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("AROON_DOWN").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.aroon_down)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_aroonosc_win = open;
        }

        if self.show_minmaxindex_win {
            if self.minmaxindex_win_symbol.is_empty() {
                self.minmaxindex_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_minmaxindex_win;
            egui::Window::new("MINMAXINDEX — combined min+max recency (period 30)")
                .open(&mut open).resizable(true).default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.minmaxindex_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.minmaxindex_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.minmaxindex_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_minmaxindex(&conn, &sym_u) { self.minmaxindex_win_snapshot = snap; self.minmaxindex_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.minmaxindex_win_symbol.to_uppercase(); self.minmaxindex_win_loading = true; self.minmaxindex_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeMinMaxIndexSnapshot { symbol: sym });
                        }
                        if self.minmaxindex_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.minmaxindex_win_snapshot;
                    if snap.symbol.is_empty() || snap.minmaxindex_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥31 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.minmaxindex_label.as_str() {
                            "FRESH_HIGH" => UP,
                            "FRESH_LOW" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — low {} ago / high {} ago — order {} — close {:.4} — as of {}",
                            snap.symbol, snap.minmaxindex_label, snap.min_index_bars_ago, snap.max_index_bars_ago, snap.extrema_order, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("minmaxindex_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Period").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.period)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Min bars ago").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.min_index_bars_ago)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Max bars ago").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.max_index_bars_ago)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Age diff (min−max)").small().strong()); ui.label(egui::RichText::new(format!("{:+}", snap.age_diff)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Extrema order").small().strong()); ui.label(egui::RichText::new(&snap.extrema_order).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_minmaxindex_win = open;
        }

        if self.show_macdext_win {
            if self.macdext_win_symbol.is_empty() {
                self.macdext_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_macdext_win;
            egui::Window::new("MACDEXT — MACD with SMA (12/26/9)")
                .open(&mut open).resizable(true).default_size([540.0, 290.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.macdext_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.macdext_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.macdext_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_macdext(&conn, &sym_u) { self.macdext_win_snapshot = snap; self.macdext_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.macdext_win_symbol.to_uppercase(); self.macdext_win_loading = true; self.macdext_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeMacdextSnapshot { symbol: sym });
                        }
                        if self.macdext_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.macdext_win_snapshot;
                    if snap.symbol.is_empty() || snap.macdext_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥37 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.macdext_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "BEAR" | "STRONG_BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — macd {:+.4} — sig {:+.4} — hist {:+.4} — close {:.4} — as of {}",
                            snap.symbol, snap.macdext_label, snap.macd, snap.signal, snap.hist, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("macdext_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("MA type").small().strong()); ui.label(egui::RichText::new(&snap.ma_type).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Fast / slow / signal").small().strong()); ui.label(egui::RichText::new(format!("{}/{}/{}", snap.fast_period, snap.slow_period, snap.signal_period)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("MACD").small().strong()); ui.label(egui::RichText::new(format!("{:+.6}", snap.macd)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Signal").small().strong()); ui.label(egui::RichText::new(format!("{:+.6}", snap.signal)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Histogram").small().strong()); ui.label(egui::RichText::new(format!("{:+.6}", snap.hist)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Hist prev").small().strong()); ui.label(egui::RichText::new(format!("{:+.6}", snap.hist_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_macdext_win = open;
        }

        if self.show_macdfix_win {
            if self.macdfix_win_symbol.is_empty() {
                self.macdfix_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_macdfix_win;
            egui::Window::new("MACDFIX — MACD with hardcoded EMA 12/26 + signal 9")
                .open(&mut open).resizable(true).default_size([540.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.macdfix_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.macdfix_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.macdfix_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_macdfix(&conn, &sym_u) { self.macdfix_win_snapshot = snap; self.macdfix_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.macdfix_win_symbol.to_uppercase(); self.macdfix_win_loading = true; self.macdfix_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeMacdfixSnapshot { symbol: sym });
                        }
                        if self.macdfix_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.macdfix_win_snapshot;
                    if snap.symbol.is_empty() || snap.macdfix_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥37 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.macdfix_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "BEAR" | "STRONG_BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — macd {:+.4} — sig {:+.4} — hist {:+.4} — close {:.4} — as of {}",
                            snap.symbol, snap.macdfix_label, snap.macd, snap.signal, snap.hist, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("macdfix_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Fast / slow (fixed)").small().strong()); ui.label(egui::RichText::new(format!("{}/{}", snap.fast_period, snap.slow_period)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Signal period").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.signal_period)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("MACD").small().strong()); ui.label(egui::RichText::new(format!("{:+.6}", snap.macd)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Signal").small().strong()); ui.label(egui::RichText::new(format!("{:+.6}", snap.signal)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Histogram").small().strong()); ui.label(egui::RichText::new(format!("{:+.6}", snap.hist)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Hist prev").small().strong()); ui.label(egui::RichText::new(format!("{:+.6}", snap.hist_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_macdfix_win = open;
        }

        if self.show_mavp_win {
            if self.mavp_win_symbol.is_empty() {
                self.mavp_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_mavp_win;
            egui::Window::new("MAVP — Moving Average with Variable Period (5..30 ramp)")
                .open(&mut open).resizable(true).default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.mavp_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.mavp_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.mavp_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_mavp(&conn, &sym_u) { self.mavp_win_snapshot = snap; self.mavp_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.mavp_win_symbol.to_uppercase(); self.mavp_win_loading = true; self.mavp_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeMavpSnapshot { symbol: sym });
                        }
                        if self.mavp_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.mavp_win_snapshot;
                    if snap.symbol.is_empty() || snap.mavp_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥32 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.mavp_label.as_str() {
                            "STRONG_UP" | "UP" => UP,
                            "DOWN" | "STRONG_DOWN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — mavp {:.4} — Δ {:+.4} — last period {} — close {:.4} — as of {}",
                            snap.symbol, snap.mavp_label, snap.mavp, snap.mavp_delta, snap.last_bar_period, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("mavp_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Period range").small().strong()); ui.label(egui::RichText::new(format!("{}..{}", snap.min_period, snap.max_period)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last-bar period").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.last_bar_period)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("MAVP").small().strong()); ui.label(egui::RichText::new(format!("{:.6}", snap.mavp)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("MAVP prev").small().strong()); ui.label(egui::RichText::new(format!("{:.6}", snap.mavp_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Delta").small().strong()); ui.label(egui::RichText::new(format!("{:+.6}", snap.mavp_delta)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_mavp_win = open;
        }
    }
}
