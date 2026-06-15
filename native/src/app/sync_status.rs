use super::market_data_sync::{
    kraken_equity_native_symbols_for_timeframe, kraken_equity_symbols_for_timeframe,
};
use super::*;

const BAR_SYNC_STATS_VISIBLE_REFRESH: std::time::Duration = std::time::Duration::from_secs(1);
const BAR_SYNC_STATS_HIDDEN_REFRESH: std::time::Duration = std::time::Duration::from_secs(15);
// Broad xStocks/Merged coverage refresh scans the whole catalog across enabled
// timeframes. During heavy sync the scheduler has its own cached worksets; the
// Sync Status snapshot is informational and should not burn the UI thread every
// 30s while 10k+ symbols are catching up.
const BAR_SYNC_STATS_HEAVY_REFRESH: std::time::Duration = std::time::Duration::from_secs(120);

fn bar_sync_stats_refresh_interval_for_broad_symbol_count(
    heavy_sync_in_progress: bool,
    sync_status_visible: bool,
    broad_symbol_count: usize,
) -> std::time::Duration {
    if heavy_sync_in_progress {
        BAR_SYNC_STATS_HEAVY_REFRESH
    } else if sync_status_visible {
        if broad_symbol_count > 2_048 {
            BAR_SYNC_STATS_HIDDEN_REFRESH
        } else {
            BAR_SYNC_STATS_VISIBLE_REFRESH
        }
    } else {
        BAR_SYNC_STATS_HIDDEN_REFRESH
    }
}

impl TyphooNApp {
    #[inline]
    pub(super) fn refresh_bar_sync_rows_if_stale(&mut self) {
        let now = std::time::Instant::now();
        let refresh_interval = bar_sync_stats_refresh_interval_for_broad_symbol_count(
            self.heavy_sync_in_progress,
            self.show_sync_status,
            self.kraken_equity_catalog_symbol_count(),
        );
        if self.bar_sync_compute_rx.is_some() {
            // A snapshot compute is already running on a worker — don't stack another.
            return;
        }
        if !self.cached_bar_sync_rows.is_empty()
            && now.duration_since(self.cached_bar_sync_rows_last) < refresh_interval
        {
            return;
        }
        // The bar-sync matrix scan (full xStocks/Merged catalog × enabled
        // timeframes) is hundreds of ms of pure CPU on a 12k-symbol universe and
        // must never run on the render thread. Snapshot the inputs (cheap next to
        // the scan itself) and compute on a blocking worker; `poll_bar_sync_compute`
        // applies the finished rows + coverage % on a later frame.
        let inputs = self.build_bar_sync_inputs();
        let (tx, rx) = std::sync::mpsc::channel();
        self.bar_sync_compute_rx = Some(rx);
        self.rt_handle.spawn_blocking(move || {
            let _ = tx.send(inputs.compute());
        });
    }

    /// Snapshot every input the bar-sync matrix scan reads into an owned, `Send`
    /// struct so the scan can run off the render thread. The clones here
    /// (detailed-stats, bar-ts cache, backfill key sets) are O(rows) but far
    /// cheaper than the per-symbol×timeframe×source status scan they feed.
    fn build_bar_sync_inputs(&self) -> BarSyncInputs {
        BarSyncInputs {
            detailed_stats: self.bg.detailed_stats.clone(),
            bar_ts_cache: self.bg.bar_ts_cache.clone(),
            cache_stats_present: self.bg.cache_stats.is_some(),
            catalog_symbol_count: self.kraken_equity_catalog_symbol_count() as u64,
            catalog_symbols: self.kraken_equity_catalog_symbols(),
            demand_symbols: self.kraken_equity_demand_symbols(),
            ws_sweep_symbols: self.kraken_equity_ws_sweep_symbols(),
            spot_symbols: self
                .kraken_sync_symbol_sectors()
                .into_iter()
                .flatten()
                .collect(),
            futures_symbols: self.kraken_futures_sync_symbols(),
            timeframes: self.enabled_standard_sync_timeframes(),
            backfill_alpaca_kraken_equities_enabled: self.backfill_alpaca_kraken_equities_enabled,
            backfill_yahoo_chart_enabled: self.backfill_yahoo_chart_enabled,
            kraken_ws_fresh_until: self.kraken_ws_fresh_until.clone(),
            alpaca_backfill_keys: self.alpaca_backfill_complete_pairs.keys().cloned().collect(),
            kraken_backfill_keys: self.kraken_backfill_complete_pairs.keys().cloned().collect(),
            kraken_futures_backfill_keys: self
                .kraken_futures_backfill_complete_pairs
                .keys()
                .cloned()
                .collect(),
        }
    }

