use super::*;

impl TyphooNApp {
    pub(super) fn render_research_ohlc_volatility_cvar_calendar_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research = research_chart_symbol(
            self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
        );

        // ── Research section ──
        if self.show_parkinson {
            if self.parkinson_symbol.is_empty() {
                self.parkinson_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_parkinson;
            egui::Window::new("PARKINSON — H-L Range Volatility")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 360.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.parkinson_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.parkinson_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.parkinson_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_parkinson(&conn, &sym_u)
                                    {
                                        self.parkinson_snapshot = snap;
                                        self.parkinson_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.parkinson_symbol.to_uppercase();
                            self.parkinson_loading = true;
                            self.parkinson_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeParkinsonSnapshot { symbol: sym });
                        }
                        if self.parkinson_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.parkinson_snapshot;
                    if snap.symbol.is_empty() || snap.vol_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 bars with H/L.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.vol_label.as_str() {
                            "VERY_LOW" | "LOW" => UP,
                            "HIGH" | "VERY_HIGH" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — {:.2}% annualized ({:.3}% daily) — {} bars — as of {}",
                                snap.symbol,
                                snap.vol_label,
                                snap.annualized_vol_pct,
                                snap.daily_vol_pct,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("parkinson_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(200.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Mean ln(H/L)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.5}", snap.mean_hl_log_ratio))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Daily σ (%)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.daily_vol_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Annualized σ (%)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.annualized_vol_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_parkinson = open;
        }

        if self.show_gkvol {
            if self.gkvol_symbol.is_empty() {
                self.gkvol_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_gkvol;
            egui::Window::new("GKVOL — Garman-Klass OHLC Volatility")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.gkvol_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.gkvol_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.gkvol_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_gkvol(&conn, &sym_u)
                                    {
                                        self.gkvol_snapshot = snap;
                                        self.gkvol_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.gkvol_symbol.to_uppercase();
                            self.gkvol_loading = true;
                            self.gkvol_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeGkvolSnapshot { symbol: sym });
                        }
                        if self.gkvol_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.gkvol_snapshot;
                    if snap.symbol.is_empty() || snap.vol_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 OHLC bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.vol_label.as_str() {
                            "VERY_LOW" | "LOW" => UP,
                            "HIGH" | "VERY_HIGH" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — {:.2}% annualized ({:.3}% daily) — {} bars — as of {}",
                                snap.symbol,
                                snap.vol_label,
                                snap.annualized_vol_pct,
                                snap.daily_vol_pct,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("gkvol_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(200.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("Range component 0.5·(ln H/L)²")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.range_component))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("C/O component k·(ln C/O)²")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.6}", snap.co_component))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Daily σ (%)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.daily_vol_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Annualized σ (%)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.annualized_vol_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_gkvol = open;
        }

