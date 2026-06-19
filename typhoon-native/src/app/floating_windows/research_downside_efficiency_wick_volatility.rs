use super::*;

impl TyphooNApp {
    pub(super) fn render_research_downside_efficiency_wick_volatility_windows(
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

        // ── Research Round 25 windows ──
        if self.show_downvol {
            if self.downvol_symbol.is_empty() {
                self.downvol_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_downvol;
            egui::Window::new("DOWNVOL — Downside Deviation / Sortino")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.downvol_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.downvol_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.downvol_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_downvol(&conn, &sym_u)
                                    {
                                        self.downvol_snapshot = snap;
                                        self.downvol_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.downvol_symbol.to_uppercase();
                            self.downvol_loading = true;
                            self.downvol_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeDownvolSnapshot { symbol: sym });
                        }
                        if self.downvol_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.downvol_snapshot;
                    if snap.symbol.is_empty() || snap.sortino_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.sortino_label.as_str() {
                            "GOOD" | "EXCELLENT" => UP,
                            "POOR" | "VERY_POOR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — Sortino {:.3} (ann {:.3}) — {} bars — as of {}",
                                snap.symbol,
                                snap.sortino_label,
                                snap.sortino_ratio,
                                snap.sortino_ratio_ann,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("downvol_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Mean log return").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.mean_log_return))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Downside deviation").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.downside_dev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Downside deviation (ann)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.downside_dev_ann))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Upside deviation").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.upside_dev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Sortino (raw)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sortino_ratio))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Sortino (annualized)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sortino_ratio_ann))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Downside % of total var")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2}%",
                                        snap.downside_pct_of_total
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_downvol = open;
        }

        if self.show_sharpr {
            if self.sharpr_symbol.is_empty() {
                self.sharpr_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_sharpr;
            egui::Window::new("SHARPR — Sharpe Ratio (rf=0)")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.sharpr_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.sharpr_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.sharpr_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_sharpr(&conn, &sym_u)
                                    {
                                        self.sharpr_snapshot = snap;
                                        self.sharpr_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.sharpr_symbol.to_uppercase();
                            self.sharpr_loading = true;
                            self.sharpr_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeSharprSnapshot { symbol: sym });
                        }
                        if self.sharpr_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.sharpr_snapshot;
                    if snap.symbol.is_empty() || snap.sharpe_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.sharpe_label.as_str() {
                            "GOOD" | "EXCELLENT" => UP,
                            "POOR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — Sharpe {:.3} (ann {:.3}) — {} bars — as of {}",
                                snap.symbol,
                                snap.sharpe_label,
                                snap.sharpe_ratio,
                                snap.sharpe_ratio_ann,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("sharpr_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Mean log return").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.mean_log_return))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Stdev log return").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.stdev_log_return))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Mean return (ann)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.mean_return_ann))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Stdev return (ann)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.stdev_return_ann))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Sharpe (raw)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sharpe_ratio))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Sharpe (annualized)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sharpe_ratio_ann))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_sharpr = open;
        }

        if self.show_effratio {
            if self.effratio_symbol.is_empty() {
                self.effratio_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_effratio;
            egui::Window::new("EFFRATIO — Kaufman Efficiency Ratio")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.effratio_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.effratio_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.effratio_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_effratio(&conn, &sym_u)
                                    {
                                        self.effratio_snapshot = snap;
                                        self.effratio_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.effratio_symbol.to_uppercase();
                            self.effratio_loading = true;
                            self.effratio_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeEffratioSnapshot { symbol: sym });
                        }
                        if self.effratio_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.effratio_snapshot;
                    if snap.symbol.is_empty() || snap.efficiency_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.efficiency_label.as_str() {
                            "TRENDING" | "STRONG_TREND" => UP,
                            "CHOP" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — ER {:.3} (signed {:+.3}) — {} bars — as of {}",
                                snap.symbol,
                                snap.efficiency_label,
                                snap.efficiency_ratio,
                                snap.signed_efficiency,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("effratio_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Start close").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.start_close))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("End close").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.end_close))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Net change").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.net_change))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Net change %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}%", snap.net_change_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Σ |Δclose|").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sum_abs_changes))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Efficiency ratio").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.efficiency_ratio))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Signed efficiency").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.signed_efficiency))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_effratio = open;
        }

        if self.show_wickbias {
            if self.wickbias_symbol.is_empty() {
                self.wickbias_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_wickbias;
            egui::Window::new("WICKBIAS — Upper vs Lower Wick Asymmetry")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.wickbias_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.wickbias_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.wickbias_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_wickbias(&conn, &sym_u)
                                    {
                                        self.wickbias_snapshot = snap;
                                        self.wickbias_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.wickbias_symbol.to_uppercase();
                            self.wickbias_loading = true;
                            self.wickbias_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeWickbiasSnapshot { symbol: sym });
                        }
                        if self.wickbias_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.wickbias_snapshot;
                    if snap.symbol.is_empty() || snap.bias_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — need ≥20 non-flat bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.bias_label.as_str() {
                            "BUYER_LEAN" | "BUYER_DEFEND" => UP,
                            "SELLER_LEAN" | "SELLER_REJECT" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — bias {:+.4} — {} bars — as of {}",
                                snap.symbol,
                                snap.bias_label,
                                snap.wick_bias_score,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("wickbias_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("Avg upper wick share").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.avg_upper_wick))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Avg lower wick share").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.avg_lower_wick))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Median upper wick").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.median_upper_wick))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Median lower wick").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.median_lower_wick))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Avg body share").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.avg_body_share))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Bias score").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.wick_bias_score))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_wickbias = open;
        }

        if self.show_volofvol {
            if self.volofvol_symbol.is_empty() {
                self.volofvol_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_volofvol;
            egui::Window::new("VOLOFVOL — Stdev of Rolling 20d Realized Vol")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.volofvol_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.volofvol_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.volofvol_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_volofvol(&conn, &sym_u)
                                    {
                                        self.volofvol_snapshot = snap;
                                        self.volofvol_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.volofvol_symbol.to_uppercase();
                            self.volofvol_loading = true;
                            self.volofvol_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeVolofvolSnapshot { symbol: sym });
                        }
                        if self.volofvol_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.volofvol_snapshot;
                    if snap.symbol.is_empty() || snap.cv_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥50 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.cv_label.as_str() {
                            "STABLE" => UP,
                            "UNSTABLE" | "CHAOTIC" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — CV {:.3} — {} RV points — as of {}",
                                snap.symbol,
                                snap.cv_label,
                                snap.cv_rv20,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("volofvol_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Mean RV20").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.mean_rv20))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Stdev RV20").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.stdev_rv20))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Min RV20").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.min_rv20))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Max RV20").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.max_rv20))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Latest RV20").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.latest_rv20))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("CV (stdev/mean)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.cv_rv20))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_volofvol = open;
        }
    }
}
