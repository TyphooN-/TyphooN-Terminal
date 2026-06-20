use super::*;

mod quant_break_test_windows;

impl TyphooNApp {
    pub(super) fn render_research_quant_risk_nonlinearity_windows(&mut self, ctx: &egui::Context) {
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

        self.render_quant_break_test_windows(ctx, &chart_sym_research);

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

        if self.show_baiperron_win {
            if self.baiperron_win_symbol.is_empty() {
                self.baiperron_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_baiperron_win;
            egui::Window::new("BAIPERRON — Bai-Perron (1998) sup-F Structural Break Search")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.baiperron_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.baiperron_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.baiperron_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_baiperron(&conn, &sym_u)
                                    {
                                        self.baiperron_win_snapshot = snap;
                                        self.baiperron_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.baiperron_win_symbol.to_uppercase();
                            self.baiperron_win_loading = true;
                            self.baiperron_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeBaiPerronSnapshot { symbol: sym });
                        }
                        if self.baiperron_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.baiperron_win_snapshot;
                    if snap.symbol.is_empty() || snap.baiperron_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.baiperron_label.as_str() {
                            "STRONG_BREAK" => DOWN,
                            "MILD_BREAK" => AXIS_TEXT,
                            "NO_BREAK" => UP,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — sup-F {:.2} at bar {} vs crit {:.2} — as of {}",
                                snap.symbol,
                                snap.baiperron_label,
                                snap.sup_f_stat,
                                snap.best_break_idx,
                                snap.critical_95,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("baiperron_summary")
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
                                ui.label(egui::RichText::new("Trim fraction π₀").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.trim_fraction))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Search range").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "[{}, {}]",
                                        snap.search_lo, snap.search_hi
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Best break idx").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.best_break_idx))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("sup-F statistic").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.sup_f_stat))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("μ pre-break").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.6}", snap.mean_pre))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("μ post-break").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.6}", snap.mean_post))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("RSS no-break").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6e}", snap.rss_no_break))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("RSS at best").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6e}", snap.rss_at_best))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Critical 95% (Andrews)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.critical_95))
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
                                ui.label(egui::RichText::new("Reject no-break").small().strong());
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
            self.show_baiperron_win = open;
        }

        if self.show_kupiecpof_win {
            if self.kupiecpof_win_symbol.is_empty() {
                self.kupiecpof_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_kupiecpof_win;
            egui::Window::new("KUPIECPOF — Kupiec (1995) Proportion-of-Failures VaR Backtest")
                .open(&mut open).resizable(true).default_size([560.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.kupiecpof_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.kupiecpof_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.kupiecpof_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_kupiecpof(&conn, &sym_u) { self.kupiecpof_win_snapshot = snap; self.kupiecpof_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.kupiecpof_win_symbol.to_uppercase(); self.kupiecpof_win_loading = true; self.kupiecpof_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeKupiecPofSnapshot { symbol: sym });
                        }
                        if self.kupiecpof_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.kupiecpof_win_snapshot;
                    if snap.symbol.is_empty() || snap.kupiec_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥90 returns (60 window + 30 test).").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.kupiec_label.as_str() {
                            "UNDER_ESTIMATED" => DOWN,
                            "OVER_ESTIMATED" => AXIS_TEXT,
                            "GOOD_FIT" => UP,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — LR_POF {:.2} vs crit {:.3} — realised {:.2}% vs nominal {:.2}% — as of {}",
                            snap.symbol, snap.kupiec_label, snap.lr_pof_stat, snap.critical_95, snap.realised_exceedance_rate * 100.0, snap.nominal_exceedance_rate * 100.0, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("kupiecpof_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Confidence level").small().strong()); ui.label(egui::RichText::new(format!("{:.2}%", snap.confidence_level * 100.0)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Nominal exceedance α").small().strong()); ui.label(egui::RichText::new(format!("{:.2}%", snap.nominal_exceedance_rate * 100.0)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Rolling window").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.rolling_window)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Test window").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.test_window)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Latest VaR (bar)").small().strong()); ui.label(egui::RichText::new(format!("{:.6}", snap.var_latest_bar)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Exceedances observed").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.n_exceedances)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Exceedances expected").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.expected_exceedances)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Realised exceedance rate").small().strong()); ui.label(egui::RichText::new(format!("{:.3}%", snap.realised_exceedance_rate * 100.0)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("LR_POF statistic").small().strong()); ui.label(egui::RichText::new(format!("{:.3}", snap.lr_pof_stat)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Critical 95% χ²(1)").small().strong()); ui.label(egui::RichText::new(format!("{:.3}", snap.critical_95)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("p-value").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.p_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Reject H0 (good fit)").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.reject_null)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_kupiecpof_win = open;
        }

        if self.show_awesome_win {
            if self.awesome_win_symbol.is_empty() {
                self.awesome_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_awesome_win;
            egui::Window::new("AWESOME — Awesome Oscillator (Bill Williams, 5/34)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.awesome_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.awesome_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.awesome_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_awesome(&conn, &sym_u)
                                    {
                                        self.awesome_win_snapshot = snap;
                                        self.awesome_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.awesome_win_symbol.to_uppercase();
                            self.awesome_win_loading = true;
                            self.awesome_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeAwesomeSnapshot { symbol: sym });
                        }
                        if self.awesome_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.awesome_win_snapshot;
                    if snap.symbol.is_empty() || snap.awesome_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥36 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.awesome_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        let color_arrow = if snap.ao_color_up { "▲" } else { "▼" };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — AO {:+.4} {} — prev {:+.4} — as of {}",
                                snap.symbol,
                                snap.awesome_label,
                                snap.ao_value,
                                color_arrow,
                                snap.ao_prev,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("awesome_summary")
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
                                ui.label(egui::RichText::new("Fast / slow").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {}",
                                        snap.fast_period, snap.slow_period
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("SMA(5) hl2").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sma_fast))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("SMA(34) hl2").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sma_slow))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("AO value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.ao_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("AO prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.ao_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Color").small().strong());
                                ui.label(
                                    egui::RichText::new(if snap.ao_color_up {
                                        "GREEN ▲"
                                    } else {
                                        "RED ▼"
                                    })
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Last close").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.last_close))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_awesome_win = open;
        }

        if self.show_peakover {
            if self.peakover_symbol.is_empty() {
                self.peakover_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_peakover;
            egui::Window::new("PEAKOVER — Peaks-Over-Threshold (EVT)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 340.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.peakover_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.peakover_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.peakover_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_peakover(&conn, &sym_u)
                                    {
                                        self.peakover_snapshot = snap;
                                        self.peakover_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.peakover_symbol.to_uppercase();
                            self.peakover_loading = true;
                            self.peakover_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputePeakoverSnapshot { symbol: sym });
                        }
                        if self.peakover_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.peakover_snapshot;
                    if snap.symbol.is_empty() || snap.peakover_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.peakover_label.as_str() {
                            "LIGHT_TAIL" => UP,
                            "EXTREME_TAIL" | "HEAVY_TAIL" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — P95 {:.3}% — as of {}",
                                snap.symbol,
                                snap.peakover_label,
                                snap.threshold_p95 * 100.0,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("peakover_summary")
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
                                ui.label(egui::RichText::new("Threshold P95").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.4}%",
                                        snap.threshold_p95 * 100.0
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Threshold P99").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.4}%",
                                        snap.threshold_p99 * 100.0
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Count > P95").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.count_p95))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Count > P99").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.count_p99))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Mean excess > P95").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.4}%",
                                        snap.mean_excess_p95 * 100.0
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Mean excess > P99").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.4}%",
                                        snap.mean_excess_p99 * 100.0
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Max excess > P95").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.4}%",
                                        snap.max_excess_p95 * 100.0
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Max excess > P99").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.4}%",
                                        snap.max_excess_p99 * 100.0
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_peakover = open;
        }
    }
}
