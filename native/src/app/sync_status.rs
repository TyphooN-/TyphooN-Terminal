use super::market_data_sync::kraken_equity_symbols_for_timeframe;
use super::*;

const BAR_SYNC_STATS_IDLE_REFRESH: std::time::Duration = std::time::Duration::from_secs(1);
const BAR_SYNC_STATS_HEAVY_REFRESH: std::time::Duration = std::time::Duration::from_secs(15);

impl TyphooNApp {
    #[inline]
    pub(super) fn refresh_bar_sync_rows_if_stale(&mut self) {
        let now = std::time::Instant::now();
        let refresh_interval = if self.heavy_sync_in_progress {
            BAR_SYNC_STATS_HEAVY_REFRESH
        } else {
            BAR_SYNC_STATS_IDLE_REFRESH
        };
        if !self.cached_bar_sync_rows.is_empty()
            && now.duration_since(self.cached_bar_sync_rows_last) < refresh_interval
        {
            return;
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
        self.add_kraken_equities_merged_rows(&mut rows, &checked_or_complete_lookup);
        sort_sync_stats_rows(&mut rows);
        let (total, healthy) = rows
            .iter()
            .filter(|row| row.broker != "Merged")
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
    }

    pub(super) fn compute_bar_sync_rows(&mut self) -> Vec<SyncStatsRow> {
        self.refresh_bar_sync_rows_if_stale();
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
                                "Merged"        => egui::Color32::from_rgb(0, 220, 220),
                                "Yahoo"         => egui::Color32::from_rgb(155, 89, 182),
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

                let now = chrono::Utc::now().timestamp();
                if self.kraken_equities_sync_pause_until_ts > now {
                    ui.separator();
                    ui.label(egui::RichText::new(format!(
                        "Kraken equities sync paused for {}s: {}",
                        self.kraken_equities_sync_pause_until_ts - now,
                        self.kraken_equities_sync_pause_reason
                    )).color(egui::Color32::from_rgb(231, 76, 60)).small());
                }
                if self.yahoo_chart_sync_pause_until_ts > now {
                    ui.separator();
                    ui.label(egui::RichText::new(format!(
                        "Yahoo Chart sync paused for {}s: {}",
                        self.yahoo_chart_sync_pause_until_ts - now,
                        self.yahoo_chart_sync_pause_reason
                    )).color(egui::Color32::from_rgb(231, 76, 60)).small());
                }
                if self.tastytrade_sync_pause_until_ts > now {
                    ui.separator();
                    ui.label(egui::RichText::new(format!(
                        "tastytrade sync paused for {}s: {}",
                        self.tastytrade_sync_pause_until_ts - now,
                        self.tastytrade_sync_pause_reason
                    )).color(egui::Color32::from_rgb(231, 76, 60)).small());
                }
            });
        self.show_sync_status = show_sync_status;
        if sync_save_after {
            self.save_session();
        }
    }

    fn add_kraken_equities_merged_rows(
        &self,
        rows: &mut Vec<SyncStatsRow>,
        checked_or_complete_lookup: &dyn Fn(&str) -> bool,
    ) {
        let timeframes = self.enabled_standard_sync_timeframes();
        if timeframes.is_empty() {
            return;
        }
        let catalog_symbols = self.kraken_equity_catalog_symbols();
        let demand_symbols = self.kraken_equity_demand_symbols();
        if catalog_symbols.is_empty() && demand_symbols.is_empty() {
            return;
        }
        let now_ms = chrono::Utc::now().timestamp_millis();
        let mut detailed: std::collections::HashMap<&str, (i64, i64)> =
            std::collections::HashMap::with_capacity(self.bg.detailed_stats.len());
        for (key, bar_count, write_ts_s) in &self.bg.detailed_stats {
            detailed.insert(key.as_str(), (*bar_count, *write_ts_s));
        }

        for raw_tf in timeframes {
            let Some(tf) = normalize_sync_timeframe_key(&raw_tf) else {
                continue;
            };
            if !self.kraken_equities_merged_source_supported(tf) {
                continue;
            }
            let symbols =
                kraken_equity_symbols_for_timeframe(&catalog_symbols, &demand_symbols, tf);
            if symbols.is_empty() {
                continue;
            }
            let mut healthy = 0u64;
            let mut stale = 0u64;
            let mut empty = 0u64;
            for symbol in &symbols {
                let symbol = normalize_market_data_symbol(symbol)
                    .replace('/', "")
                    .trim_end_matches(".EQ")
                    .to_ascii_uppercase();
                if symbol.is_empty() {
                    continue;
                }
                let status = self.kraken_equities_merged_symbol_status(
                    &symbol,
                    tf,
                    now_ms,
                    &detailed,
                    checked_or_complete_lookup,
                );
                match status {
                    MergedSyncStatus::Healthy => healthy += 1,
                    MergedSyncStatus::Stale => stale += 1,
                    MergedSyncStatus::Empty => empty += 1,
                }
            }
            let total = healthy + stale + empty;
            let pct_healthy = if total == 0 {
                0.0
            } else {
                (healthy as f32 / total as f32) * 100.0
            };
            rows.push(SyncStatsRow {
                broker: "Merged".to_string(),
                tf: tf.to_string(),
                total,
                healthy,
                stale,
                empty,
                pct_healthy,
            });
        }
    }

    fn kraken_equities_merged_source_supported(&self, tf: &str) -> bool {
        if !kraken_equities_merged_timeframe_supported(tf) {
            return false;
        }
        kraken_equity_full_universe_timeframe(tf)
            || (self.backfill_alpaca_kraken_equities_enabled
                && kraken_equity_broad_fallback_timeframe(tf)
                && alpaca_sync_target_bars(tf).is_some())
            || (self.backfill_yahoo_chart_enabled && yahoo_chart_supports_timeframe(tf))
    }

    fn kraken_equities_merged_symbol_status(
        &self,
        symbol: &str,
        tf: &str,
        now_ms: i64,
        detailed: &std::collections::HashMap<&str, (i64, i64)>,
        checked_or_complete_lookup: &dyn Fn(&str) -> bool,
    ) -> MergedSyncStatus {
        let mut saw_stale = false;
        for source in ["kraken-equities", "alpaca", "yahoo-chart"] {
            if source == "kraken-equities" && !kraken_equity_full_universe_timeframe(tf) {
                continue;
            }
            if source == "alpaca"
                && (!self.backfill_alpaca_kraken_equities_enabled
                    || !kraken_equity_broad_fallback_timeframe(tf)
                    || alpaca_sync_target_bars(tf).is_none())
            {
                continue;
            }
            if source == "yahoo-chart"
                && (!self.backfill_yahoo_chart_enabled || !yahoo_chart_supports_timeframe(tf))
            {
                continue;
            }

            let key = format!("{source}:{symbol}:{tf}");
            let Some((bar_count, write_ts_s)) = detailed.get(key.as_str()).copied() else {
                continue;
            };
            if bar_count <= 0 {
                continue;
            }
            let last_ms = self
                .bg
                .bar_ts_cache
                .get(&key)
                .map(|(_, last_ms, _)| *last_ms)
                .filter(|last_ms| *last_ms > 0)
                .unwrap_or_else(|| write_ts_s.saturating_mul(1000));
            if last_ms <= 0 {
                continue;
            }
            let Some(period_ms) = merged_sync_period_ms(tf) else {
                saw_stale = true;
                continue;
            };
            let write_ms = write_ts_s.saturating_mul(1000);
            let recently_checked = write_ms > 0 && now_ms - write_ms <= period_ms * 24;
            let bar_aged_out = now_ms - last_ms > period_ms * 24;
            if bar_aged_out && !recently_checked && !checked_or_complete_lookup(&key) {
                saw_stale = true;
            } else {
                return MergedSyncStatus::Healthy;
            }
        }
        if saw_stale {
            MergedSyncStatus::Stale
        } else {
            MergedSyncStatus::Empty
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
        let futures_symbols = self.kraken_futures_sync_symbols();
        let kraken_equity_catalog_symbols = self.kraken_equity_catalog_symbols();
        let kraken_equity_demand_symbols = self.kraken_equity_demand_symbols();
        let mut expected_sources: Vec<(&str, &str)> = vec![
            ("kraken", "Kraken"),
            ("kraken-equities", "Kraken"),
            ("kraken-futures", "Kraken"),
        ];
        if self.backfill_alpaca_kraken_equities_enabled {
            expected_sources.push(("alpaca", "Alpaca"));
        }
        if self.backfill_yahoo_chart_enabled {
            expected_sources.push(("yahoo-chart", "Yahoo"));
        }

        for (source, broker) in expected_sources {
            for tf in &timeframes {
                let Some(tf) = normalize_sync_timeframe_key(tf) else {
                    continue;
                };
                // Equities/iapi is the rate-limit bottleneck. Native Kraken
                // rows remain high-TF only, while Alpaca/Yahoo assist rows can
                // represent broad 15Min+ catalog coverage where those provider
                // lanes are enabled.
                if source == "kraken-equities" && !kraken_equity_full_universe_timeframe(tf) {
                    continue;
                }
                if source == "alpaca"
                    && (!kraken_equity_broad_fallback_timeframe(tf)
                        || alpaca_sync_target_bars(tf).is_none())
                {
                    continue;
                }

                if source == "yahoo-chart" && !yahoo_chart_supports_timeframe(tf) {
                    continue;
                }
                let symbols: Vec<String> = match source {
                    "kraken" => spot_symbols.clone(),
                    "kraken-futures" => futures_symbols.clone(),
                    "kraken-equities" | "alpaca" | "yahoo-chart" => {
                        kraken_equity_symbols_for_timeframe(
                            &kraken_equity_catalog_symbols,
                            &kraken_equity_demand_symbols,
                            tf,
                        )
                    }
                    _ => Vec::new(),
                };
                for symbol in symbols {
                    if existing.contains(&format!("{source}:{symbol}:{tf}")) {
                        continue;
                    }
                    if let Some(row) = rows
                        .iter_mut()
                        .find(|row| row.broker == broker && row.tf == tf)
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
                            broker: broker.to_string(),
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MergedSyncStatus {
    Healthy,
    Stale,
    Empty,
}

fn merged_sync_period_ms(tf: &str) -> Option<i64> {
    match normalize_sync_timeframe_key(tf)? {
        "1Min" => Some(60_000),
        "5Min" => Some(300_000),
        "15Min" => Some(900_000),
        "30Min" => Some(1_800_000),
        "1Hour" => Some(3_600_000),
        "4Hour" => Some(14_400_000),
        "1Day" => Some(86_400_000),
        "1Week" => Some(604_800_000),
        "1Month" => Some(2_592_000_000),
        _ => None,
    }
}

fn kraken_equities_merged_timeframe_supported(tf: &str) -> bool {
    kraken_equity_full_universe_timeframe(tf) || kraken_equity_broad_fallback_timeframe(tf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merged_sync_rows_skip_demand_only_low_timeframes() {
        assert!(!kraken_equities_merged_timeframe_supported("1Min"));
        assert!(!kraken_equities_merged_timeframe_supported("5Min"));
        assert!(kraken_equities_merged_timeframe_supported("15Min"));
        assert!(kraken_equities_merged_timeframe_supported("1Day"));
        assert!(kraken_equities_merged_timeframe_supported("1Month"));
    }
}
