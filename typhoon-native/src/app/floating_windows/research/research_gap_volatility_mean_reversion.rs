use super::*;

impl TyphooNApp {
    pub(super) fn render_research_gap_volatility_mean_reversion_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research = research_chart_symbol(
            self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
        );

        // ── Research section ──
        if self.show_drawup {
            if self.drawup_symbol.is_empty() {
                self.drawup_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_drawup;
            egui::Window::new("DRAWUP — Rally History")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.drawup_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.drawup_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.drawup_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_drawup(&conn, &sym_u)
                                    {
                                        self.drawup_snapshot = snap;
                                        self.drawup_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.drawup_symbol.to_uppercase();
                            self.drawup_loading = true;
                            self.drawup_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeDrawupSnapshot { symbol: sym });
                        }
                        if self.drawup_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.drawup_snapshot;
                    if snap.symbol.is_empty() || snap.rally_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥20 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.rally_label.as_str() {
                            "STRONG" | "EXPLOSIVE" => UP,
                            "MUTED" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — max drawup {:.2}% — {} bars — as of {}",
                                snap.symbol,
                                snap.rally_label,
                                snap.max_drawup_pct,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("drawup_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Max drawup").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}%", snap.max_drawup_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Trough → peak dates").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} → {}",
                                        if snap.max_drawup_trough_date.is_empty() {
                                            "—"
                                        } else {
                                            snap.max_drawup_trough_date.as_str()
                                        },
                                        if snap.max_drawup_peak_date.is_empty() {
                                            "—"
                                        } else {
                                            snap.max_drawup_peak_date.as_str()
                                        }
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Longest drawup (days)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.longest_drawup_days))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Rallies ≥5% / ≥10%").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {}",
                                        snap.rallies_5pct, snap.rallies_10pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Current drawup").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}%", snap.current_drawup_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_drawup = open;
        }

        if self.show_gapstats {
            if self.gapstats_symbol.is_empty() {
                self.gapstats_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_gapstats;
            egui::Window::new("GAPSTATS — Overnight Gap Statistics")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.gapstats_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.gapstats_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.gapstats_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_gapstats(&conn, &sym_u)
                                    {
                                        self.gapstats_snapshot = snap;
                                        self.gapstats_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.gapstats_symbol.to_uppercase();
                            self.gapstats_loading = true;
                            self.gapstats_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeGapstatsSnapshot { symbol: sym });
                        }
                        if self.gapstats_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.gapstats_snapshot;
                    if snap.symbol.is_empty() || snap.bias_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new(
                                "No data — HP cache needs ≥20 bars with valid open/close pairs.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.bias_label.as_str() {
                            "UP_BIAS" | "SLIGHT_UP" => UP,
                            "DOWN_BIAS" | "SLIGHT_DOWN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — avg gap {:.3}% — {} bars — as of {}",
                                snap.symbol,
                                snap.bias_label,
                                snap.avg_gap_pct,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("gapstats_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("Gap up / down counts").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {}",
                                        snap.gap_up_count, snap.gap_down_count
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Gap frequency").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.1}%", snap.gap_frequency_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Avg gap up / down %").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2}% / {:.2}%",
                                        snap.avg_gap_up_pct, snap.avg_gap_down_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Largest gap up / down %")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2}% / {:.2}%",
                                        snap.largest_gap_up_pct, snap.largest_gap_down_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Avg all-gap %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}%", snap.avg_gap_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_gapstats = open;
        }

        if self.show_volcluster {
            if self.volcluster_symbol.is_empty() {
                self.volcluster_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_volcluster;
            egui::Window::new("VOLCLUSTER — Volatility Clustering ACF")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.volcluster_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.volcluster_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.volcluster_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_volcluster(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.volcluster_snapshot = snap;
                                        self.volcluster_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.volcluster_symbol.to_uppercase();
                            self.volcluster_loading = true;
                            self.volcluster_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeVolclusterSnapshot { symbol: sym });
                        }
                        if self.volcluster_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.volcluster_snapshot;
                    if snap.symbol.is_empty() || snap.cluster_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.cluster_label.as_str() {
                            "NONE" => UP,
                            "STRONG" | "VERY_STRONG" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — |r| lag1 ACF {:.3} — {} bars — as of {}",
                                snap.symbol,
                                snap.cluster_label,
                                snap.abs_acf_lag1,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("volcluster_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("r² ACF (lag 1 / 5 / 20)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.3} / {:.3} / {:.3}",
                                        snap.sq_acf_lag1, snap.sq_acf_lag5, snap.sq_acf_lag20
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("|r| ACF (lag 1 / 5 / 20)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.3} / {:.3} / {:.3}",
                                        snap.abs_acf_lag1, snap.abs_acf_lag5, snap.abs_acf_lag20
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_volcluster = open;
        }

        if self.show_closeplc {
            if self.closeplc_symbol.is_empty() {
                self.closeplc_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_closeplc;
            egui::Window::new("CLOSEPLC — Close Placement in Daily Range")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.closeplc_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.closeplc_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.closeplc_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_closeplc(&conn, &sym_u)
                                    {
                                        self.closeplc_snapshot = snap;
                                        self.closeplc_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.closeplc_symbol.to_uppercase();
                            self.closeplc_loading = true;
                            self.closeplc_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCloseplcSnapshot { symbol: sym });
                        }
                        if self.closeplc_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.closeplc_snapshot;
                    if snap.symbol.is_empty() || snap.placement_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new(
                                "No data — HP cache needs ≥20 bars with high > low.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.placement_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — avg {:.3} — {} bars — as of {}",
                                snap.symbol,
                                snap.placement_label,
                                snap.avg_placement,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("closeplc_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("Mean / median placement")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.3} / {:.3}",
                                        snap.avg_placement, snap.median_placement
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Latest bar placement").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.latest_placement))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("% near high (>0.8)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.1}%", snap.pct_near_high))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("% near low (<0.2)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.1}%", snap.pct_near_low))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_closeplc = open;
        }

        if self.show_mrhl {
            if self.mrhl_symbol.is_empty() {
                self.mrhl_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_mrhl;
            egui::Window::new("MRHL — Mean-Reversion Half-Life (AR1)")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.mrhl_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.mrhl_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.mrhl_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_mrhl(&conn, &sym_u)
                                    {
                                        self.mrhl_snapshot = snap;
                                        self.mrhl_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.mrhl_symbol.to_uppercase();
                            self.mrhl_loading = true;
                            self.mrhl_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMrhlSnapshot { symbol: sym });
                        }
                        if self.mrhl_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.mrhl_snapshot;
                    if snap.symbol.is_empty() || snap.regime_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.regime_label.as_str() {
                            "PERSISTENT" | "STRONG_PERSISTENT" => UP,
                            "FAST_REVERT" | "MEAN_REVERTING" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — β {:.3} — half-life {:.2}d — {} bars — as of {}",
                                snap.symbol,
                                snap.regime_label,
                                snap.beta,
                                snap.half_life_days,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("mrhl_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("AR(1) β").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.beta))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("AR(1) α").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.alpha))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Half-life (days)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.half_life_days))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("R²").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.r_squared))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_mrhl = open;
        }
    }
}
