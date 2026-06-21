use super::*;

impl TyphooNApp {
    pub(super) fn render_volatility_drawdown_momentum_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
        // ATRANN — Annualized ATR Volatility Regime
        if self.show_atrann {
            if self.atrann_symbol.is_empty() {
                self.atrann_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_atrann;
            egui::Window::new("ATRANN — Annualized ATR")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.atrann_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.atrann_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.atrann_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_atrann(&conn, &sym_u) {
                                        self.atrann_snapshot = snap;
                                        self.atrann_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.atrann_symbol.to_uppercase();
                            self.atrann_loading = true;
                            self.atrann_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeAtrannSnapshot { symbol: sym });
                        }
                        if self.atrann_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.atrann_snapshot;
                    if snap.symbol.is_empty() || snap.regime_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — needs ≥15 cached daily bars for the subject.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.regime_label.as_str() {
                            "LOW_VOL" => UP,
                            "NORMAL_VOL" => AXIS_TEXT,
                            "HIGH_VOL" | "EXTREME_VOL" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — ATR14 {:.4} ({:.2}%) — annualized {:.2}% — {} bars — as of {}",
                            snap.symbol, snap.regime_label, snap.atr14, snap.atr14_pct,
                            snap.atr_annualized_pct, snap.bars_used, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("atrann_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Latest close").small().strong());
                            ui.label(egui::RichText::new(format!("{:.4}", snap.latest_close)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("ATR14 (price units)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.4}", snap.atr14)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("ATR14 %").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}%", snap.atr14_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Annualized (×√252)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}%", snap.atr_annualized_pct)).small().monospace());
                            ui.end_row();
                        });
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
                        }
                    }
                });
            self.show_atrann = open;
        }

        // DDHIST — Drawdown History
        if self.show_ddhist {
            if self.ddhist_symbol.is_empty() {
                self.ddhist_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_ddhist;
            egui::Window::new("DDHIST — Drawdown History")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ddhist_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.ddhist_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.ddhist_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_ddhist(&conn, &sym_u)
                                    {
                                        self.ddhist_snapshot = snap;
                                        self.ddhist_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.ddhist_symbol.to_uppercase();
                            self.ddhist_loading = true;
                            self.ddhist_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeDdhistSnapshot { symbol: sym });
                        }
                        if self.ddhist_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.ddhist_snapshot;
                    if snap.symbol.is_empty() || snap.regime_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new(
                                "No data — needs ≥20 cached daily bars for the subject.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.regime_label.as_str() {
                            "RECOVERING" | "SHALLOW" => UP,
                            "MEANINGFUL" => AXIS_TEXT,
                            "SEVERE" | "CATASTROPHIC" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — max dd {:+.2}% — current dd {:+.2}% — {} bars — as of {}",
                            snap.symbol, snap.regime_label, snap.max_drawdown_pct,
                            snap.current_drawdown_pct, snap.bars_used, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("ddhist_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Max drawdown %").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}%", snap.max_drawdown_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Max dd peak / trough date")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {}",
                                        snap.max_drawdown_peak_date, snap.max_drawdown_trough_date
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Longest drawdown (sessions)")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.longest_drawdown_days))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Corrections ≥5% / ≥10%")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {}",
                                        snap.corrections_5pct, snap.corrections_10pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Current drawdown %").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.2}%",
                                        snap.current_drawdown_pct
                                    ))
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
            self.show_ddhist = open;
        }

        // PRICEPERF — Multi-horizon Price Performance
        if self.show_priceperf {
            if self.priceperf_symbol.is_empty() {
                self.priceperf_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_priceperf;
            egui::Window::new("PRICEPERF — Price Performance")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.priceperf_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.priceperf_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.priceperf_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_priceperf(&conn, &sym_u)
                                    {
                                        self.priceperf_snapshot = snap;
                                        self.priceperf_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.priceperf_symbol.to_uppercase();
                            self.priceperf_loading = true;
                            self.priceperf_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputePriceperfSnapshot { symbol: sym });
                        }
                        if self.priceperf_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.priceperf_snapshot;
                    if snap.symbol.is_empty() || snap.trend_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new(
                                "No data — needs ≥2 cached daily bars (≥20 for trend label).",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.trend_label.as_str() {
                            "STRONG_BULL" | "BULL" => UP,
                            "NEUTRAL" => AXIS_TEXT,
                            "BEAR" | "STRONG_BEAR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — close {:.2} — 1Y {:+.2}% — YTD {:+.2}% — {} bars — as of {}",
                            snap.symbol, snap.trend_label, snap.latest_close,
                            snap.ret_1y_pct, snap.ret_ytd_pct, snap.bars_used, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("priceperf_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("1M / 3M / 6M return").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.2}% / {:+.2}% / {:+.2}%",
                                        snap.ret_1m_pct, snap.ret_3m_pct, snap.ret_6m_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("YTD / 1Y return").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.2}% / {:+.2}%",
                                        snap.ret_ytd_pct, snap.ret_1y_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Bars used").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.bars_used))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Latest close").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.latest_close))
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
            self.show_priceperf = open;
        }

        // MOMRANK_MULTI — sector-relative PRICEPERF rank
        if self.show_momrank_multi {
            if self.momrank_multi_symbol.is_empty() {
                self.momrank_multi_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_momrank_multi;
            egui::Window::new("MOMRANK_MULTI — Sector-Relative Momentum Rank")
                .open(&mut open)
                .resizable(true)
                .default_size([660.0, 460.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.momrank_multi_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.momrank_multi_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.momrank_multi_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_momrank_multi(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.momrank_multi_snapshot = snap;
                                        self.momrank_multi_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.momrank_multi_symbol.to_uppercase();
                            self.momrank_multi_loading = true;
                            self.momrank_multi_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeMomrankMultiSnapshot { symbol: sym });
                        }
                        if self.momrank_multi_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.momrank_multi_snapshot;
                    if snap.symbol.is_empty()
                        || snap.rank_label == "NO_DATA"
                        || snap.rank_label == "INSUFFICIENT_DATA"
                    {
                        ui.label(
                            egui::RichText::new(
                                "No data — needs a cached PRICEPERF snapshot on the subject and ≥3 sector peers with PRICEPERF.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.rank_label.as_str() {
                            "TOP_DECILE" | "TOP_QUARTILE" | "ABOVE_MEDIAN" => UP,
                            "BOTTOM_QUARTILE" | "BOTTOM_DECILE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — sector {} — composite pct {:.1} — rank {}/{} — as of {}",
                                snap.symbol,
                                snap.rank_label,
                                snap.sector,
                                snap.composite_percentile,
                                snap.rank_position,
                                snap.peers_with_data + 1,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("momrank_multi_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("1M / 3M / 6M return").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.2}% / {:+.2}% / {:+.2}%",
                                        snap.ret_1m_pct, snap.ret_3m_pct, snap.ret_6m_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("YTD / 1Y return").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.2}% / {:+.2}%",
                                        snap.ret_ytd_pct, snap.ret_1y_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("1M / 3M / 6M pct").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.1} / {:.1} / {:.1}",
                                        snap.pct_1m, snap.pct_3m, snap.pct_6m
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("YTD / 1Y pct").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.1} / {:.1}",
                                        snap.pct_ytd, snap.pct_1y
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Peers with data / above median")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {} horizons",
                                        snap.peers_with_data, snap.horizons_above_median
                                    ))
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
            self.show_momrank_multi = open;
        }

        // BETARANK — Beta rank vs sector peers (risk-inverted)
    }
}
