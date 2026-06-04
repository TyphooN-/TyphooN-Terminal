use super::*;

impl TyphooNApp {
    pub(super) fn render_research_round48_windows(&mut self, ctx: &egui::Context) {
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

        // ── Research Round 48 windows ──
        if self.show_efi_win {
            if self.efi_win_symbol.is_empty() {
                self.efi_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_efi_win;
            egui::Window::new("EFI — Elder Force Index (volume × Δclose, EMA13)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.efi_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.efi_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.efi_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_efi(&conn, &sym_u)
                                    {
                                        self.efi_win_snapshot = snap;
                                        self.efi_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.efi_win_symbol.to_uppercase();
                            self.efi_win_loading = true;
                            self.efi_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeEfiSnapshot { symbol: sym });
                        }
                        if self.efi_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.efi_win_snapshot;
                    if snap.symbol.is_empty() || snap.efi_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥17 bars with volume.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.efi_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — EFI {:+.2} — prev {:+.2} — as of {}",
                                snap.symbol,
                                snap.efi_label,
                                snap.efi_value,
                                snap.efi_prev,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("efi_summary")
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
                                ui.label(egui::RichText::new("EMA period").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.ema_period))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Raw force").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}", snap.raw_efi))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("EFI value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}", snap.efi_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("EFI prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}", snap.efi_prev))
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
                    }
                });
            self.show_efi_win = open;
        }

        if self.show_emv_win {
            if self.emv_win_symbol.is_empty() {
                self.emv_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_emv_win;
            egui::Window::new("EMV — Ease of Movement (Arms, SMA14)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.emv_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.emv_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.emv_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_emv(&conn, &sym_u)
                                    {
                                        self.emv_win_snapshot = snap;
                                        self.emv_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.emv_win_symbol.to_uppercase();
                            self.emv_win_loading = true;
                            self.emv_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeEmvSnapshot { symbol: sym });
                        }
                        if self.emv_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.emv_win_snapshot;
                    if snap.symbol.is_empty() || snap.emv_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥18 bars with volume.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.emv_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — EMV {:+.4} — as of {}",
                                snap.symbol, snap.emv_label, snap.emv_value, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("emv_summary")
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
                                ui.label(egui::RichText::new("SMA period").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.sma_period))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Volume scale").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.0}", snap.volume_scale))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Raw EMV").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.raw_emv))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("EMV value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.emv_value))
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
                    }
                });
            self.show_emv_win = open;
        }

        if self.show_nvi_win {
            if self.nvi_win_symbol.is_empty() {
                self.nvi_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_nvi_win;
            egui::Window::new("NVI — Negative Volume Index (Dysart/Fosback)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.nvi_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.nvi_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.nvi_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_nvi(&conn, &sym_u)
                                    {
                                        self.nvi_win_snapshot = snap;
                                        self.nvi_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.nvi_win_symbol.to_uppercase();
                            self.nvi_win_loading = true;
                            self.nvi_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeNviSnapshot { symbol: sym });
                        }
                        if self.nvi_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.nvi_win_snapshot;
                    if snap.symbol.is_empty() || snap.nvi_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 bars with volume.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.nvi_label.as_str() {
                            "BULL" => UP,
                            "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — NVI {:.2} vs signal {:.2} — as of {}",
                                snap.symbol,
                                snap.nvi_label,
                                snap.nvi_value,
                                snap.signal_value,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("nvi_summary")
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
                                ui.label(egui::RichText::new("Signal period").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.signal_period))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("NVI value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.nvi_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Signal EMA").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.signal_value))
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
                    }
                });
            self.show_nvi_win = open;
        }

        if self.show_pvi_win {
            if self.pvi_win_symbol.is_empty() {
                self.pvi_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_pvi_win;
            egui::Window::new("PVI — Positive Volume Index (Dysart/Fosback)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.pvi_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.pvi_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.pvi_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_pvi(&conn, &sym_u)
                                    {
                                        self.pvi_win_snapshot = snap;
                                        self.pvi_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.pvi_win_symbol.to_uppercase();
                            self.pvi_win_loading = true;
                            self.pvi_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputePviSnapshot { symbol: sym });
                        }
                        if self.pvi_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.pvi_win_snapshot;
                    if snap.symbol.is_empty() || snap.pvi_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥30 bars with volume.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.pvi_label.as_str() {
                            "BULL" => UP,
                            "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — PVI {:.2} vs signal {:.2} — as of {}",
                                snap.symbol,
                                snap.pvi_label,
                                snap.pvi_value,
                                snap.signal_value,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("pvi_summary")
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
                                ui.label(egui::RichText::new("Signal period").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.signal_period))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("PVI value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.pvi_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Signal EMA").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.signal_value))
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
                    }
                });
            self.show_pvi_win = open;
        }

        if self.show_coppock_win {
            if self.coppock_win_symbol.is_empty() {
                self.coppock_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_coppock_win;
            egui::Window::new("COPPOCK — Coppock Curve (WMA10 of ROC14+ROC11)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.coppock_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.coppock_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.coppock_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_coppock(&conn, &sym_u)
                                    {
                                        self.coppock_win_snapshot = snap;
                                        self.coppock_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.coppock_win_symbol.to_uppercase();
                            self.coppock_win_loading = true;
                            self.coppock_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCoppockSnapshot { symbol: sym });
                        }
                        if self.coppock_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.coppock_win_snapshot;
                    if snap.symbol.is_empty() || snap.coppock_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥26 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.coppock_label.as_str() {
                            "BUY_CROSS" | "BULL" => UP,
                            "SELL_CROSS" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — Coppock {:+.4} — prev {:+.4} — as of {}",
                                snap.symbol,
                                snap.coppock_label,
                                snap.coppock_value,
                                snap.coppock_prev,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("coppock_summary")
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
                                ui.label(egui::RichText::new("ROC fast / slow").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {}",
                                        snap.roc_fast, snap.roc_slow
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("WMA period").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.wma_period))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Coppock").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.coppock_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Coppock prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.coppock_prev))
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
                    }
                });
            self.show_coppock_win = open;
        }

        if self.show_cmo_win {
            if self.cmo_win_symbol.is_empty() {
                self.cmo_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cmo_win;
            egui::Window::new("CMO — Chande Momentum Oscillator (period 9)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cmo_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.cmo_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.cmo_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_cmo(&conn, &sym_u)
                                    {
                                        self.cmo_win_snapshot = snap;
                                        self.cmo_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cmo_win_symbol.to_uppercase();
                            self.cmo_win_loading = true;
                            self.cmo_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCmoSnapshot { symbol: sym });
                        }
                        if self.cmo_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.cmo_win_snapshot;
                    if snap.symbol.is_empty() || snap.cmo_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥11 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.cmo_label.as_str() {
                            "OVERBOUGHT" | "BULL" => UP,
                            "OVERSOLD" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — CMO {:+.2} — as of {}",
                                snap.symbol, snap.cmo_label, snap.cmo_value, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("cmo_summary")
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
                                ui.label(egui::RichText::new("Σ gains").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sum_up))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Σ losses").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sum_dn))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("CMO").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}", snap.cmo_value))
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
                    }
                });
            self.show_cmo_win = open;
        }

        if self.show_qstick_win {
            if self.qstick_win_symbol.is_empty() {
                self.qstick_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_qstick_win;
            egui::Window::new("QSTICK — Chande Q-Stick (SMA14 of candle body)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.qstick_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.qstick_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.qstick_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_qstick(&conn, &sym_u)
                                    {
                                        self.qstick_win_snapshot = snap;
                                        self.qstick_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.qstick_win_symbol.to_uppercase();
                            self.qstick_win_loading = true;
                            self.qstick_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeQstickSnapshot { symbol: sym });
                        }
                        if self.qstick_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.qstick_win_snapshot;
                    if snap.symbol.is_empty() || snap.qstick_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥16 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.qstick_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — Q-Stick {:+.4} — prev {:+.4} — as of {}",
                                snap.symbol,
                                snap.qstick_label,
                                snap.qstick_value,
                                snap.qstick_prev,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("qstick_summary")
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
                                ui.label(egui::RichText::new("Q-Stick").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.qstick_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Q-Stick prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.qstick_prev))
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
                    }
                });
            self.show_qstick_win = open;
        }

        if self.show_disparity_win {
            if self.disparity_win_symbol.is_empty() {
                self.disparity_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_disparity_win;
            egui::Window::new("DISPARITY — Disparity Index ((close/SMA14 − 1)·100)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.disparity_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.disparity_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.disparity_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_disparity(&conn, &sym_u)
                                    {
                                        self.disparity_win_snapshot = snap;
                                        self.disparity_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.disparity_win_symbol.to_uppercase();
                            self.disparity_win_loading = true;
                            self.disparity_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeDisparitySnapshot { symbol: sym });
                        }
                        if self.disparity_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.disparity_win_snapshot;
                    if snap.symbol.is_empty() || snap.disparity_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥16 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.disparity_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — Disparity {:+.2}% — as of {}",
                                snap.symbol, snap.disparity_label, snap.disparity_value, snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("disparity_summary")
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
                                ui.label(egui::RichText::new("SMA value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sma_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Disparity %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}", snap.disparity_value))
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
                    }
                });
            self.show_disparity_win = open;
        }

        if self.show_bop_win {
            if self.bop_win_symbol.is_empty() {
                self.bop_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_bop_win;
            egui::Window::new("BOP — Balance of Power (SMA14 of (close−open)/(high−low))")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.bop_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.bop_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.bop_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_bop(&conn, &sym_u)
                                    {
                                        self.bop_win_snapshot = snap;
                                        self.bop_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.bop_win_symbol.to_uppercase();
                            self.bop_win_loading = true;
                            self.bop_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeBopSnapshot { symbol: sym });
                        }
                        if self.bop_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.bop_win_snapshot;
                    if snap.symbol.is_empty() || snap.bop_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥16 bars with OHLC.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.bop_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — BOP {:+.3} — raw {:+.3} — as of {}",
                                snap.symbol,
                                snap.bop_label,
                                snap.bop_value,
                                snap.raw_bop,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("bop_summary")
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
                                ui.label(egui::RichText::new("BOP (smoothed)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.bop_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("BOP (latest raw)").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}", snap.raw_bop))
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
                    }
                });
            self.show_bop_win = open;
        }

        if self.show_schaff_win {
            if self.schaff_win_symbol.is_empty() {
                self.schaff_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_schaff_win;
            egui::Window::new("SCHAFF — Schaff Trend Cycle (STC, 23/50/10)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.schaff_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.schaff_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.schaff_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_schaff(&conn, &sym_u)
                                    {
                                        self.schaff_win_snapshot = snap;
                                        self.schaff_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.schaff_win_symbol.to_uppercase();
                            self.schaff_win_loading = true;
                            self.schaff_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeSchaffSnapshot { symbol: sym });
                        }
                        if self.schaff_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.schaff_win_snapshot;
                    if snap.symbol.is_empty() || snap.schaff_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥80 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.schaff_label.as_str() {
                            "OVERBOUGHT" | "BULL" => UP,
                            "OVERSOLD" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — STC {:.1} — prev {:.1} — as of {}",
                                snap.symbol,
                                snap.schaff_label,
                                snap.stc_value,
                                snap.stc_prev,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("schaff_summary")
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
                                ui.label(egui::RichText::new("EMA fast / slow").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {}",
                                        snap.ema_fast, snap.ema_slow
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Cycle").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.cycle))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("STC value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.stc_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("STC prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.stc_prev))
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
                    }
                });
            self.show_schaff_win = open;
        }

        if self.show_stoch_win {
            if self.stoch_win_symbol.is_empty() {
                self.stoch_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_stoch_win;
            egui::Window::new("STOCH — Lane Stochastic Oscillator (%K 14 / %D 3, smoothing 3)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.stoch_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.stoch_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.stoch_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_stoch(&conn, &sym_u)
                                    {
                                        self.stoch_win_snapshot = snap;
                                        self.stoch_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.stoch_win_symbol.to_uppercase();
                            self.stoch_win_loading = true;
                            self.stoch_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeStochSnapshot { symbol: sym });
                        }
                        if self.stoch_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.stoch_win_snapshot;
                    if snap.symbol.is_empty() || snap.stoch_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥17 bars with OHLC.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.stoch_label.as_str() {
                            "OVERBOUGHT" | "BULL" => UP,
                            "OVERSOLD" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — %K {:.2} / %D {:.2} — as of {}",
                                snap.symbol,
                                snap.stoch_label,
                                snap.percent_k,
                                snap.percent_d,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("stoch_summary")
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
                                ui.label(egui::RichText::new("%K period").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.k_period))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("%D period").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.d_period))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Smoothing").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.smoothing))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("%K").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.percent_k))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("%D").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.percent_d))
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
                    }
                });
            self.show_stoch_win = open;
        }

        if self.show_macd_win {
            if self.macd_win_symbol.is_empty() {
                self.macd_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_macd_win;
            egui::Window::new("MACD — Appel Moving Average Convergence Divergence (12/26/9)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.macd_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.macd_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.macd_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_macd(&conn, &sym_u)
                                    {
                                        self.macd_win_snapshot = snap;
                                        self.macd_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.macd_win_symbol.to_uppercase();
                            self.macd_win_loading = true;
                            self.macd_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMacdSnapshot { symbol: sym });
                        }
                        if self.macd_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.macd_win_snapshot;
                    if snap.symbol.is_empty() || snap.macd_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥35 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.macd_label.as_str() {
                            "BULL_CROSS" | "BULL" => UP,
                            "BEAR_CROSS" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — MACD {:+.4} / signal {:+.4} / hist {:+.4} — as of {}",
                                snap.symbol,
                                snap.macd_label,
                                snap.macd_value,
                                snap.signal_value,
                                snap.histogram,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("macd_summary")
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
                                ui.label(egui::RichText::new("Fast period").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.fast_period))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Slow period").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.slow_period))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Signal period").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.signal_period))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("MACD value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.macd_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Signal value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.signal_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Histogram").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.histogram))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Histogram prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.histogram_prev))
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
                    }
                });
            self.show_macd_win = open;
        }

        if self.show_vwap_win {
            if self.vwap_win_symbol.is_empty() {
                self.vwap_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_vwap_win;
            egui::Window::new("VWAP — Volume-Weighted Average Price (rolling 20-bar)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.vwap_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.vwap_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.vwap_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_vwap(&conn, &sym_u)
                                    {
                                        self.vwap_win_snapshot = snap;
                                        self.vwap_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.vwap_win_symbol.to_uppercase();
                            self.vwap_win_loading = true;
                            self.vwap_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeVwapSnapshot { symbol: sym });
                        }
                        if self.vwap_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.vwap_win_snapshot;
                    if snap.symbol.is_empty() || snap.vwap_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥20 bars with volume.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.vwap_label.as_str() {
                            "STRONG_ABOVE" | "ABOVE" => UP,
                            "STRONG_BELOW" | "BELOW" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — VWAP {:.4} — dev {:+.2}% — as of {}",
                                snap.symbol,
                                snap.vwap_label,
                                snap.vwap_value,
                                snap.deviation_pct,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("vwap_summary")
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
                                ui.label(egui::RichText::new("Window").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.window))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("VWAP value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.vwap_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Deviation %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}", snap.deviation_pct))
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
                    }
                });
            self.show_vwap_win = open;
        }

        if self.show_mcgd_win {
            if self.mcgd_win_symbol.is_empty() {
                self.mcgd_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_mcgd_win;
            egui::Window::new("MCGD — McGinley Dynamic (adaptive MA, length 14)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.mcgd_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.mcgd_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.mcgd_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_mcgd(&conn, &sym_u)
                                    {
                                        self.mcgd_win_snapshot = snap;
                                        self.mcgd_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.mcgd_win_symbol.to_uppercase();
                            self.mcgd_win_loading = true;
                            self.mcgd_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMcgdSnapshot { symbol: sym });
                        }
                        if self.mcgd_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.mcgd_win_snapshot;
                    if snap.symbol.is_empty() || snap.mcgd_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥15 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.mcgd_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — MCGD {:.4} — dev {:+.2}% — as of {}",
                                snap.symbol,
                                snap.mcgd_label,
                                snap.mcgd_value,
                                snap.deviation_pct,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("mcgd_summary")
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
                                ui.label(egui::RichText::new("MCGD value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.mcgd_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("MCGD prev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.mcgd_prev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Deviation %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}", snap.deviation_pct))
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
                    }
                });
            self.show_mcgd_win = open;
        }

        if self.show_rwi_win {
            if self.rwi_win_symbol.is_empty() {
                self.rwi_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_rwi_win;
            egui::Window::new("RWI — Poulos Random Walk Index (length 14)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 240.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.rwi_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.rwi_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.rwi_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_rwi(&conn, &sym_u)
                                    {
                                        self.rwi_win_snapshot = snap;
                                        self.rwi_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.rwi_win_symbol.to_uppercase();
                            self.rwi_win_loading = true;
                            self.rwi_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRwiSnapshot { symbol: sym });
                        }
                        if self.rwi_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.rwi_win_snapshot;
                    if snap.symbol.is_empty() || snap.rwi_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥15 bars with OHLC.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.rwi_label.as_str() {
                            "TRENDING_UP" => UP,
                            "TRENDING_DOWN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — RWI-hi {:.3} / RWI-lo {:.3} — as of {}",
                                snap.symbol,
                                snap.rwi_label,
                                snap.rwi_high,
                                snap.rwi_low,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("rwi_summary")
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
                                ui.label(egui::RichText::new("RWI high").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.rwi_high))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("RWI low").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.rwi_low))
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
                    }
                });
            self.show_rwi_win = open;
        }
    }
}
