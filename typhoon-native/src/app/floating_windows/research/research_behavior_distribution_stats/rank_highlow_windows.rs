use super::*;

impl TyphooNApp {
    pub(super) fn render_rank_highlow_windows(
        &mut self,
        ctx: &egui::Context,
        chart_sym_research: &String,
    ) {
        if self.show_betarank {
            if self.betarank_symbol.is_empty() {
                self.betarank_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_betarank;
            egui::Window::new("BETARANK — Beta Rank vs Sector Peers")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.betarank_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.betarank_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.betarank_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_betarank(&conn, &sym_u) {
                                        self.betarank_snapshot = snap;
                                        self.betarank_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.betarank_symbol.to_uppercase();
                            self.betarank_loading = true;
                            self.betarank_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeBetarankSnapshot { symbol: sym });
                        }
                        if self.betarank_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.betarank_snapshot;
                    if snap.symbol.is_empty() || snap.rank_label == "NO_DATA" || snap.rank_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — needs ≥3 sector peers with cached Fundamentals.beta.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        // Risk-inverted colors: SAFEST = green, RISKIEST = red.
                        let color = match snap.rank_label.as_str() {
                            "SAFEST_DECILE" | "SAFEST_QUARTILE" | "ABOVE_MEDIAN_SAFE" => UP,
                            "BELOW_MEDIAN_RISKY" | "BOTTOM_QUARTILE_RISKY" | "RISKIEST_DECILE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — β {:.2} — sector: {} — pct {:.1} — rank {}/{} — as of {}",
                            snap.symbol, snap.rank_label,
                            snap.subject_beta.unwrap_or(0.0), snap.sector,
                            snap.percentile_rank, snap.rank_position, snap.peers_with_data + 1, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("betarank_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Subject β").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}", snap.subject_beta.unwrap_or(0.0))).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Sector p25 / median / p75").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2} / {:.2} / {:.2}",
                                snap.sector_p25_beta, snap.sector_median_beta, snap.sector_p75_beta)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Peers considered / with data").small().strong());
                            ui.label(egui::RichText::new(format!("{} / {}", snap.peers_considered, snap.peers_with_data)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Rank (1 = safest)").small().strong());
                            ui.label(egui::RichText::new(format!("{} / {}", snap.rank_position, snap.peers_with_data + 1)).small().monospace());
                            ui.end_row();
                        });
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
                        }
                    }
                });
            self.show_betarank = open;
        }

        // PEGRANK — PEG rank vs sector peers (lower = better value)
        if self.show_pegrank {
            if self.pegrank_symbol.is_empty() {
                self.pegrank_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_pegrank;
            egui::Window::new("PEGRANK — PEG Ratio Rank vs Sector Peers")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.pegrank_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.pegrank_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.pegrank_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_pegrank(&conn, &sym_u) {
                                        self.pegrank_snapshot = snap;
                                        self.pegrank_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.pegrank_symbol.to_uppercase();
                            self.pegrank_loading = true;
                            self.pegrank_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputePegrankSnapshot { symbol: sym });
                        }
                        if self.pegrank_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.pegrank_snapshot;
                    if snap.symbol.is_empty() || snap.rank_label == "NO_DATA" || snap.rank_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — needs ≥3 sector peers with positive Fundamentals.peg_ratio.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.rank_label.as_str() {
                            "TOP_DECILE" | "TOP_QUARTILE" | "ABOVE_MEDIAN" => UP,
                            "BELOW_MEDIAN" | "BOTTOM_QUARTILE" | "BOTTOM_DECILE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — PEG {:.2} — sector: {} — pct {:.1} — rank {}/{} — as of {}",
                            snap.symbol, snap.rank_label,
                            snap.subject_peg.unwrap_or(0.0), snap.sector,
                            snap.percentile_rank, snap.rank_position, snap.peers_with_data + 1, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("pegrank_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Subject PEG").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}", snap.subject_peg.unwrap_or(0.0))).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Sector p25 / median / p75").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2} / {:.2} / {:.2}",
                                snap.sector_p25_peg, snap.sector_median_peg, snap.sector_p75_peg)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Peers considered / with data").small().strong());
                            ui.label(egui::RichText::new(format!("{} / {}", snap.peers_considered, snap.peers_with_data)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Rank (1 = best value)").small().strong());
                            ui.label(egui::RichText::new(format!("{} / {}", snap.rank_position, snap.peers_with_data + 1)).small().monospace());
                            ui.end_row();
                        });
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
                        }
                    }
                });
            self.show_pegrank = open;
        }

        // FHIGHLOW — 52-week high/low distance
        if self.show_fhighlow {
            if self.fhighlow_symbol.is_empty() {
                self.fhighlow_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_fhighlow;
            egui::Window::new("FHIGHLOW — 52-Week High/Low Distance")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.fhighlow_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.fhighlow_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.fhighlow_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_fhighlow(&conn, &sym_u)
                                    {
                                        self.fhighlow_snapshot = snap;
                                        self.fhighlow_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.fhighlow_symbol.to_uppercase();
                            self.fhighlow_loading = true;
                            self.fhighlow_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeFhighlowSnapshot { symbol: sym });
                        }
                        if self.fhighlow_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.fhighlow_snapshot;
                    if snap.symbol.is_empty() || snap.proximity_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — needs ≥20 cached daily bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.proximity_label.as_str() {
                            "AT_HIGH" | "NEAR_HIGH" => UP,
                            "AT_LOW" | "NEAR_LOW" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — close {:.2} — range {:.1}% — {} bars — as of {}",
                                snap.symbol,
                                snap.proximity_label,
                                snap.latest_close,
                                snap.range_position_pct,
                                snap.bars_used,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("fhighlow_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(220.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("52-week high").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2} on {} ({} sessions ago)",
                                        snap.high_52w, snap.high_52w_date, snap.days_since_high
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("52-week low").small().strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2} on {} ({} sessions ago)",
                                        snap.low_52w, snap.low_52w_date, snap.days_since_low
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("From high / from low").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.2}% / {:+.2}%",
                                        snap.pct_from_high, snap.pct_from_low
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Range position").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.1}%", snap.range_position_pct))
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
            self.show_fhighlow = open;
        }
    }
}
