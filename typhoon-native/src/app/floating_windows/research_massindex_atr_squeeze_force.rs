use super::*;

impl TyphooNApp {
    pub(super) fn render_research_massindex_atr_squeeze_force_windows(
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

        // ── Research section ──
        if self.show_mass_index_win {
            if self.mass_index_win_symbol.is_empty() {
                self.mass_index_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_mass_index_win;
            egui::Window::new("MASSINDEX — Dorsey Mass Index (EMA/EMA ratio, reversal bulge)")
                .open(&mut open).resizable(true).default_size([580.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.mass_index_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.mass_index_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.mass_index_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_mass_index(&conn, &sym_u) { self.mass_index_win_snapshot = snap; self.mass_index_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.mass_index_win_symbol.to_uppercase(); self.mass_index_win_loading = true; self.mass_index_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeMassIndexSnapshot { symbol: sym });
                        }
                        if self.mass_index_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.mass_index_win_snapshot;
                    if snap.symbol.is_empty() || snap.mass_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥35 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.mass_label.as_str() {
                            "REVERSAL_BULGE" => DOWN, "ELEVATED" => UP, "COMPRESSED" => AXIS_TEXT, _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — MI {:.3} (prev {:.3}) — ratio {:.4} — close {:.4} — as of {}",
                            snap.symbol, snap.mass_label, snap.mass_index, snap.mass_index_prev, snap.ratio, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("mass_index_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("EMA length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.ema_len)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Sum length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.sum_len)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("EMA(H-L)").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.ema_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("EMA-of-EMA(H-L)").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.ema_ema_range)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Ratio").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.ratio)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Mass Index").small().strong()); ui.label(egui::RichText::new(format!("{:.3}", snap.mass_index)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Mass Index prev").small().strong()); ui.label(egui::RichText::new(format!("{:.3}", snap.mass_index_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_mass_index_win = open;
        }

        if self.show_natr_win {
            if self.natr_win_symbol.is_empty() {
                self.natr_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_natr_win;
            egui::Window::new("NATR — Normalized ATR (TA-Lib, 100 × ATR / close)")
                .open(&mut open).resizable(true).default_size([540.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.natr_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.natr_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.natr_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_natr(&conn, &sym_u) { self.natr_win_snapshot = snap; self.natr_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.natr_win_symbol.to_uppercase(); self.natr_win_loading = true; self.natr_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeNatrSnapshot { symbol: sym });
                        }
                        if self.natr_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.natr_win_snapshot;
                    if snap.symbol.is_empty() || snap.natr_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥15 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.natr_label.as_str() {
                            "HIGH_VOL" => DOWN, "ELEVATED" => UP, "LOW_VOL" => AXIS_TEXT, _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — NATR {:.3}% (prev {:.3}%) — ATR {:.4} — close {:.4} — as of {}",
                            snap.symbol, snap.natr_label, snap.natr_value, snap.natr_prev, snap.atr_value, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("natr_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("ATR").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.atr_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("NATR %").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.natr_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("NATR prev %").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.natr_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_natr_win = open;
        }

        if self.show_ttm_squeeze_win {
            if self.ttm_squeeze_win_symbol.is_empty() {
                self.ttm_squeeze_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ttm_squeeze_win;
            egui::Window::new("TTM_SQUEEZE — Carter's BB ⊂ KC Regime + Momentum (20)")
                .open(&mut open).resizable(true).default_size([600.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.ttm_squeeze_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.ttm_squeeze_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.ttm_squeeze_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_ttm_squeeze(&conn, &sym_u) { self.ttm_squeeze_win_snapshot = snap; self.ttm_squeeze_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ttm_squeeze_win_symbol.to_uppercase(); self.ttm_squeeze_win_loading = true; self.ttm_squeeze_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeTtmSqueezeSnapshot { symbol: sym });
                        }
                        if self.ttm_squeeze_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.ttm_squeeze_win_snapshot;
                    if snap.symbol.is_empty() || snap.squeeze_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥21 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.squeeze_label.as_str() {
                            "FIRE_UP" => UP, "FIRE_DOWN" => DOWN, "SQUEEZE_ON" => AXIS_TEXT, _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — momentum {:+.4} (prev {:+.4}) — squeeze_on {} — close {:.4} — as of {}",
                            snap.symbol, snap.squeeze_label, snap.momentum, snap.momentum_prev, snap.squeeze_on, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("ttm_squeeze_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("BB upper").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.bb_upper)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("BB lower").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.bb_lower)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("KC upper").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.kc_upper)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("KC lower").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.kc_lower)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Squeeze ON").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.squeeze_on)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Momentum").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.momentum)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Momentum prev").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.momentum_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_ttm_squeeze_win = open;
        }

        if self.show_force_index_win {
            if self.force_index_win_symbol.is_empty() {
                self.force_index_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_force_index_win;
            egui::Window::new("FORCE_INDEX — Elder Force Index (EMA of volume × Δclose, 13)")
                .open(&mut open).resizable(true).default_size([580.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.force_index_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.force_index_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.force_index_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_force_index(&conn, &sym_u) { self.force_index_win_snapshot = snap; self.force_index_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.force_index_win_symbol.to_uppercase(); self.force_index_win_loading = true; self.force_index_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeForceIndexSnapshot { symbol: sym });
                        }
                        if self.force_index_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.force_index_win_snapshot;
                    if snap.symbol.is_empty() || snap.force_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥15 bars with volume.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.force_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP, "STRONG_BEAR" | "BEAR" => DOWN, _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — EMA {:.2} (prev {:.2}) — raw {:.2} — close {:.4} — as of {}",
                            snap.symbol, snap.force_label, snap.force_ema, snap.force_ema_prev, snap.force_raw, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("force_index_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Raw force").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.force_raw)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Force EMA").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.force_ema)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Force EMA prev").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.force_ema_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last volume").small().strong()); ui.label(egui::RichText::new(format!("{:.0}", snap.last_volume)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_force_index_win = open;
        }

        if self.show_trange_win {
            if self.trange_win_symbol.is_empty() {
                self.trange_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_trange_win;
            egui::Window::new("TRANGE — True Range (raw, single-bar, gap-aware)")
                .open(&mut open).resizable(true).default_size([580.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.trange_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.trange_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.trange_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_trange(&conn, &sym_u) { self.trange_win_snapshot = snap; self.trange_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.trange_win_symbol.to_uppercase(); self.trange_win_loading = true; self.trange_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeTrangeSnapshot { symbol: sym });
                        }
                        if self.trange_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.trange_win_snapshot;
                    if snap.symbol.is_empty() || snap.trange_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥21 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.trange_label.as_str() {
                            "EXPANSION" => DOWN, "CONTRACTION" => AXIS_TEXT, _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — TR {:.4} (prev {:.4}) — mean(20) {:.4} — ratio {:.3} — close {:.4} — as of {}",
                            snap.symbol, snap.trange_label, snap.trange_value, snap.trange_prev, snap.mean_trange_20, snap.trange_ratio, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("trange_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("TR value").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.trange_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("TR prev").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.trange_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Mean TR(20)").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.mean_trange_20)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("TR ratio").small().strong()); ui.label(egui::RichText::new(format!("{:.3}", snap.trange_ratio)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last high").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_high)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last low").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_low)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Prev close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.prev_close)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_trange_win = open;
        }
    }
}
