use super::*;

impl TyphooNApp {
    pub(super) fn render_bands_intraday_guppy_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
        if self.show_seb_win {
            if self.seb_win_symbol.is_empty() {
                self.seb_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_seb_win;
            egui::Window::new("SEB — Standard Error Bands (linreg endpoint ± k·SE)")
                .open(&mut open).resizable(true).default_size([580.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.seb_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.seb_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.seb_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_seb(&conn, &sym_u) { self.seb_win_snapshot = snap; self.seb_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.seb_win_symbol.to_uppercase(); self.seb_win_loading = true; self.seb_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeSebSnapshot { symbol: sym });
                        }
                        if self.seb_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.seb_win_snapshot;
                    if snap.symbol.is_empty() || snap.seb_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥22 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.seb_label.as_str() {
                            "ABOVE_BAND" | "UPPER_HALF" => UP,
                            "BELOW_BAND" | "LOWER_HALF" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — mid {:.4} [ {:.4} … {:.4} ] — pos {:.1}% — close {:.4} — as of {}", snap.symbol, snap.seb_label, snap.middle, snap.lower, snap.upper, snap.position_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("seb_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("k·SE").small().strong()); ui.label(egui::RichText::new(format!("{:.1}", snap.num_se)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Upper").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.upper)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Middle (linreg)").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.middle)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Lower").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.lower)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Bandwidth").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.bandwidth)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Position %").small().strong()); ui.label(egui::RichText::new(format!("{:.1}%", snap.position_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_seb_win = open;
        }

        if self.show_imi_win {
            if self.imi_win_symbol.is_empty() {
                self.imi_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_imi_win;
            egui::Window::new("IMI — Chande's Intraday Momentum Index (RSI-style on close − open)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.imi_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.imi_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.imi_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_imi(&conn, &sym_u)
                                    {
                                        self.imi_win_snapshot = snap;
                                        self.imi_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.imi_win_symbol.to_uppercase();
                            self.imi_win_loading = true;
                            self.imi_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeImiSnapshot { symbol: sym });
                        }
                        if self.imi_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.imi_win_snapshot;
                    if snap.symbol.is_empty() || snap.imi_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥16 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.imi_label.as_str() {
                            "OVERBOUGHT" | "BULL" => UP,
                            "OVERSOLD" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — IMI {:.2} — close {:.4} — as of {}",
                                snap.symbol,
                                snap.imi_label,
                                snap.imi_value,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("imi_summary")
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
                                ui.label(egui::RichText::new("Length").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.length))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ΣUp (c−o > 0)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sum_gains))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ΣDown (c−o < 0)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sum_losses))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("IMI").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.imi_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("IMI prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.imi_prev))
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
            self.show_imi_win = open;
        }

        if self.show_gmma_win {
            if self.gmma_win_symbol.is_empty() {
                self.gmma_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_gmma_win;
            egui::Window::new("GMMA — Guppy Multiple Moving Average (6 short + 6 long EMA groups)")
                .open(&mut open).resizable(true).default_size([620.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.gmma_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.gmma_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.gmma_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_gmma(&conn, &sym_u) { self.gmma_win_snapshot = snap; self.gmma_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.gmma_win_symbol.to_uppercase(); self.gmma_win_loading = true; self.gmma_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeGmmaSnapshot { symbol: sym });
                        }
                        if self.gmma_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.gmma_win_snapshot;
                    if snap.symbol.is_empty() || snap.gmma_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥62 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.gmma_label.as_str() {
                            "STRONG_UPTREND" | "UPTREND" => UP,
                            "STRONG_DOWNTREND" | "DOWNTREND" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — short-avg {:.4} · long-avg {:.4} · gap {:+.2}% — close {:.4} — as of {}",
                            snap.symbol, snap.gmma_label, snap.short_ema_avg, snap.long_ema_avg, snap.group_gap_pct, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("gmma_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Short group (3,5,8,10,12,15)").small().strong()); ui.label(egui::RichText::new(format!("min {:.4} · avg {:.4} · max {:.4}", snap.short_min, snap.short_ema_avg, snap.short_max)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Long group (30,35,40,45,50,60)").small().strong()); ui.label(egui::RichText::new(format!("min {:.4} · avg {:.4} · max {:.4}", snap.long_min, snap.long_ema_avg, snap.long_max)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Short compression").small().strong()); ui.label(egui::RichText::new(format!("{:.2}%", snap.short_compression_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Long compression").small().strong()); ui.label(egui::RichText::new(format!("{:.2}%", snap.long_compression_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Group gap").small().strong()); ui.label(egui::RichText::new(format!("{:+.2}%", snap.group_gap_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_gmma_win = open;
        }

        if self.show_maenv_win {
            if self.maenv_win_symbol.is_empty() {
                self.maenv_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_maenv_win;
            egui::Window::new("MAENV — Moving Average Envelope (SMA₂₀ ± 2.5%)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.maenv_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.maenv_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.maenv_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_maenv(&conn, &sym_u)
                                    {
                                        self.maenv_win_snapshot = snap;
                                        self.maenv_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.maenv_win_symbol.to_uppercase();
                            self.maenv_win_loading = true;
                            self.maenv_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMaenvSnapshot { symbol: sym });
                        }
                        if self.maenv_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.maenv_win_snapshot;
                    if snap.symbol.is_empty() || snap.maenv_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥21 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.maenv_label.as_str() {
                            "ABOVE_BAND" | "UPPER_HALF" => UP,
                            "BELOW_BAND" | "LOWER_HALF" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — mid {:.4} · pos {:.1}% — close {:.4} — as of {}",
                                snap.symbol,
                                snap.maenv_label,
                                snap.middle,
                                snap.position_pct,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("maenv_summary")
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
                                ui.label(egui::RichText::new("Length / band%").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / ±{:.2}%",
                                        snap.length, snap.pct_band
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Upper").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.upper))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Middle (SMA₂₀)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.middle))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Lower").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.lower))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Bandwidth").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}%", snap.bandwidth_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Position within band").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.1}%", snap.position_pct))
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
            self.show_maenv_win = open;
        }
    }
}
