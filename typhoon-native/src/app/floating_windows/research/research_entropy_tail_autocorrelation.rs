use super::*;

impl TyphooNApp {
    pub(super) fn render_research_entropy_tail_autocorrelation_windows(
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
        if self.show_entropy {
            if self.entropy_symbol.is_empty() {
                self.entropy_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_entropy;
            egui::Window::new("ENTROPY — Shannon Return Entropy")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.entropy_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.entropy_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.entropy_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_entropy(&conn, &sym_u)
                                    {
                                        self.entropy_snapshot = snap;
                                        self.entropy_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.entropy_symbol.to_uppercase();
                            self.entropy_loading = true;
                            self.entropy_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeEntropySnapshot { symbol: sym });
                        }
                        if self.entropy_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.entropy_snapshot;
                    if snap.symbol.is_empty() || snap.entropy_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.entropy_label.as_str() {
                            "LOW_ENTROPY" => UP,
                            "VERY_HIGH_ENTROPY" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — H={:.3} bits — norm {:.3} — as of {}",
                                snap.symbol,
                                snap.entropy_label,
                                snap.entropy_bits,
                                snap.normalised_entropy,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("entropy_summary")
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
                                ui.label(egui::RichText::new("Bins").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.num_bins))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Entropy H (bits)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.entropy_bits))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Max entropy (bits)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.max_entropy_bits))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Normalised H/H_max").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.normalised_entropy))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_entropy = open;
        }

        if self.show_rachev {
            if self.rachev_symbol.is_empty() {
                self.rachev_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_rachev;
            egui::Window::new("RACHEV — Conditional Tail Expectation Ratio")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 350.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.rachev_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.rachev_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.rachev_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_rachev(&conn, &sym_u)
                                    {
                                        self.rachev_snapshot = snap;
                                        self.rachev_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.rachev_symbol.to_uppercase();
                            self.rachev_loading = true;
                            self.rachev_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRachevSnapshot { symbol: sym });
                        }
                        if self.rachev_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.rachev_snapshot;
                    if snap.symbol.is_empty() || snap.rachev_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.rachev_label.as_str() {
                            "RIGHT_HEAVY" | "STRONG_RIGHT_TAIL" => UP,
                            "STRONG_LEFT_TAIL" | "LEFT_HEAVY" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — Rachev(5%)={:.3} — as of {}",
                                snap.symbol, snap.rachev_label, snap.rachev_5pct, snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("rachev_summary")
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
                                ui.label(egui::RichText::new("ES right 5% (%)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.es_right_5pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ES left 5% (%)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.es_left_5pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Rachev 5%").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.rachev_5pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ES right 1% (%)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.es_right_1pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ES left 1% (%)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.es_left_1pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Rachev 1%").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.rachev_1pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_rachev = open;
        }

        if self.show_gpr {
            if self.gpr_symbol.is_empty() {
                self.gpr_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_gpr;
            egui::Window::new("GPR — Gain-to-Pain Ratio")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 350.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.gpr_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.gpr_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.gpr_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_gpr(&conn, &sym_u)
                                    {
                                        self.gpr_snapshot = snap;
                                        self.gpr_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.gpr_symbol.to_uppercase();
                            self.gpr_loading = true;
                            self.gpr_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeGprSnapshot { symbol: sym });
                        }
                        if self.gpr_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.gpr_snapshot;
                    if snap.symbol.is_empty() || snap.gpr_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.gpr_label.as_str() {
                            "GOOD" | "EXCELLENT" => UP,
                            "DEEP_PAIN" | "NEGATIVE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — GPR {:+.3} — PF {:.3} — as of {}",
                                snap.symbol,
                                snap.gpr_label,
                                snap.gain_to_pain,
                                snap.profit_factor,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("gpr_summary")
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
                                ui.label(
                                    egui::RichText::new("Sum all returns (%)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.4}",
                                        snap.sum_all_returns_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Sum gains (%)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sum_gains_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Sum |losses| (%)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sum_losses_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Gain-to-Pain").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.gain_to_pain))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Profit Factor").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.profit_factor))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Wins / Losses").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {}",
                                        snap.win_count, snap.loss_count
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_gpr = open;
        }

        if self.show_pacf {
            if self.pacf_symbol.is_empty() {
                self.pacf_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_pacf;
            egui::Window::new("PACF — Partial Autocorrelation")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.pacf_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.pacf_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.pacf_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_pacf(&conn, &sym_u)
                                    {
                                        self.pacf_snapshot = snap;
                                        self.pacf_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.pacf_symbol.to_uppercase();
                            self.pacf_loading = true;
                            self.pacf_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputePacfSnapshot { symbol: sym });
                        }
                        if self.pacf_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.pacf_snapshot;
                    if snap.symbol.is_empty() || snap.pacf_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.pacf_label.as_str() {
                            "NO_STRUCTURE" => UP,
                            "STRONG_STRUCTURE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — {} sig lags — max |PACF| {:.4} at lag {} — as of {}",
                                snap.symbol,
                                snap.pacf_label,
                                snap.significant_lags,
                                snap.max_abs_pacf,
                                snap.max_abs_lag,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("pacf_summary")
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
                                ui.label(egui::RichText::new("Bartlett 95% crit").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("±{:.4}", snap.bartlett_crit_95))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                let pacfs = [
                                    snap.pacf_lag1,
                                    snap.pacf_lag2,
                                    snap.pacf_lag3,
                                    snap.pacf_lag4,
                                    snap.pacf_lag5,
                                ];
                                for (i, &v) in pacfs.iter().enumerate() {
                                    let sig = v.abs() > snap.bartlett_crit_95;
                                    let lbl = format!("PACF lag {}", i + 1);
                                    let val_str =
                                        format!("{:+.4}{}", v, if sig { " *" } else { "" });
                                    ui.label(egui::RichText::new(lbl).small().strong());
                                    ui.label(
                                        egui::RichText::new(val_str)
                                            .small()
                                            .monospace()
                                            .color(if sig { DOWN } else { AXIS_TEXT }),
                                    );
                                    ui.end_row();
                                }
                            });
                    }
                });
            self.show_pacf = open;
        }

        if self.show_apen {
            if self.apen_symbol.is_empty() {
                self.apen_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_apen;
            egui::Window::new("APEN — Approximate Entropy")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.apen_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.apen_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.apen_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_apen(&conn, &sym_u)
                                    {
                                        self.apen_snapshot = snap;
                                        self.apen_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.apen_symbol.to_uppercase();
                            self.apen_loading = true;
                            self.apen_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeApenSnapshot { symbol: sym });
                        }
                        if self.apen_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.apen_snapshot;
                    if snap.symbol.is_empty() || snap.apen_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.apen_label.as_str() {
                            "REGULAR" => UP,
                            "HIGHLY_COMPLEX" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — ApEn {:.4} — as of {}",
                                snap.symbol, snap.apen_label, snap.apen, snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("apen_summary")
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
                                ui.label(egui::RichText::new("Embedding dim m").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.embed_dim))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Tolerance r").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.tolerance))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Phi^m").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.phi_m))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Phi^(m+1)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.phi_m1))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ApEn").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.apen))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_apen = open;
        }
    }
}
