use super::*;

impl TyphooNApp {
    pub(super) fn render_balance_calendar_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
        if self.show_bbwidth_win {
            if self.bbwidth_win_symbol.is_empty() {
                self.bbwidth_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_bbwidth_win;
            egui::Window::new("BBWIDTH — Bollinger Bandwidth (SMA₂₀ ± 2σ, 125-bar percentile)")
                .open(&mut open).resizable(true).default_size([640.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.bbwidth_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.bbwidth_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.bbwidth_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_bbwidth(&conn, &sym_u) { self.bbwidth_win_snapshot = snap; self.bbwidth_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.bbwidth_win_symbol.to_uppercase(); self.bbwidth_win_loading = true; self.bbwidth_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeBbwidthSnapshot { symbol: sym });
                        }
                        if self.bbwidth_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.bbwidth_win_snapshot;
                    if snap.symbol.is_empty() || snap.bbw_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥20 bars (125 for percentile).").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.bbw_label.as_str() {
                            "SQUEEZE" => DOWN,
                            "EXPANDED" => UP,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — BBW {:.4} — pct {:.1} — mid {:.4} — close {:.4} — as of {}", snap.symbol, snap.bbw_label, snap.bbw_value, snap.bbw_percentile, snap.middle, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("bbwidth_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Stdev width").small().strong()); ui.label(egui::RichText::new(format!("±{:.1}σ", snap.num_stdev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("BBW").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.bbw_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("BBW prev").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.bbw_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("125-bar percentile").small().strong()); ui.label(egui::RichText::new(format!("{:.1}", snap.bbw_percentile)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Upper").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.upper)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Middle").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.middle)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Lower").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.lower)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_bbwidth_win = open;
        }

        if self.show_elderimp_win {
            if self.elderimp_win_symbol.is_empty() {
                self.elderimp_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_elderimp_win;
            egui::Window::new("ELDERIMP — Elder Impulse System (13-EMA slope + MACD hist slope)")
                .open(&mut open).resizable(true).default_size([620.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.elderimp_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.elderimp_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.elderimp_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_elderimp(&conn, &sym_u) { self.elderimp_win_snapshot = snap; self.elderimp_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.elderimp_win_symbol.to_uppercase(); self.elderimp_win_loading = true; self.elderimp_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeElderImpSnapshot { symbol: sym });
                        }
                        if self.elderimp_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.elderimp_win_snapshot;
                    if snap.symbol.is_empty() || snap.impulse_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥35 bars.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.impulse_label.as_str() {
                            "GREEN" => UP,
                            "RED" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — EMA {:.4} (slope {:+.4}) — hist {:+.4} (slope {:+.4}) — close {:.4} — as of {}", snap.symbol, snap.impulse_label, snap.ema_value, snap.ema_slope, snap.macd_hist, snap.macd_hist_slope, snap.last_close, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("elderimp_summary").striped(true).num_columns(2).min_col_width(200.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Bars used").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.bars_used)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("EMA length").small().strong()); ui.label(egui::RichText::new(format!("{}", snap.ema_length)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("EMA").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.ema_value)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("EMA slope").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.ema_slope)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("MACD hist").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.macd_hist)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("MACD hist prev").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.macd_hist_prev)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("MACD hist slope").small().strong()); ui.label(egui::RichText::new(format!("{:+.4}", snap.macd_hist_slope)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Last close").small().strong()); ui.label(egui::RichText::new(format!("{:.4}", snap.last_close)).small().monospace()); ui.end_row();
                        });
                        if !snap.note.is_empty() { ui.separator(); ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    }
                });
            self.show_elderimp_win = open;
        }

        if self.show_rmi_win {
            if self.rmi_win_symbol.is_empty() {
                self.rmi_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_rmi_win;
            egui::Window::new("RMI — Relative Momentum Index (Altman; RSI on 5-bar momentum)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.rmi_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.rmi_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.rmi_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_rmi(&conn, &sym_u)
                                    {
                                        self.rmi_win_snapshot = snap;
                                        self.rmi_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.rmi_win_symbol.to_uppercase();
                            self.rmi_win_loading = true;
                            self.rmi_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRmiSnapshot { symbol: sym });
                        }
                        if self.rmi_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.rmi_win_snapshot;
                    if snap.symbol.is_empty() || snap.rmi_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥25 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.rmi_label.as_str() {
                            "OVERBOUGHT" | "BULL" => UP,
                            "OVERSOLD" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — RMI {:.2} — close {:.4} — as of {}",
                                snap.symbol,
                                snap.rmi_label,
                                snap.rmi_value,
                                snap.last_close,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("rmi_summary")
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
                                ui.label(egui::RichText::new("Length").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.length))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Momentum length").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.momentum_length))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("RMI").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.rmi_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("RMI prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.rmi_prev))
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
            self.show_rmi_win = open;
        }

        if self.show_expcal_win {
            if self.expcal_win_symbol.is_empty() {
                self.expcal_win_symbol = chart_sym_research.clone();
            }
            if self.expcal_win_calendar.is_empty() {
                let today = chrono::Local::now().date_naive();
                self.expcal_win_calendar = typhoon_engine::core::research::compute_market_calendar(
                    today,
                    self.expcal_win_horizon_days,
                );
            }
            let mut open = self.show_expcal_win;
            egui::Window::new(
                "EXPCAL — Options Expiration Calendar (Tier 1 market · Tier 2 per-symbol)",
            )
            .open(&mut open)
            .resizable(true)
            .default_size([780.0, 480.0])
            .max_size([780.0, 560.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.expcal_win_tab, 0, "Market calendar");
                    ui.selectable_value(&mut self.expcal_win_tab, 1, "Symbol chain");
                });
                ui.separator();
                if self.expcal_win_tab == 0 {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Horizon days:").color(AXIS_TEXT));
                        let mut h = self.expcal_win_horizon_days as i32;
                        if ui
                            .add(egui::DragValue::new(&mut h).range(7..=730))
                            .changed()
                        {
                            self.expcal_win_horizon_days = h.max(7) as u32;
                            let today = chrono::Local::now().date_naive();
                            self.expcal_win_calendar =
                                typhoon_engine::core::research::compute_market_calendar(
                                    today,
                                    self.expcal_win_horizon_days,
                                );
                        }
                        if ui.button("Regenerate").clicked() {
                            let today = chrono::Local::now().date_naive();
                            self.expcal_win_calendar =
                                typhoon_engine::core::research::compute_market_calendar(
                                    today,
                                    self.expcal_win_horizon_days,
                                );
                        }
                    });
                    ui.separator();
                    if self.expcal_win_calendar.is_empty() {
                        ui.label(
                            egui::RichText::new("No upcoming Fridays in horizon.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        egui::ScrollArea::vertical()
                            .id_salt("expcal_tier1_scroll")
                            .show(ui, |ui| {
                                egui::Grid::new("expcal_tier1_grid")
                                    .striped(true)
                                    .num_columns(4)
                                    .min_col_width(90.0)
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("Date").small().strong());
                                        ui.label(egui::RichText::new("Weekday").small().strong());
                                        ui.label(egui::RichText::new("DTE").small().strong());
                                        ui.label(egui::RichText::new("Type").small().strong());
                                        ui.end_row();
                                        for e in &self.expcal_win_calendar {
                                            let color = match e.expiry_type.as_str() {
                                                "TRIPLE_WITCHING" => DOWN,
                                                "QUARTERLY" => UP,
                                                "LEAPS" => AXIS_TEXT,
                                                "MONTHLY" => UP,
                                                _ => AXIS_TEXT,
                                            };
                                            ui.label(
                                                egui::RichText::new(&e.date).small().monospace(),
                                            );
                                            ui.label(egui::RichText::new(&e.weekday).small());
                                            ui.label(
                                                egui::RichText::new(format!("{}", e.days_from_now))
                                                    .small()
                                                    .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(&e.expiry_type)
                                                    .small()
                                                    .color(color)
                                                    .strong(),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
                } else {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.expcal_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.expcal_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.expcal_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_symbol_expirations(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.expcal_win_snapshot = snap;
                                        self.expcal_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.expcal_win_symbol.to_uppercase();
                            self.expcal_win_loading = true;
                            self.expcal_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeSymbolExpirations { symbol: sym });
                        }
                        if self.expcal_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.expcal_win_snapshot;
                    if snap.symbol.is_empty() {
                        ui.label(
                            egui::RichText::new(
                                "No data — run OPTIONS first to cache the chain, then Compute.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                    } else if snap.expirations.is_empty() {
                        ui.label(
                            egui::RichText::new(if snap.note.is_empty() {
                                "Chain present but no expirations parsed."
                            } else {
                                snap.note.as_str()
                            })
                            .color(AXIS_TEXT)
                            .small(),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} expirations — underlying {:.4} — as of {}",
                                snap.symbol,
                                snap.expirations.len(),
                                snap.underlying_price,
                                snap.as_of
                            ))
                            .strong(),
                        );
                        if !snap.next_triple_witching.is_empty() {
                            ui.label(
                                egui::RichText::new(format!(
                                    "Next triple witching: {}",
                                    snap.next_triple_witching
                                ))
                                .color(DOWN)
                                .strong(),
                            );
                        }
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .id_salt("expcal_tier2_scroll")
                            .show(ui, |ui| {
                                egui::Grid::new("expcal_tier2_grid")
                                    .striped(true)
                                    .num_columns(9)
                                    .min_col_width(68.0)
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("Date").small().strong());
                                        ui.label(egui::RichText::new("DTE").small().strong());
                                        ui.label(egui::RichText::new("Type").small().strong());
                                        ui.label(egui::RichText::new("Calls").small().strong());
                                        ui.label(egui::RichText::new("Puts").small().strong());
                                        ui.label(egui::RichText::new("Call Vol").small().strong());
                                        ui.label(egui::RichText::new("Put Vol").small().strong());
                                        ui.label(egui::RichText::new("Call OI").small().strong());
                                        ui.label(
                                            egui::RichText::new("Put OI / PCR").small().strong(),
                                        );
                                        ui.end_row();
                                        for ex in &snap.expirations {
                                            let color = match ex.expiry_type.as_str() {
                                                "TRIPLE_WITCHING" => DOWN,
                                                "QUARTERLY" | "MONTHLY" => UP,
                                                _ => AXIS_TEXT,
                                            };
                                            ui.label(
                                                egui::RichText::new(&ex.date).small().monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{}",
                                                    ex.days_to_expiry
                                                ))
                                                .small()
                                                .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(&ex.expiry_type)
                                                    .small()
                                                    .color(color)
                                                    .strong(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("{}", ex.call_count))
                                                    .small()
                                                    .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("{}", ex.put_count))
                                                    .small()
                                                    .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:.0}",
                                                    ex.total_call_volume
                                                ))
                                                .small()
                                                .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:.0}",
                                                    ex.total_put_volume
                                                ))
                                                .small()
                                                .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:.0}",
                                                    ex.total_call_oi
                                                ))
                                                .small()
                                                .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:.0} / {:.2}",
                                                    ex.total_put_oi, ex.put_call_ratio
                                                ))
                                                .small()
                                                .monospace(),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            });
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
                        }
                    }
                }
            });
            self.show_expcal_win = open;
        }
    }
}
