use super::*;

impl TyphooNApp {
    pub(super) fn compute_bar_sync_rows(&mut self) -> Vec<SyncStatsRow> {
        let now = std::time::Instant::now();
        if !self.cached_bar_sync_rows.is_empty()
            && now.duration_since(self.cached_bar_sync_rows_last)
                < std::time::Duration::from_secs(1)
        {
            return self.cached_bar_sync_rows.clone();
        }
        let now_ms = chrono::Utc::now().timestamp_millis();
        let checked_or_complete_lookup = |key: &str| -> bool {
            let mut parts = key.splitn(3, ':');
            let Some(prefix) = parts.next() else {
                return false;
            };
            let Some(symbol) = parts.next() else {
                return false;
            };
            let Some(tf) = parts.next() else {
                return false;
            };
            // Kraken Spot WS OHLC snapshots/updates are authoritative liveness checks for
            // subscribed low-timeframe pairs. Illiquid pairs may have an old last trade,
            // but if WS just delivered the recent-window snapshot/update, the cache is in
            // sync; counting that row stale keeps auto full-tilt pinned forever and wastes
            // REST budget chasing bars the market has not printed.
            if prefix == "kraken"
                && Self::kraken_ws_pair_is_fresh_at(&self.kraken_ws_fresh_until, symbol, tf, now_ms)
            {
                return true;
            }
            let fetch_key = alpaca_fetch_key(symbol, tf);
            match prefix {
                "alpaca" => self.alpaca_backfill_complete_pairs.contains_key(&fetch_key),
                "kraken" | "kraken-equities" => {
                    self.kraken_backfill_complete_pairs.contains_key(&fetch_key)
                }
                "kraken-futures" => self
                    .kraken_futures_backfill_complete_pairs
                    .contains_key(&fetch_key),
                "tastytrade" => self
                    .tastytrade_backfill_complete_pairs
                    .contains_key(&fetch_key),
                _ => false,
            }
        };
        let mut rows = compute_bar_sync_stats(
            &self.bg.detailed_stats,
            &self.bg.bar_ts_cache,
            &checked_or_complete_lookup,
        );
        self.add_expected_kraken_sync_rows(&mut rows);
        sort_sync_stats_rows(&mut rows);
        let (total, healthy) = rows
            .iter()
            .fold((0u64, 0u64), |(t, h), row| (t + row.total, h + row.healthy));
        self.cached_bar_sync_overall_pct = if total == 0 {
            100.0
        } else {
            (healthy as f32 / total as f32) * 100.0
        };
        // Latched flag with hysteresis: engage below 97%, release at 99%.
        // Read by `full_tilt_sync_enabled` to keep request pressure high
        // until coverage actually catches up, then drop back to the balanced
        // cadence on AC and the battery-saving cadence on battery.
        let pct = self.cached_bar_sync_overall_pct;
        if self.auto_full_tilt_active {
            if pct >= 99.0 {
                self.auto_full_tilt_active = false;
            }
        } else if pct < 97.0 && total > 0 {
            self.auto_full_tilt_active = true;
        }
        self.cached_bar_sync_rows = rows;
        self.cached_bar_sync_rows_last = now;
        self.cached_bar_sync_rows.clone()
    }

