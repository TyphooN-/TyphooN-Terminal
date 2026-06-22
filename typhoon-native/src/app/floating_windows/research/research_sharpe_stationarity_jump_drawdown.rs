use super::*;

impl TyphooNApp {
    pub(super) fn render_research_sharpe_stationarity_jump_drawdown_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research = research_chart_symbol(
            self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
        );

        // ── Research section ──
        if self.show_psr {
            if self.psr_symbol.is_empty() {
                self.psr_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_psr;
            egui::Window::new("PSR — Probabilistic Sharpe Ratio")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.psr_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.psr_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.psr_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_psr(&conn, &sym_u)
                                    {
                                        self.psr_snapshot = snap;
                                        self.psr_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.psr_symbol.to_uppercase();
                            self.psr_loading = true;
                            self.psr_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputePsrSnapshot { symbol: sym });
                        }
                        if self.psr_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.psr_snapshot;
                    if snap.symbol.is_empty() || snap.psr_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.psr_label.as_str() {
                            "VERY_HIGH" | "HIGH" => UP,
                            "VERY_LOW" | "LOW" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — PSR {:.4} — SR {:.3} — skew {:+.3} — kurt {:.2} — as of {}",
                            snap.symbol, snap.psr_label, snap.psr, snap.sharpe,
                            snap.skewness, snap.kurtosis, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("psr_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("PSR(SR*=0)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.psr))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Annualized Sharpe").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.sharpe))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Skewness γ₃").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.skewness))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Kurtosis γ₄").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.kurtosis))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("SR benchmark").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sr_benchmark))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Bars used").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.bars_used))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_psr = open;
        }

        if self.show_adf {
            if self.adf_symbol.is_empty() {
                self.adf_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_adf;
            egui::Window::new("ADF — Dickey-Fuller Unit-Root Test")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.adf_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.adf_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.adf_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_adf(&conn, &sym_u)
                                    {
                                        self.adf_snapshot = snap;
                                        self.adf_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.adf_symbol.to_uppercase();
                            self.adf_loading = true;
                            self.adf_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeAdfSnapshot { symbol: sym });
                        }
                        if self.adf_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.adf_snapshot;
                    if snap.symbol.is_empty() || snap.adf_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new(
                                "No data — HP cache needs ≥30 bars with positive closes.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.adf_label.as_str() {
                            "STATIONARY" => UP,
                            "NON_STATIONARY" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — t {:+.3} — β {:+.4} — crit5% {:+.2} — reject {} — as of {}",
                            snap.symbol, snap.adf_label, snap.t_statistic, snap.beta,
                            snap.crit_5pct, snap.reject_unit_root, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("adf_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("β (slope)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.6}", snap.beta))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("SE(β)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.se_beta))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("t-statistic").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.t_statistic))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Crit 1%").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.crit_1pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Crit 5%").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.crit_5pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Crit 10%").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.crit_10pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Reject unit root").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.reject_unit_root))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_adf = open;
        }

        if self.show_mnkendall {
            if self.mnkendall_symbol.is_empty() {
                self.mnkendall_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_mnkendall;
            egui::Window::new("MNKENDALL — Mann-Kendall Trend Test")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.mnkendall_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.mnkendall_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.mnkendall_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_mnkendall(&conn, &sym_u)
                                    {
                                        self.mnkendall_snapshot = snap;
                                        self.mnkendall_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.mnkendall_symbol.to_uppercase();
                            self.mnkendall_loading = true;
                            self.mnkendall_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMnkendallSnapshot { symbol: sym });
                        }
                        if self.mnkendall_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.mnkendall_snapshot;
                    if snap.symbol.is_empty() || snap.mk_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new(
                                "No data — HP cache needs ≥30 bars with positive closes.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.mk_label.as_str() {
                            "STRONG_UP" | "UP" => UP,
                            "STRONG_DOWN" | "DOWN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — S {} — z {:+.3} — p {:.4} — τ {:+.3} — as of {}",
                                snap.symbol,
                                snap.mk_label,
                                snap.s_statistic,
                                snap.z_statistic,
                                snap.p_value,
                                snap.tau,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("mnkendall_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("S-statistic").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.s_statistic))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Variance").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.1}", snap.variance))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("z-statistic").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.z_statistic))
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
                                ui.label(egui::RichText::new("Kendall τ").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.tau))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Reject no-trend").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.reject_no_trend))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Bars used").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.bars_used))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_mnkendall = open;
        }

        if self.show_bipower {
            if self.bipower_symbol.is_empty() {
                self.bipower_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_bipower;
            egui::Window::new("BIPOWER — Bipower Variation / Jump Ratio")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.bipower_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.bipower_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.bipower_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_bipower(&conn, &sym_u)
                                    {
                                        self.bipower_snapshot = snap;
                                        self.bipower_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.bipower_symbol.to_uppercase();
                            self.bipower_loading = true;
                            self.bipower_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeBipowerSnapshot { symbol: sym });
                        }
                        if self.bipower_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.bipower_snapshot;
                    if snap.symbol.is_empty() || snap.jump_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.jump_label.as_str() {
                            "NO_JUMPS" => UP,
                            "HEAVY_JUMPS" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — jump {:.1}% — cont vol {:.2}% — RV vol {:.2}% — as of {}",
                            snap.symbol, snap.jump_label, snap.jump_pct,
                            snap.continuous_vol_ann_pct, snap.realized_vol_ann_pct, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("bipower_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("Realized variance (RV)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.realized_var))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Bipower variation (BPV)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.bipower_var))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Continuous vol (ann %)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.3}",
                                        snap.continuous_vol_ann_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Realized vol (ann %)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.3}",
                                        snap.realized_vol_ann_pct
                                    ))
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
                                ui.label(egui::RichText::new("Jump %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.jump_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_bipower = open;
        }

        if self.show_dddur {
            if self.dddur_symbol.is_empty() {
                self.dddur_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_dddur;
            egui::Window::new("DDDUR — Drawdown Duration Statistics")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.dddur_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.dddur_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.dddur_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_dddur(&conn, &sym_u)
                                    {
                                        self.dddur_snapshot = snap;
                                        self.dddur_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.dddur_symbol.to_uppercase();
                            self.dddur_loading = true;
                            self.dddur_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeDddurSnapshot { symbol: sym });
                        }
                        if self.dddur_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.dddur_snapshot;
                    if snap.symbol.is_empty() || snap.dddur_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.dddur_label.as_str() {
                            "MOSTLY_DRY" => UP,
                            "DEEP_WATER" | "PERSISTENT_DD" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — underwater {:.1}% — events {} — max {} bars — as of {}",
                                snap.symbol,
                                snap.dddur_label,
                                snap.pct_time_underwater,
                                snap.dd_event_count,
                                snap.max_dd_duration_bars,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("dddur_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("Drawdown events (closed)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.dd_event_count))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Max duration (bars)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.max_dd_duration_bars))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Mean duration").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2}",
                                        snap.mean_dd_duration_bars
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Median duration").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2}",
                                        snap.median_dd_duration_bars
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Total bars underwater")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.total_bars_underwater))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("% time underwater").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.pct_time_underwater))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Currently underwater").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.currently_underwater))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Current DD duration").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}",
                                        snap.current_dd_duration_bars
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_dddur = open;
        }
    }
}
