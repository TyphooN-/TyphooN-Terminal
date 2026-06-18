use super::*;
use crate::app::app_runtime_support::deferred_chart_load_interval;

pub(super) const MTF_GRID_TIMEFRAMES: [(&str, Timeframe); 9] = [
    ("M1", Timeframe::M1),
    ("M5", Timeframe::M5),
    ("M15", Timeframe::M15),
    ("M30", Timeframe::M30),
    ("H1", Timeframe::H1),
    ("H4", Timeframe::H4),
    ("D1", Timeframe::D1),
    ("W1", Timeframe::W1),
    ("MN1", Timeframe::MN1),
];

/// One MTF grid cell snapshot: `(tf_label, close, sma200, kama, fisher, fisher_signal)`.
/// `None` for an indicator means "no value" (not loaded / insufficient history).
pub(super) type MtfStatusRow = (
    &'static str,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MtfChartGroup {
    pub(super) symbol: String,
    pub(super) indices: Vec<usize>,
}

fn mtf_timeframe_rank(tf: Timeframe) -> Option<usize> {
    MTF_GRID_TIMEFRAMES
        .iter()
        .position(|(_, candidate)| *candidate == tf)
}

pub(super) fn mtf_grid_symbol_key(symbol: &str) -> String {
    let mut candidate = bare_symbol_from_key(symbol);
    if let Some(stripped) = candidate.strip_suffix(".EQ") {
        candidate = stripped.to_string();
    }
    candidate
}

fn kraken_position_covers_balance_asset(positions: &[PositionInfo], asset: &str) -> bool {
    let display = TyphooNApp::kraken_display_asset(asset);
    let bare_display = display.strip_suffix(".EQ").unwrap_or(display.as_str());
    positions.iter().any(|pos| {
        if !pos.qty.is_finite() || pos.qty <= 0.0 || !pos.side.eq_ignore_ascii_case("long") {
            return false;
        }
        let pos_symbol = typhoon_engine::core::kraken::normalize_pair_symbol(&pos.symbol)
            .replace('/', "")
            .to_ascii_uppercase();
        let pos_base = TyphooNApp::kraken_base_asset_for_pair(&pos_symbol);
        TyphooNApp::kraken_asset_keys_match(&display, &pos_symbol)
            || TyphooNApp::kraken_asset_keys_match(bare_display, &pos_symbol)
            || TyphooNApp::kraken_asset_keys_match(&display, &pos_base)
            || TyphooNApp::kraken_asset_keys_match(bare_display, &pos_base)
    })
}

pub(super) fn mtf_visible_chart_groups(
    charts: &[ChartState],
    visible: &[bool],
) -> Vec<MtfChartGroup> {
    let mut groups: Vec<MtfChartGroup> = Vec::new();
    for (idx, chart) in charts.iter().enumerate() {
        if !visible.get(idx).copied().unwrap_or(true)
            || mtf_timeframe_rank(chart.timeframe).is_none()
        {
            continue;
        }
        let symbol = mtf_grid_symbol_key(&chart.symbol);
        if symbol.is_empty() {
            continue;
        }
        if let Some(group) = groups.iter_mut().find(|group| group.symbol == symbol) {
            group.indices.push(idx);
        } else {
            groups.push(MtfChartGroup {
                symbol,
                indices: vec![idx],
            });
        }
    }
    for group in &mut groups {
        group
            .indices
            .sort_by_key(|idx| mtf_timeframe_rank(charts[*idx].timeframe).unwrap_or(usize::MAX));
    }
    groups
}

/// True iff `raw` becomes `target_upper` after stripping `'/'` and uppercasing
/// (ASCII). Avoids the per-call `raw.replace('/', "").to_uppercase()` allocation
/// that build_trade_overlay used to do once per scanned position.
fn symbol_matches_no_alloc(raw: &str, target_upper: &str) -> bool {
    let mut t = target_upper.bytes();
    for byte in raw.bytes() {
        if byte == b'/' {
            continue;
        }
        let upper = byte.to_ascii_uppercase();
        match t.next() {
            Some(tb) if tb == upper => {}
            _ => return false,
        }
    }
    t.next().is_none()
}

impl TyphooNApp {
    pub(crate) fn tick_dirty_indicator_recompute(&mut self) {
        // ── recompute indicators when periods changed in UI ──────────────
        if self.indicators_dirty {
            self.indicators_dirty = false;
            let mut gpu = self.gpu_indicators.take();
            // MAX PERFORMANCE: During heavy sync, completely skip indicator computation
            // for everything except the single active chart, and even then only if
            // we are not in a forming bar update (which has its own O(1) path).
            if let Some(chart) = self.charts.get_mut(self.active_tab) {
                if chart.bars.is_empty() {
                    // O(1) skip
                } else if !self.heavy_sync_in_progress {
                    chart.compute_indicators_gpu(gpu.as_mut());
                } else if chart.forming_bar_dirty {
                    chart.compute_indicators_gpu(gpu.as_mut());
                }
            }
            self.gpu_indicators = gpu;
        }
    }

    pub(crate) fn tick_deferred_chart_loads(&mut self, ctx: &egui::Context, now_instant: std::time::Instant) {
        // ── deferred chart loading: non-blocking, paced attempts ──
        // Uses try_load() which returns false if cache Mutex is contended (compaction, broker sync).
        // Failed loads stay queued. The actual load is still expensive — cache read + GPU
        // indicators + MTF overlays — so pace restored MTF grids instead of burning
        // consecutive UI frames while broad sync/news/SEC/fundamentals are active.
        if !self.deferred_chart_loads.is_empty() {
            let load_interval = if self.mtf_enabled {
                // Much more aggressive loading for open MTF tabs
                std::time::Duration::from_millis(80)
            } else {
                deferred_chart_load_interval(self.heavy_sync_in_progress, self.mtf_enabled)
            };
            if now_instant.duration_since(self.deferred_chart_last_load_at) >= load_interval {
                let idx = self.deferred_chart_loads[0]; // VecDeque supports indexing
                let _focused_chart = self.mtf_focused.unwrap_or(self.active_tab);
                // All open chart tabs (including background MTF cells and non-active
                // single-chart tabs) should load proactively so data+indicators are
                // ready when user switches. No more "click to load" behavior.
                if false {  // was: defer_inactive_mtf_cell during heavy sync
                    if let Some(skipped_idx) = self.deferred_chart_loads.pop_front() {
                        self.deferred_chart_loads.push_back(skipped_idx);
                    }
                    self.deferred_chart_last_load_at = now_instant;
                    ctx.request_repaint_after(load_interval);
                } else {
                    let mut loaded = false;
                    if let Some(cache) = self.cache.clone() {
                        if let Some(chart) = self.charts.get_mut(idx) {
                            let mut gpu = self.gpu_indicators.take();
                            loaded = chart.try_load(&cache, &mut self.log, gpu.as_mut());
                            self.gpu_indicators = gpu;
                        } else {
                            loaded = true; // invalid index, skip
                        }
                    }
                    if loaded {
                        self.deferred_chart_last_load_at = now_instant;
                        if let Some(done_idx) = self.deferred_chart_loads.pop_front() {
                            self.deferred_chart_load_set.remove(&done_idx);
                        }
                    }
                    // If !loaded, leave in queue — will retry after the pacing interval
                    // when the Mutex is free.
                }
            }
        }
    }
}

