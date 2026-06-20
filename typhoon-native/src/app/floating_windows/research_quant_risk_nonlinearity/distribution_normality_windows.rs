use super::*;

impl TyphooNApp {
    pub(super) fn render_distribution_normality_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
        if self.show_kuiper_win {
            if self.kuiper_win_symbol.is_empty() {
                self.kuiper_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_kuiper_win;
            egui::Window::new("KUIPER — Kuiper (1960) Two-Sided CDF Goodness-of-Fit vs Normal")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.kuiper_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.kuiper_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.kuiper_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_kuiper(&conn, &sym_u)
                                    {
                                        self.kuiper_win_snapshot = snap;
                                        self.kuiper_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.kuiper_win_symbol.to_uppercase();
                            self.kuiper_win_loading = true;
                            self.kuiper_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeKuiperSnapshot { symbol: sym });
                        }
                        if self.kuiper_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.kuiper_win_snapshot;
                    if snap.symbol.is_empty() || snap.kuiper_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.kuiper_label.as_str() {
                            "STRONG_DEPART" => DOWN,
                            "MILD_DEPART" => AXIS_TEXT,
                            "NORMAL" => UP,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — V* {:.3} vs crit {:.3} — p≈{:.4} — as of {}",
                                snap.symbol,
                                snap.kuiper_label,
                                snap.v_stat_adj,
                                snap.critical_95,
                                snap.p_value_approx,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("kuiper_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(200.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Bars used").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.bars_used))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Sample μ").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.6}", snap.mean))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Sample σ").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.stdev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("D⁺").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.d_plus))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("D⁻").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.d_minus))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("V = D⁺+D⁻").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.v_stat))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("V* (Stephens mod)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.v_stat_adj))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Critical 95%").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.critical_95))
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
                                ui.label(egui::RichText::new("Reject normality").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.reject_null))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
                        }
                    }
                });
            self.show_kuiper_win = open;
        }

        if self.show_dagostino_win {
            if self.dagostino_win_symbol.is_empty() {
                self.dagostino_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_dagostino_win;
            egui::Window::new("DAGOSTINO — D'Agostino-Pearson (1973) K² Omnibus Normality")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.dagostino_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.dagostino_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.dagostino_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_dagostino(&conn, &sym_u)
                                    {
                                        self.dagostino_win_snapshot = snap;
                                        self.dagostino_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.dagostino_win_symbol.to_uppercase();
                            self.dagostino_win_loading = true;
                            self.dagostino_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeDagostinoSnapshot { symbol: sym });
                        }
                        if self.dagostino_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.dagostino_win_snapshot;
                    if snap.symbol.is_empty() || snap.dagostino_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥20 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.dagostino_label.as_str() {
                            "BOTH_DEPART" => DOWN,
                            "SKEW_DOMINANT" => AXIS_TEXT,
                            "KURT_DOMINANT" => AXIS_TEXT,
                            "NORMAL" => UP,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — K² {:.2} vs crit {:.3} — p {:.4} — as of {}",
                                snap.symbol,
                                snap.dagostino_label,
                                snap.k2_stat,
                                snap.critical_95,
                                snap.p_value,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("dagostino_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(200.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Bars used").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.bars_used))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Skewness").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.skewness))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Excess kurtosis").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.excess_kurtosis))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("z_skew (D'Agostino)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.z_skew))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("z_kurt (Anscombe-Glynn)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.z_kurt))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("K² = z_skew²+z_kurt²").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.k2_stat))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Critical 95%").small().strong());
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
                                ui.label(egui::RichText::new("Reject normality").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.reject_null))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
                        }
                    }
                });
            self.show_dagostino_win = open;
        }
    }
}
