use super::*;

impl TyphooNApp {
    pub(super) fn render_quant_break_test_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
        // ── (Quant Stats) popup windows ──
        if self.show_modsharpe_win {
            if self.modsharpe_win_symbol.is_empty() {
                self.modsharpe_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_modsharpe_win;
            egui::Window::new("MODSHARPE — Pezier-White Adjusted Sharpe Ratio")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.modsharpe_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.modsharpe_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.modsharpe_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_modsharpe(&conn, &sym_u)
                                    {
                                        self.modsharpe_win_snapshot = snap;
                                        self.modsharpe_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.modsharpe_win_symbol.to_uppercase();
                            self.modsharpe_win_loading = true;
                            self.modsharpe_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeModSharpeSnapshot { symbol: sym });
                        }
                        if self.modsharpe_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.modsharpe_win_snapshot;
                    if snap.symbol.is_empty() || snap.modsharpe_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.modsharpe_label.as_str() {
                            "STRONG_POS" | "MODERATE_POS" => UP,
                            "STRONG_NEG" | "MODERATE_NEG" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — SR {:+.3} → ASR {:+.3} (× {:+.3}) — as of {}",
                                snap.symbol,
                                snap.modsharpe_label,
                                snap.sharpe_ratio,
                                snap.adjusted_sharpe,
                                snap.adjustment_factor,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("modsharpe_summary")
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
                                ui.label(egui::RichText::new("Annualisation").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "√{:.0}",
                                        snap.annualization_factor
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Mean return / bar").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.6}", snap.mean_return_bar))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Stdev / bar").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.stdev_return_bar))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Skewness").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.skewness))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Excess kurtosis").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.excess_kurtosis))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Sharpe (classical)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.sharpe_ratio))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Adjusted Sharpe").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.adjusted_sharpe))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Adjustment factor").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.adjustment_factor))
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
            self.show_modsharpe_win = open;
        }

        if self.show_hsiehtest_win {
            if self.hsiehtest_win_symbol.is_empty() {
                self.hsiehtest_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_hsiehtest_win;
            egui::Window::new("HSIEHTEST — Hsieh Third-Moment Nonlinearity Test")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.hsiehtest_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.hsiehtest_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.hsiehtest_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_hsiehtest(&conn, &sym_u)
                                    {
                                        self.hsiehtest_win_snapshot = snap;
                                        self.hsiehtest_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.hsiehtest_win_symbol.to_uppercase();
                            self.hsiehtest_win_loading = true;
                            self.hsiehtest_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeHsiehTestSnapshot { symbol: sym });
                        }
                        if self.hsiehtest_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.hsiehtest_win_snapshot;
                    if snap.symbol.is_empty() || snap.hsieh_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥50 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.hsieh_label.as_str() {
                            "STRONG_NONLIN" => DOWN,
                            "MILD_NONLIN" => AXIS_TEXT,
                            _ => UP,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — max|z| {:.3} vs crit {:.2} — as of {}",
                                snap.symbol,
                                snap.hsieh_label,
                                snap.max_abs_z,
                                snap.critical_95,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("hsieh_summary")
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
                                ui.label(egui::RichText::new("AR order").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.ar_order))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("T(1,1)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.t_11))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("T(2,2)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.t_22))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("z(1,1)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.z_11))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("z(2,2)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.z_22))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("max |z|").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.max_abs_z))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Critical 95%").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.critical_95))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Reject linearity").small().strong());
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
            self.show_hsiehtest_win = open;
        }

        if self.show_chowbreak_win {
            if self.chowbreak_win_symbol.is_empty() {
                self.chowbreak_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_chowbreak_win;
            egui::Window::new("CHOWBREAK — Chow Structural-Break F-Test (midpoint)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.chowbreak_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.chowbreak_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.chowbreak_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_chowbreak(&conn, &sym_u)
                                    {
                                        self.chowbreak_win_snapshot = snap;
                                        self.chowbreak_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.chowbreak_win_symbol.to_uppercase();
                            self.chowbreak_win_loading = true;
                            self.chowbreak_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeChowBreakSnapshot { symbol: sym });
                        }
                        if self.chowbreak_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.chowbreak_win_snapshot;
                    if snap.symbol.is_empty() || snap.chowbreak_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥40 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.chowbreak_label.as_str() {
                            "STRONG_BREAK" => DOWN,
                            "MILD_BREAK" => AXIS_TEXT,
                            _ => UP,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — F {:.3} vs crit {:.2} — as of {}",
                                snap.symbol,
                                snap.chowbreak_label,
                                snap.f_stat,
                                snap.critical_95,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("chow_summary")
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
                                ui.label(egui::RichText::new("Break index").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.break_point_idx))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Mean pre-break").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.6}", snap.mean_pre))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Mean post-break").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.6}", snap.mean_post))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("RSS pooled").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6e}", snap.rss_pooled))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("RSS unrestricted").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6e}", snap.rss_unrestricted))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("F statistic").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.f_stat))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("df (num, den)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "({}, {})",
                                        snap.df_num, snap.df_den
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Critical 95%").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.critical_95))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Reject (no break)").small().strong());
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
            self.show_chowbreak_win = open;
        }

        if self.show_driftburst_win {
            if self.driftburst_win_symbol.is_empty() {
                self.driftburst_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_driftburst_win;
            egui::Window::new("DRIFTBURST — Christensen-Oomen-Renò (2018) Drift-Burst Statistic")
                .open(&mut open).resizable(true).default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.driftburst_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.driftburst_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.driftburst_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_driftburst(&conn, &sym_u) { self.driftburst_win_snapshot = snap; self.driftburst_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.driftburst_win_symbol.to_uppercase(); self.driftburst_win_loading = true; self.driftburst_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeDriftBurstSnapshot { symbol: sym });
                        }
                        if self.driftburst_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.driftburst_win_snapshot;
                    if snap.symbol.is_empty() || snap.driftburst_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥50 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.driftburst_label.as_str() {
                            "STRONG_BURST" => if snap.max_stat_signed >= 0.0 { UP } else { DOWN },
                            "MILD_BURST" => AXIS_TEXT,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — max|T| {:.3} (signed {:+.3}) — excursions>3: {} — as of {}",
                            snap.symbol, snap.driftburst_label, snap.max_abs_statistic, snap.max_stat_signed, snap.excursions_gt_3, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("driftburst_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Kernel bandwidth").small().strong()); ui.label(egui::RichText::new(format!("{:.1} bars", snap.kernel_bandwidth_bars)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("max |T(t)|").small().strong()); ui.label(egui::RichText::new(format!("{:.3}", snap.max_abs_statistic)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("signed T at max").small().strong()); ui.label(egui::RichText::new(format!("{:+.3}", snap.max_stat_signed)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("offset from end").small().strong()); ui.label(egui::RichText::new(format!("{} bars back", snap.max_at_offset)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("|T|>3 excursions").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.excursions_gt_3)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Critical 99%").small().strong()); ui.label(egui::RichText::new(format!("{:.2}", snap.critical_99_approx)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_driftburst_win = open;
        }
    }
}
