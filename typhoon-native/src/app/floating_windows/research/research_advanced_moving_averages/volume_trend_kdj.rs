use super::*;

impl TyphooNApp {
    pub(super) fn render_volume_trend_kdj_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
        if self.show_adl_win {
            if self.adl_win_symbol.is_empty() {
                self.adl_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_adl_win;
            egui::Window::new(
                "ADL — Chaikin Accumulation/Distribution Line (cumulative MFM · volume)",
            )
            .open(&mut open)
            .resizable(true)
            .default_size([560.0, 260.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.adl_win_symbol).desired_width(100.0),
                    );
                    if ui.button("Use Chart").clicked() {
                        self.adl_win_symbol = chart_sym_research.clone();
                    }
                    if ui.button("Load Cached").clicked() {
                        if let Some(ref cache) = self.cache {
                            if let Ok(conn) = cache.connection() {
                                let sym_u = self.adl_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) =
                                    typhoon_engine::core::research::get_adl(&conn, &sym_u)
                                {
                                    self.adl_win_snapshot = snap;
                                    self.adl_win_symbol = sym_u;
                                }
                            }
                        }
                    }
                    if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                        let sym = self.adl_win_symbol.to_uppercase();
                        self.adl_win_loading = true;
                        self.adl_win_symbol = sym.clone();
                        let _ = self
                            .broker_tx
                            .send(BrokerCmd::ComputeAdlSnapshot { symbol: sym });
                    }
                    if self.adl_win_loading {
                        ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                    }
                });
                ui.separator();
                let snap = &self.adl_win_snapshot;
                if snap.symbol.is_empty() || snap.adl_label == "INSUFFICIENT_DATA" {
                    ui.label(
                        egui::RichText::new("No data — HP cache needs ≥22 bars.")
                            .color(AXIS_TEXT)
                            .small(),
                    );
                } else {
                    let color = match snap.adl_label.as_str() {
                        "STRONG_ACCUMULATION" | "ACCUMULATION" => UP,
                        "STRONG_DISTRIBUTION" | "DISTRIBUTION" => DOWN,
                        _ => AXIS_TEXT,
                    };
                    ui.label(
                        egui::RichText::new(format!(
                            "{} — {} — ADL {:.2} · slope/bar {:+.2} — close {:.4} — as of {}",
                            snap.symbol,
                            snap.adl_label,
                            snap.adl_value,
                            snap.slope_per_bar,
                            snap.last_close,
                            snap.as_of
                        ))
                        .strong()
                        .color(color),
                    );
                    ui.separator();
                    egui::Grid::new("adl_summary")
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
                            ui.label(egui::RichText::new("ADL value").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:.2}", snap.adl_value))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("ADL prev").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:.2}", snap.adl_prev))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(
                                egui::RichText::new(format!("ADL SMA({})", snap.adl_sma_length))
                                    .small()
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:.2}", snap.adl_sma))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("Slope per bar").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:+.2}", snap.slope_per_bar))
                                    .small()
                                    .monospace(),
                            );
                            ui.end_row();
                            ui.label(egui::RichText::new("Price Δ (20-bar)").small().strong());
                            ui.label(
                                egui::RichText::new(format!("{:+.2}%", snap.price_delta_pct))
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
            self.show_adl_win = open;
        }

        if self.show_vhf_win {
            if self.vhf_win_symbol.is_empty() {
                self.vhf_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_vhf_win;
            egui::Window::new("VHF — Vertical Horizontal Filter (trending vs ranging, 28-bar)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.vhf_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.vhf_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.vhf_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_vhf(&conn, &sym_u)
                                    {
                                        self.vhf_win_snapshot = snap;
                                        self.vhf_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.vhf_win_symbol.to_uppercase();
                            self.vhf_win_loading = true;
                            self.vhf_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeVhfSnapshot { symbol: sym });
                        }
                        if self.vhf_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.vhf_win_snapshot;
                    if snap.symbol.is_empty() || snap.vhf_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.vhf_label.as_str() {
                            "STRONG_TREND" | "TREND" => UP,
                            "STRONG_RANGING" | "RANGING" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — VHF {:.4} (prev {:.4}) — close {:.4} — as of {}",
                                snap.symbol,
                                snap.vhf_label,
                                snap.vhf_value,
                                snap.vhf_prev,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("vhf_summary")
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
                                ui.label(egui::RichText::new("Highest high").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.highest_high))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Lowest low").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.lowest_low))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Σ|Δclose|").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sum_abs_delta))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("VHF").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.vhf_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("VHF prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.vhf_prev))
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
            self.show_vhf_win = open;
        }

        if self.show_vroc_win {
            if self.vroc_win_symbol.is_empty() {
                self.vroc_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_vroc_win;
            egui::Window::new("VROC — Volume Rate of Change (14-bar ROC of volume)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.vroc_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.vroc_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.vroc_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_vroc(&conn, &sym_u)
                                    {
                                        self.vroc_win_snapshot = snap;
                                        self.vroc_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.vroc_win_symbol.to_uppercase();
                            self.vroc_win_loading = true;
                            self.vroc_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeVrocSnapshot { symbol: sym });
                        }
                        if self.vroc_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.vroc_win_snapshot;
                    if snap.symbol.is_empty() || snap.vroc_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥16 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.vroc_label.as_str() {
                            "SURGE" | "ELEVATED" => UP,
                            "COLLAPSE" | "QUIET" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — VROC {:+.2}% (prev {:+.2}%) — close {:.4} — as of {}",
                                snap.symbol,
                                snap.vroc_label,
                                snap.vroc_value,
                                snap.vroc_prev,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("vroc_summary")
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
                                ui.label(egui::RichText::new("Volume now").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.0}", snap.volume_now))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Volume 14 bars ago").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.0}", snap.volume_then))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("VROC").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}%", snap.vroc_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("VROC prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}%", snap.vroc_prev))
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
            self.show_vroc_win = open;
        }

        if self.show_kdj_win {
            if self.kdj_win_symbol.is_empty() {
                self.kdj_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_kdj_win;
            egui::Window::new("KDJ — Chinese Stochastic Variant (%K, %D, J = 3K − 2D)")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.kdj_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.kdj_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.kdj_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_kdj(&conn, &sym_u)
                                    {
                                        self.kdj_win_snapshot = snap;
                                        self.kdj_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.kdj_win_symbol.to_uppercase();
                            self.kdj_win_loading = true;
                            self.kdj_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeKdjSnapshot { symbol: sym });
                        }
                        if self.kdj_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.kdj_win_snapshot;
                    if snap.symbol.is_empty() || snap.kdj_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥14 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.kdj_label.as_str() {
                            "OVERBOUGHT" | "BULL" => UP,
                            "OVERSOLD" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — K {:.2} · D {:.2} · J {:.2} — close {:.4} — as of {}",
                                snap.symbol,
                                snap.kdj_label,
                                snap.k_value,
                                snap.d_value,
                                snap.j_value,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("kdj_summary")
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
                                ui.label(
                                    egui::RichText::new("Stoch length / smoothing")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {}",
                                        snap.stoch_length, snap.k_smooth
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("RSV (raw stochastic %)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.rsv))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("K (EMA₁/₃ of RSV)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.k_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("D (EMA₁/₃ of K)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.d_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("J (3·K − 2·D)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.j_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("J prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.j_prev))
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
            self.show_kdj_win = open;
        }
    }
}