    /// Apply the result of an off-thread bar-sync recompute, if one has
    /// finished. Cheap: a non-blocking channel poll plus a move of the rows.
    pub(super) fn poll_bar_sync_compute(&mut self) {
        let Some(rx) = self.bar_sync_compute_rx.as_ref() else {
            return;
        };
        match rx.try_recv() {
            Ok(result) => {
                self.cached_bar_sync_overall_pct = result.overall_pct;
                // Latched flag with hysteresis: engage below 97%, release at 99%.
                // Read by `full_tilt_sync_enabled` to keep request pressure high
                // until coverage actually catches up, then drop back to the
                // balanced cadence on AC and the battery-saving cadence on battery.
                if self.auto_full_tilt_active {
                    if result.overall_pct >= 99.0 {
                        self.auto_full_tilt_active = false;
                    }
                } else if result.overall_pct < 97.0 && result.total > 0 {
                    self.auto_full_tilt_active = true;
                }
                self.cached_bar_sync_rows = result.rows;
                self.cached_bar_sync_rows_last = std::time::Instant::now();
                self.bar_sync_compute_rx = None;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                // Worker dropped the sender without sending (should not happen);
                // clear the slot so a later frame can retry.
                self.bar_sync_compute_rx = None;
            }
        }
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
            .resizable(true).default_size([760.0, 480.0])
            .scroll([false, true])
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Bar sync % per broker / timeframe").color(AXIS_TEXT).small());
                ui.label(egui::RichText::new("healthy = fresh or provider-settled · unhealthy = stale or empty").color(AXIS_TEXT).small());
                self.render_sync_timeframe_controls(ui, &mut sync_save_after);
                let tradable_count = self.kraken_equity_catalog_symbol_count();
                if tradable_count > 0 {
                    ui.label(
                        egui::RichText::new(format!(
                            "Kraken Equities (Tradable): {tradable_count} catalog symbols. This is the denominator for Merged plus Alpaca/Yahoo assist target lists; native Kraken Equities history rows stay demand-scoped."
                        ))
                        .color(AXIS_TEXT)
                        .small(),
                    );
                }
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
                    if rows.is_empty() {
                        ui.label(
                            egui::RichText::new("Cache metadata is still loading; sync health will appear after the storage snapshot is available.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        return;
                    }
                    egui::Grid::new("sync_grid").striped(true).num_columns(6).min_col_width(60.0).show(ui, |ui| {
                        ui.label(egui::RichText::new("Broker").color(AXIS_TEXT).small().strong());
                        ui.label(egui::RichText::new("TF").color(AXIS_TEXT).small().strong());
                        ui.label(egui::RichText::new("Symbols").color(AXIS_TEXT).small().strong());
                        ui.label(egui::RichText::new("Healthy").color(AXIS_TEXT).small().strong());
                        ui.label(egui::RichText::new("Unhealthy").color(AXIS_TEXT).small().strong());
                        ui.label(egui::RichText::new("% Synced").color(AXIS_TEXT).small().strong());
                        ui.end_row();
                        for row in &rows {
                            let broker_color = match row.broker.as_str() {
                                "Alpaca"        => egui::Color32::from_rgb(52, 152, 219),
                                "Kraken Spot" | "Kraken Equities" | "Kraken Equities (Tradable)" | "Kraken Futures" => egui::Color32::from_rgb(255, 130, 60),
                                "Merged"        => egui::Color32::from_rgb(0, 220, 220),
                                "Yahoo"         => egui::Color32::from_rgb(155, 89, 182),
                                _ => AXIS_TEXT,
                            };
                            let cells = sync_stats_row_status_cells(row);
                            let broker_response = ui.label(egui::RichText::new(&row.broker).color(broker_color).small().monospace().strong());
                            if let Some(note) = row.note.as_deref() {
                                broker_response.on_hover_text(note);
                            }
                            ui.label(egui::RichText::new(&row.tf).color(AXIS_TEXT).small().monospace());
                            ui.label(egui::RichText::new(cells.symbols).small());
                            let healthy_response = ui.label(egui::RichText::new(cells.healthy).color(egui::Color32::from_rgb(26, 188, 156)).small());
                            if row.settled > 0 {
                                healthy_response.on_hover_text(format!(
                                    "Includes {} settled row(s): provider history/window was recently checked or exhausted, so refetching now is not expected to produce newer historical bars.",
                                    row.settled
                                ));
                            }
                            let unhealthy_response = ui.label(egui::RichText::new(cells.stale_or_empty).color(AXIS_TEXT).small());
                            if row.stale > 0 || row.empty > 0 {
                                unhealthy_response.on_hover_text(format!(
                                    "Unhealthy = stale + empty. Stale: {} cached symbol/timeframe rows have aged beyond the freshness window and need a refresh/check. Empty: {} expected rows have no usable bars cached yet.",
                                    row.stale, row.empty
                                ));
                            }
                            let pct_color = if sync_stats_row_is_informational(row) {
                                AXIS_TEXT
                            } else if row.total == 0 {
                                egui::Color32::from_rgb(150, 150, 150)
                            } else if row.pct_healthy >= 90.0 {
                                egui::Color32::from_rgb(26, 188, 156)
                            } else if row.pct_healthy >= 50.0 {
                                egui::Color32::from_rgb(241, 196, 15)
                            } else {
                                egui::Color32::from_rgb(231, 76, 60)
                            };
                            let pct_text = match (sync_stats_row_is_informational(row), row.total, row.note.as_deref()) {
                                (true, _, _) => "catalog".to_string(),
                                (false, 0, Some(note)) => note.to_string(),
                                _ => format!("{:.1}%", row.pct_healthy),
                            };
                            ui.label(
                                egui::RichText::new(pct_text)
                                    .color(pct_color)
                                    .small()
                                    .strong(),
                            );
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
            });
        self.show_sync_status = show_sync_status;
        if sync_save_after {
            self.save_session();
        }
    }
}

/// Owned snapshot of every app input the bar-sync matrix scan reads, so the
/// scan (hundreds of ms on a 12k-symbol universe) can run on a blocking worker
/// instead of the render thread. Built by `TyphooNApp::build_bar_sync_inputs`.
pub(super) struct BarSyncInputs {
    detailed_stats: Vec<(String, i64, i64)>,
    bar_ts_cache: std::collections::HashMap<String, (i64, i64, i64)>,
    cache_stats_present: bool,
    catalog_symbol_count: u64,
    catalog_symbols: Vec<String>,
    demand_symbols: Vec<String>,
    ws_sweep_symbols: Vec<String>,
    spot_symbols: Vec<String>,
    futures_symbols: Vec<String>,
    timeframes: Vec<String>,
    backfill_alpaca_kraken_equities_enabled: bool,
    backfill_yahoo_chart_enabled: bool,
    kraken_ws_fresh_until: std::collections::HashMap<(String, String), i64>,
    alpaca_backfill_keys: std::collections::HashSet<String>,
    kraken_backfill_keys: std::collections::HashSet<String>,
    kraken_futures_backfill_keys: std::collections::HashSet<String>,
}

/// Result of an off-thread bar-sync recompute, applied by `poll_bar_sync_compute`.
pub(crate) struct BarSyncResult {
    rows: Vec<SyncStatsRow>,
    overall_pct: f32,
    total: u64,
}

impl BarSyncInputs {
    /// Run the full bar-sync matrix scan. Pure CPU over the owned snapshot — no
    /// app state, no I/O — so it is safe to call from a blocking worker thread.
    pub(super) fn compute(self) -> BarSyncResult {
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
            if matches!(prefix, "kraken" | "kraken-equities")
                && TyphooNApp::kraken_ws_pair_is_fresh_at(
                    &self.kraken_ws_fresh_until,
                    symbol,
                    tf,
                    now_ms,
                )
            {
                return true;
            }
            let fetch_key = alpaca_fetch_key(symbol, tf);
            match prefix {
                "alpaca" => self.alpaca_backfill_keys.contains(&fetch_key),
                "kraken" | "kraken-equities" => self.kraken_backfill_keys.contains(&fetch_key),
                "kraken-futures" => self.kraken_futures_backfill_keys.contains(&fetch_key),
                _ => false,
            }
        };
        let mut rows = compute_bar_sync_stats(
            &self.detailed_stats,
            &self.bar_ts_cache,
            &checked_or_complete_lookup,
        );
        self.add_kraken_equities_tradable_catalog_row(&mut rows);
        self.add_expected_kraken_sync_rows(&mut rows);
        self.add_kraken_equities_merged_rows(&mut rows, &checked_or_complete_lookup);
        sort_sync_stats_rows(&mut rows);
        let (total, healthy) = rows
            .iter()
            .filter(|row| row.broker != "Merged" && !sync_stats_row_is_informational(row))
            .fold((0u64, 0u64), |(t, h), row| (t + row.total, h + row.healthy));
        let overall_pct = if total == 0 {
            100.0
        } else {
            (healthy as f32 / total as f32) * 100.0
        };
        BarSyncResult {
            rows,
            overall_pct,
            total,
        }
    }

    fn add_kraken_equities_tradable_catalog_row(&self, rows: &mut Vec<SyncStatsRow>) {
        let total = self.catalog_symbol_count;
        if total == 0 {
            return;
        }
        rows.push(SyncStatsRow {
            broker: "Kraken Equities (Tradable)".to_string(),
            tf: "Catalog".to_string(),
            total,
            healthy: total,
            stale: 0,
            empty: 0,
            settled: 0,
            note: Some(
                "Full Kraken Securities/xStocks tradable catalog. This reference universe forms the Merged, Alpaca assist, and Yahoo assist sync target lists; native Kraken Equities history rows remain demand-scoped."
                    .to_string(),
            ),
            pct_healthy: 100.0,
        });
    }

    fn add_kraken_equities_merged_rows(
        &self,
        rows: &mut Vec<SyncStatsRow>,
        checked_or_complete_lookup: &dyn Fn(&str) -> bool,
    ) {
        let timeframes = self.timeframes.clone();
        if timeframes.is_empty() {
            return;
        }
        let catalog_symbols = self.catalog_symbols.clone();
        let demand_symbols = self.demand_symbols.clone();
        if catalog_symbols.is_empty() && demand_symbols.is_empty() {
            return;
        }
        let now_ms = chrono::Utc::now().timestamp_millis();
        let mut detailed: std::collections::HashMap<&str, (i64, i64)> =
            std::collections::HashMap::with_capacity(self.detailed_stats.len());
        for (key, bar_count, write_ts_s) in &self.detailed_stats {
            detailed.insert(key.as_str(), (*bar_count, *write_ts_s));
        }

        for raw_tf in timeframes {
            let Some(tf) = normalize_sync_timeframe_key(&raw_tf) else {
                continue;
            };
            if !self.kraken_equities_merged_source_supported(tf) {
                continue;
            }
            let symbols = self.kraken_equities_merged_symbols_for_timeframe(
                &catalog_symbols,
                &demand_symbols,
                tf,
            );
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
                settled: 0,
                note: None,
                pct_healthy,
            });
        }
    }

