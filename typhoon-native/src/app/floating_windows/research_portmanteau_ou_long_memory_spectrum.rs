use super::*;

impl TyphooNApp {
    pub(super) fn render_research_portmanteau_ou_long_memory_spectrum_windows(
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

        // ── Research Round 41 windows ──
        if self.show_mcleodli {
            if self.mcleodli_symbol.is_empty() {
                self.mcleodli_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_mcleodli;
            egui::Window::new("MCLEODLI — McLeod-Li Squared-Returns Portmanteau")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.mcleodli_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.mcleodli_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.mcleodli_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_mcleodli(&conn, &sym_u)
                                    {
                                        self.mcleodli_snapshot = snap;
                                        self.mcleodli_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.mcleodli_symbol.to_uppercase();
                            self.mcleodli_loading = true;
                            self.mcleodli_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMcLeodLiSnapshot { symbol: sym });
                        }
                        if self.mcleodli_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.mcleodli_snapshot;
                    if snap.symbol.is_empty() || snap.mcleodli_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.mcleodli_label.as_str() {
                            "NO_ARCH" => UP,
                            "MILD_ARCH" | "STRONG_ARCH" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — Q {:.3} — as of {}",
                                snap.symbol, snap.mcleodli_label, snap.q_stat, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("mcl_summary")
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
                                ui.label(egui::RichText::new("Lag h").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.lag_h))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Q statistic").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.q_stat))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Critical χ²(h) 95%").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.critical_95))
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
                                ui.label(egui::RichText::new("Reject NO_ARCH").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.reject_null))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_mcleodli = open;
        }

        if self.show_oufit {
            if self.oufit_symbol.is_empty() {
                self.oufit_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_oufit;
            egui::Window::new("OUFIT — Ornstein-Uhlenbeck Mean-Reversion Fit")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 340.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.oufit_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.oufit_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.oufit_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_oufit(&conn, &sym_u)
                                    {
                                        self.oufit_snapshot = snap;
                                        self.oufit_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.oufit_symbol.to_uppercase();
                            self.oufit_loading = true;
                            self.oufit_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeOuFitSnapshot { symbol: sym });
                        }
                        if self.oufit_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.oufit_snapshot;
                    if snap.symbol.is_empty() || snap.oufit_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.oufit_label.as_str() {
                            "FAST_REVERT" | "MODERATE_REVERT" => UP,
                            "TRENDING" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        let hl_s = if snap.half_life_bars.is_finite() {
                            format!("{:.2} bars", snap.half_life_bars)
                        } else {
                            "∞".to_string()
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — half-life {} — as of {}",
                                snap.symbol, snap.oufit_label, hl_s, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("ou_summary")
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
                                ui.label(egui::RichText::new("θ (speed)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.5}", snap.theta))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("μ (long-run log-price)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.mu))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("σ (diffusion)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.5}", snap.sigma))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Half-life (bars)").small().strong());
                                ui.label(egui::RichText::new(hl_s).small().monospace());
                                ui.end_row();
                                ui.label(egui::RichText::new("Residual sd").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.5}", snap.residual_sd))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("R²").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.r_squared))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_oufit = open;
        }

        if self.show_gph {
            if self.gph_symbol.is_empty() {
                self.gph_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_gph;
            egui::Window::new("GPH — Geweke-Porter-Hudak Long-Memory d̂")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.gph_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.gph_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.gph_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_gph(&conn, &sym_u)
                                    {
                                        self.gph_snapshot = snap;
                                        self.gph_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.gph_symbol.to_uppercase();
                            self.gph_loading = true;
                            self.gph_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeGphSnapshot { symbol: sym });
                        }
                        if self.gph_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.gph_snapshot;
                    if snap.symbol.is_empty() || snap.gph_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥64 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.gph_label.as_str() {
                            "SHORT_MEMORY" => UP,
                            "NONSTATIONARY" | "ANTIPERSISTENT" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — d̂ {:+.3} — as of {}",
                                snap.symbol, snap.gph_label, snap.d_estimate, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("gph_summary")
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
                                ui.label(egui::RichText::new("m (bandwidth)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.m_freqs))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("d̂ estimate").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.d_estimate))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Standard error").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.d_stderr))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("t-stat (H0: d=0)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}", snap.t_stat))
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
                            });
                    }
                });
            self.show_gph = open;
        }

        if self.show_burgspec {
            if self.burgspec_symbol.is_empty() {
                self.burgspec_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_burgspec;
            egui::Window::new("BURGSPEC — Burg Maximum-Entropy AR Spectrum")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 340.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.burgspec_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.burgspec_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.burgspec_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_burgspec(&conn, &sym_u)
                                    {
                                        self.burgspec_snapshot = snap;
                                        self.burgspec_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.burgspec_symbol.to_uppercase();
                            self.burgspec_loading = true;
                            self.burgspec_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeBurgSpecSnapshot { symbol: sym });
                        }
                        if self.burgspec_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.burgspec_snapshot;
                    if snap.symbol.is_empty() || snap.burgspec_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥32 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.burgspec_label.as_str() {
                            "STRONG_AR_CYCLE" | "MODERATE_AR_CYCLE" => UP,
                            "WEAK_AR_CYCLE" | "NO_AR_CYCLE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — period {:.1} bars — as of {}",
                                snap.symbol,
                                snap.burgspec_label,
                                snap.dominant_period_bars,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("burg_summary")
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
                                ui.label(egui::RichText::new("AR order p").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.ar_order))
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
                                ui.label(egui::RichText::new("Peak power").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3e}", snap.peak_power))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Mean power").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3e}", snap.mean_power))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Peak / mean").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.peak_to_mean_ratio))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_burgspec = open;
        }

        if self.show_kendalltau {
            if self.kendalltau_symbol.is_empty() {
                self.kendalltau_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_kendalltau;
            egui::Window::new("KENDALLTAU — Kendall's Tau Lag-1 Rank Autocorrelation")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.kendalltau_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.kendalltau_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.kendalltau_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_kendalltau(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.kendalltau_snapshot = snap;
                                        self.kendalltau_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.kendalltau_symbol.to_uppercase();
                            self.kendalltau_loading = true;
                            self.kendalltau_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeKendallTauSnapshot { symbol: sym });
                        }
                        if self.kendalltau_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.kendalltau_snapshot;
                    if snap.symbol.is_empty() || snap.kendalltau_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.kendalltau_label.as_str() {
                            "NO_RANK_AUTO" => UP,
                            "STRONG_POS" | "STRONG_NEG" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — τ {:+.4} — as of {}",
                                snap.symbol, snap.kendalltau_label, snap.tau, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("ktau_summary")
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
                                ui.label(egui::RichText::new("Pair count").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.pair_count))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Concordant").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.concordant))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Discordant").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.discordant))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("τ").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.tau))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("z-stat").small().strong());
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
                            });
                    }
                });
            self.show_kendalltau = open;
        }
    }
}
