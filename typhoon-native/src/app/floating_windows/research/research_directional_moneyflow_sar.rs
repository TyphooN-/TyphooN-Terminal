use super::*;

impl TyphooNApp {
    pub(super) fn render_research_directional_moneyflow_sar_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

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
                    super::render::render_adx_snapshot(ui, &self.adx_win_snapshot);
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
                    super::render::render_cci_snapshot(ui, &self.cci_win_snapshot);
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
                    super::render::render_cmf_snapshot(ui, &self.cmf_win_snapshot);
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
                    super::render::render_mfi_snapshot(ui, &self.mfi_win_snapshot);
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
                    super::render::render_psar_snapshot(ui, &self.psar_win_snapshot);
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
                    super::render::render_vortex_snapshot(ui, &self.vortex_win_snapshot);
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
                    super::render::render_chop_snapshot(ui, &self.chop_win_snapshot);
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
                    super::render::render_obv_snapshot(ui, &self.obv_win_snapshot);
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
                    super::render::render_trix_snapshot(ui, &self.trix_win_snapshot);
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
                    super::render::render_hma_snapshot(ui, &self.hma_win_snapshot);
                });
            self.show_hma_win = open;
        }
    }
}
