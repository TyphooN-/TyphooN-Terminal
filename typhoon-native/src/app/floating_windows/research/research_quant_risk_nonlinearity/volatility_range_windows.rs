use super::*;

impl TyphooNApp {
    pub(super) fn render_volatility_range_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
        if self.show_hlvclust_win {
            if self.hlvclust_win_symbol.is_empty() {
                self.hlvclust_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_hlvclust_win;
            egui::Window::new("HLVCLUST — Parkinson High-Low Volatility Clustering")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.hlvclust_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.hlvclust_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.hlvclust_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_hlvclust(&conn, &sym_u)
                                    {
                                        self.hlvclust_win_snapshot = snap;
                                        self.hlvclust_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.hlvclust_win_symbol.to_uppercase();
                            self.hlvclust_win_loading = true;
                            self.hlvclust_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeHlvClustSnapshot { symbol: sym });
                        }
                        if self.hlvclust_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.hlvclust_win_snapshot;
                    if snap.symbol.is_empty() || snap.hlvclust_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 valid H/L bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.hlvclust_label.as_str() {
                            "STRONG_CLUST" => DOWN,
                            "MILD_CLUST" => AXIS_TEXT,
                            _ => UP,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — Q {:.2} vs crit {:.2} — as of {}",
                                snap.symbol,
                                snap.hlvclust_label,
                                snap.lb_q_stat,
                                snap.critical_95,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("hlvclust_summary")
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
                                ui.label(egui::RichText::new("Lag h").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.lag_h))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Parkinson σ / bar").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.parkinson_vol_bar))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Parkinson σ (ann)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.4}",
                                        snap.parkinson_vol_annualised
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("AC lag 1").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.ac_lag1))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("AC lag 5").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.ac_lag5))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Ljung-Box Q").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.lb_q_stat))
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
                                ui.label(
                                    egui::RichText::new("Reject (no cluster)").small().strong(),
                                );
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
            self.show_hlvclust_win = open;
        }

        if self.show_yangzhang_win {
            if self.yangzhang_win_symbol.is_empty() {
                self.yangzhang_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_yangzhang_win;
            egui::Window::new("YANGZHANG — Yang-Zhang (2000) Range-Volatility Estimator")
                .open(&mut open).resizable(true).default_size([560.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.yangzhang_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.yangzhang_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.yangzhang_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_yangzhang(&conn, &sym_u) { self.yangzhang_win_snapshot = snap; self.yangzhang_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.yangzhang_win_symbol.to_uppercase(); self.yangzhang_win_loading = true; self.yangzhang_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeYangZhangSnapshot { symbol: sym });
                        }
                        if self.yangzhang_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.yangzhang_win_snapshot;
                    if snap.symbol.is_empty() || snap.yangzhang_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥30 valid OHLC bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.yangzhang_label.as_str() {
                            "VERY_HIGH" => DOWN,
                            "HIGH" => DOWN,
                            "MODERATE" => AXIS_TEXT,
                            "LOW" => UP,
                            "VERY_LOW" => UP,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — σ_YZ(ann) {:.2}% vs σ_CC(ann) {:.2}% — eff {:.2}× — as of {}",
                            snap.symbol, snap.yangzhang_label, snap.yz_vol_annualised_pct, snap.cc_vol_annualised_pct, snap.efficiency_vs_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("yangzhang_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("σ²_O (overnight)").small().strong()); ui.label(egui::RichText::new(format!("{:.6e}", snap.overnight_var)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("σ²_C (open→close)").small().strong()); ui.label(egui::RichText::new(format!("{:.6e}", snap.open_to_close_var)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("σ²_RS (Rogers-Satchell)").small().strong()); ui.label(egui::RichText::new(format!("{:.6e}", snap.rs_component)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("k weight").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.k_weight)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("σ_YZ per bar").small().strong()); ui.label(egui::RichText::new(format!("{:.6}", snap.yz_vol_bar)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("σ_YZ annualised").small().strong()); ui.label(egui::RichText::new(format!("{:.3}%", snap.yz_vol_annualised_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("σ_CC annualised").small().strong()); ui.label(egui::RichText::new(format!("{:.3}%", snap.cc_vol_annualised_pct)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Efficiency σ_CC/σ_YZ").small().strong()); ui.label(egui::RichText::new(format!("{:.3}", snap.efficiency_vs_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_yangzhang_win = open;
        }
    }
}
