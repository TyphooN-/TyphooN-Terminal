use super::*;

impl TyphooNApp {
    pub(super) fn render_research_sterling_kelly_stat_tests_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research = research_chart_symbol(
            self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
        );

        // ── Research section ──

        if self.show_sterling {
            if self.sterling_symbol.is_empty() {
                self.sterling_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_sterling;
            egui::Window::new("STERLING — Sterling Ratio")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 360.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.sterling_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.sterling_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.sterling_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_sterling(&conn, &sym_u) {
                                        self.sterling_snapshot = snap;
                                        self.sterling_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.sterling_symbol.to_uppercase();
                            self.sterling_loading = true;
                            self.sterling_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeSterlingSnapshot { symbol: sym });
                        }
                        if self.sterling_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.sterling_snapshot;
                    if snap.symbol.is_empty() || snap.sterling_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥30 bars.").color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.sterling_label.as_str() {
                            "GOOD" | "EXCELLENT" => UP,
                            "POOR" | "VERY_POOR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — ratio {:.3} — ann ret {:+.2}% — mean worst {} dd {:.2}% — as of {}",
                            snap.symbol, snap.sterling_label, snap.sterling_ratio,
                            snap.annualized_return_pct, snap.worst_n, snap.mean_worst_dd_pct, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("sterling_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Sterling ratio").small().strong());
                            ui.label(egui::RichText::new(format!("{:.4}", snap.sterling_ratio)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Annualized return %").small().strong());
                            ui.label(egui::RichText::new(format!("{:+.3}%", snap.annualized_return_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Mean worst-N drawdown %").small().strong());
                            ui.label(egui::RichText::new(format!("{:.3}%", snap.mean_worst_dd_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Worst-N size").small().strong());
                            ui.label(egui::RichText::new(format!("{}", snap.worst_n)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Total dd events").small().strong());
                            ui.label(egui::RichText::new(format!("{}", snap.dd_event_count)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Bars used").small().strong());
                            ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace());
                            ui.end_row();
                        });
                    }
                });
            self.show_sterling = open;
        }

        if self.show_kellyf {
            if self.kellyf_symbol.is_empty() {
                self.kellyf_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_kellyf;
            egui::Window::new("KELLYF — Kelly Fraction / Optimal Leverage")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.kellyf_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.kellyf_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.kellyf_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_kellyf(&conn, &sym_u)
                                    {
                                        self.kellyf_snapshot = snap;
                                        self.kellyf_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.kellyf_symbol.to_uppercase();
                            self.kellyf_loading = true;
                            self.kellyf_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeKellyfSnapshot { symbol: sym });
                        }
                        if self.kellyf_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.kellyf_snapshot;
                    if snap.symbol.is_empty() || snap.kelly_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new(
                                "No data — HP cache needs ≥30 returns with wins and losses.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.kelly_label.as_str() {
                            "MODERATE" | "AGGRESSIVE" => UP,
                            "SKIP" | "ALL_IN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — f* {:.4} — half {:.4} — p {:.2} — b {:.3} — as of {}",
                                snap.symbol,
                                snap.kelly_label,
                                snap.kelly_fraction,
                                snap.half_kelly,
                                snap.win_rate,
                                snap.win_loss_ratio,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("kellyf_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("Kelly fraction (f*)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.kelly_fraction))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Half Kelly").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.half_kelly))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Win rate (p)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.win_rate))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Loss rate (q)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.loss_rate))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Avg win %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}%", snap.avg_win_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Avg loss %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}%", snap.avg_loss_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Win/loss ratio (b)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.win_loss_ratio))
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
            self.show_kellyf = open;
        }

        if self.show_ljungb {
            if self.ljungb_symbol.is_empty() {
                self.ljungb_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ljungb;
            egui::Window::new("LJUNGB — Ljung-Box Q-Statistic (h=10)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 340.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ljungb_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ljungb_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ljungb_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ljungb(&conn, &sym_u)
                                    {
                                        self.ljungb_snapshot = snap;
                                        self.ljungb_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ljungb_symbol.to_uppercase();
                            self.ljungb_loading = true;
                            self.ljungb_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeLjungbSnapshot { symbol: sym });
                        }
                        if self.ljungb_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.ljungb_snapshot;
                    if snap.symbol.is_empty() || snap.ljungb_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥40 returns (h=10).")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.ljungb_label.as_str() {
                            "WHITE_NOISE" => UP,
                            "STRONG_DEP" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — Q {:.3} — p {:.4} — h {} — reject {} — {} bars — as of {}",
                            snap.symbol, snap.ljungb_label, snap.q_statistic, snap.p_value,
                            snap.lag_h, snap.reject_white_noise, snap.bars_used, snap.as_of,
                        )).strong().color(color));
                    }
                });
            self.show_ljungb = open;
        }

        if self.show_runstest {
            if self.runstest_symbol.is_empty() {
                self.runstest_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_runstest;
            egui::Window::new("RUNSTEST — Wald-Wolfowitz Runs Test")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.runstest_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.runstest_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.runstest_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_runstest(&conn, &sym_u)
                                    {
                                        self.runstest_snapshot = snap;
                                        self.runstest_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.runstest_symbol.to_uppercase();
                            self.runstest_loading = true;
                            self.runstest_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRunstestSnapshot { symbol: sym });
                        }
                        if self.runstest_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.runstest_snapshot;
                    if snap.symbol.is_empty() || snap.runs_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥20 signed returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.runs_label.as_str() {
                            "RANDOM" => UP,
                            "STRONG_CLUST" | "MOD_CLUST" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — z {:+.3} — p {:.4} — runs {}/{:.1} — as of {}",
                                snap.symbol,
                                snap.runs_label,
                                snap.z_statistic,
                                snap.p_value,
                                snap.runs_observed,
                                snap.runs_expected,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("runstest_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Runs observed").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.runs_observed))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Runs expected").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.runs_expected))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Runs std").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.runs_std))
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
                                ui.label(egui::RichText::new("Positive days").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.positive_days))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Negative days").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.negative_days))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Reject randomness").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.reject_randomness))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_runstest = open;
        }

        if self.show_zeroret {
            if self.zeroret_symbol.is_empty() {
                self.zeroret_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_zeroret;
            egui::Window::new("ZERORET — Zero-Return-Day Fraction (Lesmond-Ogden-Trzcinka)")
                .open(&mut open)
                .resizable(true)
                .default_size([580.0, 340.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.zeroret_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.zeroret_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.zeroret_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_zeroret(&conn, &sym_u) {
                                        self.zeroret_snapshot = snap;
                                        self.zeroret_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.zeroret_symbol.to_uppercase();
                            self.zeroret_loading = true;
                            self.zeroret_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeZeroretSnapshot { symbol: sym });
                        }
                        if self.zeroret_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.zeroret_snapshot;
                    if snap.symbol.is_empty() || snap.zero_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥30 returns.").color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.zero_label.as_str() {
                            "HIGHLY_LIQUID" | "LIQUID" => UP,
                            "ILLIQUID" | "VERY_ILLIQUID" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — zero {:.2}% ({}/{}) — longest streak {} — ε {:.0e} — as of {}",
                            snap.symbol, snap.zero_label,
                            snap.zero_day_pct, snap.zero_day_count, snap.bars_used,
                            snap.longest_zero_streak, snap.epsilon, snap.as_of,
                        )).strong().color(color));
                    }
                });
            self.show_zeroret = open;
        }
    }
}