        if self.show_rsvol {
            if self.rsvol_symbol.is_empty() {
                self.rsvol_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_rsvol;
            egui::Window::new("RSVOL — Rogers-Satchell OHLC Volatility")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 340.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.rsvol_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.rsvol_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.rsvol_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_rsvol(&conn, &sym_u) {
                                        self.rsvol_snapshot = snap;
                                        self.rsvol_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.rsvol_symbol.to_uppercase();
                            self.rsvol_loading = true;
                            self.rsvol_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeRsvolSnapshot { symbol: sym });
                        }
                        if self.rsvol_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.rsvol_snapshot;
                    if snap.symbol.is_empty() || snap.vol_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — HP cache needs ≥30 OHLC bars.").color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.vol_label.as_str() {
                            "VERY_LOW" | "LOW" => UP,
                            "HIGH" | "VERY_HIGH" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — {:.2}% annualized ({:.3}% daily) — {} bars — drift-independent — as of {}",
                            snap.symbol, snap.vol_label, snap.annualized_vol_pct, snap.daily_vol_pct, snap.bars_used, snap.as_of,
                        )).strong().color(color));
                    }
                });
            self.show_rsvol = open;
        }

        if self.show_cvar {
            if self.cvar_symbol.is_empty() {
                self.cvar_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cvar;
            egui::Window::new("CVAR — Conditional VaR / Expected Shortfall")
                .open(&mut open)
                .resizable(true)
                .default_size([600.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cvar_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.cvar_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.cvar_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_cvar(&conn, &sym_u)
                                    {
                                        self.cvar_snapshot = snap;
                                        self.cvar_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cvar_symbol.to_uppercase();
                            self.cvar_loading = true;
                            self.cvar_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCvarSnapshot { symbol: sym });
                        }
                        if self.cvar_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.cvar_snapshot;
                    if snap.symbol.is_empty() || snap.cvar_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥100 log returns.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.cvar_label.as_str() {
                            "MINIMAL" | "LOW" => UP,
                            "HIGH" | "EXTREME" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — ES(5%) {:+.3}% — ES(1%) {:+.3}% — {} bars — as of {}",
                                snap.symbol,
                                snap.cvar_label,
                                snap.cvar_5pct_ret_pct,
                                snap.cvar_1pct_ret_pct,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("cvar_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("VaR (5%) daily return")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}%", snap.var_5pct_ret_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("CVaR / ES (5%)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}%", snap.cvar_5pct_ret_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Tail days (5%)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.tail_days_5pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("VaR (1%) daily return")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}%", snap.var_1pct_ret_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("CVaR / ES (1%)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}%", snap.cvar_1pct_ret_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Tail days (1%)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.tail_days_1pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_cvar = open;
        }

        if self.show_doweffect {
            if self.doweffect_symbol.is_empty() {
                self.doweffect_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_doweffect;
            egui::Window::new("DOWEFFECT — Day-of-Week Intraday Seasonality")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 460.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.doweffect_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.doweffect_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.doweffect_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_doweffect(&conn, &sym_u)
                                    {
                                        self.doweffect_snapshot = snap;
                                        self.doweffect_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.doweffect_symbol.to_uppercase();
                            self.doweffect_loading = true;
                            self.doweffect_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeDoweffectSnapshot { symbol: sym });
                        }
                        if self.doweffect_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.doweffect_snapshot;
                    if snap.symbol.is_empty() || snap.dow_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new(
                                "No data — HP cache needs ≥100 bars with ≥10 per weekday.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.dow_label.as_str() {
                            "STRONG_EFFECT" | "MILD_EFFECT" => UP,
                            "INCONSISTENT" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        const DOWS: [&str; 5] = ["Mon", "Tue", "Wed", "Thu", "Fri"];
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — best {} ({:.0}%) / worst {} ({:.0}%) — {} wks — as of {}",
                            snap.symbol, snap.dow_label,
                            DOWS[snap.best_dow_idx], snap.best_dow_hit_pct,
                            DOWS[snap.worst_dow_idx], snap.worst_dow_hit_pct,
                            snap.weeks_covered, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("doweffect_grid")
                            .striped(true)
                            .num_columns(4)
                            .min_col_width(100.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Day").small().strong());
                                ui.label(egui::RichText::new("Hit %").small().strong());
                                ui.label(egui::RichText::new("Mean ret %").small().strong());
                                ui.label(egui::RichText::new("N").small().strong());
                                ui.end_row();
                                for i in 0..5 {
                                    ui.label(egui::RichText::new(DOWS[i]).small());
                                    ui.label(
                                        egui::RichText::new(format!("{:.1}%", snap.dow_hit_pct[i]))
                                            .small()
                                            .monospace(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{:+.3}%",
                                            snap.dow_mean_ret_pct[i]
                                        ))
                                        .small()
                                        .monospace(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{}",
                                            snap.dow_sample_count[i]
                                        ))
                                        .small()
                                        .monospace(),
                                    );
                                    ui.end_row();
                                }
                            });
                    }
                });
            self.show_doweffect = open;
        }
    }
}
