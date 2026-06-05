use super::*;

impl TyphooNApp {
    pub(super) fn render_research_residual_iid_heteroskedastic_cycles_windows(
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

        // ── Research Round 40 windows ──
        if self.show_durbinwatson {
            if self.durbinwatson_symbol.is_empty() {
                self.durbinwatson_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_durbinwatson;
            egui::Window::new("DURBINWATSON — Durbin-Watson Residual Autocorrelation")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.durbinwatson_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.durbinwatson_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.durbinwatson_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_durbinwatson(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.durbinwatson_snapshot = snap;
                                        self.durbinwatson_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.durbinwatson_symbol.to_uppercase();
                            self.durbinwatson_loading = true;
                            self.durbinwatson_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeDurbinWatsonSnapshot { symbol: sym });
                        }
                        if self.durbinwatson_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.durbinwatson_snapshot;
                    if snap.symbol.is_empty() || snap.dw_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥40 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.dw_label.as_str() {
                            "NO_AUTOCORR" => UP,
                            "STRONG_POS" | "STRONG_NEG" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — d {:.4} — as of {}",
                                snap.symbol, snap.dw_label, snap.dw_stat, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("dw_summary")
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
                                ui.label(egui::RichText::new("DW d-statistic").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.dw_stat))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Implied ρ̂").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.rho_estimate))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_durbinwatson = open;
        }

        if self.show_bdstest {
            if self.bdstest_symbol.is_empty() {
                self.bdstest_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_bdstest;
            egui::Window::new("BDSTEST — Brock-Dechert-Scheinkman iid Test")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.bdstest_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.bdstest_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.bdstest_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_bdstest(&conn, &sym_u)
                                    {
                                        self.bdstest_snapshot = snap;
                                        self.bdstest_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.bdstest_symbol.to_uppercase();
                            self.bdstest_loading = true;
                            self.bdstest_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeBdsTestSnapshot { symbol: sym });
                        }
                        if self.bdstest_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.bdstest_snapshot;
                    if snap.symbol.is_empty() || snap.bds_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥60 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.bds_label.as_str() {
                            "IID_CONFIRMED" => UP,
                            "WEAK_DEPENDENCE" | "STRONG_DEPENDENCE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — BDS {:+.3} — as of {}",
                                snap.symbol, snap.bds_label, snap.bds_stat, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("bds_summary")
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
                                ui.label(egui::RichText::new("ε / σ").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.epsilon_mult))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("BDS statistic").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.bds_stat))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("p-value (2-sided)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.p_value_two_sided))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Reject iid null").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.reject_null))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_bdstest = open;
        }

        if self.show_breuschpagan {
            if self.breuschpagan_symbol.is_empty() {
                self.breuschpagan_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_breuschpagan;
            egui::Window::new("BREUSCHPAGAN — Breusch-Pagan Heteroskedasticity LM Test")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.breuschpagan_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.breuschpagan_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.breuschpagan_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_breuschpagan(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.breuschpagan_snapshot = snap;
                                        self.breuschpagan_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.breuschpagan_symbol.to_uppercase();
                            self.breuschpagan_loading = true;
                            self.breuschpagan_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeBreuschPaganSnapshot { symbol: sym });
                        }
                        if self.breuschpagan_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.breuschpagan_snapshot;
                    if snap.symbol.is_empty() || snap.bp_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥40 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.bp_label.as_str() {
                            "HOMOSKEDASTIC" => UP,
                            "MILD_HETERO" | "STRONG_HETERO" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — LM {:.3} — as of {}",
                                snap.symbol, snap.bp_label, snap.lm_stat, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("bp_summary")
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
                                    egui::RichText::new("LM statistic (n×R²)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.lm_stat))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Aux-regression R²").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.r_squared))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Degrees of freedom").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.df))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("χ²(df) 95% critical").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.critical_95))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Reject homoskedasticity")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.reject_null))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_breuschpagan = open;
        }

        if self.show_turnpts {
            if self.turnpts_symbol.is_empty() {
                self.turnpts_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_turnpts;
            egui::Window::new("TURNPTS — Bartels Turning-Points Test")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.turnpts_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.turnpts_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.turnpts_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_turnpts(&conn, &sym_u)
                                    {
                                        self.turnpts_snapshot = snap;
                                        self.turnpts_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.turnpts_symbol.to_uppercase();
                            self.turnpts_loading = true;
                            self.turnpts_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeTurnPtsSnapshot { symbol: sym });
                        }
                        if self.turnpts_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.turnpts_snapshot;
                    if snap.symbol.is_empty() || snap.turnpts_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥40 closes.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.turnpts_label.as_str() {
                            "RANDOM_IID" => UP,
                            "OVER_TURNING" | "UNDER_TURNING" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — z {:+.3} — as of {}",
                                snap.symbol, snap.turnpts_label, snap.z_stat, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("turnpts_summary")
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
                                ui.label(
                                    egui::RichText::new("Observed turning pts").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.observed_turnpts))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Expected 2(n−2)/3").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.1}", snap.expected_turnpts))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Variance").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.variance_turnpts))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("z-statistic").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.z_stat))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("p (2-sided)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.p_value_two_sided))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Reject randomness").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.reject_null))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_turnpts = open;
        }

        if self.show_periodogram {
            if self.periodogram_symbol.is_empty() {
                self.periodogram_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_periodogram;
            egui::Window::new("PERIODOGRAM — Direct-DFT Dominant-Cycle Detection")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.periodogram_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.periodogram_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.periodogram_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_periodogram(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.periodogram_snapshot = snap;
                                        self.periodogram_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.periodogram_symbol.to_uppercase();
                            self.periodogram_loading = true;
                            self.periodogram_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputePeriodogramSnapshot { symbol: sym });
                        }
                        if self.periodogram_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.periodogram_snapshot;
                    if snap.symbol.is_empty() || snap.periodogram_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥60 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.periodogram_label.as_str() {
                            "STRONG_CYCLE" | "MODERATE_CYCLE" => UP,
                            "WEAK_CYCLE" | "NO_CYCLE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — period {:.1} bars — as of {}",
                                snap.symbol,
                                snap.periodogram_label,
                                snap.dominant_period_bars,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("pgram_summary")
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
                                    egui::RichText::new("Frequencies evaluated")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.n_freqs))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Dominant frequency").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.5}", snap.dominant_freq))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Dominant period (bars)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.1}",
                                        snap.dominant_period_bars
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Dominant power").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3e}", snap.dominant_power))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Total power").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3e}", snap.total_power))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Dominant / total").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.3}",
                                        snap.dominant_power_ratio
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_periodogram = open;
        }
    }
}