impl TyphooNApp {
    pub(super) fn close_partial_active_symbol(&mut self) {
        let Some((symbol, _)) = self.active_trade_symbol_and_price() else {
            self.log.push_back(LogEntry::warn(
                "Close Partial: active chart symbol unavailable",
            ));
            return;
        };
        let (send_alpaca, send_kraken) = self.selected_live_broker_targets();
        if !send_alpaca && !send_kraken {
            self.log.push_back(LogEntry::warn(
                "Close Partial: no broker connected for selected target",
            ));
            return;
        }
        let sl = self.sl_enabled.then_some(self.sl_price).flatten();
        let tp = self.tp_enabled.then_some(self.tp_price).flatten();
        let mut any = false;

        if send_alpaca {
            if let Some(pos) = self
                .live_positions
                .iter()
                .find(|pos| pos.symbol.eq_ignore_ascii_case(&symbol))
            {
                let half_qty = pos.qty.abs() / 2.0;
                if half_qty > 0.0 {
                    let remaining_qty = (pos.qty.abs() - half_qty).max(0.0);
                    let _ = self.broker_tx.send(BrokerCmd::ClosePosition {
                        symbol: symbol.clone(),
                        qty: Some(half_qty),
                    });
                    if remaining_qty > 0.0 && (sl.is_some() || tp.is_some()) {
                        let _ = self.broker_tx.send(BrokerCmd::AlpacaSyncExits {
                            symbol: symbol.clone(),
                            sl_price: sl,
                            tp_price: tp,
                            wait_for_qty_at_most: Some(remaining_qty),
                        });
                    }
                    any = true;
                    self.log.push_back(LogEntry::info(format!(
                        "Close Partial: Alpaca {} {:.4}",
                        symbol, half_qty
                    )));
                }
            } else {
                self.log.push_back(LogEntry::warn(format!(
                    "Close Partial: no Alpaca position found for {}",
                    symbol
                )));
            }
        }
        if send_kraken {
            if let Some(pos) = self
                .kr_positions
                .iter()
                .find(|pos| pos.symbol.eq_ignore_ascii_case(&symbol))
            {
                let half_qty = pos.qty.abs() / 2.0;
                if half_qty > 0.0 {
                    let remaining_qty = (pos.qty.abs() - half_qty).max(0.0);
                    let _ = self.broker_tx.send(BrokerCmd::KrakenClosePosition {
                        pair: symbol.clone(),
                        volume: Some(half_qty),
                    });
                    if remaining_qty > 0.0 && (sl.is_some() || tp.is_some()) {
                        let _ = self.broker_tx.send(BrokerCmd::KrakenSyncExits {
                            pair: symbol.clone(),
                            sl_price: sl,
                            tp_price: tp,
                            wait_for_position: true,
                            wait_for_qty_at_most: Some(remaining_qty),
                        });
                    }
                    any = true;
                    self.log.push_back(LogEntry::info(format!(
                        "Close Partial: Kraken {} {:.6}",
                        symbol, half_qty
                    )));
                }
            } else {
                self.log.push_back(LogEntry::warn(format!(
                    "Close Partial: no Kraken position found for {}",
                    symbol
                )));
            }
        }
        if !any {
            self.log.push_back(LogEntry::warn(
                "Close Partial: no position size available to reduce",
            ));
        }
    }