    pub(super) fn render_sync_status_window(&mut self, ctx: &egui::Context) {
        if !self.show_sync_status {
            return;
        }
        let rows = self.compute_bar_sync_rows();
        let broker_totals = compute_bar_sync_broker_totals(&rows);
        let mut sync_save_after = false;
        let mut show_sync_status = self.show_sync_status;
        egui::Window::new("Sync Status")
            .open(&mut show_sync_status)
            .resizable(true).default_size([560.0, 480.0])
            .scroll([false, true])
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Bar sync % per broker / timeframe").color(AXIS_TEXT).small());
                ui.label(egui::RichText::new("healthy = last bar within 24× TF period · stale beyond · empty = cached blob has no bars").color(AXIS_TEXT).small());
                if self.alpaca_enabled {
                    self.render_alpaca_sync_profile_controls(ui, &mut sync_save_after, "sync_status");
                }
                self.render_sync_timeframe_controls(ui, &mut sync_save_after);
                ui.separator();

                // Per-broker summary chips
                ui.horizontal_wrapped(|ui| {
                    for (broker, total, healthy, pct) in &broker_totals {
                        let color = if *total == 0 {
                            egui::Color32::from_rgb(150, 150, 150)
                        } else if *pct >= 90.0 {
                            egui::Color32::from_rgb(26, 188, 156)
                        } else if *pct >= 50.0 {
                            egui::Color32::from_rgb(241, 196, 15)
                        } else {
                            egui::Color32::from_rgb(231, 76, 60)
                        };
                        ui.label(egui::RichText::new(format!(
                            "{}: {:.1}% ({}/{})",
                            broker, pct, healthy, total,
                        )).color(color).monospace().strong());
                        ui.label(egui::RichText::new("|").color(AXIS_TEXT));
                    }
                });
                ui.separator();

                egui::ScrollArea::vertical().id_salt("sync_scroll").auto_shrink(false).show(ui, |ui| {
                    egui::Grid::new("sync_grid").striped(true).num_columns(6).min_col_width(60.0).show(ui, |ui| {
                        ui.label(egui::RichText::new("Broker").color(AXIS_TEXT).small().strong());
                        ui.label(egui::RichText::new("TF").color(AXIS_TEXT).small().strong());
                        ui.label(egui::RichText::new("Symbols").color(AXIS_TEXT).small().strong());
                        ui.label(egui::RichText::new("Healthy").color(AXIS_TEXT).small().strong());
                        ui.label(egui::RichText::new("Stale").color(AXIS_TEXT).small().strong());
                        ui.label(egui::RichText::new("% Synced").color(AXIS_TEXT).small().strong());
                        ui.end_row();
                        for row in &rows {
                            let broker_color = match row.broker.as_str() {
                                "MT5"           => egui::Color32::from_rgb(26, 188, 156),
                                "Alpaca"        => egui::Color32::from_rgb(52, 152, 219),
                                "Tastytrade"    => egui::Color32::from_rgb(170, 100, 220),
                                "Kraken"        => egui::Color32::from_rgb(255, 130, 60),
                                _ => AXIS_TEXT,
                            };
                            ui.label(egui::RichText::new(&row.broker).color(broker_color).small().monospace().strong());
                            ui.label(egui::RichText::new(&row.tf).color(AXIS_TEXT).small().monospace());
                            ui.label(egui::RichText::new(format!("{}", row.total)).small());
                            ui.label(egui::RichText::new(format!("{}", row.healthy)).color(egui::Color32::from_rgb(26, 188, 156)).small());
                            ui.label(egui::RichText::new(format!("{}", row.stale + row.empty)).color(AXIS_TEXT).small());
                            let pct_color = if row.total == 0 {
                                egui::Color32::from_rgb(150, 150, 150)
                            } else if row.pct_healthy >= 90.0 {
                                egui::Color32::from_rgb(26, 188, 156)
                            } else if row.pct_healthy >= 50.0 {
                                egui::Color32::from_rgb(241, 196, 15)
                            } else {
                                egui::Color32::from_rgb(231, 76, 60)
                            };
                            ui.label(egui::RichText::new(format!("{:.1}%", row.pct_healthy))
                                .color(pct_color).small().strong());
                            ui.end_row();
                        }
                    });
                });
            });
        self.show_sync_status = show_sync_status;
        if sync_save_after {
            self.save_session();
        }
    }

    fn add_expected_kraken_sync_rows(&self, rows: &mut Vec<SyncStatsRow>) {
        let timeframes = self.enabled_standard_sync_timeframes();
        if timeframes.is_empty() {
            return;
        }
        let existing: std::collections::HashSet<String> = self
            .bg
            .detailed_stats
            .iter()
            .map(|(key, _, _)| key.clone())
            .collect();
        let spot_symbols: Vec<String> = self
            .kraken_sync_symbol_sectors()
            .into_iter()
            .flatten()
            .collect();
        let expected_sources = [
            ("kraken", spot_symbols),
            ("kraken-equities", self.kraken_equity_sync_symbols()),
            ("kraken-futures", self.kraken_futures_sync_symbols()),
        ];

        for (source, symbols) in expected_sources {
            for symbol in symbols {
                for tf in &timeframes {
                    let Some(tf) = normalize_sync_timeframe_key(tf) else {
                        continue;
                    };
                    // Equities/iapi is the rate-limit bottleneck. The broad
                    // universe lane only targets D1/W1/MN1; intraday history
                    // remains demand/focus-driven so the sync grid does not
                    // manufacture tens of thousands of slow, low-value expected
                    // rows and hold full-tilt open for days.
                    if source == "kraken-equities" && !kraken_equity_full_universe_timeframe(tf) {
                        continue;
                    }
                    if existing.contains(&format!("{source}:{symbol}:{tf}")) {
                        continue;
                    }
                    if let Some(row) = rows
                        .iter_mut()
                        .find(|row| row.broker == "Kraken" && row.tf == tf)
                    {
                        row.total += 1;
                        row.empty += 1;
                        row.pct_healthy = if row.total == 0 {
                            0.0
                        } else {
                            (row.healthy as f32 / row.total as f32) * 100.0
                        };
                    } else {
                        rows.push(SyncStatsRow {
                            broker: "Kraken".to_string(),
                            tf: tf.to_string(),
                            total: 1,
                            healthy: 0,
                            stale: 0,
                            empty: 1,
                            pct_healthy: 0.0,
                        });
                    }
                }
            }
        }
    }
}
