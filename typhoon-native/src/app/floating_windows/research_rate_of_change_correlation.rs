use super::*;

mod correlation_extrema_windows;
mod rate_of_change_windows;

impl TyphooNApp {
    pub(super) fn render_research_rate_of_change_correlation_windows(
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

        self.render_rate_of_change_windows(ctx, &chart_sym_research);

        self.render_correlation_extrema_windows(ctx, &chart_sym_research);

        if self.show_bbands_win {
            if self.bbands_win_symbol.is_empty() {
                self.bbands_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_bbands_win;
            egui::Window::new("BBANDS — Bollinger Bands (SMA₂₀ ± 2σ)")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.bbands_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.bbands_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.bbands_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_bbands(&conn, &sym_u)
                                    {
                                        self.bbands_win_snapshot = snap;
                                        self.bbands_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.bbands_win_symbol.to_uppercase();
                            self.bbands_win_loading = true;
                            self.bbands_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeBbandsSnapshot { symbol: sym });
                        }
                        if self.bbands_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.bbands_win_snapshot;
                    if snap.symbol.is_empty() || snap.bbands_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥21 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.bbands_label.as_str() {
                            "ABOVE_UPPER" => UP,
                            "BELOW_LOWER" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — close {:.4} — %B {:.1}% — bw {:.2}% — as of {}",
                                snap.symbol,
                                snap.bbands_label,
                                snap.last_close,
                                snap.pct_b,
                                snap.bandwidth,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("bbands_summary")
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
                                ui.label(egui::RichText::new("Period").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.period))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("σ multiplier").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.num_std))
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
                                ui.label(egui::RichText::new("Middle (SMA)").small().strong());
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
                                ui.label(egui::RichText::new("Upper prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.upper_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Middle prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.middle_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Lower prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.lower_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Close").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.last_close))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("%B").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.pct_b))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Bandwidth %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.bandwidth))
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
            self.show_bbands_win = open;
        }

        if self.show_ad_win {
            if self.ad_win_symbol.is_empty() {
                self.ad_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ad_win;
            egui::Window::new("AD — Chaikin Accumulation/Distribution Line (TA-Lib)")
                .open(&mut open).resizable(true).default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.ad_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.ad_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.ad_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_ad(&conn, &sym_u) { self.ad_win_snapshot = snap; self.ad_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ad_win_symbol.to_uppercase(); self.ad_win_loading = true; self.ad_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeAdSnapshot { symbol: sym });
                        }
                        if self.ad_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.ad_win_snapshot;
                    if snap.symbol.is_empty() || snap.ad_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥12 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.ad_label.as_str() {
                            "STRONG_ACCUM" | "ACCUM" => UP,
                            "DIST" | "STRONG_DIST" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — AD {:.2} (Δ {:+.2}) — slope {:+.4} — close {:.4} — as of {}",
                            snap.symbol, snap.ad_label, snap.ad, snap.ad_delta, snap.ad_slope, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("ad_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("AD").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.ad)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("AD prev").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.ad_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("AD delta").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.ad_delta)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("AD slope (10)").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.ad_slope)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_ad_win = open;
        }

        if self.show_adosc_win {
            if self.adosc_win_symbol.is_empty() {
                self.adosc_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_adosc_win;
            egui::Window::new("ADOSC — Chaikin A/D Oscillator (3/10 TA-Lib)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.adosc_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.adosc_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.adosc_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_adosc(&conn, &sym_u)
                                    {
                                        self.adosc_win_snapshot = snap;
                                        self.adosc_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.adosc_win_symbol.to_uppercase();
                            self.adosc_win_loading = true;
                            self.adosc_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeAdoscSnapshot { symbol: sym });
                        }
                        if self.adosc_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.adosc_win_snapshot;
                    if snap.symbol.is_empty() || snap.adosc_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥12 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.adosc_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "BEAR" | "STRONG_BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — ADOSC {:+.2} (prev {:+.2}) — close {:.4} — as of {}",
                                snap.symbol,
                                snap.adosc_label,
                                snap.adosc,
                                snap.adosc_prev,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("adosc_summary")
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
                                ui.label(egui::RichText::new("Fast period").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.fast_period))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Slow period").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.slow_period))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ADOSC").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.adosc))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ADOSC prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.adosc_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("AD ref").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.ad_ref))
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
            self.show_adosc_win = open;
        }

        if self.show_sum_win {
            if self.sum_win_symbol.is_empty() {
                self.sum_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_sum_win;
            egui::Window::new("SUM — rolling sum of close (period 30)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.sum_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.sum_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.sum_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_sum(&conn, &sym_u)
                                    {
                                        self.sum_win_snapshot = snap;
                                        self.sum_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.sum_win_symbol.to_uppercase();
                            self.sum_win_loading = true;
                            self.sum_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeSumSnapshot { symbol: sym });
                        }
                        if self.sum_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.sum_win_snapshot;
                    if snap.symbol.is_empty() || snap.sum_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥31 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.sum_label.as_str() {
                            "STRONG_UP" | "UP" => UP,
                            "DOWN" | "STRONG_DOWN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — sum {:.2} (Δ {:+.2}, {:+.2}%) — close {:.4} — as of {}",
                                snap.symbol,
                                snap.sum_label,
                                snap.sum,
                                snap.sum_delta,
                                snap.sum_pct_change,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("sum_summary")
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
                                ui.label(egui::RichText::new("Period").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.period))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Sum").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sum))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Sum prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sum_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Sum Δ").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.sum_delta))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Sum %Δ").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}", snap.sum_pct_change))
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
            self.show_sum_win = open;
        }

        if self.show_linreg_intercept_win {
            if self.linreg_intercept_win_symbol.is_empty() {
                self.linreg_intercept_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_linreg_intercept_win;
            egui::Window::new("LINEARREG_INTERCEPT — regression intercept (period 14)")
                .open(&mut open).resizable(true).default_size([540.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.linreg_intercept_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.linreg_intercept_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.linreg_intercept_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_linreg_intercept(&conn, &sym_u) { self.linreg_intercept_win_snapshot = snap; self.linreg_intercept_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.linreg_intercept_win_symbol.to_uppercase(); self.linreg_intercept_win_loading = true; self.linreg_intercept_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeLinearRegInterceptSnapshot { symbol: sym });
                        }
                        if self.linreg_intercept_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.linreg_intercept_win_snapshot;
                    if snap.symbol.is_empty() || snap.linreg_intercept_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥15 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.linreg_intercept_label.as_str() {
                            "STRONG_ADVANCE" | "ADVANCE" => UP,
                            "DECLINE" | "STRONG_DECLINE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — b {:.4} — close {:.4} — drift {:+.4} ({:+.2}%) — as of {}",
                            snap.symbol, snap.linreg_intercept_label, snap.intercept, snap.last_close, snap.drift, snap.drift_pct, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("linreg_intercept_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Period").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.period)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Intercept (b)").small().strong()); ui.label(egui::RichText::new(format!("{:.6}", snap.intercept)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Intercept prev").small().strong()); ui.label(egui::RichText::new(format!("{:.6}", snap.intercept_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Slope (m)").small().strong()); ui.label(egui::RichText::new(format!("{:+.6}", snap.slope)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Drift").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.drift)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Drift %").small().strong()); ui.label(egui::RichText::new(format!("{:+.2}", snap.drift_pct)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_linreg_intercept_win = open;
        }
    }
}
