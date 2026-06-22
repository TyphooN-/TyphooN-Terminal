use super::*;

impl TyphooNApp {
    pub(super) fn render_research_sector_factor_drift_ranks_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research = research_chart_symbol(
            self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
        );

        // ── Research section ──

        // VRK — Value Rank vs sector peers
        if self.show_vrk {
            if self.vrk_symbol.is_empty() {
                self.vrk_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_vrk;
            egui::Window::new("VRK — Value Rank vs Sector Peers")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 360.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.vrk_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.vrk_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.vrk_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_vrk(&conn, &sym_u) {
                                        self.vrk_snapshot = snap;
                                        self.vrk_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.vrk_symbol.to_uppercase();
                            self.vrk_loading = true;
                            self.vrk_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeVrkSnapshot { symbol: sym });
                        }
                        if self.vrk_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.vrk_snapshot;
                    if snap.symbol.is_empty() || snap.rank_label == "NO_DATA" {
                        ui.label(egui::RichText::new("No data — needs a VAL snapshot on the subject and ≥3 VAL-carrying peers in the same sector.")
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
                            "{} — {} — pct {:.0} — rank {}/{} — sector {} — as of {}",
                            snap.symbol, snap.rank_label, snap.percentile_rank,
                            snap.rank_position, snap.peers_considered + 1, snap.sector, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("vrk_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Subject composite").small().strong());
                            ui.label(egui::RichText::new(format!("{:.1}", snap.composite_score)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Sector median / p25 / p75").small().strong());
                            ui.label(egui::RichText::new(format!("{:.1} / {:.1} / {:.1}", snap.sector_median_score, snap.sector_p25, snap.sector_p75)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Peers considered / with data").small().strong());
                            ui.label(egui::RichText::new(format!("{} / {}", snap.peers_considered, snap.peers_with_data)).small().monospace());
                            ui.end_row();
                        });
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
                        }
                    }
                });
            self.show_vrk = open;
        }

        // QRK — Quality Rank vs sector peers
        if self.show_qrk {
            if self.qrk_symbol.is_empty() {
                self.qrk_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_qrk;
            egui::Window::new("QRK — Quality Rank vs Sector Peers")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 360.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.qrk_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.qrk_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.qrk_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_qrk(&conn, &sym_u) {
                                        self.qrk_snapshot = snap;
                                        self.qrk_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.qrk_symbol.to_uppercase();
                            self.qrk_loading = true;
                            self.qrk_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeQrkSnapshot { symbol: sym });
                        }
                        if self.qrk_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.qrk_snapshot;
                    if snap.symbol.is_empty() || snap.rank_label == "NO_DATA" {
                        ui.label(egui::RichText::new("No data — needs a QUAL snapshot on the subject, fundamentals w/ sector, and ≥3 peers in the same sector with QUAL snapshots.")
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
                            "{} — {} — pct {:.0} — rank {}/{} — sector {} — as of {}",
                            snap.symbol, snap.rank_label, snap.percentile_rank,
                            snap.rank_position, snap.peers_considered + 1, snap.sector, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("qrk_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Subject composite").small().strong());
                            ui.label(egui::RichText::new(format!("{:.1}", snap.composite_score)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Sector median / p25 / p75").small().strong());
                            ui.label(egui::RichText::new(format!("{:.1} / {:.1} / {:.1}", snap.sector_median_score, snap.sector_p25, snap.sector_p75)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Peers considered / with data").small().strong());
                            ui.label(egui::RichText::new(format!("{} / {}", snap.peers_considered, snap.peers_with_data)).small().monospace());
                            ui.end_row();
                        });
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
                        }
                    }
                });
            self.show_qrk = open;
        }

        // RRK — Risk Rank vs sector peers (inverted — higher pct = SAFER)
        if self.show_rrk {
            if self.rrk_symbol.is_empty() {
                self.rrk_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_rrk;
            egui::Window::new("RRK — Risk Rank vs Sector Peers (Higher = Safer)")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 360.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.rrk_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.rrk_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.rrk_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_rrk(&conn, &sym_u) {
                                        self.rrk_snapshot = snap;
                                        self.rrk_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.rrk_symbol.to_uppercase();
                            self.rrk_loading = true;
                            self.rrk_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeRrkSnapshot { symbol: sym });
                        }
                        if self.rrk_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.rrk_snapshot;
                    if snap.symbol.is_empty() || snap.rank_label == "NO_DATA" {
                        ui.label(egui::RichText::new("No data — needs a RISK snapshot on the subject, fundamentals w/ sector, and ≥3 peers in the same sector with RISK snapshots.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.rank_label.as_str() {
                            "SAFEST_DECILE" | "SAFEST_QUARTILE" | "ABOVE_MEDIAN_SAFE" => UP,
                            "BELOW_MEDIAN_RISKY" | "BOTTOM_QUARTILE_RISKY" | "RISKIEST_DECILE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — safe pct {:.0} — rank {}/{} — sector {} — as of {}",
                            snap.symbol, snap.rank_label, snap.percentile_rank,
                            snap.rank_position, snap.peers_considered + 1, snap.sector, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("rrk_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Subject composite (higher = riskier)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.1}", snap.composite_score)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Sector median / p25 / p75 (risk)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.1} / {:.1} / {:.1}", snap.sector_median_score, snap.sector_p25, snap.sector_p75)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Peers considered / with data").small().strong());
                            ui.label(egui::RichText::new(format!("{} / {}", snap.peers_considered, snap.peers_with_data)).small().monospace());
                            ui.end_row();
                        });
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
                        }
                    }
                });
            self.show_rrk = open;
        }

        // RELEPSGR — Relative 3y EPS CAGR vs sector median
        if self.show_relepsgr {
            if self.relepsgr_symbol.is_empty() {
                self.relepsgr_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_relepsgr;
            egui::Window::new("RELEPSGR — Relative 3y EPS CAGR vs Sector")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.relepsgr_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.relepsgr_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.relepsgr_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_relepsgr(&conn, &sym_u) {
                                        self.relepsgr_snapshot = snap;
                                        self.relepsgr_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.relepsgr_symbol.to_uppercase();
                            self.relepsgr_loading = true;
                            self.relepsgr_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeRelepsgrSnapshot { symbol: sym });
                        }
                        if self.relepsgr_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.relepsgr_snapshot;
                    if snap.symbol.is_empty() || snap.relative_label == "NO_DATA" {
                        ui.label(egui::RichText::new("No data — needs ≥4 annual income rows on subject, fundamentals w/ sector, and ≥3 peers in the same sector with ≥4 annual EPS rows.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.relative_label.as_str() {
                            "FAR_ABOVE" | "ABOVE" => UP,
                            "BELOW" | "FAR_BELOW" | "CAGR_NEGATIVE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — {:.1}% CAGR — gap {:+.1}pp — sector {} — as of {}",
                            snap.symbol, snap.relative_label, snap.symbol_cagr_pct,
                            snap.gap_to_median_pp, snap.sector, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("relepsgr_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Latest / earliest EPS").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2} / {:.2} ({} yrs)", snap.latest_eps, snap.earliest_eps, snap.years_used)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Sector median / p25 / p75 CAGR").small().strong());
                            ui.label(egui::RichText::new(format!("{:.1}% / {:.1}% / {:.1}%", snap.sector_median_cagr_pct, snap.sector_p25_cagr_pct, snap.sector_p75_cagr_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Peers considered / with data").small().strong());
                            ui.label(egui::RichText::new(format!("{} / {}", snap.peers_considered, snap.peers_with_data)).small().monospace());
                            ui.end_row();
                        });
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
                        }
                    }
                });
            self.show_relepsgr = open;
        }

        // PEAD — Post-Earnings-Announcement Drift
        if self.show_pead {
            if self.pead_symbol.is_empty() {
                self.pead_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_pead;
            egui::Window::new("PEAD — Post-Earnings-Announcement Drift")
                .open(&mut open)
                .resizable(true)
                .default_size([720.0, 480.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.pead_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.pead_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.pead_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_pead(&conn, &sym_u) {
                                        self.pead_snapshot = snap;
                                        self.pead_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.pead_symbol.to_uppercase();
                            self.pead_loading = true;
                            self.pead_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputePeadSnapshot { symbol: sym });
                        }
                        if self.pead_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.pead_snapshot;
                    if snap.symbol.is_empty() || snap.drift_direction_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — needs ≥3 cached EarningsSurprise rows and historical price bars spanning each event + 10 trading days forward.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.drift_direction_label.as_str() {
                            "DRIFT_UP" => UP,
                            "DRIFT_DOWN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — {} events used — avg 5d {:+.2}% — as of {}",
                            snap.symbol, snap.drift_direction_label, snap.events_used,
                            snap.avg_drift_5d_pct, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("pead_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Avg drift 1d / 3d / 5d / 10d").small().strong());
                            ui.label(egui::RichText::new(format!("{:+.2}% / {:+.2}% / {:+.2}% / {:+.2}%",
                                snap.avg_drift_1d_pct, snap.avg_drift_3d_pct, snap.avg_drift_5d_pct, snap.avg_drift_10d_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Beat 5d / Miss 5d").small().strong());
                            ui.label(egui::RichText::new(format!("{:+.2}% / {:+.2}%", snap.beat_event_drift_5d_pct, snap.miss_event_drift_5d_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Latest event (date / surprise / 5d drift)").small().strong());
                            ui.label(egui::RichText::new(format!("{} / {:+.2}% / {:+.2}%",
                                snap.latest_event_date, snap.latest_event_surprise_pct, snap.latest_event_drift_5d_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Events in cache / used").small().strong());
                            ui.label(egui::RichText::new(format!("{} / {}", snap.num_events, snap.events_used)).small().monospace());
                            ui.end_row();
                        });
                        if !snap.rows.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new("Per-event detail").strong().small());
                            egui::ScrollArea::vertical().max_height(200.0).id_salt("pead_rows").show(ui, |ui| {
                                egui::Grid::new("pead_events").striped(true).num_columns(7).min_col_width(60.0).show(ui, |ui| {
                                    ui.label(egui::RichText::new("Date").small().strong());
                                    ui.label(egui::RichText::new("Class").small().strong());
                                    ui.label(egui::RichText::new("Surprise %").small().strong());
                                    ui.label(egui::RichText::new("1d").small().strong());
                                    ui.label(egui::RichText::new("3d").small().strong());
                                    ui.label(egui::RichText::new("5d").small().strong());
                                    ui.label(egui::RichText::new("10d").small().strong());
                                    ui.end_row();
                                    for row in &snap.rows {
                                        let rc = match row.classification.as_str() {
                                            "BEAT" => UP,
                                            "MISS" => DOWN,
                                            _ => AXIS_TEXT,
                                        };
                                        ui.label(egui::RichText::new(&row.event_date).small().monospace());
                                        ui.label(egui::RichText::new(&row.classification).small().monospace().color(rc));
                                        ui.label(egui::RichText::new(format!("{:+.1}%", row.surprise_pct)).small().monospace());
                                        ui.label(egui::RichText::new(format!("{:+.2}%", row.drift_1d_pct)).small().monospace());
                                        ui.label(egui::RichText::new(format!("{:+.2}%", row.drift_3d_pct)).small().monospace());
                                        ui.label(egui::RichText::new(format!("{:+.2}%", row.drift_5d_pct)).small().monospace());
                                        ui.label(egui::RichText::new(format!("{:+.2}%", row.drift_10d_pct)).small().monospace());
                                        ui.end_row();
                                    }
                                });
                            });
                        }
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
                        }
                    }
                });
            self.show_pead = open;
        }
    }
}
