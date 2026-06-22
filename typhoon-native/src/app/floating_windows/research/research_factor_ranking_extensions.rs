use super::*;

impl TyphooNApp {
    pub(super) fn render_research_factor_ranking_extensions_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research = research_chart_symbol(
            self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
        );

        // ── Research section ──
        // SIZEF — Size Factor Rank vs Sector Peers
        if self.show_sizef {
            if self.sizef_symbol.is_empty() {
                self.sizef_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_sizef;
            egui::Window::new("SIZEF — Size Factor Rank vs Sector Peers")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 360.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.sizef_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.sizef_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.sizef_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_sizef(&conn, &sym_u) {
                                        self.sizef_snapshot = snap;
                                        self.sizef_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.sizef_symbol.to_uppercase();
                            self.sizef_loading = true;
                            self.sizef_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeSizefSnapshot { symbol: sym });
                        }
                        if self.sizef_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.sizef_snapshot;
                    if snap.symbol.is_empty() || snap.rank_label == "NO_DATA" {
                        ui.label(egui::RichText::new("No data — needs fundamentals w/ market_cap on the subject and ≥3 sector peers with market_cap.")
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
                            "{} — {} — {} — pct {:.0} — rank {}/{} — sector {} — as of {}",
                            snap.symbol, snap.tier_label, snap.rank_label, snap.percentile_rank,
                            snap.rank_position, snap.peers_considered + 1, snap.sector, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("sizef_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Subject market cap").small().strong());
                            ui.label(egui::RichText::new(format!("${:.2}B", snap.market_cap / 1e9)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("log(cap)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.3}", snap.log_market_cap)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Sector median / p25 / p75 cap").small().strong());
                            ui.label(egui::RichText::new(format!("${:.2}B / ${:.2}B / ${:.2}B",
                                snap.sector_median_cap / 1e9, snap.sector_p25_cap / 1e9, snap.sector_p75_cap / 1e9)).small().monospace());
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
            self.show_sizef = open;
        }

        // MOMF — Momentum Factor Rank vs Sector Peers
        if self.show_momf {
            if self.momf_symbol.is_empty() {
                self.momf_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_momf;
            egui::Window::new("MOMF — Momentum Factor Rank vs Sector Peers")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 360.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.momf_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.momf_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.momf_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_momf(&conn, &sym_u) {
                                        self.momf_snapshot = snap;
                                        self.momf_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.momf_symbol.to_uppercase();
                            self.momf_loading = true;
                            self.momf_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeMomfSnapshot { symbol: sym });
                        }
                        if self.momf_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.momf_snapshot;
                    if snap.symbol.is_empty() || snap.rank_label == "NO_DATA" {
                        ui.label(egui::RichText::new("No data — needs a MOMENTUM snapshot on the subject, fundamentals w/ sector, and ≥3 peers in the same sector with MOMENTUM snapshots.")
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
                        egui::Grid::new("momf_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Subject momentum composite").small().strong());
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
            self.show_momf = open;
        }

        // PEADRANK — Post-Earnings Drift Rank vs Sector Peers
        if self.show_peadrank {
            if self.peadrank_symbol.is_empty() {
                self.peadrank_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_peadrank;
            egui::Window::new("PEADRANK — PEAD Drift Rank vs Sector Peers")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 360.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.peadrank_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.peadrank_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.peadrank_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_peadrank(&conn, &sym_u) {
                                        self.peadrank_snapshot = snap;
                                        self.peadrank_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.peadrank_symbol.to_uppercase();
                            self.peadrank_loading = true;
                            self.peadrank_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputePeadrankSnapshot { symbol: sym });
                        }
                        if self.peadrank_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.peadrank_snapshot;
                    if snap.symbol.is_empty() || snap.rank_label == "NO_DATA" {
                        ui.label(egui::RichText::new("No data — needs a PEAD snapshot on the subject (≥3 events used), fundamentals w/ sector, and ≥3 peers in the same sector with qualifying PEAD snapshots.")
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
                        egui::Grid::new("peadrank_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Subject avg drift (5d, %)").small().strong());
                            ui.label(egui::RichText::new(format!("{:+.2}%", snap.avg_drift_5d_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Sector median / p25 / p75 drift").small().strong());
                            ui.label(egui::RichText::new(format!("{:+.2}% / {:+.2}% / {:+.2}%",
                                snap.sector_median_drift_5d_pct, snap.sector_p25_drift_5d_pct, snap.sector_p75_drift_5d_pct)).small().monospace());
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
            self.show_peadrank = open;
        }

        // FQM — Fundamental Quality Meter
        if self.show_fqm {
            if self.fqm_symbol.is_empty() {
                self.fqm_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_fqm;
            egui::Window::new("FQM — Fundamental Quality Meter")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.fqm_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.fqm_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.fqm_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_fqm(&conn, &sym_u) {
                                        self.fqm_snapshot = snap;
                                        self.fqm_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.fqm_symbol.to_uppercase();
                            self.fqm_loading = true;
                            self.fqm_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeFqmSnapshot { symbol: sym });
                        }
                        if self.fqm_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.fqm_snapshot;
                    if snap.symbol.is_empty() || snap.operator_label == "NO_DATA" {
                        ui.label(egui::RichText::new("No data — needs at least one of Piotroski / Margins / Accruals cached for this symbol.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.operator_label.as_str() {
                            "ELITE_OPERATOR" | "STRONG_OPERATOR" => UP,
                            "WEAK_OPERATOR" | "BROKEN_OPERATOR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — composite {:.1}/100 — {} inputs — as of {}",
                            snap.symbol, snap.operator_label, snap.composite_score,
                            snap.inputs_available, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("fqm_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Piotroski (9pt)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.0} — {}", snap.piotroski_score, snap.piotroski_label)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Operating margin (TTM %)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}% — {}", snap.operating_margin_pct, snap.margin_trend_label)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Cash conversion (TTM %)").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}% — {}", snap.cash_conversion_pct, snap.accruals_trend_label)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Components (PTFS / MARGINS / ACRL)").small().strong());
                            let find = |key: &str| snap.components.iter().find(|c| c.name.eq_ignore_ascii_case(key)).map(|c| c.score).unwrap_or(0.0);
                            ui.label(egui::RichText::new(format!("{:.1} / {:.1} / {:.1}",
                                find("Piotroski F"),
                                find("Margins"),
                                find("Accruals"))).small().monospace());
                            ui.end_row();
                        });
                        if !snap.note.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT));
                        }
                    }
                });
            self.show_fqm = open;
        }

        // REVRANK — Relative 3y Revenue CAGR vs Sector Median
        if self.show_revrank {
            if self.revrank_symbol.is_empty() {
                self.revrank_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_revrank;
            egui::Window::new("REVRANK — Relative 3y Revenue CAGR vs Sector")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.revrank_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.revrank_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.revrank_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_revrank(&conn, &sym_u) {
                                        self.revrank_snapshot = snap;
                                        self.revrank_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.revrank_symbol.to_uppercase();
                            self.revrank_loading = true;
                            self.revrank_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeRevrankSnapshot { symbol: sym });
                        }
                        if self.revrank_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.revrank_snapshot;
                    if snap.symbol.is_empty() || snap.relative_label == "NO_DATA" || snap.relative_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — needs ≥3y of income statements on the subject and ≥3 sector peers w/ matching history.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.relative_label.as_str() {
                            "FAR_ABOVE_SECTOR" | "ABOVE_SECTOR" => UP,
                            "BELOW_SECTOR" | "FAR_BELOW_SECTOR" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — {:+.2}pp vs sector — sector {} — as of {}",
                            snap.symbol, snap.relative_label, snap.gap_to_median_pp,
                            snap.sector, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("revrank_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Subject 3y rev CAGR").small().strong());
                            ui.label(egui::RichText::new(format!("{:+.2}%", snap.symbol_cagr_pct)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Latest / earliest revenue").small().strong());
                            ui.label(egui::RichText::new(format!("${:.2}B / ${:.2}B ({} yrs)",
                                snap.latest_revenue / 1e9, snap.earliest_revenue / 1e9, snap.years_used)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Sector median / p25 / p75 CAGR").small().strong());
                            ui.label(egui::RichText::new(format!("{:+.2}% / {:+.2}% / {:+.2}%",
                                snap.sector_median_cagr_pct, snap.sector_p25_cagr_pct, snap.sector_p75_cagr_pct)).small().monospace());
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
            self.show_revrank = open;
        }

        // LEVRANK — Leverage Rank vs Sector Peers
        if self.show_levrank {
            if self.levrank_symbol.is_empty() {
                self.levrank_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_levrank;
            egui::Window::new("LEVRANK — Leverage Rank vs Sector")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.levrank_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.levrank_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.levrank_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_levrank(&conn, &sym_u) {
                                        self.levrank_snapshot = snap;
                                        self.levrank_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.levrank_symbol.to_uppercase();
                            self.levrank_loading = true;
                            self.levrank_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeLevrankSnapshot { symbol: sym });
                        }
                        if self.levrank_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.levrank_snapshot;
                    if snap.symbol.is_empty() || snap.rank_label == "NO_DATA" {
                        ui.label(egui::RichText::new("No data — needs a cached LEV snapshot for the subject and ≥3 sector peers with positive equity.")
                            .color(AXIS_TEXT).small());
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else if snap.rank_label == "NEGATIVE_EQUITY" {
                        ui.label(egui::RichText::new(format!(
                            "{} — NEGATIVE_EQUITY — total equity {:.0} (D/E undefined) — as of {}",
                            snap.symbol, snap.total_equity, snap.as_of,
                        )).strong().color(DOWN));
                        if !snap.note.is_empty() { ui.label(egui::RichText::new(&snap.note).small().color(AXIS_TEXT)); }
                    } else {
                        let color = match snap.rank_label.as_str() {
                            "SAFEST_DECILE" | "SAFEST_QUARTILE" | "ABOVE_MEDIAN_SAFE" => UP,
                            "BELOW_MEDIAN_RISKY" | "BOTTOM_QUARTILE_RISKY" | "RISKIEST_DECILE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!(
                            "{} — {} — pct {:.0} — rank {}/{} — sector {} — as of {}",
                            snap.symbol, snap.rank_label, snap.percentile_rank,
                            snap.rank_position, snap.peers_considered + 1, snap.sector, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("levrank_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Subject D/E").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2}", snap.debt_to_equity)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Subject total debt / equity").small().strong());
                            ui.label(egui::RichText::new(format!("${:.2}B / ${:.2}B",
                                snap.total_debt / 1e9, snap.total_equity / 1e9)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Sector median / p25 / p75 D/E").small().strong());
                            ui.label(egui::RichText::new(format!("{:.2} / {:.2} / {:.2}",
                                snap.sector_median_d2e, snap.sector_p25_d2e, snap.sector_p75_d2e)).small().monospace());
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
            self.show_levrank = open;
        }

        // OPERANK — Operating Quality Rank vs Sector Peers
        if self.show_operank {
            if self.operank_symbol.is_empty() {
                self.operank_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_operank;
            egui::Window::new("OPERANK — Operating Quality Rank vs Sector")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.operank_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.operank_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.operank_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_operank(&conn, &sym_u) {
                                        self.operank_snapshot = snap;
                                        self.operank_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.operank_symbol.to_uppercase();
                            self.operank_loading = true;
                            self.operank_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeOperankSnapshot { symbol: sym });
                        }
                        if self.operank_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.operank_snapshot;
                    if snap.symbol.is_empty() || snap.rank_label == "NO_DATA" {
                        ui.label(egui::RichText::new("No data — needs a cached MARGINS snapshot for the subject and ≥3 sector peers.")
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
                        egui::Grid::new("operank_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Subject op margin").small().strong());
                            ui.label(egui::RichText::new(format!("{:+.2}% — {}", snap.operating_margin_pct, snap.margin_trend_label)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Sector median / p25 / p75").small().strong());
                            ui.label(egui::RichText::new(format!("{:+.2}% / {:+.2}% / {:+.2}%",
                                snap.sector_median_margin_pct, snap.sector_p25_margin_pct, snap.sector_p75_margin_pct)).small().monospace());
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
            self.show_operank = open;
        }

        // FQMRANK — Fundamental Quality Meter Rank vs Sector Peers
        if self.show_fqmrank {
            if self.fqmrank_symbol.is_empty() {
                self.fqmrank_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_fqmrank;
            egui::Window::new("FQMRANK — Fundamental Quality Rank vs Sector")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.fqmrank_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.fqmrank_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.fqmrank_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_fqmrank(&conn, &sym_u) {
                                        self.fqmrank_snapshot = snap;
                                        self.fqmrank_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.fqmrank_symbol.to_uppercase();
                            self.fqmrank_loading = true;
                            self.fqmrank_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeFqmrankSnapshot { symbol: sym });
                        }
                        if self.fqmrank_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.fqmrank_snapshot;
                    if snap.symbol.is_empty() || snap.rank_label == "NO_DATA" {
                        ui.label(egui::RichText::new("No data — needs a cached FQM snapshot for the subject and ≥3 sector peers.")
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
                            "{} — {} — composite {:.1} — {} — pct {:.0} — rank {}/{} — sector {} — as of {}",
                            snap.symbol, snap.rank_label, snap.composite_score, snap.operator_label,
                            snap.percentile_rank, snap.rank_position, snap.peers_considered + 1, snap.sector, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("fqmrank_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Subject FQM composite").small().strong());
                            ui.label(egui::RichText::new(format!("{:.1}", snap.composite_score)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Sector median / p25 / p75").small().strong());
                            ui.label(egui::RichText::new(format!("{:.1} / {:.1} / {:.1}",
                                snap.sector_median_score, snap.sector_p25, snap.sector_p75)).small().monospace());
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
            self.show_fqmrank = open;
        }

        // LIQRANK — Liquidity Rank vs Sector Peers
        if self.show_liqrank {
            if self.liqrank_symbol.is_empty() {
                self.liqrank_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_liqrank;
            egui::Window::new("LIQRANK — Liquidity Rank vs Sector")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.liqrank_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.liqrank_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.liqrank_symbol.to_uppercase();
                                    if let Ok(Some(snap)) = typhoon_engine::core::research::get_liqrank(&conn, &sym_u) {
                                        self.liqrank_snapshot = snap;
                                        self.liqrank_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.liqrank_symbol.to_uppercase();
                            self.liqrank_loading = true;
                            self.liqrank_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeLiqrankSnapshot { symbol: sym });
                        }
                        if self.liqrank_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.liqrank_snapshot;
                    if snap.symbol.is_empty() || snap.rank_label == "NO_DATA" {
                        ui.label(egui::RichText::new("No data — needs a cached LIQ snapshot for the subject and ≥3 sector peers.")
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
                            "{} — {} — tier {} — pct {:.0} — rank {}/{} — sector {} — as of {}",
                            snap.symbol, snap.rank_label, snap.tier_label, snap.percentile_rank,
                            snap.rank_position, snap.peers_considered + 1, snap.sector, snap.as_of,
                        )).strong().color(color));
                        ui.separator();
                        egui::Grid::new("liqrank_summary").striped(true).num_columns(2).min_col_width(220.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Subject avg daily $ volume").small().strong());
                            ui.label(egui::RichText::new(format!("${:.1}M", snap.avg_daily_dollar_volume / 1e6)).small().monospace());
                            ui.end_row();
                            ui.label(egui::RichText::new("Sector median / p25 / p75 ADV$").small().strong());
                            ui.label(egui::RichText::new(format!("${:.1}M / ${:.1}M / ${:.1}M",
                                snap.sector_median_dollar_volume / 1e6,
                                snap.sector_p25_dollar_volume / 1e6,
                                snap.sector_p75_dollar_volume / 1e6)).small().monospace());
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
            self.show_liqrank = open;
        }

        // TLRANK — 30-day Liquidity Rank vs Sector Peers
        if self.show_tlrank {
            if self.tlrank_symbol.is_empty() {
                self.tlrank_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_tlrank;
            egui::Window::new("TLRANK — 30-Day Liquidity Rank")
                .open(&mut open)
                .resizable(true)
                .default_size([660.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.tlrank_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.tlrank_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.tlrank_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_tlrank(&conn, &sym_u)
                                    {
                                        self.tlrank_snapshot = snap;
                                        self.tlrank_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.tlrank_symbol.to_uppercase();
                            self.tlrank_loading = true;
                            self.tlrank_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeTlrankSnapshot { symbol: sym });
                        }
                        if self.tlrank_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.tlrank_snapshot;
                    if snap.symbol.is_empty() || snap.rank_label == "NO_DATA" {
                        ui.label(
                            egui::RichText::new(
                                "No data — needs cached daily bars for the subject and at least 3 same-sector peers.",
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
                            "BELOW_MEDIAN" | "BOTTOM_QUARTILE" | "BOTTOM_DECILE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — tier {} — 30d ADV$ ${:.1}M — rank {}/{} — as of {}",
                                snap.symbol,
                                snap.rank_label,
                                snap.tier_label,
                                snap.avg_30d_dollar_volume / 1e6,
                                snap.rank_position,
                                snap.peers_considered + 1,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("tlrank_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(230.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("Subject 30d ADV$ / valid bars")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "${:.1}M / {}",
                                        snap.avg_30d_dollar_volume / 1e6,
                                        snap.bars_used
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Sector median / p25 / p75 30d ADV$")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "${:.1}M / ${:.1}M / ${:.1}M",
                                        snap.sector_median_dollar_volume / 1e6,
                                        snap.sector_p25_dollar_volume / 1e6,
                                        snap.sector_p75_dollar_volume / 1e6
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
            self.show_tlrank = open;
        }
    }
}
