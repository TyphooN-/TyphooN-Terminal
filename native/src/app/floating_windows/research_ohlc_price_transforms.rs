use super::*;

impl TyphooNApp {
    pub(super) fn render_research_ohlc_price_transforms_windows(&mut self, ctx: &egui::Context) {
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

        // ── Research Round 66 windows: AVGPRICE / MEDPRICE / TYPPRICE / WCLPRICE / VARIANCE ──
        if self.show_avgprice_win {
            if self.avgprice_win_symbol.is_empty() {
                self.avgprice_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_avgprice_win;
            egui::Window::new("AVGPRICE — OHLC average (O+H+L+C)/4")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.avgprice_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.avgprice_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.avgprice_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_avgprice(&conn, &sym_u)
                                    {
                                        self.avgprice_win_snapshot = snap;
                                        self.avgprice_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.avgprice_win_symbol.to_uppercase();
                            self.avgprice_win_loading = true;
                            self.avgprice_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeAvgpriceSnapshot { symbol: sym });
                        }
                        if self.avgprice_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.avgprice_win_snapshot;
                    if snap.symbol.is_empty() || snap.avgprice_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥1 bar.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.avgprice_label.as_str() {
                            "ABOVE_CLOSE" => UP,
                            "BELOW_CLOSE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — avgprice {:.4} — close {:.4} — Δ {:+.3}% — as of {}",
                                snap.symbol,
                                snap.avgprice_label,
                                snap.avgprice,
                                snap.close,
                                snap.delta_pct,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("avgprice_summary")
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
                                ui.label(egui::RichText::new("AVGPRICE").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.avgprice))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("AVGPRICE prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.avgprice_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Open").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.open))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("High").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.high))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Low").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.low))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Close").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.close))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Δ% vs close").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}%", snap.delta_pct))
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
            self.show_avgprice_win = open;
        }

        if self.show_medprice_win {
            if self.medprice_win_symbol.is_empty() {
                self.medprice_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_medprice_win;
            egui::Window::new("MEDPRICE — range median (H+L)/2")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.medprice_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.medprice_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.medprice_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_medprice(&conn, &sym_u)
                                    {
                                        self.medprice_win_snapshot = snap;
                                        self.medprice_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.medprice_win_symbol.to_uppercase();
                            self.medprice_win_loading = true;
                            self.medprice_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMedpriceSnapshot { symbol: sym });
                        }
                        if self.medprice_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.medprice_win_snapshot;
                    if snap.symbol.is_empty() || snap.medprice_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥1 bar.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.medprice_label.as_str() {
                            "ABOVE_MID" => UP,
                            "BELOW_MID" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — medprice {:.4} — close {:.4} — Δ {:+.3}% — as of {}",
                                snap.symbol,
                                snap.medprice_label,
                                snap.medprice,
                                snap.close,
                                snap.delta_pct,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("medprice_summary")
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
                                ui.label(egui::RichText::new("MEDPRICE").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.medprice))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("MEDPRICE prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.medprice_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("High").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.high))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Low").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.low))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Close").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.close))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Δ% vs close").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}%", snap.delta_pct))
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
            self.show_medprice_win = open;
        }

        if self.show_typprice_win {
            if self.typprice_win_symbol.is_empty() {
                self.typprice_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_typprice_win;
            egui::Window::new("TYPPRICE — typical price (H+L+C)/3")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.typprice_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.typprice_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.typprice_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_typprice(&conn, &sym_u)
                                    {
                                        self.typprice_win_snapshot = snap;
                                        self.typprice_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.typprice_win_symbol.to_uppercase();
                            self.typprice_win_loading = true;
                            self.typprice_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeTypPriceSnapshot { symbol: sym });
                        }
                        if self.typprice_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.typprice_win_snapshot;
                    if snap.symbol.is_empty() || snap.typprice_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥1 bar.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.typprice_label.as_str() {
                            "ABOVE_CLOSE" => UP,
                            "BELOW_CLOSE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — typprice {:.4} — close {:.4} — Δ {:+.3}% — as of {}",
                                snap.symbol,
                                snap.typprice_label,
                                snap.typprice,
                                snap.close,
                                snap.delta_pct,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("typprice_summary")
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
                                ui.label(egui::RichText::new("TYPPRICE").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.typprice))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("TYPPRICE prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.typprice_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("High").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.high))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Low").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.low))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Close").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.close))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Δ% vs close").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}%", snap.delta_pct))
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
            self.show_typprice_win = open;
        }

        if self.show_wclprice_win {
            if self.wclprice_win_symbol.is_empty() {
                self.wclprice_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_wclprice_win;
            egui::Window::new("WCLPRICE — weighted close (H+L+2C)/4")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.wclprice_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.wclprice_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.wclprice_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_wclprice(&conn, &sym_u)
                                    {
                                        self.wclprice_win_snapshot = snap;
                                        self.wclprice_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.wclprice_win_symbol.to_uppercase();
                            self.wclprice_win_loading = true;
                            self.wclprice_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeWclPriceSnapshot { symbol: sym });
                        }
                        if self.wclprice_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.wclprice_win_snapshot;
                    if snap.symbol.is_empty() || snap.wclprice_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥1 bar.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.wclprice_label.as_str() {
                            "ABOVE_CLOSE" => UP,
                            "BELOW_CLOSE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — wclprice {:.4} — close {:.4} — Δ {:+.3}% — as of {}",
                                snap.symbol,
                                snap.wclprice_label,
                                snap.wclprice,
                                snap.close,
                                snap.delta_pct,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("wclprice_summary")
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
                                ui.label(egui::RichText::new("WCLPRICE").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.wclprice))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("WCLPRICE prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.wclprice_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("High").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.high))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Low").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.low))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Close").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.close))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Δ% vs close").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}%", snap.delta_pct))
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
            self.show_wclprice_win = open;
        }

        if self.show_variance_win {
            if self.variance_win_symbol.is_empty() {
                self.variance_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_variance_win;
            egui::Window::new("VARIANCE — close variance (5-bar population, TA-Lib default)")
                .open(&mut open).resizable(true).default_size([540.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.variance_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.variance_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.variance_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_variance(&conn, &sym_u) { self.variance_win_snapshot = snap; self.variance_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.variance_win_symbol.to_uppercase(); self.variance_win_loading = true; self.variance_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeVarianceSnapshot { symbol: sym });
                        }
                        if self.variance_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.variance_win_snapshot;
                    if snap.symbol.is_empty() || snap.variance_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥5 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.variance_label.as_str() {
                            "HIGH_VOL" | "ELEVATED" => DOWN, "LOW_VOL" => UP, _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — variance {:.6} — stddev {:.4} — CV {:.3}% — close {:.4} — as of {}",
                            snap.symbol, snap.variance_label, snap.variance, snap.stddev, snap.cv, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("variance_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Period").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.period)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Mean").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.mean)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Variance").small().strong()); ui.label(egui::RichText::new(format!("{:.6}", snap.variance)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Variance prev").small().strong()); ui.label(egui::RichText::new(format!("{:.6}", snap.variance_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Stddev").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.stddev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("CV %").small().strong()); ui.label(egui::RichText::new(format!("{:.3}%", snap.cv)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_variance_win = open;
        }
    }
}