    fn kraken_equities_merged_symbols_for_timeframe(
        &self,
        catalog_symbols: &[String],
        demand_symbols: &[String],
        tf: &str,
    ) -> Vec<String> {
        // Full-catalog M1/M5 is not reachable today: Alpaca assist is disabled
        // for those rows, Yahoo assist is unsupported, and native Kraken WS only
        // exists for tokenized xStocks. Keep the Merged denominator honest so
        // Sync Status does not show a permanent 1% red row and tempt the
        // scheduler into wasting assist-provider RPM on ignored low-TF rows.
        if matches!(tf, "1Min" | "5Min") {
            let ws_symbols = self.ws_sweep_symbols.clone();
            if !ws_symbols.is_empty() {
                return ws_symbols;
            }
            return demand_symbols.to_vec();
        }
        kraken_equity_symbols_for_timeframe(catalog_symbols, demand_symbols, tf)
    }

    fn kraken_equities_merged_source_supported(&self, tf: &str) -> bool {
        if !kraken_equities_merged_timeframe_supported(tf) {
            return false;
        }
        kraken_equity_full_universe_timeframe(tf)
            || (tf == "1Month" && kraken_equity_full_universe_timeframe("1Day"))
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
        let merged_key = chart_merged_equity_cache_key(symbol, tf);
        if let Some((bar_count, write_ts_s)) = detailed.get(merged_key.as_str()).copied() {
            if bar_count > 0 {
                let last_ms = self
                    .bar_ts_cache
                    .get(&merged_key)
                    .map(|(_, last_ms, _)| *last_ms)
                    .filter(|last_ms| *last_ms > 0)
                    .unwrap_or_else(|| write_ts_s.saturating_mul(1000));
                if let Some(period_ms) = merged_sync_period_ms(tf) {
                    let write_ms = write_ts_s.saturating_mul(1000);
                    let recently_checked = write_ms > 0 && now_ms - write_ms <= period_ms * 24;
                    let bar_aged_out = now_ms - last_ms > period_ms * 24;
                    if bar_aged_out && !recently_checked && !checked_or_complete_lookup(&merged_key)
                    {
                        saw_stale = true;
                    } else {
                        return MergedSyncStatus::Healthy;
                    }
                } else {
                    saw_stale = true;
                }
            }
        }
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
        let timeframes = self.timeframes.clone();
        if timeframes.is_empty()
            || (!self.cache_stats_present && self.detailed_stats.is_empty())
        {
            return;
        }
        let existing: std::collections::HashSet<String> = self
            .detailed_stats
            .iter()
            .map(|(key, _, _)| key.clone())
            .collect();
        let spot_symbols: Vec<String> = self.spot_symbols.clone();
        let futures_symbols = self.futures_symbols.clone();
        let kraken_equity_catalog_symbols = self.catalog_symbols.clone();
        let kraken_equity_demand_symbols = self.demand_symbols.clone();
        let mut expected_sources: Vec<(&str, &str)> = vec![
            ("kraken", "Kraken Spot"),
            ("kraken-equities", "Kraken Equities"),
            ("kraken-futures", "Kraken Futures"),
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
                // Kraken Equities/xStocks is WS-first through W1. Monthly rows
                // are constructed-only and belong under Merged, not native Kraken
                // provider rows. Alpaca/Yahoo assist rows remain broad 15Min+
                // only where those provider lanes are enabled.
                if source == "kraken-equities" && !kraken_equity_full_universe_timeframe(tf) {
                    continue;
                }
                if matches!(source, "kraken" | "kraken-futures") && tf == "1Month" {
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
                    "kraken-equities" => kraken_equity_native_symbols_for_timeframe(
                        &kraken_equity_catalog_symbols,
                        &kraken_equity_demand_symbols,
                        tf,
                    ),
                    "alpaca" | "yahoo-chart" => kraken_equity_symbols_for_timeframe(
                        &kraken_equity_catalog_symbols,
                        &kraken_equity_demand_symbols,
                        tf,
                    ),
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
                            settled: 0,
                            note: None,
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
    fn merged_sync_rows_support_native_and_constructed_kraken_equities_timeframes() {
        assert!(kraken_equities_merged_timeframe_supported("1Min"));
        assert!(kraken_equities_merged_timeframe_supported("5Min"));
        assert!(kraken_equities_merged_timeframe_supported("15Min"));
        assert!(kraken_equities_merged_timeframe_supported("1Day"));
        assert!(kraken_equities_merged_timeframe_supported("1Month"));
    }

    #[test]
    fn bar_sync_refresh_keeps_visible_fast_but_hidden_background_slow() {
        assert_eq!(
            bar_sync_stats_refresh_interval_for_broad_symbol_count(false, true, 0),
            BAR_SYNC_STATS_VISIBLE_REFRESH
        );
        assert_eq!(
            bar_sync_stats_refresh_interval_for_broad_symbol_count(false, false, 0),
            BAR_SYNC_STATS_HIDDEN_REFRESH
        );
        assert_eq!(
            bar_sync_stats_refresh_interval_for_broad_symbol_count(true, true, 0),
            BAR_SYNC_STATS_HEAVY_REFRESH
        );
        assert_eq!(
            bar_sync_stats_refresh_interval_for_broad_symbol_count(true, false, 0),
            BAR_SYNC_STATS_HEAVY_REFRESH
        );
        assert_eq!(
            bar_sync_stats_refresh_interval_for_broad_symbol_count(false, true, 12_312),
            BAR_SYNC_STATS_HIDDEN_REFRESH,
            "visible Sync Status should not rebuild 12k-symbol broad coverage every second"
        );
        assert_eq!(
            bar_sync_stats_refresh_interval_for_broad_symbol_count(false, true, 128),
            BAR_SYNC_STATS_VISIBLE_REFRESH
        );
        assert!(BAR_SYNC_STATS_HIDDEN_REFRESH > BAR_SYNC_STATS_VISIBLE_REFRESH);
        assert!(BAR_SYNC_STATS_HEAVY_REFRESH >= std::time::Duration::from_secs(120));
        assert!(BAR_SYNC_STATS_HEAVY_REFRESH > BAR_SYNC_STATS_HIDDEN_REFRESH);
    }
}
