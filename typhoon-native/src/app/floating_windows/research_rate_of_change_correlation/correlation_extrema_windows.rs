use super::*;

impl TyphooNApp {
    pub(super) fn render_correlation_extrema_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
        if self.show_correl_win {
            if self.correl_win_symbol.is_empty() {
                self.correl_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_correl_win;
            egui::Window::new("CORREL — lag-1 autocorrelation of close (period 30)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.correl_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.correl_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.correl_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_correl(&conn, &sym_u)
                                    {
                                        self.correl_win_snapshot = snap;
                                        self.correl_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.correl_win_symbol.to_uppercase();
                            self.correl_win_loading = true;
                            self.correl_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCorrelSnapshot { symbol: sym });
                        }
                        if self.correl_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.correl_win_snapshot;
                    if snap.symbol.is_empty() || snap.correl_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥32 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.correl_label.as_str() {
                            "STRONG_MOMO" | "MOMO" => UP,
                            "STRONG_MEAN_REVERT" | "MEAN_REVERT" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — ρ {:+.4} — close {:.4} — as of {}",
                                snap.symbol,
                                snap.correl_label,
                                snap.correl,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("correl_summary")
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
                                ui.label(egui::RichText::new("Period").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.period))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ρ").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.correl))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ρ prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.correl_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("mean(x)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.mean_x))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("mean(y)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.mean_y))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("σ(x)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.stddev_x))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("σ(y)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.stddev_y))
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
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
                        }
                    }
                });
            self.show_correl_win = open;
        }

        if self.show_min_win {
            if self.min_win_symbol.is_empty() {
                self.min_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_min_win;
            egui::Window::new("MIN — rolling minimum of close (period 30)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.min_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.min_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.min_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_min(&conn, &sym_u)
                                    {
                                        self.min_win_snapshot = snap;
                                        self.min_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.min_win_symbol.to_uppercase();
                            self.min_win_loading = true;
                            self.min_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMinSnapshot { symbol: sym });
                        }
                        if self.min_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.min_win_snapshot;
                    if snap.symbol.is_empty() || snap.min_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.min_label.as_str() {
                            "NEAR_LOW" => DOWN,
                            "NEAR_HIGH" => UP,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — min {:.4} — close {:.4} — pos {:.1}% — as of {}",
                                snap.symbol,
                                snap.min_label,
                                snap.min_val,
                                snap.last_close,
                                snap.position_pct,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("min_summary")
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
                                ui.label(egui::RichText::new("Period").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.period))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Min value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.min_val))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Min prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.min_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Max ref").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.max_ref))
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
                                ui.label(egui::RichText::new("Position %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.position_pct))
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
            self.show_min_win = open;
        }

        if self.show_max_win {
            if self.max_win_symbol.is_empty() {
                self.max_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_max_win;
            egui::Window::new("MAX — rolling maximum of close (period 30)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.max_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.max_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.max_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_max(&conn, &sym_u)
                                    {
                                        self.max_win_snapshot = snap;
                                        self.max_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.max_win_symbol.to_uppercase();
                            self.max_win_loading = true;
                            self.max_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMaxSnapshot { symbol: sym });
                        }
                        if self.max_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.max_win_snapshot;
                    if snap.symbol.is_empty() || snap.max_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.max_label.as_str() {
                            "NEAR_HIGH" => UP,
                            "NEAR_LOW" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — max {:.4} — close {:.4} — pos {:.1}% — as of {}",
                                snap.symbol,
                                snap.max_label,
                                snap.max_val,
                                snap.last_close,
                                snap.position_pct,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("max_summary")
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
                                ui.label(egui::RichText::new("Period").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.period))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Max value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.max_val))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Max prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.max_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Min ref").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.min_ref))
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
                                ui.label(egui::RichText::new("Position %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.position_pct))
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
            self.show_max_win = open;
        }

        if self.show_minmax_win {
            if self.minmax_win_symbol.is_empty() {
                self.minmax_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_minmax_win;
            egui::Window::new("MINMAX — rolling range (min/max of close, period 30)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.minmax_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.minmax_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.minmax_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_minmax(&conn, &sym_u)
                                    {
                                        self.minmax_win_snapshot = snap;
                                        self.minmax_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.minmax_win_symbol.to_uppercase();
                            self.minmax_win_loading = true;
                            self.minmax_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMinMaxSnapshot { symbol: sym });
                        }
                        if self.minmax_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.minmax_win_snapshot;
                    if snap.symbol.is_empty() || snap.minmax_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.minmax_label.as_str() {
                            "RANGE_WIDE" => UP,
                            "RANGE_TIGHT" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — [{:.4}..{:.4}] — width {:.4} ({:.2}%) — as of {}",
                                snap.symbol,
                                snap.minmax_label,
                                snap.min_val,
                                snap.max_val,
                                snap.range_width,
                                snap.range_pct,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("minmax_summary")
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
                                ui.label(egui::RichText::new("Period").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.period))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Min value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.min_val))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Max value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.max_val))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Range width").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.range_width))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Range %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.range_pct))
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
                                ui.label(egui::RichText::new("Position %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.position_pct))
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
            self.show_minmax_win = open;
        }

        if self.show_minindex_win {
            if self.minindex_win_symbol.is_empty() {
                self.minindex_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_minindex_win;
            egui::Window::new("MININDEX — recency of rolling-window low (period 30)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.minindex_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.minindex_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.minindex_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_minindex(&conn, &sym_u)
                                    {
                                        self.minindex_win_snapshot = snap;
                                        self.minindex_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.minindex_win_symbol.to_uppercase();
                            self.minindex_win_loading = true;
                            self.minindex_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMinIndexSnapshot { symbol: sym });
                        }
                        if self.minindex_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.minindex_win_snapshot;
                    if snap.symbol.is_empty() || snap.min_index_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.min_index_label.as_str() {
                            "FRESH_LOW" | "RECENT_LOW" => DOWN,
                            "STALE_LOW" => UP,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — low {:.4} — {} bars ago — close {:.4} — as of {}",
                                snap.symbol,
                                snap.min_index_label,
                                snap.min_val,
                                snap.min_index_bars_ago,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("minindex_summary")
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
                                ui.label(egui::RichText::new("Period").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.period))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Min value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.min_val))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Min bars ago").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.min_index_bars_ago))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Min bars ago (prev)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}",
                                        snap.min_index_bars_ago_prev
                                    ))
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
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
                        }
                    }
                });
            self.show_minindex_win = open;
        }

        if self.show_maxindex_win {
            if self.maxindex_win_symbol.is_empty() {
                self.maxindex_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_maxindex_win;
            egui::Window::new("MAXINDEX — recency of rolling-window high (period 30)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.maxindex_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.maxindex_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.maxindex_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_maxindex(&conn, &sym_u)
                                    {
                                        self.maxindex_win_snapshot = snap;
                                        self.maxindex_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.maxindex_win_symbol.to_uppercase();
                            self.maxindex_win_loading = true;
                            self.maxindex_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMaxIndexSnapshot { symbol: sym });
                        }
                        if self.maxindex_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.maxindex_win_snapshot;
                    if snap.symbol.is_empty() || snap.max_index_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.max_index_label.as_str() {
                            "FRESH_HIGH" | "RECENT_HIGH" => UP,
                            "STALE_HIGH" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — high {:.4} — {} bars ago — close {:.4} — as of {}",
                                snap.symbol,
                                snap.max_index_label,
                                snap.max_val,
                                snap.max_index_bars_ago,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("maxindex_summary")
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
                                ui.label(egui::RichText::new("Period").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.period))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Max value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.max_val))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Max bars ago").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.max_index_bars_ago))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Max bars ago (prev)").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}",
                                        snap.max_index_bars_ago_prev
                                    ))
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
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
                        }
                    }
                });
            self.show_maxindex_win = open;
        }
    }
}
