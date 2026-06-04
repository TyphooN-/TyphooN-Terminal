use super::*;

impl TyphooNApp {
    pub(super) fn render_research_round42_windows(&mut self, ctx: &egui::Context) {
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

        // ── Research Round 42 windows ──
        if self.show_squeeze_win {
            if self.squeeze_win_symbol.is_empty() {
                self.squeeze_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_squeeze_win;
            egui::Window::new("SQUEEZE — Short-Squeeze Composite")
                .open(&mut open).resizable(true).default_size([560.0, 360.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.squeeze_win_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.squeeze_win_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.squeeze_win_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_squeeze(&conn, &sym_u) { self.squeeze_win_snapshot = snap; self.squeeze_win_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.squeeze_win_symbol.to_uppercase(); self.squeeze_win_loading = true; self.squeeze_win_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeSqueezeSnapshot { symbol: sym });
                        }
                        if self.squeeze_win_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.squeeze_win_snapshot;
                    if snap.symbol.is_empty() || snap.squeeze_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — need ≥3 of 5 axes (short interest, IV, relvol, HP bars).").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.squeeze_label.as_str() {
                            "NO_SQUEEZE" | "WATCH" => UP,
                            "STRONG" | "EXTREME" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — composite {:.1}/100 — as of {}", snap.symbol, snap.squeeze_label, snap.composite_score, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("squeeze_summary").striped(true).num_columns(3).min_col_width(120.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Axis").small().strong());
                            ui.label(egui::RichText::new("Raw").small().strong());
                            ui.label(egui::RichText::new("Score 0..100").small().strong()); ui.end_row();
                            ui.label(egui::RichText::new("Short % float").small()); ui.label(egui::RichText::new(format!("{:.2}%", snap.short_percent_of_float)).small().monospace()); ui.label(egui::RichText::new(format!("{:.0}", snap.short_float_score)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Days to cover").small()); ui.label(egui::RichText::new(format!("{:.2}d", snap.days_to_cover)).small().monospace()); ui.label(egui::RichText::new(format!("{:.0}", snap.days_to_cover_score)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("20d momentum").small()); ui.label(egui::RichText::new(format!("{:+.2}%", snap.momentum_20d_pct)).small().monospace()); ui.label(egui::RichText::new(format!("{:.0}", snap.momentum_score)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("RelVol 20d").small()); ui.label(egui::RichText::new(format!("{:.2}×", snap.relvol_20d)).small().monospace()); ui.label(egui::RichText::new(format!("{:.0}", snap.relvol_score)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("IV rank").small()); ui.label(egui::RichText::new(format!("{:.1}", snap.iv_rank)).small().monospace()); ui.label(egui::RichText::new(format!("{:.0}", snap.iv_rank_score)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Axes present").small().strong()); ui.label(egui::RichText::new(format!("{}/5", snap.inputs_present)).small().monospace()); ui.label(""); ui.end_row();
                        });
                    }
                });
            self.show_squeeze_win = open;
        }

        if self.show_squeezerank {
            if self.squeezerank_symbol.is_empty() {
                self.squeezerank_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_squeezerank;
            egui::Window::new("SQUEEZERANK — Cross-Symbol Squeeze Percentile")
                .open(&mut open).resizable(true).default_size([520.0, 260.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.squeezerank_symbol).desired_width(100.0));
                        if ui.button("Use Chart").clicked() { self.squeezerank_symbol = chart_sym_research.clone(); }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache { if let Ok(conn) = cache.connection() {
                                let sym_u = self.squeezerank_symbol.to_uppercase();
                                if let Ok(Some(snap)) = typhoon_engine::core::research::get_squeezerank(&conn, &sym_u) { self.squeezerank_snapshot = snap; self.squeezerank_symbol = sym_u; }
                            }}
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.squeezerank_symbol.to_uppercase(); self.squeezerank_loading = true; self.squeezerank_symbol = sym.clone();
                            let _ = self.broker_tx.send(BrokerCmd::ComputeSqueezeRankSnapshot { symbol: sym });
                        }
                        if self.squeezerank_loading { ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small()); }
                    });
                    ui.separator();
                    let snap = &self.squeezerank_snapshot;
                    if snap.symbol.is_empty() || snap.squeezerank_label == "INSUFFICIENT_DATA" {
                        ui.label(egui::RichText::new("No data — need ≥5 symbols with SQUEEZE rows. Try the Watchlist Refresh first.").color(AXIS_TEXT).small());
                    } else {
                        let color = match snap.squeezerank_label.as_str() {
                            "TOP_1PCT" | "TOP_5PCT" => DOWN,
                            "TOP_10PCT" => AXIS_TEXT,
                            _ => UP,
                        };
                        ui.label(egui::RichText::new(format!("{} — {} — rank {}/{} — percentile {:.1} — as of {}", snap.symbol, snap.squeezerank_label, snap.rank, snap.peer_count, snap.percentile, snap.as_of)).strong().color(color));
                        ui.separator();
                        egui::Grid::new("sqzrank_summary").striped(true).num_columns(2).min_col_width(180.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Composite score").small().strong()); ui.label(egui::RichText::new(format!("{:.1}", snap.composite_score)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Rank").small().strong()); ui.label(egui::RichText::new(format!("{} / {}", snap.rank, snap.peer_count)).small().monospace()); ui.end_row();
                            ui.label(egui::RichText::new("Percentile").small().strong()); ui.label(egui::RichText::new(format!("{:.1}", snap.percentile)).small().monospace()); ui.end_row();
                        });
                    }
                });
            self.show_squeezerank = open;
        }

        if self.show_squeeze_watchlist {
            let mut open = self.show_squeeze_watchlist;
            egui::Window::new("SQUEEZE Watchlist — Top-N by Composite Score")
                .open(&mut open)
                .resizable(true)
                .default_size([720.0, 480.0])
                .max_size([720.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .add(egui::Button::new("Refresh (rescan cache)").fill(BTN_MG))
                            .clicked()
                        {
                            self.squeeze_watchlist_loading = true;
                            let _ = self.broker_tx.send(BrokerCmd::RefreshSqueezeWatchlist);
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let mut rows =
                                        typhoon_engine::core::research::get_all_squeeze(&conn)
                                            .unwrap_or_default();
                                    rows.sort_by(|a, b| {
                                        b.composite_score
                                            .partial_cmp(&a.composite_score)
                                            .unwrap_or(std::cmp::Ordering::Equal)
                                    });
                                    self.squeeze_watchlist_rows = rows;
                                }
                            }
                        }
                        if self.squeeze_watchlist_loading {
                            ui.label(egui::RichText::new("Scanning…").color(AXIS_TEXT).small());
                        }
                        ui.label(
                            egui::RichText::new(format!(
                                "{} rows",
                                self.squeeze_watchlist_rows.len()
                            ))
                            .color(AXIS_TEXT)
                            .small(),
                        );
                    });
                    ui.separator();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        egui::Grid::new("squeeze_watch_grid")
                            .striped(true)
                            .num_columns(8)
                            .min_col_width(60.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Symbol").small().strong());
                                ui.label(egui::RichText::new("Label").small().strong());
                                ui.label(egui::RichText::new("Composite").small().strong());
                                ui.label(egui::RichText::new("Short%").small().strong());
                                ui.label(egui::RichText::new("DTC").small().strong());
                                ui.label(egui::RichText::new("20d Mom").small().strong());
                                ui.label(egui::RichText::new("RelVol").small().strong());
                                ui.label(egui::RichText::new("IVrank").small().strong());
                                ui.end_row();
                                let mut shown = 0usize;
                                for row in self.squeeze_watchlist_rows.iter() {
                                    if row.squeeze_label == "INSUFFICIENT_DATA" {
                                        continue;
                                    }
                                    let color = match row.squeeze_label.as_str() {
                                        "EXTREME" | "STRONG" => DOWN,
                                        "ELEVATED" => AXIS_TEXT,
                                        _ => UP,
                                    };
                                    ui.label(
                                        egui::RichText::new(&row.symbol)
                                            .small()
                                            .monospace()
                                            .color(color)
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new(&row.squeeze_label)
                                            .small()
                                            .color(color),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("{:.1}", row.composite_score))
                                            .small()
                                            .monospace(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{:.1}%",
                                            row.short_percent_of_float
                                        ))
                                        .small()
                                        .monospace(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("{:.1}d", row.days_to_cover))
                                            .small()
                                            .monospace(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{:+.1}%",
                                            row.momentum_20d_pct
                                        ))
                                        .small()
                                        .monospace(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("{:.2}×", row.relvol_20d))
                                            .small()
                                            .monospace(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("{:.0}", row.iv_rank))
                                            .small()
                                            .monospace(),
                                    );
                                    ui.end_row();
                                    shown += 1;
                                    if shown >= 50 {
                                        break;
                                    }
                                }
                            });
                    });
                });
            self.show_squeeze_watchlist = open;
        }

        if self.show_bbsqueeze {
            if self.bbsqueeze_symbol.is_empty() {
                self.bbsqueeze_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_bbsqueeze;
            egui::Window::new("BBSQUEEZE — Bollinger-Band Width Squeeze")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.bbsqueeze_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.bbsqueeze_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.bbsqueeze_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_bbsqueeze(&conn, &sym_u)
                                    {
                                        self.bbsqueeze_snapshot = snap;
                                        self.bbsqueeze_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.bbsqueeze_symbol.to_uppercase();
                            self.bbsqueeze_loading = true;
                            self.bbsqueeze_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeBbsqueezeSnapshot { symbol: sym });
                        }
                        if self.bbsqueeze_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.bbsqueeze_snapshot;
                    if snap.symbol.is_empty() || snap.bbsqueeze_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥140 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.bbsqueeze_label.as_str() {
                            "TIGHT_SQUEEZE" => DOWN,
                            "MODERATE_SQUEEZE" => AXIS_TEXT,
                            "EXPANSION" => UP,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — width pct {:.1} — as of {}",
                                snap.symbol,
                                snap.bbsqueeze_label,
                                snap.bb_width_percentile,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("bbsq_summary")
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
                                ui.label(egui::RichText::new("BB width current").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.5}", snap.bb_width_current))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("BB width min 120").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.5}", snap.bb_width_min_120))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("BB width max 120").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.5}", snap.bb_width_max_120))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Width percentile").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.1}", snap.bb_width_percentile))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Upper band").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.upper_band))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Mid band").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.mid_band))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Lower band").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.lower_band))
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
            self.show_bbsqueeze = open;
        }

        if self.show_donchian_win {
            if self.donchian_win_symbol.is_empty() {
                self.donchian_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_donchian_win;
            egui::Window::new("DONCHIAN — 20-Bar Channel Breakout")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 300.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.donchian_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.donchian_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.donchian_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_donchian(&conn, &sym_u)
                                    {
                                        self.donchian_win_snapshot = snap;
                                        self.donchian_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.donchian_win_symbol.to_uppercase();
                            self.donchian_win_loading = true;
                            self.donchian_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeDonchianSnapshot { symbol: sym });
                        }
                        if self.donchian_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.donchian_win_snapshot;
                    if snap.symbol.is_empty() || snap.donchian_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥21 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.donchian_label.as_str() {
                            "BREAKOUT_UP" => UP,
                            "BREAKOUT_DOWN" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — pos {:.1}% — as of {}",
                                snap.symbol,
                                snap.donchian_label,
                                snap.channel_position_pct,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("donchian_summary")
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
                                ui.label(egui::RichText::new("Upper channel").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.upper_channel))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Mid channel").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.mid_channel))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Lower channel").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.lower_channel))
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
                                ui.label(
                                    egui::RichText::new("Channel position %").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.1}",
                                        snap.channel_position_pct
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Breakout upper").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.breakout_upper))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Breakout lower").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{}", snap.breakout_lower))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_donchian_win = open;
        }

        if self.show_kama_win {
            if self.kama_win_symbol.is_empty() {
                self.kama_win_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_kama_win;
            egui::Window::new("KAMA — Kaufman Adaptive MA / Efficiency Ratio")
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 280.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.kama_win_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.kama_win_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.kama_win_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_kama(&conn, &sym_u)
                                    {
                                        self.kama_win_snapshot = snap;
                                        self.kama_win_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.kama_win_symbol.to_uppercase();
                            self.kama_win_loading = true;
                            self.kama_win_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeKamaSnapshot { symbol: sym });
                        }
                        if self.kama_win_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.kama_win_snapshot;
                    if snap.symbol.is_empty() || snap.kama_label == "INSUFFICIENT_DATA" {
                        ui.label(
                            egui::RichText::new("No data — HP cache needs ≥25 bars.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else {
                        let color = match snap.kama_label.as_str() {
                            "STRONG_TREND" | "MODERATE_TREND" => UP,
                            "CHOPPY" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — ER {:.3} — slope {:+.2}% — as of {}",
                                snap.symbol,
                                snap.kama_label,
                                snap.efficiency_ratio,
                                snap.kama_slope_pct,
                                snap.as_of
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("kama_summary")
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
                                ui.label(egui::RichText::new("Efficiency ratio").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.3}", snap.efficiency_ratio))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("KAMA value").small().strong());
                                ui.label(
                                    egui::RichText::new(format!("{:.4}", snap.kama_value))
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
                                ui.label(
                                    egui::RichText::new("KAMA 5-bar slope %").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:+.2}", snap.kama_slope_pct))
                                        .small()
                                        .monospace(),
                                );
                                ui.end_row();
                            });
                    }
                });
            self.show_kama_win = open;
        }
    }
}
