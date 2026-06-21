use super::*;

impl TyphooNApp {
    pub(super) fn render_volatility_correlation_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
        // RVCONE — Realized Volatility Cone (multi-horizon)
        if self.show_rvcone {
            if self.rvcone_symbol.is_empty() {
                self.rvcone_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_rvcone;
            egui::Window::new("RVCONE — Realized Volatility Cone")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.rvcone_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.rvcone_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.rvcone_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_rvcone(&conn, &sym_u)
                                    {
                                        self.rvcone_snapshot = snap;
                                        self.rvcone_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.rvcone_symbol.to_uppercase();
                            self.rvcone_loading = true;
                            self.rvcone_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeRvconeSnapshot { symbol: sym });
                        }
                        if self.rvcone_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.rvcone_snapshot;
                    if snap.symbol.is_empty() || snap.cone_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — needs ≥21 cached daily bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.cone_label.as_str() {
                            "COMPRESSED" | "BELOW_AVG" => UP,
                            "ELEVATED" | "EXTREME" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — 20d RV {:.1}% — pct {:.1} — {} bars — as of {}",
                                snap.symbol,
                                snap.cone_label,
                                snap.rv20_pct,
                                snap.rv20_percentile,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("rvcone_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("20d / 60d / 120d / 252d RV")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.1}% / {:.1}% / {:.1}% / {:.1}%",
                                        snap.rv20_pct,
                                        snap.rv60_pct,
                                        snap.rv120_pct,
                                        snap.rv252_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("20d rolling min / median / max")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.1}% / {:.1}% / {:.1}%",
                                        snap.rv20_min_pct, snap.rv20_median_pct, snap.rv20_max_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Latest 20d percentile")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.1}", snap.rv20_percentile))
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
            self.show_rvcone = open;
        }

        // CALPB — Calendar Period Breakdowns
        if self.show_calpb {
            if self.calpb_symbol.is_empty() {
                self.calpb_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_calpb;
            egui::Window::new("CALPB — Calendar Period Breakdowns")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.calpb_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.calpb_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.calpb_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_calpb(&conn, &sym_u)
                                    {
                                        self.calpb_snapshot = snap;
                                        self.calpb_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.calpb_symbol.to_uppercase();
                            self.calpb_loading = true;
                            self.calpb_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCalpbSnapshot { symbol: sym });
                        }
                        if self.calpb_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.calpb_snapshot;
                    if snap.symbol.is_empty() || snap.momentum_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — needs ≥20 cached daily bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.momentum_label.as_str() {
                            "ACCELERATING" => UP,
                            "DECELERATING" | "REVERSING" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — {} {} — close {:.2} — QTD {:+.2}% — as of {}",
                                snap.symbol,
                                snap.momentum_label,
                                snap.current_year,
                                snap.current_quarter,
                                snap.latest_close,
                                snap.qtd_pct,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("calpb_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("MTD / QTD / YTD").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.2}% / {:+.2}% / {:+.2}%",
                                        snap.mtd_pct, snap.qtd_pct, snap.ytd_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Prior quarter / prior year")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.2}% / {:+.2}%",
                                        snap.prior_quarter_pct, snap.prior_year_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Current period").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} {}",
                                        snap.current_year, snap.current_quarter
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
                            });
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
                        }
                    }
                });
            self.show_calpb = open;
        }

        // CORRSTK — rolling correlation vs SPY / sector ETF
        if self.show_corrstk {
            if self.corrstk_symbol.is_empty() {
                self.corrstk_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_corrstk;
            egui::Window::new("CORRSTK — Benchmark Correlation")
                .open(&mut open)
                .resizable(true)
                .default_size([680.0, 470.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.corrstk_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.corrstk_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.corrstk_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_corrstk(&conn, &sym_u)
                                    {
                                        self.corrstk_snapshot = snap;
                                        self.corrstk_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.corrstk_symbol.to_uppercase();
                            let symbol_sector = if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    typhoon_engine::core::fundamentals::get_fundamentals(
                                        &conn, &sym,
                                    )
                                    .ok()
                                    .flatten()
                                    .map(|f| f.sector)
                                    .unwrap_or_default()
                                } else {
                                    String::new()
                                }
                            } else {
                                String::new()
                            };
                            self.corrstk_loading = true;
                            self.corrstk_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeCorrstkSnapshot {
                                symbol: sym,
                                symbol_sector,
                                fmp_key: self.fmp_key.clone(),
                            });
                        }
                        if self.corrstk_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.corrstk_snapshot;
                    if snap.symbol.is_empty() || snap.correlation_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new(
                                "No data — needs overlapping daily bars for the symbol and at least one benchmark (SPY or sector ETF).",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.correlation_label.as_str() {
                            "INDEX_LOCKSTEP" | "SECTOR_LOCKSTEP" => UP,
                            "INVERSE_INDEX" | "INVERSE_SECTOR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — dominant {} — SPY 252d {:.2} — sector 252d {:.2} — as of {}",
                                snap.symbol,
                                snap.correlation_label,
                                snap.dominant_benchmark,
                                snap.corr_spy_252d,
                                snap.corr_sector_252d,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("corrstk_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(230.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Benchmarks").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {}",
                                        snap.market_benchmark,
                                        if snap.sector_benchmark.is_empty() {
                                            "n/a"
                                        } else {
                                            snap.sector_benchmark.as_str()
                                        }
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("SPY corr 20 / 60 / 252").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2} / {:.2} / {:.2}",
                                        snap.corr_spy_20d, snap.corr_spy_60d, snap.corr_spy_252d
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("SPY β / R² / overlap").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2} / {:.2} / {}",
                                        snap.beta_spy_252d,
                                        snap.r_squared_spy_252d,
                                        snap.overlaps_spy_252d
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Sector corr 20 / 60 / 252")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2} / {:.2} / {:.2}",
                                        snap.corr_sector_20d,
                                        snap.corr_sector_60d,
                                        snap.corr_sector_252d
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Sector β / R² / overlap")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2} / {:.2} / {}",
                                        snap.beta_sector_252d,
                                        snap.r_squared_sector_252d,
                                        snap.overlaps_sector_252d
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
            self.show_corrstk = open;
        }

        // CORRRANK — benchmark-linkage rank vs sector peers
        if self.show_corrrank {
            if self.corrrank_symbol.is_empty() {
                self.corrrank_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_corrrank;
            egui::Window::new("CORRRANK — Benchmark Linkage Rank")
                .open(&mut open)
                .resizable(true)
                .default_size([700.0, 430.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.corrrank_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.corrrank_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.corrrank_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_corrrank(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.corrrank_snapshot = snap;
                                        self.corrrank_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.corrrank_symbol.to_uppercase();
                            self.corrrank_loading = true;
                            self.corrrank_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeCorrrankSnapshot { symbol: sym });
                        }
                        if self.corrrank_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.corrrank_snapshot;
                    if snap.symbol.is_empty() || snap.rank_label == "NO_DATA" {
                        ui.label(
                            egui::RichText::new(
                                "No data — needs a cached CORRSTK snapshot for the subject and at least 3 same-sector peers.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.subject_correlation_label.as_str() {
                            "INDEX_LOCKSTEP" | "SECTOR_LOCKSTEP" => UP,
                            "INVERSE_INDEX" | "INVERSE_SECTOR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — {} {} |corr| {:.2} — rank {}/{} — as of {}",
                                snap.symbol,
                                snap.rank_label,
                                snap.benchmark_kind,
                                snap.benchmark_name,
                                snap.subject_abs_corr_252d,
                                snap.rank_position,
                                snap.peers_considered + 1,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("corrrank_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(240.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("Selected benchmark basis")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {}",
                                        snap.benchmark_kind, snap.benchmark_name
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Subject corr / |corr| / β / R²")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2} / {:.2} / {:.2} / {:.2}",
                                        snap.subject_corr_252d,
                                        snap.subject_abs_corr_252d,
                                        snap.subject_beta_252d,
                                        snap.subject_r_squared_252d
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Sector median / p25 / p75 |corr|")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2} / {:.2} / {:.2}",
                                        snap.sector_median_abs_corr_252d,
                                        snap.sector_p25_abs_corr_252d,
                                        snap.sector_p75_abs_corr_252d
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Percentile / peers considered")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.0} / {} with data ({})",
                                        snap.percentile_rank,
                                        snap.peers_with_data,
                                        snap.peers_considered
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
            self.show_corrrank = open;
        }
    }
}
