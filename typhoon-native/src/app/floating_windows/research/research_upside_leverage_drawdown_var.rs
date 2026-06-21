use super::*;

impl TyphooNApp {
    pub(super) fn render_research_upside_leverage_drawdown_var_windows(
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

        // ── Research section ──
        if self.show_upr {
            if self.upr_symbol.is_empty() {
                self.upr_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_upr;
            egui::Window::new("UPR — Upside Potential Ratio")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.upr_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.upr_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.upr_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_upr(&conn, &sym_u)
                                    {
                                        self.upr_snapshot = snap;
                                        self.upr_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.upr_symbol.to_uppercase();
                            self.upr_loading = true;
                            self.upr_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeUprSnapshot { symbol: sym });
                        }
                        if self.upr_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.upr_snapshot;
                    if snap.symbol.is_empty() || snap.upr_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.upr_label.as_str() {
                            "HIGH_UPSIDE" | "VERY_HIGH_UPSIDE" => UP,
                            "LOW_UPSIDE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — UPR {:.4} — as of {}",
                                snap.symbol, snap.upr_label, snap.upr, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("upr_summary")
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
                                ui.label(egui::RichText::new("UPM₁").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.upm1))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("LPM₂").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.8}", snap.lpm2))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Downside dev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.downside_dev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("UPR").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.upr))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_upr = open;
        }

        if self.show_levereff {
            if self.levereff_symbol.is_empty() {
                self.levereff_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_levereff;
            egui::Window::new("LEVEREFF — Leverage Effect")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.levereff_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.levereff_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.levereff_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_levereff(&conn, &sym_u)
                                    {
                                        self.levereff_snapshot = snap;
                                        self.levereff_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.levereff_symbol.to_uppercase();
                            self.levereff_loading = true;
                            self.levereff_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeLevereffSnapshot { symbol: sym });
                        }
                        if self.levereff_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.levereff_snapshot;
                    if snap.symbol.is_empty() || snap.lever_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.lever_label.as_str() {
                            "SYMMETRIC" => UP,
                            "STRONG_LEVERAGE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — corr {:+.4} — asym {:.3} — as of {}",
                                snap.symbol,
                                snap.lever_label,
                                snap.corr_r_nextsq,
                                snap.asym_ratio,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("levereff_summary")
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
                                ui.label(egui::RichText::new("corr(rₜ, rₜ₊₁²)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.corr_r_nextsq))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Mean |r| after neg (%)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.mean_vol_after_neg))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Mean |r| after pos (%)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.mean_vol_after_pos))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Asymmetry ratio").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.asym_ratio))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_levereff = open;
        }

        if self.show_drawdar {
            if self.drawdar_symbol.is_empty() {
                self.drawdar_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_drawdar;
            egui::Window::new("DRAWDAR — Drawdown-at-Risk")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 350.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.drawdar_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.drawdar_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.drawdar_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_drawdar(&conn, &sym_u)
                                    {
                                        self.drawdar_snapshot = snap;
                                        self.drawdar_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.drawdar_symbol.to_uppercase();
                            self.drawdar_loading = true;
                            self.drawdar_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeDrawdarSnapshot { symbol: sym });
                        }
                        if self.drawdar_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.drawdar_snapshot;
                    if snap.symbol.is_empty() || snap.drawdar_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — needs ≥30 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.drawdar_label.as_str() {
                            "LOW_DD_RISK" => UP,
                            "SEVERE_DD_RISK" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — DaR(5%) {:.2}% — max dd {:.2}% — as of {}",
                                snap.symbol,
                                snap.drawdar_label,
                                snap.dar_5pct,
                                snap.max_dd_pct,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("drawdar_summary")
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
                                ui.label(egui::RichText::new("DaR 5% (%)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.dar_5pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("CDaR 5% (%)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.cdar_5pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("DaR 1% (%)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.dar_1pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("CDaR 1% (%)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.cdar_1pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Max dd (%)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.max_dd_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Mean dd (%)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.mean_dd_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_drawdar = open;
        }

        if self.show_varhalf {
            if self.varhalf_symbol.is_empty() {
                self.varhalf_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_varhalf;
            egui::Window::new("VARHALF — Volatility Half-Life")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.varhalf_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.varhalf_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.varhalf_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_varhalf(&conn, &sym_u)
                                    {
                                        self.varhalf_snapshot = snap;
                                        self.varhalf_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.varhalf_symbol.to_uppercase();
                            self.varhalf_loading = true;
                            self.varhalf_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeVarhalfSnapshot { symbol: sym });
                        }
                        if self.varhalf_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.varhalf_snapshot;
                    if snap.symbol.is_empty() || snap.varhalf_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥50 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.varhalf_label.as_str() {
                            "FAST_REVERT" => UP,
                            "VERY_PERSISTENT" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — HL {:.1} days — β {:.4} — as of {}",
                                snap.symbol,
                                snap.varhalf_label,
                                snap.half_life_days,
                                snap.ar1_beta,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("varhalf_summary")
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
                                ui.label(egui::RichText::new("Vol observations").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.vol_obs))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("AR(1) β").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.ar1_beta))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("AR(1) α").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.ar1_alpha))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("R²").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.ar1_r2))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Half-life (days)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.1}", snap.half_life_days))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_varhalf = open;
        }

        if self.show_gini {
            if self.gini_symbol.is_empty() {
                self.gini_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_gini;
            egui::Window::new("GINI — Return Concentration")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.gini_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.gini_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.gini_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_gini(&conn, &sym_u)
                                    {
                                        self.gini_snapshot = snap;
                                        self.gini_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.gini_symbol.to_uppercase();
                            self.gini_loading = true;
                            self.gini_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeGiniSnapshot { symbol: sym });
                        }
                        if self.gini_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.gini_snapshot;
                    if snap.symbol.is_empty() || snap.gini_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.gini_label.as_str() {
                            "LOW_CONCENTRATION" => UP,
                            "VERY_HIGH_CONCENTRATION" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — Gini {:.4} — as of {}",
                                snap.symbol, snap.gini_label, snap.gini_coeff, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("gini_summary")
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
                                ui.label(egui::RichText::new("Gini coefficient").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.gini_coeff))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Mean |r| (%)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.mean_abs_return_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Median |r| (%)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.4}",
                                        snap.median_abs_return_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_gini = open;
        }
    }
}
