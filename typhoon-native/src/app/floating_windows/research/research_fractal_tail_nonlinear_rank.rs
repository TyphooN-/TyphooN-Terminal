use super::*;

impl TyphooNApp {
    pub(super) fn render_research_fractal_tail_nonlinear_rank_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research = research_chart_symbol(
            self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
        );

        // ── Research section ──
        if self.show_higuchi {
            if self.higuchi_symbol.is_empty() {
                self.higuchi_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_higuchi;
            egui::Window::new("HIGUCHI — Higuchi Fractal Dimension (1988)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.higuchi_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.higuchi_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.higuchi_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_higuchi(&conn, &sym_u)
                                    {
                                        self.higuchi_snapshot = snap;
                                        self.higuchi_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.higuchi_symbol.to_uppercase();
                            self.higuchi_loading = true;
                            self.higuchi_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeHiguchiSnapshot { symbol: sym });
                        }
                        if self.higuchi_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.higuchi_snapshot;
                    if snap.symbol.is_empty() || snap.higuchi_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥100 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.higuchi_label.as_str() {
                            "SMOOTH" => UP,
                            "ROUGH" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — FD {:.4} — as of {}",
                                snap.symbol, snap.higuchi_label, snap.fractal_dim, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("higuchi_summary")
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
                                ui.label(egui::RichText::new("k_max").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.k_max))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Fractal dim (FD)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.fractal_dim))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("R² (log-k fit)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.r_squared))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("log-k points").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.log_k_count))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_higuchi = open;
        }

        if self.show_pickands {
            if self.pickands_symbol.is_empty() {
                self.pickands_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_pickands;
            egui::Window::new("PICKANDS — Pickands 1975 Tail-Index Estimator")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.pickands_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.pickands_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.pickands_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_pickands(&conn, &sym_u)
                                    {
                                        self.pickands_snapshot = snap;
                                        self.pickands_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.pickands_symbol.to_uppercase();
                            self.pickands_loading = true;
                            self.pickands_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputePickandsSnapshot { symbol: sym });
                        }
                        if self.pickands_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.pickands_snapshot;
                    if snap.symbol.is_empty() || snap.pickands_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥80 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.pickands_label.as_str() {
                            "WEIBULL_BOUNDED" | "GUMBEL_EXPONENTIAL" => UP,
                            "FRECHET_HEAVY" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — γ̂ {:+.4} — as of {}",
                                snap.symbol, snap.pickands_label, snap.gamma_hat, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("pickands_summary")
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
                                ui.label(egui::RichText::new("k (order-stat)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.k_index))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("γ̂ (Pickands)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.gamma_hat))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Tail α = 1/γ̂").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.tail_index))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("x_k").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.5}", snap.x_k))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("x_2k").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.5}", snap.x_2k))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("x_4k").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.5}", snap.x_4k))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_pickands = open;
        }

        if self.show_kappa3 {
            if self.kappa3_symbol.is_empty() {
                self.kappa3_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_kappa3;
            egui::Window::new("KAPPA3 — Kaplan-Knowles 2004 Kappa-3 Ratio")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.kappa3_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.kappa3_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.kappa3_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_kappa3(&conn, &sym_u)
                                    {
                                        self.kappa3_snapshot = snap;
                                        self.kappa3_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.kappa3_symbol.to_uppercase();
                            self.kappa3_loading = true;
                            self.kappa3_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeKappa3Snapshot { symbol: sym });
                        }
                        if self.kappa3_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.kappa3_snapshot;
                    if snap.symbol.is_empty() || snap.kappa3_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.kappa3_label.as_str() {
                            "STRONG" | "POSITIVE" => UP,
                            "NEGATIVE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — κ3 {:+.4} — as of {}",
                                snap.symbol, snap.kappa3_label, snap.kappa3, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("kappa3_summary")
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
                                ui.label(egui::RichText::new("MAR").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.mar))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Excess μ (annualised)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.excess_mean))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("LPM3").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3e}", snap.lpm3))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("LPM3^(1/3) (ann)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.lpm3_root))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Kappa-3").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.kappa3))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Sortino (reference)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.sortino_compare))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_kappa3 = open;
        }

        if self.show_lyapunov {
            if self.lyapunov_symbol.is_empty() {
                self.lyapunov_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_lyapunov;
            egui::Window::new("LYAPUNOV — Largest Lyapunov Exponent (Rosenstein 1993)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.lyapunov_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.lyapunov_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.lyapunov_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_lyapunov(&conn, &sym_u)
                                    {
                                        self.lyapunov_snapshot = snap;
                                        self.lyapunov_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.lyapunov_symbol.to_uppercase();
                            self.lyapunov_loading = true;
                            self.lyapunov_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeLyapunovSnapshot { symbol: sym });
                        }
                        if self.lyapunov_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.lyapunov_snapshot;
                    if snap.symbol.is_empty() || snap.lyapunov_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥100 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.lyapunov_label.as_str() {
                            "STABLE" | "PERIODIC" => UP,
                            "CHAOTIC" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — λ {:+.5} — as of {}",
                                snap.symbol, snap.lyapunov_label, snap.lambda_max, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("lyapunov_summary")
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
                                ui.label(egui::RichText::new("Time delay τ").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.time_delay))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("λ_max (per bar)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.5}", snap.lambda_max))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("R² (fit)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.r_squared))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Steps used").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.steps_used))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_lyapunov = open;
        }

        if self.show_rankac {
            if self.rankac_symbol.is_empty() {
                self.rankac_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_rankac;
            egui::Window::new("RANKAC — Spearman Rank Autocorrelation (lags 1/5/10)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.rankac_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.rankac_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.rankac_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_rankac(&conn, &sym_u)
                                    {
                                        self.rankac_snapshot = snap;
                                        self.rankac_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.rankac_symbol.to_uppercase();
                            self.rankac_loading = true;
                            self.rankac_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRankacSnapshot { symbol: sym });
                        }
                        if self.rankac_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.rankac_snapshot;
                    if snap.symbol.is_empty() || snap.rankac_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.rankac_label.as_str() {
                            "INDEPENDENT" | "WEAK_DEPENDENCE" => UP,
                            "STRONG_DEPENDENCE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — max|ρ| {:.4} — as of {}",
                                snap.symbol, snap.rankac_label, snap.max_abs_rho, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("rankac_summary")
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
                                ui.label(egui::RichText::new("ρ(lag 1)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.rho_lag1))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ρ(lag 5)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.rho_lag5))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ρ(lag 10)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.rho_lag10))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("mean |ρ|").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.mean_abs_rho))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("max |ρ|").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.max_abs_rho))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_rankac = open;
        }
    }
}
