use super::*;

impl TyphooNApp {
    pub(super) fn render_research_normality_lmoments_price_impact_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research = research_chart_symbol(
            self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
        );

        // ── Research section ──
        if self.show_ksnorm {
            if self.ksnorm_symbol.is_empty() {
                self.ksnorm_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ksnorm;
            egui::Window::new("KSNORM — Kolmogorov-Smirnov Normality Test")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ksnorm_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ksnorm_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ksnorm_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ksnorm(&conn, &sym_u)
                                    {
                                        self.ksnorm_snapshot = snap;
                                        self.ksnorm_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ksnorm_symbol.to_uppercase();
                            self.ksnorm_loading = true;
                            self.ksnorm_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeKsnormSnapshot { symbol: sym });
                        }
                        if self.ksnorm_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.ksnorm_snapshot;
                    if snap.symbol.is_empty() || snap.ksnorm_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.ksnorm_label.as_str() {
                            "NORMAL" => UP,
                            "STRONG_NON_NORMAL" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — D {:.4} — as of {}",
                                snap.symbol, snap.ksnorm_label, snap.ks_statistic, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("ksnorm_summary")
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
                                ui.label(egui::RichText::new("D statistic").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.ks_statistic))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Critical 10%").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.4}  (reject {})",
                                        snap.critical_10pct, snap.reject_10pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Critical 5%").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.4}  (reject {})",
                                        snap.critical_5pct, snap.reject_5pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Critical 1%").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.4}  (reject {})",
                                        snap.critical_1pct, snap.reject_1pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Sample μ").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.mean))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Sample σ").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.5}", snap.sigma))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_ksnorm = open;
        }

        if self.show_adtest {
            if self.adtest_symbol.is_empty() {
                self.adtest_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_adtest;
            egui::Window::new("ADTEST — Anderson-Darling Normality Test")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.adtest_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.adtest_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.adtest_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_adtest(&conn, &sym_u)
                                    {
                                        self.adtest_snapshot = snap;
                                        self.adtest_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.adtest_symbol.to_uppercase();
                            self.adtest_loading = true;
                            self.adtest_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeAdtestSnapshot { symbol: sym });
                        }
                        if self.adtest_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.adtest_snapshot;
                    if snap.symbol.is_empty() || snap.adtest_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.adtest_label.as_str() {
                            "NORMAL" => UP,
                            "STRONG_NON_NORMAL" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — A²_adj {:.4} — as of {}",
                                snap.symbol, snap.adtest_label, snap.ad_adjusted, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("adtest_summary")
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
                                ui.label(egui::RichText::new("A²").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.ad_statistic))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("A² adjusted").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.ad_adjusted))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("p-value (approx)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.p_value_approx))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Critical 10%").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.3}  (reject {})",
                                        snap.critical_10pct, snap.reject_10pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Critical 5%").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.3}  (reject {})",
                                        snap.critical_5pct, snap.reject_5pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Critical 1%").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.3}  (reject {})",
                                        snap.critical_1pct, snap.reject_1pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_adtest = open;
        }

        if self.show_lmom {
            if self.lmom_symbol.is_empty() {
                self.lmom_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_lmom;
            egui::Window::new("LMOM — L-Moments (Hosking 1990)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.lmom_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.lmom_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.lmom_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_lmom(&conn, &sym_u)
                                    {
                                        self.lmom_snapshot = snap;
                                        self.lmom_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.lmom_symbol.to_uppercase();
                            self.lmom_loading = true;
                            self.lmom_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeLmomSnapshot { symbol: sym });
                        }
                        if self.lmom_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.lmom_snapshot;
                    if snap.symbol.is_empty() || snap.lmom_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.lmom_label.as_str() {
                            "NEAR_SYMMETRIC" | "LIGHT_TAILS" => UP,
                            "HEAVY_LEFT" | "HEAVY_RIGHT" | "HEAVY_TAILS" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — τ3 {:+.4} τ4 {:+.4} — as of {}",
                                snap.symbol,
                                snap.lmom_label,
                                snap.tau3_skew,
                                snap.tau4_kurt,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("lmom_summary")
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
                                ui.label(egui::RichText::new("L1 (mean)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.l1_mean))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("L2 (scale)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.l2_scale))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("L3").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.l3))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("L4").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.l4))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("τ3 (L-skew)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.tau3_skew))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("τ4 (L-kurt)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.tau4_kurt))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_lmom = open;
        }

        if self.show_kylelam {
            if self.kylelam_symbol.is_empty() {
                self.kylelam_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_kylelam;
            egui::Window::new("KYLELAM — Kyle's Price-Impact λ")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.kylelam_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.kylelam_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.kylelam_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_kylelam(&conn, &sym_u)
                                    {
                                        self.kylelam_snapshot = snap;
                                        self.kylelam_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.kylelam_symbol.to_uppercase();
                            self.kylelam_loading = true;
                            self.kylelam_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeKylelamSnapshot { symbol: sym });
                        }
                        if self.kylelam_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.kylelam_snapshot;
                    if snap.symbol.is_empty() || snap.kylelam_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 bars with volume.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.kylelam_label.as_str() {
                            "HIGH_IMPACT" => DOWN,
                            "LOW_IMPACT" | "NO_SIGNAL" => UP,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — λ {:.3e} — R² {:.4} — as of {}",
                                snap.symbol,
                                snap.kylelam_label,
                                snap.kyle_lambda,
                                snap.r_squared,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("kylelam_summary")
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
                                ui.label(egui::RichText::new("Kyle λ").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3e}", snap.kyle_lambda))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("mean |Δp|").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.5}", snap.mean_abs_dp))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("mean V").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.1}", snap.mean_volume))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Correlation ρ").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.correlation))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("R²").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.r_squared))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_kylelam = open;
        }
    }
}
