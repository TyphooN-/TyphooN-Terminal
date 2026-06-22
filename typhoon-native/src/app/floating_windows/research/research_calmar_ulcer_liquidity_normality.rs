use super::*;

impl TyphooNApp {
    pub(super) fn render_research_calmar_ulcer_liquidity_normality_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research = research_chart_symbol(
            self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
        );

        // ── Research section ──
        if self.show_calmar {
            if self.calmar_symbol.is_empty() {
                self.calmar_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_calmar;
            egui::Window::new("CALMAR — Calmar Ratio (Return / Max Drawdown)")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.calmar_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.calmar_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.calmar_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_calmar(&conn, &sym_u)
                                    {
                                        self.calmar_snapshot = snap;
                                        self.calmar_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.calmar_symbol.to_uppercase();
                            self.calmar_loading = true;
                            self.calmar_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCalmarSnapshot { symbol: sym });
                        }
                        if self.calmar_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.calmar_snapshot;
                    if snap.symbol.is_empty() || snap.calmar_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.calmar_label.as_str() {
                            "GOOD" | "EXCELLENT" => UP,
                            "VERY_POOR" | "POOR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — Calmar {:.2} — {} bars — as of {}",
                                snap.symbol,
                                snap.calmar_label,
                                snap.calmar_ratio,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("calmar_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Total return").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}%", snap.total_return_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Annualized return").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.2}%",
                                        snap.annualized_return_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Max drawdown").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}%", snap.max_drawdown_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Calmar ratio").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.calmar_ratio))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_calmar = open;
        }

        if self.show_ulcer {
            if self.ulcer_symbol.is_empty() {
                self.ulcer_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ulcer;
            egui::Window::new("ULCER — Ulcer Index + Martin Ratio (UPI)")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ulcer_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ulcer_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ulcer_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ulcer(&conn, &sym_u)
                                    {
                                        self.ulcer_snapshot = snap;
                                        self.ulcer_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ulcer_symbol.to_uppercase();
                            self.ulcer_loading = true;
                            self.ulcer_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeUlcerSnapshot { symbol: sym });
                        }
                        if self.ulcer_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.ulcer_snapshot;
                    if snap.symbol.is_empty() || snap.ulcer_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.ulcer_label.as_str() {
                            "LOW_PAIN" => UP,
                            "HIGH" | "SEVERE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — UI {:.2} — Martin {:.2} — {} bars — as of {}",
                                snap.symbol,
                                snap.ulcer_label,
                                snap.ulcer_index,
                                snap.martin_ratio,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("ulcer_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Ulcer index").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.ulcer_index))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Mean drawdown %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}%", snap.mean_drawdown_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Max drawdown %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}%", snap.max_drawdown_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("% in drawdown").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.1}%", snap.pct_in_drawdown))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Annualized return").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.2}%",
                                        snap.annualized_return_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Martin ratio (UPI)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.martin_ratio))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_ulcer = open;
        }

        if self.show_varratio {
            if self.varratio_symbol.is_empty() {
                self.varratio_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_varratio;
            egui::Window::new("VARRATIO — Lo-MacKinlay Variance Ratio")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.varratio_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.varratio_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.varratio_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_varratio(&conn, &sym_u)
                                    {
                                        self.varratio_snapshot = snap;
                                        self.varratio_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.varratio_symbol.to_uppercase();
                            self.varratio_loading = true;
                            self.varratio_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeVarratioSnapshot { symbol: sym });
                        }
                        if self.varratio_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.varratio_snapshot;
                    if snap.symbol.is_empty() || snap.rw_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥40 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.rw_label.as_str() {
                            "TRENDING" | "STRONG_TREND" => UP,
                            "STRONG_REVERT" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — VR(5) {:.3} — z(5) {:+.2} — {} bars — as of {}",
                                snap.symbol,
                                snap.rw_label,
                                snap.vr_5,
                                snap.z_stat_5,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("varratio_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("VR(2)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.vr_2))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("VR(5)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.vr_5))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("VR(10)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.vr_10))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("VR(20)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.vr_20))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("z-stat(2)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.z_stat_2))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("z-stat(5)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.z_stat_5))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_varratio = open;
        }

        if self.show_amihud {
            if self.amihud_symbol.is_empty() {
                self.amihud_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_amihud;
            egui::Window::new("AMIHUD — Amihud Illiquidity Ratio")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.amihud_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.amihud_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.amihud_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_amihud(&conn, &sym_u)
                                    {
                                        self.amihud_snapshot = snap;
                                        self.amihud_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.amihud_symbol.to_uppercase();
                            self.amihud_loading = true;
                            self.amihud_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeAmihudSnapshot { symbol: sym });
                        }
                        if self.amihud_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.amihud_snapshot;
                    if snap.symbol.is_empty() || snap.illiq_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 bars with volume.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.illiq_label.as_str() {
                            "VERY_LIQUID" | "LIQUID" => UP,
                            "ILLIQUID" | "VERY_ILLIQUID" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — ILLIQ {:.4} — {} bars — as of {}",
                                snap.symbol,
                                snap.illiq_label,
                                snap.mean_illiq,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("amihud_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Mean ILLIQ (×1e6)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.mean_illiq))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Median ILLIQ (×1e6)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.median_illiq))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("90th pctile ILLIQ").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.illiq_90th))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Avg daily $ volume").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("${:.0}", snap.avg_dollar_volume))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_amihud = open;
        }

        if self.show_jbnorm {
            if self.jbnorm_symbol.is_empty() {
                self.jbnorm_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_jbnorm;
            egui::Window::new("JBNORM — Jarque-Bera Normality Test")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.jbnorm_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.jbnorm_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.jbnorm_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_jbnorm(&conn, &sym_u)
                                    {
                                        self.jbnorm_snapshot = snap;
                                        self.jbnorm_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.jbnorm_symbol.to_uppercase();
                            self.jbnorm_loading = true;
                            self.jbnorm_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeJbnormSnapshot { symbol: sym });
                        }
                        if self.jbnorm_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.jbnorm_snapshot;
                    if snap.symbol.is_empty() || snap.normal_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.normal_label.as_str() {
                            "NORMAL" => UP,
                            "NON_NORMAL" | "STRONGLY_NON_NORMAL" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — JB {:.2} — p {:.4} — {} bars — as of {}",
                                snap.symbol,
                                snap.normal_label,
                                snap.jb_statistic,
                                snap.jb_pvalue,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("jbnorm_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Skewness").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.skewness))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Excess kurtosis").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.excess_kurtosis))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("JB statistic").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.jb_statistic))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("p-value (χ²(2))").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.jb_pvalue))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_jbnorm = open;
        }
    }
}
