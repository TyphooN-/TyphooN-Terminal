use super::*;

impl TyphooNApp {
    pub(super) fn render_research_round38_windows(&mut self, ctx: &egui::Context) {
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

        // ── Research Round 38 windows ──
        if self.show_bnsjump {
            if self.bnsjump_symbol.is_empty() {
                self.bnsjump_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_bnsjump;
            egui::Window::new("BNSJUMP — Barndorff-Nielsen-Shephard Jump-Test Z")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.bnsjump_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.bnsjump_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.bnsjump_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_bnsjump(&conn, &sym_u)
                                    {
                                        self.bnsjump_snapshot = snap;
                                        self.bnsjump_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.bnsjump_symbol.to_uppercase();
                            self.bnsjump_loading = true;
                            self.bnsjump_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeBnsjumpSnapshot { symbol: sym });
                        }
                        if self.bnsjump_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.bnsjump_snapshot;
                    if snap.symbol.is_empty() || snap.bnsjump_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.bnsjump_label.as_str() {
                            "NO_JUMP" => UP,
                            "STRONG_JUMP" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — z {:+.3} — as of {}",
                                snap.symbol, snap.bnsjump_label, snap.jump_z_stat, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("bnsjump_summary")
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
                                ui.label(egui::RichText::new("Realised variance").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3e}", snap.realized_variance))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Bipower variance").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3e}", snap.bipower_variance))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Jump ratio").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.jump_ratio))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Z-statistic").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.jump_z_stat))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("p-value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.p_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_bnsjump = open;
        }

        if self.show_pproot {
            if self.pproot_symbol.is_empty() {
                self.pproot_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_pproot;
            egui::Window::new("PPROOT — Phillips-Perron Unit-Root Test")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.pproot_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.pproot_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.pproot_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_pproot(&conn, &sym_u)
                                    {
                                        self.pproot_snapshot = snap;
                                        self.pproot_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.pproot_symbol.to_uppercase();
                            self.pproot_loading = true;
                            self.pproot_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputePprootSnapshot { symbol: sym });
                        }
                        if self.pproot_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.pproot_snapshot;
                    if snap.symbol.is_empty() || snap.pproot_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 closes.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.pproot_label.as_str() {
                            "STATIONARY_STRONG" | "STATIONARY_WEAK" => UP,
                            "UNIT_ROOT" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — Z(t) {:+.3} — as of {}",
                                snap.symbol, snap.pproot_label, snap.z_t, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("pproot_summary")
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
                                ui.label(egui::RichText::new("ρ̂").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.5}", snap.rho_hat))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Raw t(ρ=1)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.t_rho))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("PP Z(ρ)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.z_rho))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("PP Z(t)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.z_t))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Lag truncation q").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.lag_truncation))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_pproot = open;
        }

        if self.show_mfdfa {
            if self.mfdfa_symbol.is_empty() {
                self.mfdfa_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_mfdfa;
            egui::Window::new("MFDFA — Multifractal DFA (q ∈ {-2, 0, +2})")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.mfdfa_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.mfdfa_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.mfdfa_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_mfdfa(&conn, &sym_u)
                                    {
                                        self.mfdfa_snapshot = snap;
                                        self.mfdfa_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.mfdfa_symbol.to_uppercase();
                            self.mfdfa_loading = true;
                            self.mfdfa_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMfdfaSnapshot { symbol: sym });
                        }
                        if self.mfdfa_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.mfdfa_snapshot;
                    if snap.symbol.is_empty() || snap.mfdfa_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥120 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.mfdfa_label.as_str() {
                            "MONOFRACTAL" | "WEAK_MULTIFRACTAL" => UP,
                            "STRONG_MULTIFRACTAL" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — Δh {:+.4} — as of {}",
                                snap.symbol, snap.mfdfa_label, snap.delta_h, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("mfdfa_summary")
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
                                ui.label(egui::RichText::new("h(q=−2)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.h_q_neg2))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("h(q=0)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.h_q_zero))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("h(q=+2)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.h_q_pos2))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Δh (h(-2)-h(+2))").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.delta_h))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Scales used").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.scales_used))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_mfdfa = open;
        }

        if self.show_hillks {
            if self.hillks_symbol.is_empty() {
                self.hillks_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_hillks;
            egui::Window::new("HILLKS — Hill-Tail KS Goodness-of-Fit")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.hillks_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.hillks_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.hillks_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_hillks(&conn, &sym_u)
                                    {
                                        self.hillks_snapshot = snap;
                                        self.hillks_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.hillks_symbol.to_uppercase();
                            self.hillks_loading = true;
                            self.hillks_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeHillksSnapshot { symbol: sym });
                        }
                        if self.hillks_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.hillks_snapshot;
                    if snap.symbol.is_empty() || snap.hillks_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥50 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.hillks_label.as_str() {
                            "GOOD_FIT" | "ACCEPTABLE_FIT" => UP,
                            "REJECT" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — D {:.4} — as of {}",
                                snap.symbol, snap.hillks_label, snap.ks_statistic, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("hillks_summary")
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
                                ui.label(egui::RichText::new("k (tail size)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.k_order))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("α̂ (Hill)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.alpha_hat))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("KS statistic D").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.ks_statistic))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("KS critical 5%").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.ks_critical_5pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_hillks = open;
        }

        if self.show_tsi {
            if self.tsi_symbol.is_empty() {
                self.tsi_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_tsi;
            egui::Window::new("TSI — True Strength Index (Blau 1991)")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.tsi_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.tsi_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.tsi_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_tsi(&conn, &sym_u)
                                    {
                                        self.tsi_snapshot = snap;
                                        self.tsi_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.tsi_symbol.to_uppercase();
                            self.tsi_loading = true;
                            self.tsi_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeTsiSnapshot { symbol: sym });
                        }
                        if self.tsi_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.tsi_snapshot;
                    if snap.symbol.is_empty() || snap.tsi_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥60 closes.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.tsi_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "BEAR" | "STRONG_BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — TSI {:+.2} — as of {}",
                                snap.symbol, snap.tsi_label, snap.tsi_value, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("tsi_summary")
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
                                ui.label(egui::RichText::new("EMA long / short").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {}",
                                        snap.ema_long, snap.ema_short
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("TSI value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}", snap.tsi_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Signal (EMA short)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}", snap.signal_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("TSI − signal").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}", snap.tsi_minus_signal))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_tsi = open;
        }
    }
}
