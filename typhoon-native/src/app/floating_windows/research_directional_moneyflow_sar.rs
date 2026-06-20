use super::*;

impl TyphooNApp {
    pub(super) fn render_research_directional_moneyflow_sar_windows(
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
        if self.show_adx_win {
            if self.adx_win_symbol.is_empty() {
                self.adx_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_adx_win;
            egui::Window::new("ADX — Wilder's Directional Index (14)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.adx_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.adx_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.adx_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_adx(&conn, &sym_u)
                                    {
                                        self.adx_win_snapshot = snap;
                                        self.adx_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.adx_win_symbol.to_uppercase();
                            self.adx_win_loading = true;
                            self.adx_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeAdxSnapshot { symbol: sym });
                        }
                        if self.adx_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.adx_win_snapshot;
                    if snap.symbol.is_empty() || snap.adx_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥29 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.adx_label.as_str() {
                            "STRONG_TREND" | "TREND" => UP,
                            "NO_TREND" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — ADX {:.2} — +DI {:.2} — −DI {:.2} — as of {}",
                                snap.symbol,
                                snap.adx_label,
                                snap.adx,
                                snap.plus_di,
                                snap.minus_di,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("adx_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(180.0)
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
                                ui.label(egui::RichText::new("+DI").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.plus_di))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("−DI").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.minus_di))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("DX").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.dx))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ADX").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.adx))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("ATR").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.atr))
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
            self.show_adx_win = open;
        }

        if self.show_cci_win {
            if self.cci_win_symbol.is_empty() {
                self.cci_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cci_win;
            egui::Window::new("CCI — Commodity Channel Index (20)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cci_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.cci_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.cci_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_cci(&conn, &sym_u)
                                    {
                                        self.cci_win_snapshot = snap;
                                        self.cci_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cci_win_symbol.to_uppercase();
                            self.cci_win_loading = true;
                            self.cci_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCciSnapshot { symbol: sym });
                        }
                        if self.cci_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.cci_win_snapshot;
                    if snap.symbol.is_empty() || snap.cci_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥20 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.cci_label.as_str() {
                            "OVERBOUGHT" => DOWN,
                            "OVERSOLD" => UP,
                            "BULL" => UP,
                            "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — CCI {:+.2} — tp {:.4} — as of {}",
                                snap.symbol,
                                snap.cci_label,
                                snap.cci_value,
                                snap.typical_price,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("cci_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(180.0)
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
                                ui.label(egui::RichText::new("Typical price").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.typical_price))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("TP SMA").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.tp_sma))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Mean abs dev").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.mean_abs_dev))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("CCI value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}", snap.cci_value))
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
            self.show_cci_win = open;
        }

        if self.show_cmf_win {
            if self.cmf_win_symbol.is_empty() {
                self.cmf_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_cmf_win;
            egui::Window::new("CMF — Chaikin Money Flow (20)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cmf_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.cmf_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.cmf_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_cmf(&conn, &sym_u)
                                    {
                                        self.cmf_win_snapshot = snap;
                                        self.cmf_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.cmf_win_symbol.to_uppercase();
                            self.cmf_win_loading = true;
                            self.cmf_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCmfSnapshot { symbol: sym });
                        }
                        if self.cmf_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.cmf_win_snapshot;
                    if snap.symbol.is_empty() || snap.cmf_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥20 bars with volume.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.cmf_label.as_str() {
                            "STRONG_ACCUM" | "ACCUM" => UP,
                            "STRONG_DIST" | "DIST" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — CMF {:+.3} — vol {:.0} — as of {}",
                                snap.symbol,
                                snap.cmf_label,
                                snap.cmf_value,
                                snap.volume_sum,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("cmf_summary")
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
                                ui.label(egui::RichText::new("CMF value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.cmf_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Σ MFV").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.0}",
                                        snap.money_flow_volume_sum
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Σ volume").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.0}", snap.volume_sum))
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
            self.show_cmf_win = open;
        }

        if self.show_mfi_win {
            if self.mfi_win_symbol.is_empty() {
                self.mfi_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_mfi_win;
            egui::Window::new("MFI — Money Flow Index (14)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.mfi_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.mfi_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.mfi_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_mfi(&conn, &sym_u)
                                    {
                                        self.mfi_win_snapshot = snap;
                                        self.mfi_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.mfi_win_symbol.to_uppercase();
                            self.mfi_win_loading = true;
                            self.mfi_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMfiSnapshot { symbol: sym });
                        }
                        if self.mfi_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.mfi_win_snapshot;
                    if snap.symbol.is_empty() || snap.mfi_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥15 bars with volume.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.mfi_label.as_str() {
                            "OVERBOUGHT" => DOWN,
                            "OVERSOLD" => UP,
                            "BULL" => UP,
                            "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — MFI {:.2} — ratio {:.3} — as of {}",
                                snap.symbol,
                                snap.mfi_label,
                                snap.mfi_value,
                                snap.money_flow_ratio,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("mfi_summary")
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
                                ui.label(egui::RichText::new("MFI value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.mfi_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("+MF sum").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.0}", snap.positive_mf_sum))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("−MF sum").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.0}", snap.negative_mf_sum))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("MF ratio").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.money_flow_ratio))
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
            self.show_mfi_win = open;
        }

        if self.show_psar_win {
            if self.psar_win_symbol.is_empty() {
                self.psar_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_psar_win;
            egui::Window::new("PSAR — Parabolic Stop-And-Reverse")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.psar_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.psar_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.psar_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_psar(&conn, &sym_u)
                                    {
                                        self.psar_win_snapshot = snap;
                                        self.psar_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.psar_win_symbol.to_uppercase();
                            self.psar_win_loading = true;
                            self.psar_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputePsarSnapshot { symbol: sym });
                        }
                        if self.psar_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.psar_win_snapshot;
                    if snap.symbol.is_empty() || snap.psar_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥4 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.psar_label.as_str() {
                            "STRONG_UP" | "UP" => UP,
                            "STRONG_DOWN" | "DOWN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — SAR {:.4} — dist {:+.2}% — bars {} — as of {}",
                                snap.symbol,
                                snap.psar_label,
                                snap.sar_value,
                                snap.distance_pct,
                                snap.bars_in_trend,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("psar_summary")
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
                                ui.label(
                                    egui::RichText::new("AF start / step / max")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2} / {:.2} / {:.2}",
                                        snap.af_start, snap.af_step, snap.af_max
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Current AF").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.acceleration_factor))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("SAR value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sar_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Extreme point").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.extreme_point))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Trend").small().strong());
                                ui.label(
                                    egui::RichText::new(if snap.trend_is_up {
                                        "UP"
                                    } else {
                                        "DOWN"
                                    })
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Bars in trend").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.bars_in_trend))
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
            self.show_psar_win = open;
        }

        if self.show_vortex_win {
            if self.vortex_win_symbol.is_empty() {
                self.vortex_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_vortex_win;
            egui::Window::new("VORTEX — Vortex Indicator (14)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.vortex_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.vortex_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.vortex_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_vortex(&conn, &sym_u)
                                    {
                                        self.vortex_win_snapshot = snap;
                                        self.vortex_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.vortex_win_symbol.to_uppercase();
                            self.vortex_win_loading = true;
                            self.vortex_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeVortexSnapshot { symbol: sym });
                        }
                        if self.vortex_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.vortex_win_snapshot;
                    if snap.symbol.is_empty() || snap.vortex_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥15 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.vortex_label.as_str() {
                            "BULL_CROSS" | "BULL" => UP,
                            "BEAR_CROSS" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — VI+ {:.3} / VI− {:.3} — Δ {:+.3} — as of {}",
                                snap.symbol,
                                snap.vortex_label,
                                snap.vi_plus,
                                snap.vi_minus,
                                snap.vi_diff,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("vortex_summary")
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
                                ui.label(egui::RichText::new("VI+").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.vi_plus))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("VI−").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.vi_minus))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("VI diff").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.vi_diff))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Σ TR").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sum_tr))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Σ VM+").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sum_vm_plus))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Σ VM−").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sum_vm_minus))
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
            self.show_vortex_win = open;
        }

        if self.show_chop_win {
            if self.chop_win_symbol.is_empty() {
                self.chop_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_chop_win;
            egui::Window::new("CHOP — Choppiness Index (14)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.chop_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.chop_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.chop_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_chop(&conn, &sym_u)
                                    {
                                        self.chop_win_snapshot = snap;
                                        self.chop_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.chop_win_symbol.to_uppercase();
                            self.chop_win_loading = true;
                            self.chop_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeChopSnapshot { symbol: sym });
                        }
                        if self.chop_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.chop_win_snapshot;
                    if snap.symbol.is_empty() || snap.chop_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥15 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.chop_label.as_str() {
                            "TRENDING" => UP,
                            "CHOP" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — CI {:.2} — range {:.4} — as of {}",
                                snap.symbol,
                                snap.chop_label,
                                snap.chop_value,
                                snap.range_span,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("chop_summary")
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
                                ui.label(egui::RichText::new("CI value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.2}", snap.chop_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Σ TR").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.sum_tr))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Range high").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.range_high))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Range low").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.range_low))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Range span").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.range_span))
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
            self.show_chop_win = open;
        }

        if self.show_obv_win {
            if self.obv_win_symbol.is_empty() {
                self.obv_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_obv_win;
            egui::Window::new("OBV — On-Balance Volume (20-bar slope)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.obv_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.obv_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.obv_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_obv(&conn, &sym_u)
                                    {
                                        self.obv_win_snapshot = snap;
                                        self.obv_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.obv_win_symbol.to_uppercase();
                            self.obv_win_loading = true;
                            self.obv_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeObvSnapshot { symbol: sym });
                        }
                        if self.obv_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.obv_win_snapshot;
                    if snap.symbol.is_empty() || snap.obv_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥21 bars with volume.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.obv_label.as_str() {
                            "STRONG_UP" | "UP" => UP,
                            "STRONG_DOWN" | "DOWN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — OBV {:.0} — Δ {:+.2}% — as of {}",
                                snap.symbol,
                                snap.obv_label,
                                snap.obv_value,
                                snap.obv_change_pct,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("obv_summary")
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
                                ui.label(egui::RichText::new("Slope window").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.slope_window))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("OBV value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.0}", snap.obv_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("20-bar slope").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}", snap.obv_slope))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("20-bar change").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}%", snap.obv_change_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("20-bar min").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.0}", snap.obv_min_20))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("20-bar max").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.0}", snap.obv_max_20))
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
            self.show_obv_win = open;
        }

        if self.show_trix_win {
            if self.trix_win_symbol.is_empty() {
                self.trix_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_trix_win;
            egui::Window::new("TRIX — Triple-EMA Oscillator (15/9)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.trix_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.trix_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.trix_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_trix(&conn, &sym_u)
                                    {
                                        self.trix_win_snapshot = snap;
                                        self.trix_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.trix_win_symbol.to_uppercase();
                            self.trix_win_loading = true;
                            self.trix_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeTrixSnapshot { symbol: sym });
                        }
                        if self.trix_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.trix_win_snapshot;
                    if snap.symbol.is_empty() || snap.trix_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥55 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.trix_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "STRONG_BEAR" | "BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — TRIX {:+.4} — signal {:+.4} — hist {:+.4} — as of {}",
                                snap.symbol,
                                snap.trix_label,
                                snap.trix_value,
                                snap.signal_value,
                                snap.histogram,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("trix_summary")
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
                                ui.label(egui::RichText::new("Signal period").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.signal_period))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("TRIX %Δ").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.4}", snap.trix_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Signal").small().strong());
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
                                ui.label(egui::RichText::new("EMA³ level").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.ema3_value))
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
            self.show_trix_win = open;
        }

        if self.show_hma_win {
            if self.hma_win_symbol.is_empty() {
                self.hma_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_hma_win;
            egui::Window::new("HMA — Hull Moving Average (20)")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.hma_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.hma_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.hma_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_hma(&conn, &sym_u)
                                    {
                                        self.hma_win_snapshot = snap;
                                        self.hma_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.hma_win_symbol.to_uppercase();
                            self.hma_win_loading = true;
                            self.hma_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeHmaSnapshot { symbol: sym });
                        }
                        if self.hma_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.hma_win_snapshot;
                    if snap.symbol.is_empty() || snap.hma_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥29 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.hma_label.as_str() {
                            "STRONG_UP" | "UP" => UP,
                            "STRONG_DOWN" | "DOWN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — HMA {:.4} — slope {:+.2}% — vs close {:+.2}% — as of {}",
                                snap.symbol,
                                snap.hma_label,
                                snap.hma_value,
                                snap.hma_slope_pct,
                                snap.hma_vs_close_pct,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("hma_summary")
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
                                ui.label(egui::RichText::new("Period / half / √").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {} / {}",
                                        snap.period, snap.half_period, snap.sqrt_period
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("HMA value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.hma_value))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("5-bar slope %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}%", snap.hma_slope_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Close vs HMA %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.3}%", snap.hma_vs_close_pct))
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
            self.show_hma_win = open;
        }
    }
}
