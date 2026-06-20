use super::*;

impl TyphooNApp {
    pub(super) fn render_research_linearreg_hilbert_stochastic_windows(
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

        // ── egui windows ──
        if self.show_linearreg_slope_win {
            if self.linearreg_slope_win_symbol.is_empty() {
                self.linearreg_slope_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_linearreg_slope_win;
            egui::Window::new("LINEARREG_SLOPE — Least-squares slope on close (TA-Lib parity)")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.linearreg_slope_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.linearreg_slope_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.linearreg_slope_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_linearreg_slope(&conn, &sym_u) { self.linearreg_slope_win_snapshot = snap; self.linearreg_slope_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.linearreg_slope_win_symbol.to_uppercase(); self.linearreg_slope_win_loading = true; self.linearreg_slope_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeLinearregSlopeSnapshot { symbol: sym });
                        }
                        if self.linearreg_slope_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.linearreg_slope_win_snapshot;
                    if snap.symbol.is_empty() || snap.slope_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥15 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.slope_label.as_str() {
                            "STRONG_UP" | "UP" => UP, "STRONG_DOWN" | "DOWN" => DOWN, _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — slope {:+.6} (prev {:+.6}) — slope_pct {:+.3}% — close {:.4} — as of {}",
                            snap.symbol, snap.slope_label, snap.slope, snap.slope_prev, snap.slope_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("linearreg_slope_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Slope").small().strong()); ui.label(egui::RichText::new(format!("{:+.6}", snap.slope)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Slope prev").small().strong()); ui.label(egui::RichText::new(format!("{:+.6}", snap.slope_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Slope % of close").small().strong()); ui.label(egui::RichText::new(format!("{:+.3}%", snap.slope_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_linearreg_slope_win = open;
        }

        if self.show_ht_dcperiod_win {
            if self.ht_dcperiod_win_symbol.is_empty() {
                self.ht_dcperiod_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ht_dcperiod_win;
            egui::Window::new("HT_DCPERIOD — Hilbert Dominant Cycle Period (Ehlers homodyne)")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.ht_dcperiod_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.ht_dcperiod_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.ht_dcperiod_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_ht_dcperiod(&conn, &sym_u) { self.ht_dcperiod_win_snapshot = snap; self.ht_dcperiod_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ht_dcperiod_win_symbol.to_uppercase(); self.ht_dcperiod_win_loading = true; self.ht_dcperiod_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeHtDcperiodSnapshot { symbol: sym });
                        }
                        if self.ht_dcperiod_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.ht_dcperiod_win_snapshot;
                    if snap.symbol.is_empty() || snap.period_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥64 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.period_label.as_str() {
                            "VERY_SHORT" | "SHORT" => DOWN, "LONG" | "VERY_LONG" => UP, _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — period {:.2} (prev {:.2}) — range [{:.2} .. {:.2}] — close {:.4} — as of {}",
                            snap.symbol, snap.period_label, snap.period, snap.period_prev, snap.period_min_64, snap.period_max_64, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("ht_dcperiod_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Period").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.period)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Period prev").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.period_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Period min (64)").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.period_min_64)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Period max (64)").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.period_max_64)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_ht_dcperiod_win = open;
        }

        if self.show_ht_trendmode_win {
            if self.ht_trendmode_win_symbol.is_empty() {
                self.ht_trendmode_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ht_trendmode_win;
            egui::Window::new("HT_TRENDMODE — Hilbert Trend vs Cycle Regime (Ehlers CV classifier)")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.ht_trendmode_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.ht_trendmode_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.ht_trendmode_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_ht_trendmode(&conn, &sym_u) { self.ht_trendmode_win_snapshot = snap; self.ht_trendmode_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ht_trendmode_win_symbol.to_uppercase(); self.ht_trendmode_win_loading = true; self.ht_trendmode_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeHtTrendmodeSnapshot { symbol: sym });
                        }
                        if self.ht_trendmode_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.ht_trendmode_win_snapshot;
                    if snap.symbol.is_empty() || snap.mode_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥64 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.mode_label.as_str() {
                            "TREND" => UP, "CYCLE" => AXIS_TEXT, _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — trendmode {} (prev {}) — lock_in_bars {} — period {:.2} — close {:.4} — as of {}",
                            snap.symbol, snap.mode_label, snap.trendmode, snap.trendmode_prev, snap.lock_in_bars, snap.period, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("ht_trendmode_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Trendmode").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.trendmode)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Trendmode prev").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.trendmode_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Lock-in bars").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.lock_in_bars)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Period").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.period)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_ht_trendmode_win = open;
        }

        if self.show_accbands_win {
            if self.accbands_win_symbol.is_empty() {
                self.accbands_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_accbands_win;
            egui::Window::new("ACCBANDS — Headley Acceleration Bands (SMA-20 of H×(1+4·(H-L)/(H+L)))")
                .open(&mut open).resizable(true).default_size([580.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.accbands_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.accbands_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.accbands_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_accbands(&conn, &sym_u) { self.accbands_win_snapshot = snap; self.accbands_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.accbands_win_symbol.to_uppercase(); self.accbands_win_loading = true; self.accbands_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeAccbandsSnapshot { symbol: sym });
                        }
                        if self.accbands_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.accbands_win_snapshot;
                    if snap.symbol.is_empty() || snap.accbands_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥21 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.accbands_label.as_str() {
                            "BREAKOUT_UP" | "UPPER" => UP, "BREAKOUT_DOWN" | "LOWER" => DOWN, _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — upper {:.4} — mid {:.4} — lower {:.4} — width {:.4} — pos {:.3} — close {:.4} — as of {}",
                            snap.symbol, snap.accbands_label, snap.acc_upper, snap.acc_middle, snap.acc_lower, snap.width, snap.position, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("accbands_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Upper band").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.acc_upper)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Middle band").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.acc_middle)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Lower band").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.acc_lower)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Width").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.width)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Position in band").small().strong()); ui.label(egui::RichText::new(format!("{:.3}", snap.position)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_accbands_win = open;
        }

        if self.show_stochf_win {
            if self.stochf_win_symbol.is_empty() {
                self.stochf_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_stochf_win;
            egui::Window::new("STOCHF — Fast Stochastic (TA-Lib, unsmoothed %K + SMA-3 %D)")
                .open(&mut open).resizable(true).default_size([560.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.stochf_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.stochf_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.stochf_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_stochf(&conn, &sym_u) { self.stochf_win_snapshot = snap; self.stochf_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.stochf_win_symbol.to_uppercase(); self.stochf_win_loading = true; self.stochf_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeStochfSnapshot { symbol: sym });
                        }
                        if self.stochf_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.stochf_win_snapshot;
                    if snap.symbol.is_empty() || snap.stochf_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥17 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.stochf_label.as_str() {
                            "OVERBOUGHT" | "BULL" => UP, "OVERSOLD" | "BEAR" => DOWN, _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — fastK {:.2} (prev {:.2}) — fastD {:.2} (prev {:.2}) — close {:.4} — as of {}",
                            snap.symbol, snap.stochf_label, snap.fastk, snap.fastk_prev, snap.fastd, snap.fastd_prev, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("stochf_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("D period").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.d_period)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("FastK").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.fastk)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("FastK prev").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.fastk_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("FastD").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.fastd)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("FastD prev").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.fastd_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_stochf_win = open;
        }
    }
}
