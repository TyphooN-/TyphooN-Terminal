use super::*;

impl TyphooNApp {
    pub(super) fn render_research_squeeze_breakout_channels_windows(
        &mut self,
        ctx: &egui::Context,
    ) {
        let chart_sym_research =
            research_chart_symbol(self.charts.get(self.active_tab).map(|c| c.symbol.as_str()));

        // ── Research section ──
        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "SQUEEZE — Short-Squeeze Composite",
                default_size: [560.0, 360.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_squeeze_win,
            &mut self.squeeze_win_symbol,
            &mut self.squeeze_win_loading,
            &mut self.squeeze_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_squeeze(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeSqueezeSnapshot { symbol },
            super::render::render_squeeze_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "SQUEEZERANK — Cross-Symbol Squeeze Percentile",
                default_size: [520.0, 260.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_squeezerank,
            &mut self.squeezerank_symbol,
            &mut self.squeezerank_loading,
            &mut self.squeezerank_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_squeezerank(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeSqueezeRankSnapshot { symbol },
            super::render::render_squeezerank_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
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

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "BBSQUEEZE — Bollinger-Band Width Squeeze",
                default_size: [520.0, 300.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_bbsqueeze,
            &mut self.bbsqueeze_symbol,
            &mut self.bbsqueeze_loading,
            &mut self.bbsqueeze_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_bbsqueeze(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeBbsqueezeSnapshot { symbol },
            super::render::render_bbsqueeze_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "DONCHIAN — 20-Bar Channel Breakout",
                default_size: [520.0, 300.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_donchian_win,
            &mut self.donchian_win_symbol,
            &mut self.donchian_win_loading,
            &mut self.donchian_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_donchian(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeDonchianSnapshot { symbol },
            super::render::render_donchian_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }

        if let Some(cmd) = window_shell::render_compute_window(
            ctx,
            window_shell::ComputeWindow {
                title: "KAMA — Kaufman Adaptive MA / Efficiency Ratio",
                default_size: [520.0, 280.0],
                max_size: None,
                chart_symbol: &chart_sym_research,
                cache: self.cache.as_deref(),
            },
            &mut self.show_kama_win,
            &mut self.kama_win_symbol,
            &mut self.kama_win_loading,
            &mut self.kama_win_snapshot,
            |conn, sym| {
                typhoon_engine::core::research::get_kama(conn, sym)
                    .ok()
                    .flatten()
            },
            |symbol| BrokerCmd::ComputeKamaSnapshot { symbol },
            super::render::render_kama_snapshot,
        ) {
            let _ = self.broker_tx.send(cmd);
        }
    }
}
