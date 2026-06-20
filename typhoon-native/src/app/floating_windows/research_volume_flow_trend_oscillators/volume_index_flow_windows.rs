use super::*;

impl TyphooNApp {
    pub(super) fn render_volume_index_flow_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
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
    }
}
