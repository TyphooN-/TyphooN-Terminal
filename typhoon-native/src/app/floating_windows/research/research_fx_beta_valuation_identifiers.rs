use super::*;

impl TyphooNApp {
    pub(super) fn render_research_fx_beta_valuation_identifiers_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // FX, beta, valuation, and identifier research
        // WCR — World Currency Rates (FX majors + crosses + EM via Yahoo /v7)
        if self.show_wcr {
            let mut open = self.show_wcr;
            egui::Window::new("WCR — World Currency Rates")
                .open(&mut open)
                .resizable(true)
                .default_size([620.0, 480.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new("Fetch").fill(BTN_MG)).clicked() {
                            self.wcr_loading = true;
                            let _ = self.broker_tx.send(BrokerCmd::FetchCurrencyRates);
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    if let Ok(Some(rows)) =
                                        typhoon_engine::core::research::get_currency_rates(&conn)
                                    {
                                        self.wcr_rates = rows;
                                    }
                                }
                            }
                        }
                        ui.separator();
                        ui.label(egui::RichText::new("Region:").color(AXIS_TEXT));
                        egui::ComboBox::from_id_salt("wcr_region_filter")
                            .selected_text(if self.wcr_region_filter.is_empty() {
                                "All".to_string()
                            } else {
                                self.wcr_region_filter.clone()
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.wcr_region_filter,
                                    String::new(),
                                    "All",
                                );
                                ui.selectable_value(
                                    &mut self.wcr_region_filter,
                                    "Majors".into(),
                                    "Majors",
                                );
                                ui.selectable_value(
                                    &mut self.wcr_region_filter,
                                    "Crosses".into(),
                                    "Crosses",
                                );
                                ui.selectable_value(&mut self.wcr_region_filter, "EM".into(), "EM");
                            });
                        if self.wcr_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    if self.wcr_rates.is_empty() {
                        ui.label(
                            egui::RichText::new("No data — click Fetch to pull Yahoo FX quotes.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            egui::Grid::new("wcr_grid")
                                .striped(true)
                                .num_columns(5)
                                .min_col_width(80.0)
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new("Pair")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Region")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Price")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Change")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("%").color(AXIS_TEXT).small().strong(),
                                    );
                                    ui.end_row();
                                    for r in &self.wcr_rates {
                                        if !self.wcr_region_filter.is_empty()
                                            && r.region != self.wcr_region_filter
                                        {
                                            continue;
                                        }
                                        let color = if r.change_pct > 0.0 {
                                            UP
                                        } else if r.change_pct < 0.0 {
                                            DOWN
                                        } else {
                                            AXIS_TEXT
                                        };
                                        ui.label(
                                            egui::RichText::new(&r.display)
                                                .small()
                                                .monospace()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new(&r.region).color(AXIS_TEXT).small(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{:.4}", r.price))
                                                .small()
                                                .monospace(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{:+.4}", r.change))
                                                .color(color)
                                                .small()
                                                .monospace(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{:+.2}%", r.change_pct))
                                                .color(color)
                                                .small()
                                                .monospace(),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });
                    }
                });
            self.show_wcr = open;
        }

        // BETA — rolling beta history (1Y/3Y/5Y vs SPY)
        if self.show_beta {
            if self.beta_symbol.is_empty() {
                self.beta_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_beta;
            egui::Window::new("BETA — Rolling Beta vs SPY")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 360.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.beta_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.beta_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.beta_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_beta(&conn, &sym_u)
                                    {
                                        self.beta_snapshot = snap;
                                        self.beta_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        let have_key = !self.fmp_key.is_empty();
                        if ui
                            .add_enabled(have_key, egui::Button::new("Fetch").fill(BTN_MG))
                            .clicked()
                        {
                            let sym = self.beta_symbol.to_uppercase();
                            self.beta_loading = true;
                            self.beta_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::FetchBetaSnapshot {
                                symbol: sym,
                                fmp_key: self.fmp_key.clone(),
                            });
                        }
                        if self.fmp_key.is_empty() {
                            ui.label(
                                egui::RichText::new("(add FMP key in Settings)")
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                        }
                        if self.beta_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_beta_snapshot(ui, &self.beta_snapshot);
                });
            self.show_beta = open;
        }

        // DDM — Gordon Growth Dividend Discount Model
        if let Some(sym) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "DDM — Gordon Growth Dividend Discount",
                default_size: [540.0, 420.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_ddm,
            &mut self.ddm_symbol,
            &mut self.ddm_loading,
            &mut self.ddm_snapshot,
            |conn, s| {
                typhoon_engine::core::research::get_ddm(conn, s)
                    .ok()
                    .flatten()
            },
            |symbol| symbol,
            super::render::render_ddm_snapshot,
        ) {
            let wacc_snap = &self.wacc_snapshot;
            let (r, src) =
                if wacc_snap.symbol.eq_ignore_ascii_case(&sym) && wacc_snap.wacc_pct > 0.0 {
                    (
                        wacc_snap.cost_of_equity_pct,
                        format!(
                            "Cost of equity {:.2}% (from WACC)",
                            wacc_snap.cost_of_equity_pct
                        ),
                    )
                } else {
                    (10.0, "default required return 10.0%".to_string())
                };
            let _ = self.broker_tx.send(BrokerCmd::ComputeDdmSnapshot {
                symbol: sym,
                required_return_pct: r,
                return_source: src,
            });
        }

        // RV — Relative Valuation (peer matrix)
        if self.show_rv {
            if self.rv_symbol.is_empty() {
                self.rv_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_rv;
            egui::Window::new("RV — Relative Valuation Matrix")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.rv_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.rv_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.rv_symbol.to_uppercase();
                                    if let Ok(Some(rv)) = typhoon_engine::core::research::get_relative_valuation(&conn, &sym_u) {
                                        self.rv_snapshot = rv;
                                        self.rv_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.rv_symbol.to_uppercase();
                            self.rv_loading = true;
                            self.rv_symbol = sym.clone();
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    if let Ok(Some(self_fund)) = typhoon_engine::core::fundamentals::get_fundamentals(&conn, &sym) {
                                        let sector = self_fund.sector.clone();
                                        let self_json = serde_json::to_string(&self_fund).unwrap_or_default();
                                        let peer_syms = typhoon_engine::core::research::get_peers(&conn, &sym)
                                            .unwrap_or(None).unwrap_or_default();
                                        let mut peers_list: Vec<typhoon_engine::core::fundamentals::Fundamentals> = Vec::new();
                                        for p in &peer_syms {
                                            if p.eq_ignore_ascii_case(&sym) { continue; }
                                            if let Ok(Some(pf)) = typhoon_engine::core::fundamentals::get_fundamentals(&conn, p) {
                                                peers_list.push(pf);
                                            }
                                        }
                                        let peers_json = serde_json::to_string(&peers_list).unwrap_or_default();
                                        let _ = self.broker_tx.send(BrokerCmd::ComputeRelativeValuation {
                                            symbol: sym, sector, self_json, peers_json,
                                        });
                                    }
                                }
                            }
                        }
                        if self.rv_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let rv = &self.rv_snapshot;
                    if rv.symbol.is_empty() || rv.rows.is_empty() {
                        ui.label(egui::RichText::new("No data — run DES/PEERS for this symbol + sector peers, then click Compute.")
                            .color(AXIS_TEXT).small());
                    } else {
                        ui.label(egui::RichText::new(format!("{} — sector {} — {} peers — as of {}",
                            rv.symbol, rv.sector, rv.peer_count, rv.as_of)).strong().color(AXIS_TEXT));
                        ui.separator();
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            egui::Grid::new("rv_grid").striped(true).num_columns(7).min_col_width(70.0).show(ui, |ui| {
                                ui.label(egui::RichText::new("Metric").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Value").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Peer median").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Peer low").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Peer high").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Z").color(AXIS_TEXT).small().strong());
                                ui.label(egui::RichText::new("Percentile").color(AXIS_TEXT).small().strong());
                                ui.end_row();
                                for r in &rv.rows {
                                    let z_color = if r.z_score.abs() > 1.5 { DOWN } else if r.z_score.abs() > 0.5 { AXIS_TEXT } else { UP };
                                    ui.label(egui::RichText::new(&r.metric).small().monospace().strong());
                                    ui.label(egui::RichText::new(format!("{:.2}", r.value)).small().monospace());
                                    ui.label(egui::RichText::new(format!("{:.2}", r.peer_median)).small().monospace());
                                    ui.label(egui::RichText::new(format!("{:.2}", r.peer_low)).small().monospace());
                                    ui.label(egui::RichText::new(format!("{:.2}", r.peer_high)).small().monospace());
                                    ui.label(egui::RichText::new(format!("{:+.2}", r.z_score)).color(z_color).small().monospace());
                                    ui.label(egui::RichText::new(format!("{:.0}%", r.percentile)).small().monospace());
                                    ui.end_row();
                                }
                            });
                        });
                    }
                });
            self.show_rv = open;
        }

        // FIGI — OpenFIGI instrument identifier lookup
        if self.show_figi {
            if self.figi_symbol.is_empty() {
                self.figi_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_figi;
            egui::Window::new("FIGI — OpenFIGI Instrument Identifiers")
                .open(&mut open)
                .resizable(true)
                .default_size([620.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Ticker:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.figi_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.figi_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.figi_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_figi(&conn, &sym_u)
                                    {
                                        self.figi_snapshot = snap;
                                        self.figi_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Lookup").fill(BTN_MG)).clicked() {
                            let sym = self.figi_symbol.to_uppercase();
                            self.figi_loading = true;
                            self.figi_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::FetchFigiIdentifiers { symbol: sym });
                        }
                        if self.figi_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_figi_snapshot(ui, &self.figi_snapshot);
                });
            self.show_figi = open;
        }
    }
}
