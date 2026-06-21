use super::*;

impl TyphooNApp {
    pub(super) fn render_research_robust_entropy_quantile_volatility_windows(
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
        if self.show_robvol {
            if self.robvol_symbol.is_empty() {
                self.robvol_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_robvol;
            egui::Window::new("ROBVOL — Robust Volatility")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.robvol_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.robvol_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.robvol_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_robvol(&conn, &sym_u)
                                    {
                                        self.robvol_snapshot = snap;
                                        self.robvol_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.robvol_symbol.to_uppercase();
                            self.robvol_loading = true;
                            self.robvol_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRobvolSnapshot { symbol: sym });
                        }
                        if self.robvol_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.robvol_snapshot;
                    if snap.symbol.is_empty() || snap.robvol_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.robvol_label.as_str() {
                            "CLEAN" => UP,
                            "HEAVY_OUTLIERS" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — MAD ratio {:.3} — as of {}",
                                snap.symbol, snap.robvol_label, snap.mad_ratio, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("robvol_summary")
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
                                ui.label(
                                    egui::RichText::new("Classical σ (annual)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.classical_sigma))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("MAD σ (annual)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.mad_sigma))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("IQR σ (annual)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.iqr_sigma))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("MAD ratio").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.mad_ratio))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("IQR ratio").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.iqr_ratio))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_robvol = open;
        }

        if self.show_renyient {
            if self.renyient_symbol.is_empty() {
                self.renyient_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_renyient;
            egui::Window::new("RENYIENT — Rényi Entropy (α=2)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.renyient_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.renyient_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.renyient_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_renyient(&conn, &sym_u)
                                    {
                                        self.renyient_snapshot = snap;
                                        self.renyient_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.renyient_symbol.to_uppercase();
                            self.renyient_loading = true;
                            self.renyient_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRenyientSnapshot { symbol: sym });
                        }
                        if self.renyient_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.renyient_snapshot;
                    if snap.symbol.is_empty() || snap.renyient_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.renyient_label.as_str() {
                            "HIGHLY_DISPERSED" => UP,
                            "CONCENTRATED" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — H_norm {:.4} — as of {}",
                                snap.symbol, snap.renyient_label, snap.renyi_normalised, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("renyient_summary")
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
                                ui.label(egui::RichText::new("Histogram bins").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.num_bins))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("α").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.1}", snap.alpha))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("H₂ raw (bits)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.renyi_raw))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("H₂ normalised").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.renyi_normalised))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Collision prob").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.collision_prob))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_renyient = open;
        }

        if self.show_retquant {
            if self.retquant_symbol.is_empty() {
                self.retquant_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_retquant;
            egui::Window::new("RETQUANT — Return Quantile Profile")
                .open(&mut open)
                .resizable(true)
                .default_size([600.0, 360.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.retquant_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.retquant_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.retquant_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_retquant(&conn, &sym_u)
                                    {
                                        self.retquant_snapshot = snap;
                                        self.retquant_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.retquant_symbol.to_uppercase();
                            self.retquant_loading = true;
                            self.retquant_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRetquantSnapshot { symbol: sym });
                        }
                        if self.retquant_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.retquant_snapshot;
                    if snap.symbol.is_empty() || snap.retquant_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.retquant_label.as_str() {
                            "SYMMETRIC" => UP,
                            "LEFT_TAIL_HEAVY" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — asymm {:.3} — IQR {:.3}% — as of {}",
                                snap.symbol,
                                snap.retquant_label,
                                snap.tail_asymmetry,
                                snap.iqr_pct,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("retquant_summary")
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
                                ui.label(egui::RichText::new("P1").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}%", snap.p01_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("P5").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}%", snap.p05_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("P10").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}%", snap.p10_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("P25").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}%", snap.p25_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("P50 (median)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}%", snap.p50_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("P75").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}%", snap.p75_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("P90").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}%", snap.p90_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("P95").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}%", snap.p95_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("P99").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}%", snap.p99_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("IQR (P75−P25)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}%", snap.iqr_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Tail asymmetry").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.tail_asymmetry))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_retquant = open;
        }

        if self.show_msent {
            if self.msent_symbol.is_empty() {
                self.msent_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_msent;
            egui::Window::new("MSENT — Multiscale Entropy")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.msent_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.msent_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.msent_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_msent(&conn, &sym_u)
                                    {
                                        self.msent_snapshot = snap;
                                        self.msent_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.msent_symbol.to_uppercase();
                            self.msent_loading = true;
                            self.msent_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMsentSnapshot { symbol: sym });
                        }
                        if self.msent_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.msent_snapshot;
                    if snap.symbol.is_empty() || snap.msent_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥100 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.msent_label.as_str() {
                            "SUSTAINED" => UP,
                            "DECAYING" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — CI {:.3} — as of {}",
                                snap.symbol,
                                snap.msent_label,
                                snap.msent_complexity_index,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("msent_summary")
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
                                ui.label(egui::RichText::new("Embed dim m").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.embed_dim))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Tolerance r").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.5}", snap.tolerance))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("SampEn τ=1").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sampen_scale1))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("SampEn τ=2").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sampen_scale2))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("SampEn τ=3").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sampen_scale3))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("SampEn τ=4").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sampen_scale4))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("SampEn τ=5").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sampen_scale5))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Complexity index").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.4}",
                                        snap.msent_complexity_index
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_msent = open;
        }

        if self.show_ewmavol {
            if self.ewmavol_symbol.is_empty() {
                self.ewmavol_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ewmavol;
            egui::Window::new("EWMAVOL — EWMA Volatility (RiskMetrics)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ewmavol_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ewmavol_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ewmavol_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ewmavol(&conn, &sym_u)
                                    {
                                        self.ewmavol_snapshot = snap;
                                        self.ewmavol_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ewmavol_symbol.to_uppercase();
                            self.ewmavol_loading = true;
                            self.ewmavol_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeEwmavolSnapshot { symbol: sym });
                        }
                        if self.ewmavol_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.ewmavol_snapshot;
                    if snap.symbol.is_empty() || snap.ewmavol_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.ewmavol_label.as_str() {
                            "NORMAL" => UP,
                            "ELEVATED" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — ratio {:.3} — as of {}",
                                snap.symbol, snap.ewmavol_label, snap.ewma_to_classical, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("ewmavol_summary")
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
                                ui.label(egui::RichText::new("λ (decay)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.lambda))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("EWMA variance").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.ewma_variance))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("EWMA σ daily").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.5}", snap.ewma_sigma_daily))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("EWMA σ annual").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.ewma_sigma_annual))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Classical σ annual").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.4}",
                                        snap.classical_sigma_annual
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("EWMA / classical").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.ewma_to_classical))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_ewmavol = open;
        }
    }
}
