use super::*;

impl TyphooNApp {
    pub(super) fn render_research_linearreg_hilbert_phase_windows(&mut self, ctx: &egui::Context) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── egui windows ──
        if self.show_linearreg_win {
            if self.linearreg_win_symbol.is_empty() {
                self.linearreg_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_linearreg_win;
            egui::Window::new("LINEARREG — TA-Lib fitted endpoint of 14-bar least-squares close")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.linearreg_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.linearreg_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.linearreg_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_linearreg(&conn, &sym_u)
                                    {
                                        self.linearreg_win_snapshot = snap;
                                        self.linearreg_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.linearreg_win_symbol.to_uppercase();
                            self.linearreg_win_loading = true;
                            self.linearreg_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeLinearregSnapshot { symbol: sym });
                        }
                        if self.linearreg_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_linearreg_snapshot(ui, &self.linearreg_win_snapshot);
                });
            self.show_linearreg_win = open;
        }

        if self.show_linearreg_angle_win {
            if self.linearreg_angle_win_symbol.is_empty() {
                self.linearreg_angle_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_linearreg_angle_win;
            egui::Window::new("LINEARREG_ANGLE — atan(slope)·180/π of 14-bar fit")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.linearreg_angle_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.linearreg_angle_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.linearreg_angle_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_linearreg_angle(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.linearreg_angle_win_snapshot = snap;
                                        self.linearreg_angle_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.linearreg_angle_win_symbol.to_uppercase();
                            self.linearreg_angle_win_loading = true;
                            self.linearreg_angle_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeLinearregAngleSnapshot { symbol: sym });
                        }
                        if self.linearreg_angle_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_linearreg_angle_snapshot(
                        ui,
                        &self.linearreg_angle_win_snapshot,
                    );
                });
            self.show_linearreg_angle_win = open;
        }

        if self.show_ht_dcphase_win {
            if self.ht_dcphase_win_symbol.is_empty() {
                self.ht_dcphase_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ht_dcphase_win;
            egui::Window::new("HT_DCPHASE — Ehlers Hilbert Dominant Cycle Phase (degrees)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ht_dcphase_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ht_dcphase_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ht_dcphase_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ht_dcphase(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.ht_dcphase_win_snapshot = snap;
                                        self.ht_dcphase_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ht_dcphase_win_symbol.to_uppercase();
                            self.ht_dcphase_win_loading = true;
                            self.ht_dcphase_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeHtDcphaseSnapshot { symbol: sym });
                        }
                        if self.ht_dcphase_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_ht_dcphase_snapshot(ui, &self.ht_dcphase_win_snapshot);
                });
            self.show_ht_dcphase_win = open;
        }

        if self.show_ht_sine_win {
            if self.ht_sine_win_symbol.is_empty() {
                self.ht_sine_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ht_sine_win;
            egui::Window::new("HT_SINE — Ehlers Sine + Leadsine cycle-turn detector")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ht_sine_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ht_sine_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ht_sine_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ht_sine(&conn, &sym_u)
                                    {
                                        self.ht_sine_win_snapshot = snap;
                                        self.ht_sine_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ht_sine_win_symbol.to_uppercase();
                            self.ht_sine_win_loading = true;
                            self.ht_sine_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeHtSineSnapshot { symbol: sym });
                        }
                        if self.ht_sine_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_ht_sine_snapshot(ui, &self.ht_sine_win_snapshot);
                });
            self.show_ht_sine_win = open;
        }

        if self.show_ht_phasor_win {
            if self.ht_phasor_win_symbol.is_empty() {
                self.ht_phasor_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ht_phasor_win;
            egui::Window::new("HT_PHASOR — Ehlers raw I/Q + magnitude + phase")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ht_phasor_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ht_phasor_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ht_phasor_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ht_phasor(&conn, &sym_u)
                                    {
                                        self.ht_phasor_win_snapshot = snap;
                                        self.ht_phasor_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ht_phasor_win_symbol.to_uppercase();
                            self.ht_phasor_win_loading = true;
                            self.ht_phasor_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeHtPhasorSnapshot { symbol: sym });
                        }
                        if self.ht_phasor_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_ht_phasor_snapshot(ui, &self.ht_phasor_win_snapshot);
                });
            self.show_ht_phasor_win = open;
        }

        if self.show_midprice_win {
            if self.midprice_win_symbol.is_empty() {
                self.midprice_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_midprice_win;
            egui::Window::new("MIDPRICE — (HHV + LLV) / 2 range midpoint (14-bar)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.midprice_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.midprice_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.midprice_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_midprice(&conn, &sym_u)
                                    {
                                        self.midprice_win_snapshot = snap;
                                        self.midprice_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.midprice_win_symbol.to_uppercase();
                            self.midprice_win_loading = true;
                            self.midprice_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMidpriceSnapshot { symbol: sym });
                        }
                        if self.midprice_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_midprice_snapshot(ui, &self.midprice_win_snapshot);
                });
            self.show_midprice_win = open;
        }

        if self.show_apo_win {
            if self.apo_win_symbol.is_empty() {
                self.apo_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_apo_win;
            egui::Window::new("APO — Absolute Price Oscillator (EMA12 − EMA26)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.apo_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.apo_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.apo_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_apo(&conn, &sym_u)
                                    {
                                        self.apo_win_snapshot = snap;
                                        self.apo_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.apo_win_symbol.to_uppercase();
                            self.apo_win_loading = true;
                            self.apo_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeApoSnapshot { symbol: sym });
                        }
                        if self.apo_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_apo_snapshot(ui, &self.apo_win_snapshot);
                });
            self.show_apo_win = open;
        }

        if self.show_mom_win {
            if self.mom_win_symbol.is_empty() {
                self.mom_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_mom_win;
            egui::Window::new("MOM — raw close − close[n−10] momentum")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.mom_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.mom_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.mom_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_mom(&conn, &sym_u)
                                    {
                                        self.mom_win_snapshot = snap;
                                        self.mom_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.mom_win_symbol.to_uppercase();
                            self.mom_win_loading = true;
                            self.mom_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMomSnapshot { symbol: sym });
                        }
                        if self.mom_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_mom_snapshot(ui, &self.mom_win_snapshot);
                });
            self.show_mom_win = open;
        }

        if self.show_sarext_win {
            if self.sarext_win_symbol.is_empty() {
                self.sarext_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_sarext_win;
            egui::Window::new("SAREXT — Extended Parabolic SAR (asymmetric long/short AF)")
                .open(&mut open)
                .resizable(true)
                .default_size([620.0, 320.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.sarext_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.sarext_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.sarext_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_sarext(&conn, &sym_u)
                                    {
                                        self.sarext_win_snapshot = snap;
                                        self.sarext_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.sarext_win_symbol.to_uppercase();
                            self.sarext_win_loading = true;
                            self.sarext_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeSarextSnapshot { symbol: sym });
                        }
                        if self.sarext_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_sarext_snapshot(ui, &self.sarext_win_snapshot);
                });
            self.show_sarext_win = open;
        }

        if self.show_adxr_win {
            if self.adxr_win_symbol.is_empty() {
                self.adxr_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_adxr_win;
            egui::Window::new("ADXR — Average Directional Movement Rating (14-bar)")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.adxr_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.adxr_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.adxr_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_adxr(&conn, &sym_u)
                                    {
                                        self.adxr_win_snapshot = snap;
                                        self.adxr_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.adxr_win_symbol.to_uppercase();
                            self.adxr_win_loading = true;
                            self.adxr_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeAdxrSnapshot { symbol: sym });
                        }
                        if self.adxr_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_adxr_snapshot(ui, &self.adxr_win_snapshot);
                });
            self.show_adxr_win = open;
        }
    }
}