    /// Load cached daily-bar prices for every regulatory-alert symbol (Reg SHO
    /// threshold OR trading halt) not already in the watchlist, off the render
    /// thread (the same SQLite-read stall pitfall as the MTF grid: a bulk
    /// bar-sync writer can hold the conn mutex). Results are merged into
    /// `regulatory_prices` so the Reg SHO and Halts windows fill their Last /
    /// Daily-close / Chg% columns for the whole list; live bid/ask still come
    /// from watchlisted symbols only (the windows are cache-based).
    pub(super) fn spawn_regulatory_price_load(&mut self) {
        let cache = match &self.cache {
            Some(c) => Arc::clone(c),
            None => return,
        };
        let in_watchlist: std::collections::HashSet<String> = self
            .watchlist_rows
            .iter()
            .map(|r| r.symbol.to_ascii_uppercase())
            .collect();
        let symbols: Vec<String> = self
            .bg
            .regulatory_alerts_by_symbol
            .keys()
            .filter(|s| !in_watchlist.contains(&s.to_ascii_uppercase()))
            .cloned()
            .collect();
        if symbols.is_empty() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let rt_handle = self.rt_handle.clone();
        rt_handle.spawn_blocking(move || {
            // Daily first — the window's columns are daily close / daily change.
            let tfs = ["1Day", "4Hour", "1Hour"];
            let mut out: Vec<(String, WatchlistRow)> = Vec::new();
            for sym in symbols {
                'search: for tf in tfs {
                    for source in ["alpaca", "kraken", "kraken-equities", "default"] {
                        for key in chart_source_cache_keys(source, &sym, tf) {
                            if let Ok(Some(raw)) = cache.get_bars_raw(&key) {
                                if let Some(mut row) =
                                    watchlist_row_from_raw_bars(&sym, &key, &raw)
                                {
                                    // For a daily bar the last close IS the daily
                                    // close — surface it so Dly Close fills too.
                                    if tf.eq_ignore_ascii_case("1Day") {
                                        row.regular_close = row.last;
                                    }
                                    out.push((sym.clone(), row));
                                    break 'search;
                                }
                            }
                        }
                    }
                }
            }
            let _ = tx.send(out);
        });
        self.regulatory_prices_rx = Some(rx);
    }

    /// Force a market-data refresh for every symbol shown in the regulatory
    /// windows (Reg SHO threshold + trading halts) by queueing a daily-bar fetch
    /// per symbol — least-fresh, or no-data, symbols first so the emptiest rows
    /// fill soonest. One `1Day` fetch per symbol (the windows' columns are daily
    /// close / daily change); the broker queue's pending cap, per-symbol cooldown
    /// and freshness classifier throttle or skip the rest. Freshly fetched bars
    /// surface through the window's throttled `spawn_regulatory_price_load` read.
    pub(super) fn refresh_regulatory_prices(&mut self) {
        if self.bg.regulatory_alerts_by_symbol.is_empty() {
            return;
        }
        // Rank by the newest cache write-ts across the same source/timeframe keys
        // the price load reads; symbols with no cached bar sort first (i64::MIN).
        // Compute the order in a block so all immutable `self.bg` borrows end
        // before the `&mut self` fetch loop.
        let symbols: Vec<String> = {
            let ts_by_key: std::collections::HashMap<&str, i64> = self
                .bg
                .detailed_stats
                .iter()
                .map(|(key, _bars, ts)| (key.as_str(), *ts))
                .collect();
            let freshness = |sym: &str| -> i64 {
                let mut newest = i64::MIN;
                for tf in ["1Day", "4Hour", "1Hour"] {
                    for source in ["alpaca", "kraken", "kraken-equities", "default"] {
                        for key in chart_source_cache_keys(source, sym, tf) {
                            if let Some(&ts) = ts_by_key.get(key.as_str()) {
                                newest = newest.max(ts);
                            }
                        }
                    }
                }
                newest
            };
            let mut ranked: Vec<(i64, String)> = self
                .bg
                .regulatory_alerts_by_symbol
                .keys()
                .map(|sym| (freshness(sym), sym.clone()))
                .collect();
            ranked.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
            ranked.into_iter().map(|(_, sym)| sym).collect()
        };
        for sym in &symbols {
            self.queue_symbol_fetch_for_source(sym, "1Day");
        }
        // Re-read the cache promptly so the table reflects fetched bars as they land.
        self.regulatory_price_read_at = None;
    }

    pub(super) fn reload_symbol(&mut self, symbol: &str, tf: Timeframe) {
        // NOTE: For live Kraken WS forming-bar updates, prefer
        // chart.apply_forming_bar_update() + chart.mark_structural_change()
        // over a full reload to hit the draw_chart early-out.
        // Full reloads should only happen on closed bars or user-initiated symbol change.
        self.reload_symbol_auto(symbol, tf);
        self.queue_open_symbol_sync_all_timeframes(symbol);
    }

    pub(super) fn queue_open_symbol_sync_all_timeframes(&mut self, symbol: &str) -> usize {
        let symbol = symbol.trim();
        if symbol.is_empty() {
            return 0;
        }
        let timeframes = self.enabled_standard_sync_timeframes();
        let mut queued = 0usize;
        for tf in timeframes {
            if self.queue_symbol_fetch_for_source(symbol, &tf) {
                queued += 1;
            }
        }
        queued
    }

    fn queue_symbol_fetch_for_source(&mut self, symbol: &str, tf_key: &str) -> bool {
        if !self.sync_timeframe_enabled(tf_key) {
            return false;
        }
        let kraken_symbol = typhoon_engine::core::kraken::normalize_pair_symbol(symbol);
        if !kraken_symbol.is_empty()
            && typhoon_engine::core::kraken::to_kraken_pair_lossy(&kraken_symbol).is_some()
            && self.queue_kraken_fetch(&kraken_symbol, tf_key)
        {
            return true;
        }
        let bare = normalize_market_data_symbol(symbol)
            .replace('/', "")
            .trim_end_matches(".EQ")
            .to_ascii_uppercase();
        if self.kraken_enabled
            && self.kraken_scrape_xstocks
            && self
                .kraken_equity_universe_symbols
                .binary_search_by(|candidate| candidate.as_str().cmp(bare.as_str()))
                .is_ok()
        {
            self.dispatch_kraken_equity_ticker(&bare);
            if self.queue_kraken_equity_fetch(&bare, tf_key) {
                return true;
            }
        }
        self.queue_alpaca_fetch(symbol, tf_key)
    }

    pub(super) fn reload_symbol_auto(&mut self, symbol: &str, tf: Timeframe) {
        if let Some(ref cache) = self.cache {
            let (chart_type, source_override) = self
                .charts
                .get(self.active_tab)
                .map(|c| (c.chart_type, c.source_override))
                .unwrap_or((ChartType::Candle, ""));
            let mut chart = ChartState::new(symbol, tf);
            chart.chart_type = chart_type;
            chart.source_override = source_override;
            let cache_ref = Arc::as_ref(cache);
            let mut gpu = self.gpu_indicators.take();
            let load_succeeded = chart.try_load(cache_ref, &mut self.log, gpu.as_mut());
            self.gpu_indicators = gpu;
            if !load_succeeded {
                // Read error (not contention — read_conn is UI-exclusive)
                self.log
                    .push_back(LogEntry::err("Cache read error — check logs"));
            } else if chart.bars.is_empty() {
                let tf_key = tf.cache_suffix();
                let kraken_symbol = typhoon_engine::core::kraken::normalize_pair_symbol(symbol);
                let kraken_supported =
                    typhoon_engine::core::kraken::to_kraken_pair_lossy(&kraken_symbol).is_some();
                if !self.sync_timeframe_enabled(tf_key) {
                    self.log.push_back(LogEntry::warn(format!(
                        "No cached data for {} {} — sync for {} is disabled",
                        symbol,
                        tf.label(),
                        sync_timeframe_short_label(tf_key)
                    )));
                } else if kraken_supported {
                    let queued = self.queue_kraken_fetch(&kraken_symbol, tf_key);
                    if queued {
                        self.log.push_back(LogEntry::info(format!(
                            "No cached data for {} {} — fetching from Kraken...",
                            symbol,
                            tf.label()
                        )));
                    }
                } else if self.queue_alpaca_fetch(symbol, tf_key) {
                    self.log.push_back(LogEntry::info(format!(
                        "No cached data for {} {} — fetching from Alpaca...",
                        symbol,
                        tf.label()
                    )));
                }
            }
            if let Some(target) = self.charts.get_mut(self.active_tab) {
                *target = chart;
            }
            // Refresh MTF Grid status for all timeframes
            self.compute_mtf_grid_status();
        } else {
            self.log.push_back(LogEntry::warn("Cache not available"));
        }
    }

    pub(super) fn queue_chart_reload(&mut self, idx: usize) {
        if idx < self.charts.len() && self.deferred_chart_load_set.insert(idx) {
            self.deferred_chart_loads.push_back(idx);
        }
    }

    pub(super) fn normalize_news_ticker_for_chart(raw: &str) -> Option<String> {
        let symbol = normalize_market_data_symbol(raw)
            .trim()
            .trim_matches(|ch: char| {
                !ch.is_ascii_alphanumeric() && ch != '.' && ch != '-' && ch != '/'
            })
            .to_ascii_uppercase();
        let valid_chars = symbol
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '/');
        let well_formed_parts = symbol
            .split(['.', '-', '/'])
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_alphanumeric()));
        if symbol.is_empty() || symbol.len() > 16 || !valid_chars || !well_formed_parts {
            None
        } else {
            Some(symbol)
        }
    }

    pub(super) fn news_article_tickers(primary_symbol: &str, tickers: &[String]) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(primary) = Self::normalize_news_ticker_for_chart(primary_symbol) {
            out.push(primary);
        }
        for ticker in tickers {
            let Some(ticker) = Self::normalize_news_ticker_for_chart(ticker) else {
                continue;
            };
            if !out.iter().any(|existing| existing == &ticker) {
                out.push(ticker);
            }
        }
        out
    }

    pub(super) fn open_news_ticker_chart(&mut self, raw_ticker: &str) -> bool {
        let Some(symbol) = Self::normalize_news_ticker_for_chart(raw_ticker) else {
            return false;
        };

        self.ensure_mtf_grid_for_symbol(&symbol);
        if let Some(existing_idx) = self.charts.iter().position(|chart| {
            chart.timeframe == Timeframe::D1
                && mtf_grid_symbol_key(&chart.symbol).eq_ignore_ascii_case(
                    &symbol
                        .replace('/', "")
                        .trim_end_matches(".EQ")
                        .to_ascii_uppercase(),
                )
        }) {
            self.active_tab = existing_idx;
        }
        self.symbol_input = symbol.clone();
        self.mtf_enabled = true;
        self.compute_mtf_grid_status();
        self.log.push_back(LogEntry::info(format!(
            "News: opened/focused {} MTF grid",
            symbol
        )));
        true
    }

    /// Compute MTF Grid indicator status for all timeframes from cache.
    /// Parallel: spawns threads for TFs not already loaded in chart tabs.
    /// Order-independent hash of the open-chart `(symbol-key, timeframe)` set. It
    /// changes whenever a chart is opened, closed, or retimeframed, so the grid's
    /// cache fallback (`mtf_grid_status`) can be recomputed for the new layout —
    /// otherwise a just-closed timeframe drops to a stale/empty cell.
    pub(super) fn mtf_open_chart_signature(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        // XOR per-chart hashes → independent of tab order; fold in the count so
        // adding then removing different charts can't alias to the same value.
        let mut acc: u64 = self.charts.len() as u64;
        for c in &self.charts {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            mtf_grid_symbol_key(&c.symbol).hash(&mut h);
            c.timeframe.label().hash(&mut h);
            acc ^= h.finish();
        }
        acc
    }

    /// Merge MTF grid rows in place, keeping them in timeframe order. `authoritative`
    /// rows (live open charts) always overwrite. Non-authoritative rows (cache /
    /// background loads) fill empty or missing cells but never clobber an existing
    /// concrete value with an all-`None` miss — that no-clobber rule is what stops
    /// already-filled cells from flickering back to grey on a throttled refresh.
    pub(super) fn mtf_grid_status_upsert(&mut self, rows: Vec<MtfStatusRow>, authoritative: bool) {
        let has_data = |r: &MtfStatusRow| {
            r.1.is_some() || r.2.is_some() || r.3.is_some() || r.4.is_some() || r.5.is_some()
        };
        for row in rows {
            match self.mtf_grid_status.iter_mut().find(|s| s.0 == row.0) {
                Some(slot) => {
                    if authoritative || has_data(&row) || !has_data(slot) {
                        *slot = row;
                    }
                }
                None => self.mtf_grid_status.push(row),
            }
        }
        self.mtf_grid_status.sort_by_key(|r| {
            MTF_GRID_TIMEFRAMES
                .iter()
                .position(|(label, _)| *label == r.0)
                .unwrap_or(usize::MAX)
        });
    }

    pub(super) fn compute_mtf_grid_status(&mut self) {
        let cache = match &self.cache {
            Some(c) => Arc::clone(c),
            None => return,
        };
        let sym = self.symbol_input.trim().to_string();
        if sym.is_empty() {
            return;
        }
        // A symbol change invalidates the whole snapshot (old symbol's values are
        // meaningless); an open/close or throttled cache-fill refresh keeps prior
        // values and only upserts, so filled cells never flicker.
        if self.mtf_grid_status_symbol != sym {
            self.mtf_grid_status.clear();
        }
        self.mtf_grid_status_symbol = sym.clone();
        self.mtf_grid_status_open_sig = self.mtf_open_chart_signature();
        self.mtf_grid_status_at = Some(std::time::Instant::now());
        let sym_key = mtf_grid_symbol_key(&sym);
        let all_tfs: &[(&'static str, Timeframe)] = &MTF_GRID_TIMEFRAMES;

        // Collect results from already-loaded charts (no thread needed)
        let mut preloaded: Vec<MtfStatusRow> = Vec::new();
        let mut need_load: Vec<(&'static str, Timeframe)> = Vec::new();

        for &(label, tf) in all_tfs {
            if let Some(c) = self.charts.iter().find(|c| {
                c.timeframe == tf
                    && !c.bars.is_empty()
                    && mtf_grid_symbol_key(&c.symbol).eq_ignore_ascii_case(&sym_key)
            }) {
                let close = c
                    .fresh_live_quote_mid()
                    .or_else(|| c.bars.last().map(|b| b.close));
                let sma = c.sma200.last().and_then(|v| *v);
                let kama = c.kama.last().and_then(|v| *v);
                let fisher = c.fisher.last().and_then(|v| *v);
                let fsig = c.fisher_signal.last().and_then(|v| *v);
                preloaded.push((label, close, sma, kama, fisher, fsig));
            } else {
                need_load.push((label, tf));
            }
        }

        // Open-chart values are authoritative — always overwrite the snapshot.
        self.mtf_grid_status_upsert(preloaded, true);

        if need_load.is_empty() {
            // All TFs came from open charts — nothing else to load.
        } else if self.heavy_sync_in_progress {
            // Don't kick off background cache/indicator loads for missing MTF
            // cells while full-universe sync is saturating the machine. Seed any
            // not-yet-present cell as grey (no-clobber keeps prior good values);
            // the throttled refresh retries once sync pressure relaxes.
            let placeholders: Vec<MtfStatusRow> = need_load
                .iter()
                .map(|(label, _)| (*label, None, None, None, None, None))
                .collect();
            self.mtf_grid_status_upsert(placeholders, false);
        } else {
            // Spawn background thread for TFs that need cache loading — don't block UI
            let (tx, rx) = std::sync::mpsc::channel();
            let need_load_owned: Vec<(&'static str, Timeframe)> = need_load;
            let rt_handle = self.rt_handle.clone();
            rt_handle.spawn_blocking(move || {
                let mut results: Vec<MtfStatusRow> = Vec::new();
                for (label, tf) in need_load_owned {
                    let mut temp = ChartState::new(&sym, tf);
                    let dsm = typhoon_engine::core::data_source::DataSourceManager::default();
                    temp.load(&cache, &mut std::collections::VecDeque::new(), None, &dsm);
                    if temp.bars.is_empty() {
                        results.push((label, None, None, None, None, None));
                    } else {
                        let close = temp.bars.last().map(|b| b.close);
                        let sma = temp.sma200.last().and_then(|v| *v);
                        let kama = temp.kama.last().and_then(|v| *v);
                        let fisher = temp.fisher.last().and_then(|v| *v);
                        let fsig = temp.fisher_signal.last().and_then(|v| *v);
                        results.push((label, close, sma, kama, fisher, fsig));
                        // Publish the canonical bars to the shared MTF cache so this
                        // symbol's MTF_MA / MultiKAMA chart overlay reuses them rather
                        // than re-reading SQLite (the same-cache unification). Moves
                        // the Vec out of `temp` (dropped next) — no clone.
                        let now_ms = chrono::Utc::now().timestamp_millis();
                        super::chart::mtf_htf_cache_put(
                            &mtf_grid_symbol_key(&sym),
                            tf.cache_suffix(),
                            std::sync::Arc::new(std::mem::take(&mut temp.bars)),
                            now_ms,
                        );
                    }
                }
                let _ = tx.send(results);
            });
            self.mtf_grid_rx = Some(rx);
        }
    }

    /// Return the unique news tickers represented by the current MTF grid charts.
    /// The grid may contain the same ticker across many timeframes/source-prefixed
    /// cache keys; news fetches should happen once per underlying ticker.
    pub(super) fn mtf_grid_news_symbols(&self) -> Vec<String> {
        fn is_timeframe_token(token: &str) -> bool {
            matches!(
                token.to_ascii_uppercase().as_str(),
                "M1" | "M5"
                    | "M15"
                    | "M30"
                    | "H1"
                    | "H4"
                    | "D1"
                    | "W1"
                    | "MN1"
                    | "1MIN"
                    | "5MIN"
                    | "15MIN"
                    | "30MIN"
                    | "1HOUR"
                    | "4HOUR"
                    | "1DAY"
                    | "1WEEK"
                    | "1MONTH"
            )
        }

        let mut symbols = std::collections::BTreeSet::new();
        for (i, chart) in self.charts.iter().enumerate() {
            if self.mtf_enabled && !self.mtf_visible.get(i).copied().unwrap_or(true) {
                continue;
            }
            let parts: Vec<&str> = chart.symbol.split(':').collect();
            let candidate = match parts.as_slice() {
                [source, sym] if !is_timeframe_token(sym) && !source.eq_ignore_ascii_case(sym) => {
                    *sym
                }
                [sym, tf] if is_timeframe_token(tf) => *sym,
                [_, sym, _tf] => *sym,
                _ => chart.symbol.as_str(),
            };
            let mut symbol = normalize_market_data_symbol(candidate).replace('/', "");
            if let Some(stripped) = symbol.strip_suffix(".EQ") {
                symbol = stripped.to_string();
            }
            if !symbol.is_empty() && !is_timeframe_token(&symbol) {
                symbols.insert(symbol);
            }
        }
        symbols.into_iter().collect()
    }

    /// Ensure this symbol has one MTF chart per supported MTF Grid timeframe.
    /// M1/M5 stay visible for native Kraken Spot and Kraken Equities; unsupported/missing assist providers render as empty/grey panes.
    pub(super) fn ensure_mtf_grid_for_symbol(&mut self, symbol: &str) {
        let symbol = symbol.trim();
        if symbol.is_empty() {
            return;
        }
        let symbol_key = normalize_market_data_symbol(symbol)
            .replace('/', "")
            .trim_end_matches(".EQ")
            .to_ascii_uppercase();
        for &(label, tf) in &MTF_GRID_TIMEFRAMES {
            let existing_idx = self.charts.iter().position(|chart| {
                chart.timeframe == tf
                    && mtf_grid_symbol_key(&chart.symbol).eq_ignore_ascii_case(&symbol_key)
            });
            let idx = if let Some(idx) = existing_idx {
                idx
            } else {
                let mut chart = ChartState::new(symbol, tf);
                if let Some(ref cache) = self.cache.clone() {
                    let mut gpu = self.gpu_indicators.take();
                    if !chart.try_load(Arc::as_ref(cache), &mut self.log, gpu.as_mut()) {
                        self.gpu_indicators = gpu;
                        self.charts.push(chart);
                        let idx = self.charts.len().saturating_sub(1);
                        self.queue_chart_reload(idx);
                        let _ = self.queue_symbol_fetch_for_source(symbol, tf.cache_suffix());
                        idx
                    } else {
                        self.gpu_indicators = gpu;
                        self.charts.push(chart);
                        self.charts.len().saturating_sub(1)
                    }
                } else {
                    self.charts.push(chart);
                    let idx = self.charts.len().saturating_sub(1);
                    let _ = self.queue_symbol_fetch_for_source(symbol, tf.cache_suffix());
                    idx
                }
            };
            while self.mtf_visible.len() < self.charts.len() {
                self.mtf_visible.push(true);
            }
            if let Some(visible) = self.mtf_visible.get_mut(idx) {
                *visible = true;
            }
            if label == "D1" {
                self.active_tab = idx;
            }
        }
    }

    /// Set up MTF grid with N columns. Creates one chart per supported MTF timeframe
    /// for the current symbol; the legacy `target` is ignored except for menu compatibility.
    pub(super) fn setup_mtf_grid(&mut self, cols: usize, _target: usize) {
        let sym = self.symbol_input.trim().to_string();
        self.ensure_mtf_grid_for_symbol(&sym);
        self.mtf_cols = cols;
        self.mtf_enabled = true;
        let symbol_count = mtf_visible_chart_groups(&self.charts, &self.mtf_visible).len();
        self.log.push_back(LogEntry::info(format!(
            "MTF grid: {} col(s), {} symbol grid(s), {} supported TFs per symbol",
            cols,
            symbol_count,
            MTF_GRID_TIMEFRAMES.len()
        )));
    }

    /// Build trade overlay for a chart: broker fills as arrows + open position lines.
    /// Aggregates same-price entries at same bar into single markers.
    pub(super) fn build_trade_overlay(&self, chart: &ChartState) -> TradeOverlay {
        let mut overlay = TradeOverlay::default();
        if chart.bars.is_empty() {
            return overlay;
        }

        // Extract bare symbol from chart symbol for matching
        // Normalize: strip source prefix, TF suffix, and slashes (SOL/USD → SOLUSD)
        let bare_sym = {
            let s = &chart.symbol;
            let parts: Vec<&str> = s.split(':').collect();
            let is_tf = matches!(
                parts.last().copied(),
                Some(
                    "1Min"
                        | "5Min"
                        | "15Min"
                        | "30Min"
                        | "1Hour"
                        | "4Hour"
                        | "1Day"
                        | "1Week"
                        | "1Month"
                )
            );
            let sym_parts = if is_tf && parts.len() > 1 {
                &parts[..parts.len() - 1]
            } else {
                &parts[..]
            };
            sym_parts
                .last()
                .copied()
                .unwrap_or(s.as_str())
                .replace('/', "")
        };
        if bare_sym.is_empty() {
            return overlay;
        }

        let first_ts = chart.bars.first().map(|b| b.ts_ms).unwrap_or(0);
        let last_ts = chart.bars.last().map(|b| b.ts_ms).unwrap_or(0);

        // Tolerance based on timeframe (a deal can be slightly after the last bar)
        let tf_tolerance_ms: i64 = match chart.timeframe {
            Timeframe::MN1 => 35 * 86_400_000, // 35 days
            Timeframe::W1 => 8 * 86_400_000,   // 8 days
            Timeframe::D1 => 2 * 86_400_000,   // 2 days
            _ => 86_400_000,                   // 1 day
        };
        // Find bar index for a timestamp (binary search on sorted bars)
        let find_bar = |ts_ms: i64| -> Option<usize> {
            if ts_ms < first_ts || ts_ms > last_ts + tf_tolerance_ms {
                return None;
            }
            match chart.bars.binary_search_by_key(&ts_ms, |b| b.ts_ms) {
                Ok(idx) => Some(idx),
                Err(idx) => {
                    if idx > 0 {
                        Some(idx - 1)
                    } else {
                        Some(0)
                    }
                }
            }
        };

        // Trade-marker accumulator shared by broker fills below.
        use std::collections::HashMap;
        let bare_upper = bare_sym.to_uppercase();
        let mut marker_map: HashMap<(usize, bool, i64), (f64, u32, String)> = HashMap::new(); // (bar_idx, is_buy, price_cents) → (total_vol, count, ticker)

        // Broker fills — add to marker map before conversion. Alpaca currently
        // enters through `recent_fills`; Kraken keeps a full REST + private-WS
        // trade deque. Both paths feed the same chart arrows.
        if self.show_alpaca_positions {
            for (sym, side, qty, price, time) in &self.recent_fills {
                let fill_sym = sym.replace('/', "").to_uppercase();
                if !fill_sym.contains(&bare_upper) && !bare_upper.contains(&fill_sym) {
                    continue;
                }
                let ts = chrono::NaiveDateTime::parse_from_str(time, "%Y-%m-%dT%H:%M:%S%.fZ")
                    .or_else(|_| chrono::NaiveDateTime::parse_from_str(time, "%Y-%m-%d %H:%M:%S"))
                    .or_else(|_| {
                        chrono::NaiveDate::parse_from_str(time, "%Y-%m-%d")
                            .map(|d| d.and_hms_opt(0, 0, 0).unwrap_or_default())
                    })
                    .map(|dt| dt.and_utc().timestamp_millis())
                    .unwrap_or(0);
                if let Some(bar_idx) = find_bar(ts) {
                    let is_buy = side == "buy";
                    let price_key = (*price * 100000.0) as i64;
                    let entry = marker_map.entry((bar_idx, is_buy, price_key)).or_insert((
                        0.0,
                        0,
                        String::new(),
                    ));
                    entry.0 += qty;
                    entry.1 += 1;
                    if !entry.2.contains("Alpaca") {
                        if !entry.2.is_empty() {
                            entry.2.push_str(", ");
                        }
                        entry.2.push_str("Alpaca");
                    }
                }
            }
        }

        if self.show_kr_positions {
            for trade in &self.kraken_trades {
                let pair_norm = typhoon_engine::core::kraken::normalize_pair_symbol(&trade.pair)
                    .replace('/', "")
                    .to_ascii_uppercase();
                let base = Self::kraken_base_asset_for_pair(&pair_norm);
                let matches_chart = symbol_matches_no_alloc(&pair_norm, &bare_upper)
                    || Self::kraken_asset_keys_match(&base, &bare_upper)
                    || pair_norm.contains(&bare_upper)
                    || bare_upper.contains(&pair_norm);
                if !matches_chart || trade.price <= 0.0 || trade.vol <= 0.0 {
                    continue;
                }
                let ts = (trade.time * 1000.0) as i64;
                if let Some(bar_idx) = find_bar(ts) {
                    let is_buy = trade.side.eq_ignore_ascii_case("buy");
                    let price_key = (trade.price * 100000.0) as i64;
                    let entry = marker_map.entry((bar_idx, is_buy, price_key)).or_insert((
                        0.0,
                        0,
                        String::new(),
                    ));
                    entry.0 += trade.vol;
                    entry.1 += 1;
                    if !entry.2.contains("Kraken") {
                        if !entry.2.is_empty() {
                            entry.2.push_str(", ");
                        }
                        entry.2.push_str("Kraken");
                    }
                }
            }
        }

        // SEC insider trades (Form 4) — show buy/sell markers on chart
        if let Some(trades) = self.bg.insider_trades.get(&bare_upper) {
            for trade in trades {
                // Parse "YYYY-MM-DD" to timestamp
                let ts = chrono::NaiveDate::parse_from_str(&trade.transaction_date, "%Y-%m-%d")
                    .map(|d| {
                        d.and_hms_opt(12, 0, 0)
                            .unwrap_or_default()
                            .and_utc()
                            .timestamp_millis()
                    })
                    .unwrap_or(0);
                if let Some(bar_idx) = find_bar(ts) {
                    let is_buy =
                        !matches!(trade.transaction_type.chars().next(), Some('S') | Some('D'));
                    // Use the bar's close price as the marker price (insider trade price may not match chart scale)
                    let price = if let Some(bar) = chart.bars.get(bar_idx) {
                        bar.close
                    } else {
                        trade.price
                    };
                    let price_key = (price * 100000.0) as i64;
                    let label = format!(
                        "SEC:{}",
                        trade.insider_name.split_whitespace().next().unwrap_or("")
                    );
                    let entry = marker_map.entry((bar_idx, is_buy, price_key)).or_insert((
                        0.0,
                        0,
                        String::new(),
                    ));
                    entry.0 += trade.shares;
                    entry.1 += 1;
                    if !entry.2.is_empty() {
                        entry.2.push_str(", ");
                    }
                    entry.2.push_str(&label);
                }
            }
        }

        // Convert marker map to sorted markers
        for ((bar_idx, is_buy, price_key), (volume, count, ticker)) in marker_map {
            overlay.markers.push(TradeMarker {
                bar_idx,
                price: price_key as f64 / 100000.0,
                volume,
                is_buy,
                count,
                ticker,
            });
        }
        overlay.markers.sort_by_key(|m| m.bar_idx);

        // Live broker position lines (Alpaca + Kraken).
        // Kraken spot crypto balances are inventory rather than broker
        // `PositionInfo` rows, but the chart still needs a visible holding
        // entry line when cost basis is known.
        let alpaca_iter: Box<dyn Iterator<Item = &PositionInfo>> = if self.show_alpaca_positions {
            Box::new(self.live_positions.iter())
        } else {
            Box::new(std::iter::empty())
        };
        let kr_iter: Box<dyn Iterator<Item = &PositionInfo>> = if self.show_kr_positions {
            Box::new(self.kr_positions.iter())
        } else {
            Box::new(std::iter::empty())
        };
        let all_broker_positions = alpaca_iter.chain(kr_iter);
        // `bare_upper` is already computed once at the top of this function;
        // recomputing it inside the loop allocated a new String per broker position.
        // Short-circuit on a no-alloc equality check before paying for the substring
        // form — most positions match exactly, and only the rare crypto-style
        // `BTCUSD` vs `BTC` case actually needs the normalized String.
        for pos in all_broker_positions {
            let keep = if symbol_matches_no_alloc(&pos.symbol, &bare_upper) {
                true
            } else {
                let pos_sym = pos.symbol.replace('/', "").to_uppercase();
                pos_sym.contains(&bare_upper) || bare_upper.contains(&pos_sym)
            };
            if !keep {
                continue;
            }
            let is_buy = pos.side == "long";
            // Kraken xStock positions are derived from cash-account balances and
            // arrive with avg_entry_price = 0.0 (the balance snapshot carries no
            // cost basis). Resolve the real entry from trade-history cost basis —
            // the same source the positions panel uses (kraken_balance_avg_price)
            // — so the line sits at the entry instead of being pinned to price 0
            // (the chart bottom), which is the BUY 0.0000 regression.
            let entry_price = if pos.avg_entry_price > 0.0 {
                pos.avg_entry_price
            } else if let Some(asset) = pos.asset_id.strip_prefix("equity_balance:") {
                self.kraken_balance_avg_price(asset).unwrap_or(0.0)
            } else {
                pos.avg_entry_price
            };
            if !(entry_price > 0.0 && entry_price.is_finite()) {
                // No usable entry price (cost basis not loaded yet) — skip rather
                // than draw a meaningless dashed line pinned to price 0.
                continue;
            }
            overlay.position_lines.push(PositionLine {
                price: entry_price,
                volume: pos.qty,
                is_buy,
                line_type: 0, // entry
            });
        }

        if self.show_kr_positions {
            for (asset, qty) in &self.kraken_balances {
                if !qty.is_finite() || *qty <= 0.0 || Self::kraken_is_cash_balance_asset(asset) {
                    continue;
                }
                if kraken_position_covers_balance_asset(&self.kr_positions, asset) {
                    continue;
                }
                let display = Self::kraken_display_asset(asset);
                let pair = Self::kraken_spot_pair_for_balance_asset(asset);
                let pair_norm = typhoon_engine::core::kraken::normalize_pair_symbol(&pair)
                    .replace('/', "")
                    .to_ascii_uppercase();
                let base = Self::kraken_base_asset_for_pair(&pair_norm);
                let matches_chart = Self::kraken_asset_keys_match(&display, &bare_upper)
                    || Self::kraken_asset_keys_match(&base, &bare_upper)
                    || symbol_matches_no_alloc(&pair_norm, &bare_upper)
                    || pair_norm.contains(&bare_upper)
                    || bare_upper.contains(&pair_norm);
                if !matches_chart {
                    continue;
                }
                let Some(avg_price) = self.kraken_balance_avg_price(asset) else {
                    continue;
                };
                if avg_price <= 0.0 || !avg_price.is_finite() {
                    continue;
                }
                overlay.position_lines.push(PositionLine {
                    price: avg_price,
                    volume: *qty,
                    is_buy: true,
                    line_type: 0,
                });
            }
        }

        // Deduplicate position lines (aggregate same price+type, sum volume).
        // Bucket by a rounded 5-decimal price key, but keep the EXACT first
        // price for display. The positions panel formats the raw avg, so a
        // truncated bucket value (pk/100000) could round differently at the
        // 4th decimal and make the on-chart label disagree with the navbar
        // (e.g. 0.1057 vs 0.1058). Preserving the real price keeps them in sync.
        {
            let mut agg: HashMap<(i64, u8), (f64, f64, bool)> = HashMap::new();
            for pl in &overlay.position_lines {
                let key = ((pl.price * 100000.0).round() as i64, pl.line_type);
                let entry = agg.entry(key).or_insert((pl.price, 0.0, pl.is_buy));
                entry.1 += pl.volume;
            }
            overlay.position_lines = agg
                .into_iter()
                .map(|((_, lt), (price, vol, is_buy))| PositionLine {
                    price,
                    volume: vol,
                    is_buy,
                    line_type: lt,
                })
                .collect();
        }

        overlay
    }

    pub(super) fn indicator_flags(&self) -> IndicatorFlags {
        IndicatorFlags {
            sma200: self.show_sma200,
            sma100: self.show_sma100,
            kama: self.show_kama,
            ema21: self.show_ema21,
            bollinger: self.show_bollinger,
            ichimoku: self.show_ichimoku,
            wma: self.show_wma,
            hma: self.show_hma,
            psar: self.show_psar,
            atr_proj: self.show_atr_proj,
            prev_levels: self.show_prev_levels,
            pivots: self.show_pivots,
            fractals: self.show_fractals,
            harmonics: self.show_harmonics,
            auto_fib: self.show_auto_fib,
            supply_demand: self.show_supply_demand,
            ehlers_ss: self.show_ehlers_ss,
            ehlers_decycler: self.show_ehlers_decycler,
            ehlers_itl: self.show_ehlers_itl,
            ehlers_mama: self.show_ehlers_mama,
            sessions: self.show_sessions,
            vol_heatmap: self.show_vol_heatmap,
            vwap: self.show_vwap,
            price_histogram: self.show_price_histogram,
            supertrend: self.show_supertrend,
            donchian: self.show_donchian,
            keltner: self.show_keltner,
            regression: self.show_regression,
            fvg: self.show_fvg,
            order_blocks: self.show_order_blocks,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn news_article_tickers_normalizes_and_deduplicates_symbols() {
        let tickers = vec![
            " aapl ".to_string(),
            "MSFT".to_string(),
            "msft".to_string(),
            "../bad".to_string(),
            "THIS-SYMBOL-IS-TOO-LONG".to_string(),
        ];

        assert_eq!(
            TyphooNApp::news_article_tickers("AAPL", &tickers),
            vec!["AAPL".to_string(), "MSFT".to_string()]
        );
    }

    #[test]
    fn news_ticker_normalization_accepts_common_market_symbols() {
        assert_eq!(
            TyphooNApp::normalize_news_ticker_for_chart(" brk.b "),
            Some("BRK.B".to_string())
        );
        assert_eq!(
            TyphooNApp::normalize_news_ticker_for_chart("BTC/USD"),
            Some("BTC/USD".to_string())
        );
        assert_eq!(TyphooNApp::normalize_news_ticker_for_chart(""), None);
        assert_eq!(
            TyphooNApp::normalize_news_ticker_for_chart("THIS-SYMBOL-IS-TOO-LONG"),
            None
        );
    }

    #[test]
    fn mtf_grid_timeframes_include_low_timeframes_for_native_kraken_pairs() {
        let labels: Vec<&str> = MTF_GRID_TIMEFRAMES
            .iter()
            .map(|(label, _)| *label)
            .collect();

        assert_eq!(
            labels,
            vec!["M1", "M5", "M15", "M30", "H1", "H4", "D1", "W1", "MN1"]
        );
    }

    #[test]
    fn mtf_grid_groups_visible_charts_by_symbol_and_sorts_each_symbol_by_timeframe() {
        let charts = vec![
            ChartState::new("kraken:WOK.EQ:1Day", Timeframe::D1),
            ChartState::new("kraken:BABYUSD:4Hour", Timeframe::H4),
            ChartState::new("kraken:WOK.EQ:15Min", Timeframe::M15),
            ChartState::new("kraken:BABYUSD:1Hour", Timeframe::H1),
            ChartState::new("kraken:WOK.EQ:1Min", Timeframe::M1),
            ChartState::new("kraken:WOK.EQ:1Week", Timeframe::W1),
            ChartState::new("kraken:BABYUSD:5Min", Timeframe::M5),
        ];
        let visible = vec![true; charts.len()];

        let groups = mtf_visible_chart_groups(&charts, &visible);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].symbol, "WOK");
        assert_eq!(groups[0].indices, vec![4, 2, 0, 5]);
        assert_eq!(groups[1].symbol, "BABYUSD");
        assert_eq!(groups[1].indices, vec![6, 3, 1]);
    }

    fn test_position(symbol: &str, qty: f64, side: &str) -> PositionInfo {
        PositionInfo {
            symbol: symbol.to_string(),
            qty,
            side: side.to_string(),
            avg_entry_price: 1.0,
            market_value: qty,
            unrealized_pl: 0.0,
            asset_class: "stock".to_string(),
            asset_id: "equity_balance:test".to_string(),
        }
    }

    #[test]
    fn kraken_balance_overlay_skips_assets_already_reported_as_positions() {
        let positions = vec![test_position("WOK", 7142.0, "long")];

        assert!(kraken_position_covers_balance_asset(&positions, "WOK.EQ"));
        assert!(kraken_position_covers_balance_asset(&positions, "WOK"));
    }

    #[test]
    fn kraken_balance_overlay_still_allows_inventory_without_position_row() {
        let positions = vec![test_position("GDC", 100.0, "long")];

        assert!(!kraken_position_covers_balance_asset(&positions, "WOK.EQ"));
        assert!(!kraken_position_covers_balance_asset(&positions, "WOK"));
    }
}
