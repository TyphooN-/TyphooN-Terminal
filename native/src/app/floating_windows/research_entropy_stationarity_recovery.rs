use super::*;

impl TyphooNApp {
    pub(super) fn render_research_entropy_stationarity_recovery_windows(
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

        // ── Research Round 34 windows ──
        if self.show_sampen {
            if self.sampen_symbol.is_empty() {
                self.sampen_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_sampen;
            egui::Window::new("SAMPEN — Sample Entropy")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.sampen_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.sampen_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.sampen_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_sampen(&conn, &sym_u)
                                    {
                                        self.sampen_snapshot = snap;
                                        self.sampen_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.sampen_symbol.to_uppercase();
                            self.sampen_loading = true;
                            self.sampen_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeSampenSnapshot { symbol: sym });
                        }
                        if self.sampen_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.sampen_snapshot;
                    if snap.symbol.is_empty()
                        || snap.sampen_label == "INSUFFICIENT_DATA"
                        || snap.sampen_label == "UNDEFINED"
                    {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.sampen_label.as_str() {
                            "REGULAR" => UP,
                            "HIGHLY_COMPLEX" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — SampEn {:.4} — as of {}",
                                snap.symbol, snap.sampen_label, snap.sampen, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("sampen_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(180.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Returns used").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.bars_used))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Embed dim (m)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.embed_dim))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Tolerance (r)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.tolerance))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("A count (m+1)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.a_count))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("B count (m)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.b_count))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Sample entropy").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sampen))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_sampen = open;
        }

        if self.show_permen {
            if self.permen_symbol.is_empty() {
                self.permen_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_permen;
            egui::Window::new("PERMEN — Permutation Entropy")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.permen_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.permen_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.permen_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_permen(&conn, &sym_u)
                                    {
                                        self.permen_snapshot = snap;
                                        self.permen_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.permen_symbol.to_uppercase();
                            self.permen_loading = true;
                            self.permen_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputePermenSnapshot { symbol: sym });
                        }
                        if self.permen_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.permen_snapshot;
                    if snap.symbol.is_empty() || snap.permen_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.permen_label.as_str() {
                            "REGULAR" => UP,
                            "HIGHLY_COMPLEX" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — H_norm {:.4} — as of {}",
                                snap.symbol, snap.permen_label, snap.permen_normalised, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("permen_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(180.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Returns used").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.bars_used))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Embed dim (m)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.embed_dim))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Patterns observed").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}/{}",
                                        snap.patterns_observed, snap.patterns_possible
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("H raw (bits)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.permen_raw))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("H normalised").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.permen_normalised))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_permen = open;
        }

        if self.show_recfact {
            if self.recfact_symbol.is_empty() {
                self.recfact_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_recfact;
            egui::Window::new("RECFACT — Recovery Factor")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.recfact_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.recfact_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.recfact_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_recfact(&conn, &sym_u)
                                    {
                                        self.recfact_snapshot = snap;
                                        self.recfact_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.recfact_symbol.to_uppercase();
                            self.recfact_loading = true;
                            self.recfact_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRecfactSnapshot { symbol: sym });
                        }
                        if self.recfact_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.recfact_snapshot;
                    if snap.symbol.is_empty() || snap.recfact_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — needs ≥20 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.recfact_label.as_str() {
                            "EXCELLENT" | "GOOD" => UP,
                            "DEEP_LOSS" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — RF {:.4} — as of {}",
                                snap.symbol, snap.recfact_label, snap.recovery_factor, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("recfact_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(180.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Bars used").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.bars_used))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Cum return (%)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.cum_return_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Max drawdown (%)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.max_drawdown_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Recovery factor").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.recovery_factor))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_recfact = open;
        }

        if self.show_kpss {
            if self.kpss_symbol.is_empty() {
                self.kpss_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_kpss;
            egui::Window::new("KPSS — Stationarity Test")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.kpss_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.kpss_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.kpss_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_kpss(&conn, &sym_u)
                                    {
                                        self.kpss_snapshot = snap;
                                        self.kpss_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.kpss_symbol.to_uppercase();
                            self.kpss_loading = true;
                            self.kpss_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeKpssSnapshot { symbol: sym });
                        }
                        if self.kpss_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.kpss_snapshot;
                    if snap.symbol.is_empty() || snap.kpss_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.kpss_label.as_str() {
                            "STATIONARY" => UP,
                            "NONSTATIONARY" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — η_μ {:.4} — reject {} — as of {}",
                                snap.symbol,
                                snap.kpss_label,
                                snap.kpss_stat,
                                snap.reject_stationary,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("kpss_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(200.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Returns used").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.bars_used))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("KPSS stat (η_μ)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.kpss_stat))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Lag truncation (ℓ)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.lag_truncation))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Crit 10%").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.crit_10))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Crit 5%").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.crit_5))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Crit 1%").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.crit_1))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Reject stationary").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.reject_stationary))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_kpss = open;
        }

        if self.show_specent {
            if self.specent_symbol.is_empty() {
                self.specent_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_specent;
            egui::Window::new("SPECENT — Spectral Entropy")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.specent_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.specent_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.specent_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_specent(&conn, &sym_u)
                                    {
                                        self.specent_snapshot = snap;
                                        self.specent_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.specent_symbol.to_uppercase();
                            self.specent_loading = true;
                            self.specent_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeSpecentSnapshot { symbol: sym });
                        }
                        if self.specent_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.specent_snapshot;
                    if snap.symbol.is_empty() || snap.specent_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.specent_label.as_str() {
                            "PERIODIC" => UP,
                            "NOISE_LIKE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — H_norm {:.4} — as of {}",
                                snap.symbol,
                                snap.specent_label,
                                snap.spectral_entropy_norm,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("specent_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(180.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Returns used").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.bars_used))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Freq bins").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.num_freqs))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("H raw (bits)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.4}",
                                        snap.spectral_entropy_raw
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("H normalised").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.4}",
                                        snap.spectral_entropy_norm
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Peak freq idx").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.peak_freq_idx))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Peak power share").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.peak_power_share))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_specent = open;
        }
    }
}
