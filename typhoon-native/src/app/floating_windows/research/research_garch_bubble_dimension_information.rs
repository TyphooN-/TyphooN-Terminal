use super::*;

impl TyphooNApp {
    pub(super) fn render_research_garch_bubble_dimension_information_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research = research_chart_symbol(
            self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
        );

        // ── Research section ──
        if self.show_garch11 {
            if self.garch11_symbol.is_empty() {
                self.garch11_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_garch11;
            egui::Window::new("GARCH11 — GARCH(1,1) Conditional Volatility Fit")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 340.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.garch11_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.garch11_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.garch11_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_garch11(&conn, &sym_u)
                                    {
                                        self.garch11_snapshot = snap;
                                        self.garch11_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.garch11_symbol.to_uppercase();
                            self.garch11_loading = true;
                            self.garch11_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeGarch11Snapshot { symbol: sym });
                        }
                        if self.garch11_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.garch11_snapshot;
                    if snap.symbol.is_empty() || snap.garch11_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥60 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.garch11_label.as_str() {
                            "LOW_PERSISTENCE" => UP,
                            "NEAR_INTEGRATED" | "HIGH_PERSISTENCE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — α+β {:.4} — as of {}",
                                snap.symbol, snap.garch11_label, snap.persistence, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("garch11_summary")
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
                                ui.label(egui::RichText::new("ω (baseline)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3e}", snap.omega))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("α (ARCH)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.alpha))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("β (GARCH)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.beta))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Persistence α+β").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.persistence))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Unconditional var").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3e}", snap.unconditional_var))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Half-life (bars)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.1}", snap.half_life_bars))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Log-likelihood").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.log_likelihood))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_garch11 = open;
        }

        if self.show_sadf {
            if self.sadf_symbol.is_empty() {
                self.sadf_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_sadf;
            egui::Window::new("SADF — Phillips-Wu-Yu Sup-ADF Bubble Test")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.sadf_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.sadf_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.sadf_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_sadf(&conn, &sym_u)
                                    {
                                        self.sadf_snapshot = snap;
                                        self.sadf_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.sadf_symbol.to_uppercase();
                            self.sadf_loading = true;
                            self.sadf_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeSadfSnapshot { symbol: sym });
                        }
                        if self.sadf_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.sadf_snapshot;
                    if snap.symbol.is_empty() || snap.sadf_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥60 closes.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.sadf_label.as_str() {
                            "STABLE" => UP,
                            "EXPLOSIVE_CONFIRMED" | "EXPLOSIVE_LIKELY" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — SADF {:+.3} — as of {}",
                                snap.symbol, snap.sadf_label, snap.sadf_stat, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("sadf_summary")
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
                                ui.label(egui::RichText::new("Min window r0").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.min_window))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Full-sample ADF t").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.adf_full))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("SADF statistic").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.sadf_stat))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Argmax end index").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.sadf_argmax_end))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Critical 5%").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.critical_95))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Reject null").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.reject_null))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_sadf = open;
        }

        if self.show_cordim {
            if self.cordim_symbol.is_empty() {
                self.cordim_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cordim;
            egui::Window::new("CORDIM — Grassberger-Procaccia Correlation Dimension D2")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cordim_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.cordim_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.cordim_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_cordim(&conn, &sym_u)
                                    {
                                        self.cordim_snapshot = snap;
                                        self.cordim_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cordim_symbol.to_uppercase();
                            self.cordim_loading = true;
                            self.cordim_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCordimSnapshot { symbol: sym });
                        }
                        if self.cordim_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.cordim_snapshot;
                    if snap.symbol.is_empty() || snap.cordim_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥60 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.cordim_label.as_str() {
                            "LOW_DIM" => UP,
                            "STOCHASTIC" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — D2 {:.3} — as of {}",
                                snap.symbol, snap.cordim_label, snap.d2, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("cordim_summary")
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
                                ui.label(egui::RichText::new("Embedding dim m").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.embed_dim))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Radii fitted").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.radii_count))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("D2 (correlation dim)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.d2))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Fit R²").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.r_squared))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_cordim = open;
        }

        if self.show_skspec {
            if self.skspec_symbol.is_empty() {
                self.skspec_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_skspec;
            egui::Window::new("SKSPEC — Rolling-Window Skewness Spectrum")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.skspec_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.skspec_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.skspec_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_skspec(&conn, &sym_u)
                                    {
                                        self.skspec_snapshot = snap;
                                        self.skspec_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.skspec_symbol.to_uppercase();
                            self.skspec_loading = true;
                            self.skspec_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeSkspecSnapshot { symbol: sym });
                        }
                        if self.skspec_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.skspec_snapshot;
                    if snap.symbol.is_empty() || snap.skspec_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥60 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.skspec_label.as_str() {
                            "STABLE_POSITIVE" | "STABLE_NEGATIVE" => UP,
                            "UNSTABLE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — mean skew {:+.3} — as of {}",
                                snap.symbol, snap.skspec_label, snap.mean_skew, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("skspec_summary")
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
                                ui.label(egui::RichText::new("Window size").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.window_size))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Mean skew").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.mean_skew))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Std of skew").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.std_skew))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Min skew").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.min_skew))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Max skew").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.max_skew))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Range (max−min)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.range_skew))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_skspec = open;
        }

        if self.show_automi {
            if self.automi_symbol.is_empty() {
                self.automi_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_automi;
            egui::Window::new("AUTOMI — Auto Mutual Information (Info-Theoretic ACF)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.automi_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.automi_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.automi_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_automi(&conn, &sym_u)
                                    {
                                        self.automi_snapshot = snap;
                                        self.automi_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.automi_symbol.to_uppercase();
                            self.automi_loading = true;
                            self.automi_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeAutomiSnapshot { symbol: sym });
                        }
                        if self.automi_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.automi_snapshot;
                    if snap.symbol.is_empty() || snap.automi_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥50 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.automi_label.as_str() {
                            "INDEPENDENT" | "WEAK" => UP,
                            "STRONG" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — MI(1) {:.4} — as of {}",
                                snap.symbol, snap.automi_label, snap.mi_lag1, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("automi_summary")
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
                                ui.label(egui::RichText::new("Bins per marginal").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.num_bins))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("MI lag-1 (bits)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.mi_lag1))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("MI lag-5").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.mi_lag5))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("MI lag-10").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.mi_lag10))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("H(X) marginal (bits)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.h_marginal))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("MI(1) / H(X)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.normalized_mi1))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_automi = open;
        }
    }
}
