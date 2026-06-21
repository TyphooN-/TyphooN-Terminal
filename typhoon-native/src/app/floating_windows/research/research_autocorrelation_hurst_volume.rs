use super::*;

impl TyphooNApp {
    pub(super) fn render_research_autocorrelation_hurst_volume_windows(
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

        // AUTOCOR — Autocorrelation at multiple lags
        if self.show_autocor {
            if self.autocor_symbol.is_empty() {
                self.autocor_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_autocor;
            egui::Window::new("AUTOCOR — Return Autocorrelation")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.autocor_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.autocor_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.autocor_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_autocor(&conn, &sym_u)
                                    {
                                        self.autocor_snapshot = snap;
                                        self.autocor_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.autocor_symbol.to_uppercase();
                            self.autocor_loading = true;
                            self.autocor_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeAutocorSnapshot { symbol: sym });
                        }
                        if self.autocor_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.autocor_snapshot;
                    if snap.symbol.is_empty() || snap.regime_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — needs ≥30 cached daily bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.regime_label.as_str() {
                            "MOMENTUM" | "STRONG_MOMENTUM" => UP,
                            "MEAN_REVERT" | "STRONG_MEAN_REVERT" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — lag1 {:.3} — {} bars — as of {}",
                                snap.symbol,
                                snap.regime_label,
                                snap.lag1_acf,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("autocor_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Lag-1 ACF").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.lag1_acf))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Lag-5 ACF").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.lag5_acf))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Lag-10 ACF").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.lag10_acf))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Lag-20 ACF").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.lag20_acf))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Mean log return").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.mean_log_return))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_autocor = open;
        }

        // HURST — Hurst exponent via R/S
        if self.show_hurst {
            if self.hurst_symbol.is_empty() {
                self.hurst_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_hurst;
            egui::Window::new("HURST — Hurst Exponent (R/S)")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.hurst_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.hurst_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.hurst_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_hurst(&conn, &sym_u)
                                    {
                                        self.hurst_snapshot = snap;
                                        self.hurst_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.hurst_symbol.to_uppercase();
                            self.hurst_loading = true;
                            self.hurst_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeHurstSnapshot { symbol: sym });
                        }
                        if self.hurst_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.hurst_snapshot;
                    if snap.symbol.is_empty() || snap.memory_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — needs ≥40 cached daily bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.memory_label.as_str() {
                            "PERSISTENT" | "STRONG_PERSISTENT" => UP,
                            "MEAN_REVERT" | "STRONG_MEAN_REVERT" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — H {:.3} — {} bars — as of {}",
                                snap.symbol,
                                snap.memory_label,
                                snap.hurst_exponent,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("hurst_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Hurst exponent H").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.hurst_exponent))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("R/S scales fit").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.scales_used))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Min / max scale").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {}",
                                        snap.min_scale, snap.max_scale
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Memory label").small().strong());
                                ui.label(
                                    egui::RichText::new(&snap.memory_label).small().monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_hurst = open;
        }

        // HITRATE — Multi-horizon hit rate
        if self.show_hitrate {
            if self.hitrate_symbol.is_empty() {
                self.hitrate_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_hitrate;
            egui::Window::new("HITRATE — Multi-Horizon Win Rate")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.hitrate_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.hitrate_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.hitrate_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_hitrate(&conn, &sym_u)
                                    {
                                        self.hitrate_snapshot = snap;
                                        self.hitrate_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.hitrate_symbol.to_uppercase();
                            self.hitrate_loading = true;
                            self.hitrate_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeHitrateSnapshot { symbol: sym });
                        }
                        if self.hitrate_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.hitrate_snapshot;
                    if snap.symbol.is_empty() || snap.hit_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — needs ≥20 cached daily bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.hit_label.as_str() {
                            "BULLISH" | "WEAK_BULLISH" => UP,
                            "BEARISH" | "WEAK_BEARISH" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — 20d {:.1}% — 60d {:.1}% — {} bars — as of {}",
                                snap.symbol,
                                snap.hit_label,
                                snap.hitrate_20d,
                                snap.hitrate_60d,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("hitrate_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("5d hit rate").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.1}%", snap.hitrate_5d))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("20d hit rate").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.1}%", snap.hitrate_20d))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("60d hit rate").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.1}%", snap.hitrate_60d))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("252d hit rate").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.1}%", snap.hitrate_252d))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Up / down / flat days")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {} / {}",
                                        snap.up_days, snap.down_days, snap.flat_days
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_hitrate = open;
        }

        // GLASYM — Gain/loss asymmetry
        if self.show_glasym {
            if self.glasym_symbol.is_empty() {
                self.glasym_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_glasym;
            egui::Window::new("GLASYM — Gain/Loss Asymmetry")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.glasym_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.glasym_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.glasym_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_glasym(&conn, &sym_u)
                                    {
                                        self.glasym_snapshot = snap;
                                        self.glasym_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.glasym_symbol.to_uppercase();
                            self.glasym_loading = true;
                            self.glasym_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeGlasymSnapshot { symbol: sym });
                        }
                        if self.glasym_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.glasym_snapshot;
                    if snap.symbol.is_empty() || snap.asymmetry_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — needs ≥20 cached daily bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.asymmetry_label.as_str() {
                            "UPSIDE_HEAVY" | "SLIGHT_UPSIDE" => UP,
                            "DOWNSIDE_HEAVY" | "SLIGHT_DOWNSIDE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — ratio {:.2} — {} bars — as of {}",
                                snap.symbol,
                                snap.asymmetry_label,
                                snap.magnitude_ratio,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("glasym_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("Avg up-day / down-day magnitude")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.3}% / {:.3}%",
                                        snap.avg_up_pct, snap.avg_down_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Median up-day / down-day magnitude")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.3}% / {:.3}%",
                                        snap.median_up_pct, snap.median_down_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Magnitude ratio (avg up / avg down)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.magnitude_ratio))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Up / down days count").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {}",
                                        snap.up_days, snap.down_days
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_glasym = open;
        }

        // VOLRATIO — Up vs down volume ratio
        if self.show_volratio {
            if self.volratio_symbol.is_empty() {
                self.volratio_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_volratio;
            egui::Window::new("VOLRATIO — Up/Down Volume Ratio")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.volratio_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.volratio_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.volratio_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_volratio(&conn, &sym_u)
                                    {
                                        self.volratio_snapshot = snap;
                                        self.volratio_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.volratio_symbol.to_uppercase();
                            self.volratio_loading = true;
                            self.volratio_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeVolratioSnapshot { symbol: sym });
                        }
                        if self.volratio_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.volratio_snapshot;
                    if snap.symbol.is_empty() || snap.flow_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥20 bars with volume.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.flow_label.as_str() {
                            "ACCUMULATION" | "SLIGHT_ACCUMULATION" => UP,
                            "DISTRIBUTION" | "SLIGHT_DISTRIBUTION" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — ratio {:.2} — {} bars — as of {}",
                                snap.symbol,
                                snap.flow_label,
                                snap.up_down_volume_ratio,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("volratio_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("Avg up-day / down-day volume")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.0} / {:.0}",
                                        snap.avg_up_volume, snap.avg_down_volume
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Median up-day / down-day volume")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.0} / {:.0}",
                                        snap.median_up_volume, snap.median_down_volume
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Up/down volume ratio").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.3}",
                                        snap.up_down_volume_ratio
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Max up-day / down-day volume")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.0} / {:.0}",
                                        snap.max_up_volume, snap.max_down_volume
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Up / down days count").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {}",
                                        snap.up_days, snap.down_days
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_volratio = open;
        }
    }
}
