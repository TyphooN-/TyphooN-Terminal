use super::*;

impl TyphooNApp {
    pub(super) fn render_research_momentum_gap_atr_drawdown_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // SURPSTK — Earnings Surprise Streak
        if self.show_surpstk {
            if self.surpstk_symbol.is_empty() {
                self.surpstk_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_surpstk;
            egui::Window::new("SURPSTK — Earnings Surprise Streak")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.surpstk_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.surpstk_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.surpstk_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_surpstk(&conn, &sym_u)
                                    {
                                        self.surpstk_snapshot = snap;
                                        self.surpstk_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.surpstk_symbol.to_uppercase();
                            self.surpstk_loading = true;
                            self.surpstk_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeSurpstkSnapshot { symbol: sym });
                        }
                        if self.surpstk_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_surpstk_snapshot(ui, &self.surpstk_snapshot);
                });
            self.show_surpstk = open;
        }

        // DVDRANK — Dividend Growth Rank vs sector peers
        if self.show_dvdrank {
            if self.dvdrank_symbol.is_empty() {
                self.dvdrank_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_dvdrank;
            egui::Window::new("DVDRANK — Dividend Growth Rank")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 360.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.dvdrank_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.dvdrank_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.dvdrank_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_dvdrank(&conn, &sym_u)
                                    {
                                        self.dvdrank_snapshot = snap;
                                        self.dvdrank_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.dvdrank_symbol.to_uppercase();
                            self.dvdrank_loading = true;
                            self.dvdrank_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeDvdrankSnapshot { symbol: sym });
                        }
                        if self.dvdrank_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_dvdrank_snapshot(ui, &self.dvdrank_snapshot);
                });
            self.show_dvdrank = open;
        }

        // EARMRANK — Earnings Momentum Rank vs sector peers
        if self.show_earmrank {
            if self.earmrank_symbol.is_empty() {
                self.earmrank_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_earmrank;
            egui::Window::new("EARMRANK — Earnings Momentum Rank")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 360.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.earmrank_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.earmrank_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.earmrank_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_earmrank(&conn, &sym_u)
                                    {
                                        self.earmrank_snapshot = snap;
                                        self.earmrank_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.earmrank_symbol.to_uppercase();
                            self.earmrank_loading = true;
                            self.earmrank_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeEarmrankSnapshot { symbol: sym });
                        }
                        if self.earmrank_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_earmrank_snapshot(ui, &self.earmrank_snapshot);
                });
            self.show_earmrank = open;
        }

        // UPDGRANK — Upgrade/Downgrade Rank vs sector peers
        if self.show_updgrank {
            if self.updgrank_symbol.is_empty() {
                self.updgrank_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_updgrank;
            egui::Window::new("UPDGRANK — Upgrade/Downgrade Rank")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 360.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.updgrank_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.updgrank_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.updgrank_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_updgrank(&conn, &sym_u)
                                    {
                                        self.updgrank_snapshot = snap;
                                        self.updgrank_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.updgrank_symbol.to_uppercase();
                            self.updgrank_loading = true;
                            self.updgrank_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeUpdgrankSnapshot { symbol: sym });
                        }
                        if self.updgrank_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_updgrank_snapshot(ui, &self.updgrank_snapshot);
                });
            self.show_updgrank = open;
        }

        // GY — Gap Yearly (253-bar gap census)
        if self.show_gy {
            if self.gy_symbol.is_empty() {
                self.gy_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_gy;
            egui::Window::new("GY — Gap Yearly (253d census)")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.gy_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.gy_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.gy_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_gy(&conn, &sym_u)
                                    {
                                        self.gy_snapshot = snap;
                                        self.gy_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.gy_symbol.to_uppercase();
                            self.gy_loading = true;
                            self.gy_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeGySnapshot { symbol: sym });
                        }
                        if self.gy_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_gy_snapshot(ui, &self.gy_snapshot);
                });
            self.show_gy = open;
        }

        // DES — Daily Event Streak
        if self.show_des {
            if self.des_symbol.is_empty() {
                self.des_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_des;
            egui::Window::new("DES — Daily Event Streak")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.des_symbol).desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.des_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.des_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_des(&conn, &sym_u)
                                    {
                                        self.des_snapshot = snap;
                                        self.des_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.des_symbol.to_uppercase();
                            self.des_loading = true;
                            self.des_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeDesSnapshot { symbol: sym });
                        }
                        if self.des_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_des_snapshot(ui, &self.des_snapshot);
                });
            self.show_des = open;
        }

        // DVDYIELDRANK — Dividend Yield Rank vs Sector Peers
        if self.show_dvdyieldrank {
            if self.dvdyieldrank_symbol.is_empty() {
                self.dvdyieldrank_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_dvdyieldrank;
            egui::Window::new("DVDYIELDRANK — Dividend Yield Rank")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.dvdyieldrank_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.dvdyieldrank_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.dvdyieldrank_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_dvdyieldrank(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.dvdyieldrank_snapshot = snap;
                                        self.dvdyieldrank_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.dvdyieldrank_symbol.to_uppercase();
                            self.dvdyieldrank_loading = true;
                            self.dvdyieldrank_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeDvdyieldrankSnapshot { symbol: sym });
                        }
                        if self.dvdyieldrank_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_dvdyieldrank_snapshot(ui, &self.dvdyieldrank_snapshot);
                });
            self.show_dvdyieldrank = open;
        }

        // SHRANK — Short Interest Rank (risk-inverted)
        if self.show_shrank {
            if self.shrank_symbol.is_empty() {
                self.shrank_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_shrank;
            egui::Window::new("SHRANK — Short Interest Rank")
                .open(&mut open)
                .resizable(true)
                .default_size([640.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.shrank_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.shrank_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.shrank_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_shrank(&conn, &sym_u)
                                    {
                                        self.shrank_snapshot = snap;
                                        self.shrank_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.shrank_symbol.to_uppercase();
                            self.shrank_loading = true;
                            self.shrank_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeShrankSnapshot { symbol: sym });
                        }
                        if self.shrank_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_shrank_snapshot(ui, &self.shrank_snapshot);
                });
            self.show_shrank = open;
        }

        // SHORTRANK_DELTA — Short Interest Trend Rank (risk-inverted)
        if self.show_shortrank_delta {
            if self.shortrank_delta_symbol.is_empty() {
                self.shortrank_delta_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_shortrank_delta;
            egui::Window::new("SHORTRANK_DELTA — Short Interest Trend Rank")
                .open(&mut open)
                .resizable(true)
                .default_size([700.0, 420.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.shortrank_delta_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.shortrank_delta_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.shortrank_delta_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_shortrank_delta(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.shortrank_delta_snapshot = snap;
                                        self.shortrank_delta_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.shortrank_delta_symbol.to_uppercase();
                            self.shortrank_delta_loading = true;
                            self.shortrank_delta_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeShortrankDeltaSnapshot { symbol: sym });
                        }
                        if self.shortrank_delta_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    ui.separator();
                    let snap = &self.shortrank_delta_snapshot;
                    if snap.symbol.is_empty()
                        || snap.rank_label == "NO_DATA"
                        || snap.rank_label == "INSUFFICIENT_DATA"
                    {
                        ui.label(
                            egui::RichText::new(
                                "No data — needs short-interest history for the subject and at least 3 same-sector peers. History accumulates from fundamentals scrapes and SHORT_INTEREST fetches.",
                            )
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        if !snap.note.is_empty() {
                            ui.label(egui::RichText::new(&snap.note).color(DOWN).small());
                        }
                    } else {
                        let color = match snap.rank_label.as_str() {
                            "SAFEST_DECILE" | "SAFEST_QUARTILE" | "ABOVE_MEDIAN_SAFE" => UP,
                            "BELOW_MEDIAN_RISKY"
                            | "BOTTOM_QUARTILE_RISKY"
                            | "RISKIEST_DECILE" => DOWN,
                            _ => AXIS_TEXT,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {} — {} {:+.2} pts — rank {}/{} — as of {}",
                                snap.symbol,
                                snap.rank_label,
                                snap.subject_trend_label,
                                snap.delta_short_pct_points,
                                snap.rank_position,
                                snap.peers_considered + 1,
                                snap.as_of,
                            ))
                            .strong()
                            .color(color),
                        );
                        ui.separator();
                        egui::Grid::new("shortrank_delta_summary")
                            .striped(true)
                            .num_columns(2)
                            .min_col_width(240.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("Window / history span").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}d / {} → {}",
                                        snap.lookback_days,
                                        snap.history_start_date,
                                        snap.history_end_date
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Short % float / delta")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2}% from {:.2}% ({:+.2} pts)",
                                        snap.latest_short_pct_of_float,
                                        snap.prior_short_pct_of_float,
                                        snap.delta_short_pct_points
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Short ratio / prior").small().strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.2} / {:.2}",
                                        snap.latest_short_ratio, snap.prior_short_ratio
                                    ))
                                    .small()
                                    .monospace(),
                                );
                                ui.end_row();
                                ui.label(
                                    egui::RichText::new("Sector median / p25 / p75 delta")
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:+.2} / {:+.2} / {:+.2}",
                                        snap.sector_median_delta_pct_pts,
                                        snap.sector_p25_delta_pct_pts,
                                        snap.sector_p75_delta_pct_pts
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
            self.show_shortrank_delta = open;
        }

        // INSIDERCONC — Insider ownership concentration vs sector peers
        if self.show_insiderconc {
            if self.insiderconc_symbol.is_empty() {
                self.insiderconc_symbol = chart_sym_research.clone();
            }
            let mut open = self.show_insiderconc;
            egui::Window::new("INSIDERCONC — Insider Ownership Concentration")
                .open(&mut open)
                .resizable(true)
                .default_size([720.0, 440.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.insiderconc_symbol)
                                .desired_width(100.0),
                        );
                        if ui.button("Use Chart").clicked() {
                            self.insiderconc_symbol = chart_sym_research.clone();
                        }
                        if ui.button("Load Cached").clicked() {
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let sym_u = self.insiderconc_symbol.to_uppercase();
                                    if let Ok(Some(snap)) =
                                        typhoon_engine::core::research::get_insiderconc(
                                            &conn, &sym_u,
                                        )
                                    {
                                        self.insiderconc_snapshot = snap;
                                        self.insiderconc_symbol = sym_u;
                                    }
                                }
                            }
                        }
                        if ui.add(egui::Button::new("Compute").fill(BTN_MG)).clicked() {
                            let sym = self.insiderconc_symbol.to_uppercase();
                            self.insiderconc_loading = true;
                            self.insiderconc_symbol = sym.clone();
                            let _ = self
                                .broker_tx
                                .send(BrokerCmd::ComputeInsiderconcSnapshot { symbol: sym });
                        }
                        if self.insiderconc_loading {
                            ui.label(egui::RichText::new("Loading…").color(AXIS_TEXT).small());
                        }
                    });
                    super::render::render_insiderconc_snapshot(ui, &self.insiderconc_snapshot);
                });
            self.show_insiderconc = open;
        }
    }
}
