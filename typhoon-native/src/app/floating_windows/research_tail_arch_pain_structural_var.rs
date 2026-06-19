use super::*;

impl TyphooNApp {
    pub(super) fn render_research_tail_arch_pain_structural_var_windows(
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

        // ── Research Round 31 windows ──
        if self.show_hilltail {
            if self.hilltail_symbol.is_empty() {
                self.hilltail_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_hilltail;
            egui::Window::new("HILLTAIL — Hill Tail-Index Estimator")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.hilltail_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.hilltail_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.hilltail_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_hilltail(&conn, &sym_u) {
                                        self.hilltail_snapshot = snap;
                                        self.hilltail_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.hilltail_symbol.to_uppercase();
                            self.hilltail_loading = true;
                            self.hilltail_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeHilltailSnapshot { symbol: sym });
                        }
                        if self.hilltail_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.hilltail_snapshot;
                    if snap.symbol.is_empty() || snap.tail_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥50 returns.").color(AXIS_TEXT).small());
                        if !snap.note.is_empty() { ui.label(egui::RichText::new(&snap.note).color(DOWN).small()); }
                    } else {
                        let color = match snap.tail_label.as_str() {
                            "GAUSSIAN_LIKE" | "LIGHT_TAIL" => UP,
                            "HEAVY_TAIL" | "VERY_HEAVY_TAIL" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — α(|r|)={:.2} — α(left)={:.2} — α(right)={:.2} — as of {}",
                            snap.symbol, snap.tail_label, snap.hill_alpha_abs, snap.hill_alpha_left, snap.hill_alpha_right, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("hilltail_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong());
                            ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("k order statistics").small().strong());
                            ui.label(egui::RichText::new(format!("{}", snap.k_order_stats)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Threshold |r|(k+1)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.6}", snap.threshold_abs)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Hill α (|r|)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.4}", snap.hill_alpha_abs)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Hill α (left tail)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.4}", snap.hill_alpha_left)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Hill α (right tail)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.4}", snap.hill_alpha_right)).small().monospace());
                            ui.end_row();
                        });
                    }
                });
            self.show_hilltail = open;
        }

        if self.show_archlm {
            if self.archlm_symbol.is_empty() {
                self.archlm_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_archlm;
            egui::Window::new("ARCHLM — Engle ARCH-LM Test")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.archlm_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.archlm_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.archlm_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_archlm(&conn, &sym_u)
                                    {
                                        self.archlm_snapshot = snap;
                                        self.archlm_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.archlm_symbol.to_uppercase();
                            self.archlm_loading = true;
                            self.archlm_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeArchlmSnapshot { symbol: sym });
                        }
                        if self.archlm_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.archlm_snapshot;
                    if snap.symbol.is_empty() || snap.arch_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥35 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.arch_label.as_str() {
                            "NO_ARCH" => UP,
                            "STRONG_ARCH" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — LM={:.3} — p={:.4} — as of {}",
                                snap.symbol,
                                snap.arch_label,
                                snap.lm_statistic,
                                snap.p_value,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("archlm_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Returns used").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.bars_used))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Lags q").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.q_lags))
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
                                ui.label(egui::RichText::new("LM = n·R²").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.lm_statistic))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("p-value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.p_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("crit χ²(5) @5% / @1%").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.3} / {:.3}",
                                        snap.crit_5pct_chi2, snap.crit_1pct_chi2
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Reject homoskedastic").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.reject_homoskedastic))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_archlm = open;
        }

        if self.show_painratio {
            if self.painratio_symbol.is_empty() {
                self.painratio_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_painratio;
            egui::Window::new("PAINRATIO — Pain Index + Pain Ratio")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 360.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.painratio_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.painratio_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.painratio_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_painratio(&conn, &sym_u)
                                    {
                                        self.painratio_snapshot = snap;
                                        self.painratio_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.painratio_symbol.to_uppercase();
                            self.painratio_loading = true;
                            self.painratio_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputePainratioSnapshot { symbol: sym });
                        }
                        if self.painratio_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.painratio_snapshot;
                    if snap.symbol.is_empty() || snap.pain_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.pain_label.as_str() {
                            "LOW_PAIN" | "MILD_PAIN" => UP,
                            "HIGH_PAIN" | "SEVERE_PAIN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — pain {:.2}% — ratio {:.3} — ann ret {:.2}% — as of {}",
                                snap.symbol,
                                snap.pain_label,
                                snap.pain_index_pct,
                                snap.pain_ratio,
                                snap.annualized_return_pct,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("painratio_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Bars used").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.bars_used))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Pain index (mean |dd|, %)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.pain_index_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Annualized return (%)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.4}",
                                        snap.annualized_return_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Pain ratio").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.pain_ratio))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Max drawdown (%)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.max_dd_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_painratio = open;
        }

        if self.show_cusum {
            if self.cusum_symbol.is_empty() {
                self.cusum_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cusum;
            egui::Window::new("CUSUM — Brown-Durbin-Evans Structural Break Test")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cusum_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.cusum_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.cusum_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_cusum(&conn, &sym_u)
                                    {
                                        self.cusum_snapshot = snap;
                                        self.cusum_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cusum_symbol.to_uppercase();
                            self.cusum_loading = true;
                            self.cusum_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCusumSnapshot { symbol: sym });
                        }
                        if self.cusum_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.cusum_snapshot;
                    if snap.symbol.is_empty() || snap.cusum_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.cusum_label.as_str() {
                            "STABLE" => UP,
                            "BREAK_DETECTED" | "STRONG_BREAK" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — D={:.3} — dir {} — bar {} — as of {}",
                                snap.symbol,
                                snap.cusum_label,
                                snap.test_statistic,
                                snap.direction_at_max,
                                snap.max_abs_bar,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("cusum_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Returns used").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.bars_used))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("max |S_t|").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.max_abs_cusum))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("D = max|S_t|/√n").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.test_statistic))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("bar at max").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.max_abs_bar))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("direction at max").small().strong());
                                ui.label(
                                    egui::RichText::new(&snap.direction_at_max)
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("crit 10% / 5% / 1%").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2} / {:.2} / {:.2}",
                                        snap.crit_10pct, snap.crit_5pct, snap.crit_1pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Reject stability").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.reject_stability))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_cusum = open;
        }

        if self.show_cfvar {
            if self.cfvar_symbol.is_empty() {
                self.cfvar_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cfvar;
            egui::Window::new("CFVAR — Cornish-Fisher Modified VaR")
                .open(&mut open)
                .resizable(true)
                .default_size([620.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.cfvar_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.cfvar_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.cfvar_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_cfvar(&conn, &sym_u) {
                                        self.cfvar_snapshot = snap;
                                        self.cfvar_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cfvar_symbol.to_uppercase();
                            self.cfvar_loading = true;
                            self.cfvar_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCfvarSnapshot { symbol: sym });
                        }
                        if self.cfvar_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.cfvar_snapshot;
                    if snap.symbol.is_empty() || snap.cfvar_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥30 returns.").color(AXIS_TEXT).small());
                        if !snap.note.is_empty() { ui.label(egui::RichText::new(&snap.note).color(DOWN).small()); }
                    } else {
                        let color = match snap.cfvar_label.as_str() {
                            "BENIGN" => UP,
                            "EXTREME_DEVIATION" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — CF-VaR(5%)={:.2}% vs Gauss {:.2}% — adj {:+.3}pp — as of {}",
                            snap.symbol, snap.cfvar_label, snap.cf_var_5pct_pct, snap.gauss_var_5pct_pct, snap.cf_adjustment_5pct_pct, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("cfvar_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Returns used").small().strong());
                            ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Mean ret (%)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.4}", snap.mean_ret_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("σ ret (%)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.4}", snap.sigma_ret_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Skewness γ₃").small().strong());
                            ui.label(egui::RichText::new(format!("{:.4}", snap.skewness)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Excess kurtosis γ₄").small().strong());
                            ui.label(egui::RichText::new(format!("{:.4}", snap.excess_kurtosis)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Gauss VaR 5% (%)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.4}", snap.gauss_var_5pct_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("CF-VaR 5% (%)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.4}", snap.cf_var_5pct_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Gauss VaR 1% (%)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.4}", snap.gauss_var_1pct_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("CF-VaR 1% (%)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.4}", snap.cf_var_1pct_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Adj 5% (pp)").small().strong());
                            ui.label(egui::RichText::new(format!("{:+.4}", snap.cf_adjustment_5pct_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Skew term @ 5%").small().strong());
                            ui.label(egui::RichText::new(format!("{:+.4}", snap.skew_term_5pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Kurt term @ 5%").small().strong());
                            ui.label(egui::RichText::new(format!("{:+.4}", snap.kurt_term_5pct)).small().monospace());
                            ui.end_row();
                        });
                    }
                });
            self.show_cfvar = open;
        }
    }
}
