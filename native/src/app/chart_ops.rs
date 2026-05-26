use super::*;

const MTF_GRID_TIMEFRAMES: [(&str, Timeframe); 7] = [
    ("M15", Timeframe::M15),
    ("M30", Timeframe::M30),
    ("H1", Timeframe::H1),
    ("H4", Timeframe::H4),
    ("D1", Timeframe::D1),
    ("W1", Timeframe::W1),
    ("MN1", Timeframe::MN1),
];

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
    pub(super) fn close_partial_active_symbol(&mut self) {
        let Some((symbol, _)) = self.active_trade_symbol_and_price() else {
            self.log.push_back(LogEntry::warn(
                "Close Partial: active chart symbol unavailable",
            ));
            return;
        };
        let (send_alpaca, send_tt, send_kraken) = self.selected_live_broker_targets();
        if !send_alpaca && !send_tt && !send_kraken {
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
        if send_tt {
            if let Some(pos) = self
                .tt_positions
                .iter()
                .find(|pos| pos.symbol.eq_ignore_ascii_case(&symbol))
            {
                let half_qty = (pos.qty.abs() / 2.0).floor() as i64;
                if half_qty > 0 {
                    let remaining_qty = pos.qty.abs() - half_qty as f64;
                    let _ = self.broker_tx.send(BrokerCmd::TastytradeClosePositionQty {
                        symbol: symbol.clone(),
                        qty: Some(half_qty),
                    });
                    if remaining_qty > 0.0 && (sl.is_some() || tp.is_some()) {
                        let _ = self.broker_tx.send(BrokerCmd::TastytradeSyncExits {
                            symbol: symbol.clone(),
                            sl_price: sl,
                            tp_price: tp,
                            wait_for_position: true,
                            wait_for_qty_at_most: Some(remaining_qty),
                        });
                    }
                    any = true;
                    self.log.push_back(LogEntry::info(format!(
                        "Close Partial: tastytrade {} {}",
                        symbol, half_qty
                    )));
                } else {
                    self.log.push_back(LogEntry::warn(format!(
                        "Close Partial: tastytrade {} is too small to halve cleanly",
                        symbol
                    )));
                }
            } else {
                self.log.push_back(LogEntry::warn(format!(
                    "Close Partial: no tastytrade position found for {}",
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

    pub(super) fn reload_symbol(&mut self, symbol: &str, tf: Timeframe) {
        // NOTE: For live Kraken WS forming-bar updates, prefer
        // chart.apply_forming_bar_update() + chart.mark_structural_change()
        // over a full reload to hit the draw_chart early-out.
        // Full reloads should only happen on closed bars or user-initiated symbol change.
        let source_override = self.charts.get(self.active_tab).and_then(|chart| {
            chart
                .symbol_matches(symbol)
                .then(|| chart.source_override.clone())
                .flatten()
        });
        self.reload_symbol_with_source(symbol, tf, source_override);
        self.queue_open_symbol_sync_all_timeframes(symbol);
    }

    pub(super) fn queue_open_symbol_sync_all_timeframes(&mut self, symbol: &str) -> usize {
        let symbol = symbol.trim();
        if symbol.is_empty() {
            return 0;
        }
        let source_override = self
            .charts
            .get(self.active_tab)
            .and_then(|chart| chart.source_override.clone());
        let timeframes = self.enabled_standard_sync_timeframes();
        let mut queued = 0usize;
        for tf in timeframes {
            if self.queue_symbol_fetch_for_source(symbol, &tf, source_override.as_deref()) {
                queued += 1;
            }
        }
        queued
    }

    fn queue_symbol_fetch_for_source(
        &mut self,
        symbol: &str,
        tf_key: &str,
        source_override: Option<&str>,
    ) -> bool {
        if !self.sync_timeframe_enabled(tf_key) {
            return false;
        }
        if let Some(source) = source_override.filter(|s| !s.trim().is_empty() && *s != "auto") {
            let source_symbol = preferred_chart_symbol_for_source(source, symbol);
            return match source {
                "alpaca" => self.queue_alpaca_fetch(&source_symbol, tf_key),
                "tastytrade" => self.queue_tastytrade_fetch(&source_symbol, tf_key),
                "kraken" => self.queue_kraken_fetch(&source_symbol, tf_key),
                "kraken-equities" => {
                    self.dispatch_kraken_equity_ticker(&source_symbol);
                    self.queue_kraken_equity_fetch(&source_symbol, tf_key)
                }
                "kraken-futures" => self.queue_kraken_futures_fetch(&source_symbol, tf_key),
                _ => false,
            };
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
        if self.tt_connected
            && self.tastytrade_has_symbol(symbol)
            && self.queue_tastytrade_fetch(symbol, tf_key)
        {
            return true;
        }
        self.queue_alpaca_fetch(symbol, tf_key)
    }

    pub(super) fn reload_symbol_with_source(
        &mut self,
        symbol: &str,
        tf: Timeframe,
        source_override: Option<String>,
    ) {
        if let Some(ref cache) = self.cache {
            let chart_type = self
                .charts
                .get(self.active_tab)
                .map(|c| c.chart_type)
                .unwrap_or(ChartType::Candle);
            let mut chart = ChartState::new(symbol, tf);
            chart.chart_type = chart_type;
            chart.source_override = source_override.filter(|s| !s.trim().is_empty());
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
                let tasty_supported = self.tt_connected && self.tastytrade_has_symbol(symbol);
                if !self.sync_timeframe_enabled(tf_key) {
                    self.log.push_back(LogEntry::warn(format!(
                        "No cached data for {} {} — sync for {} is disabled",
                        symbol,
                        tf.label(),
                        sync_timeframe_short_label(tf_key)
                    )));
                } else if let Some(source) = chart.source_override.clone() {
                    let source_symbol = preferred_chart_symbol_for_source(&source, symbol);
                    let queued = match source.as_str() {
                        "alpaca" => self.queue_alpaca_fetch(&source_symbol, tf_key),
                        "tastytrade" => self.queue_tastytrade_fetch(&source_symbol, tf_key),
                        "kraken" => self.queue_kraken_fetch(&source_symbol, tf_key),
                        "kraken-equities" => {
                            self.dispatch_kraken_equity_ticker(&source_symbol);
                            self.queue_kraken_equity_fetch(&source_symbol, tf_key)
                        }
                        "kraken-futures" => self.queue_kraken_futures_fetch(&source_symbol, tf_key),
                        _ => false,
                    };
                    if queued {
                        self.log.push_back(LogEntry::info(format!(
                            "No cached data for {} {} from {} — fetching...",
                            symbol,
                            tf.label(),
                            cache_source_label(&source)
                        )));
                    } else {
                        self.log.push_back(LogEntry::warn(format!(
                            "No cached data for {} {} from {}",
                            symbol,
                            tf.label(),
                            cache_source_label(&source)
                        )));
                    }
                } else if kraken_supported {
                    let queued = self.queue_kraken_fetch(&kraken_symbol, tf_key);
                    if queued {
                        self.log.push_back(LogEntry::info(format!(
                            "No cached data for {} {} — fetching from Kraken...",
                            symbol,
                            tf.label()
                        )));
                    }
                } else if tasty_supported {
                    if self.queue_tastytrade_fetch(symbol, tf_key) {
                        self.log.push_back(LogEntry::info(format!(
                            "No cached data for {} {} — fetching from tastytrade...",
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

        if let Some(existing_idx) = self.charts.iter().position(|chart| {
            chart.timeframe == Timeframe::D1
                && normalize_market_data_symbol(&chart.symbol)
                    .replace('/', "")
                    .trim_end_matches(".EQ")
                    .eq_ignore_ascii_case(
                        &symbol
                            .replace('/', "")
                            .trim_end_matches(".EQ")
                            .to_ascii_uppercase(),
                    )
        }) {
            self.active_tab = existing_idx;
            while self.mtf_visible.len() < self.charts.len() {
                self.mtf_visible.push(true);
            }
            if let Some(visible) = self.mtf_visible.get_mut(existing_idx) {
                *visible = true;
            }
            self.symbol_input = symbol.clone();
            self.mtf_enabled = true;
            self.compute_mtf_grid_status();
            self.log.push_back(LogEntry::info(format!(
                "News: focused existing {} D1 chart tab",
                symbol
            )));
            return true;
        }

        let mut chart = ChartState::new(&symbol, Timeframe::D1);
        if let Some(ref cache) = self.cache.clone() {
            let mut gpu = self.gpu_indicators.take();
            if !chart.try_load(Arc::as_ref(cache), &mut self.log, gpu.as_mut()) {
                self.gpu_indicators = gpu;
                self.charts.push(chart);
                let idx = self.charts.len().saturating_sub(1);
                self.queue_chart_reload(idx);
            } else {
                self.gpu_indicators = gpu;
                self.charts.push(chart);
            }
        } else {
            self.charts.push(chart);
        }
        self.active_tab = self.charts.len().saturating_sub(1);
        self.symbol_input = symbol.clone();
        while self.mtf_visible.len() < self.charts.len() {
            self.mtf_visible.push(true);
        }
        if let Some(visible) = self.mtf_visible.get_mut(self.active_tab) {
            *visible = true;
        }
        self.mtf_enabled = true;
        let queued =
            self.queue_symbol_fetch_for_source(&symbol, Timeframe::D1.cache_suffix(), None);
        self.compute_mtf_grid_status();
        self.log.push_back(LogEntry::info(if queued {
            format!("News: opened {} D1 chart tab and queued data fetch", symbol)
        } else {
            format!("News: opened {} D1 chart tab", symbol)
        }));
        true
    }

    pub(super) fn chart_source_options(
        &self,
        symbol: &str,
        tf: Timeframe,
    ) -> Vec<(String, &'static str)> {
        let Some(cache) = self.cache.as_ref() else {
            return Vec::new();
        };
        let tf_key = tf.cache_suffix();
        CHART_SOURCE_ORDER
            .iter()
            .filter_map(|(source, label)| {
                let has_bars = chart_source_cache_keys(source, symbol, tf_key)
                    .iter()
                    .chain(
                        (source == &"kraken-equities")
                            .then(|| chart_source_cache_keys(source, symbol, "quote"))
                            .unwrap_or_default()
                            .iter(),
                    )
                    .any(|key| matches!(cache.get_incremental_start(key), Ok(Some(_))));
                has_bars.then(|| ((*source).to_string(), *label))
            })
            .collect()
    }

    /// Compute MTF Grid indicator status for all timeframes from cache.
    /// Parallel: spawns threads for TFs not already loaded in chart tabs.
    pub(super) fn compute_mtf_grid_status(&mut self) {
        self.mtf_grid_status.clear();
        let cache = match &self.cache {
            Some(c) => Arc::clone(c),
            None => return,
        };
        let sym = self.symbol_input.trim().to_string();
        if sym.is_empty() {
            return;
        }
        let source_override = self
            .charts
            .get(self.active_tab)
            .and_then(|chart| chart.source_override.clone());

        let all_tfs: &[(&'static str, Timeframe)] = &MTF_GRID_TIMEFRAMES;

        // Collect results from already-loaded charts (no thread needed)
        let mut preloaded: Vec<(
            &'static str,
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<f64>,
        )> = Vec::new();
        let mut need_load: Vec<(&'static str, Timeframe)> = Vec::new();

        for &(label, tf) in all_tfs {
            if let Some(c) = self
                .charts
                .iter()
                .find(|c| c.timeframe == tf && !c.bars.is_empty())
            {
                let close = c.bars.last().map(|b| b.close);
                let sma = c.sma200.last().and_then(|v| *v);
                let kama = c.kama.last().and_then(|v| *v);
                let fisher = c.fisher.last().and_then(|v| *v);
                let fsig = c.fisher_signal.last().and_then(|v| *v);
                preloaded.push((label, close, sma, kama, fisher, fsig));
            } else {
                need_load.push((label, tf));
            }
        }

        if need_load.is_empty() {
            // All TFs already loaded — just use preloaded data, no blocking work needed
            let tf_idx: std::collections::HashMap<&str, usize> = all_tfs
                .iter()
                .enumerate()
                .map(|(i, &(l, _))| (l, i))
                .collect();
            preloaded.sort_by_key(|r| tf_idx.get(r.0).copied().unwrap_or(99));
            self.mtf_grid_status = preloaded;
        } else {
            // Spawn background thread for TFs that need cache loading — don't block UI
            self.mtf_grid_status = preloaded; // show what we have immediately
            let (tx, rx) = std::sync::mpsc::channel();
            let need_load_owned: Vec<(&'static str, Timeframe)> = need_load;
            let all_tfs_idx: std::collections::HashMap<&'static str, usize> = all_tfs
                .iter()
                .enumerate()
                .map(|(i, &(l, _))| (l, i))
                .collect();
            let rt_handle = self.rt_handle.clone();
            rt_handle.spawn_blocking(move || {
                let mut results: Vec<_> = Vec::new();
                for (label, tf) in need_load_owned {
                    let mut temp = ChartState::new(&sym, tf);
                    temp.source_override = source_override.clone();
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
                    }
                }
                results.sort_by_key(|r| all_tfs_idx.get(r.0).copied().unwrap_or(99));
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

    /// Set up MTF grid with N columns and target chart count.
    /// Creates charts for M15+ timeframes, filling up to `target` charts.
    pub(super) fn setup_mtf_grid(&mut self, cols: usize, target: usize) {
        let all_tfs = MTF_GRID_TIMEFRAMES.map(|(_, tf)| tf);
        let sym = self.symbol_input.trim().to_string();
        let source_override = self
            .charts
            .get(self.active_tab)
            .and_then(|chart| chart.source_override.clone());
        // Grow charts to target count
        while self.charts.len() < target {
            let tf_idx = self.charts.len() % all_tfs.len();
            let mut chart = ChartState::new(&sym, all_tfs[tf_idx]);
            chart.source_override = source_override.clone();
            if let Some(ref cache) = self.cache {
                {
                    let mut gpu = self.gpu_indicators.take();
                    if !chart.try_load(cache, &mut self.log, gpu.as_mut()) {
                        self.queue_chart_reload(0);
                    }
                    self.gpu_indicators = gpu;
                }
            }
            self.charts.push(chart);
        }
        // Load any existing charts that have empty bars (e.g. from session restore)
        if let Some(ref cache) = self.cache {
            let mut retry_first_chart = false;
            for chart in &mut self.charts {
                if chart.bars.is_empty() {
                    {
                        let mut gpu = self.gpu_indicators.take();
                        if !chart.try_load(cache, &mut self.log, gpu.as_mut()) {
                            retry_first_chart = true;
                        }
                        self.gpu_indicators = gpu;
                    }
                }
            }
            if retry_first_chart {
                self.queue_chart_reload(0);
            }
        }
        self.mtf_cols = cols;
        self.mtf_enabled = true;
        self.log.push_back(LogEntry::info(format!(
            "MTF grid: {}×{} ({} charts)",
            cols,
            (target + cols - 1) / cols,
            self.charts.len()
        )));
    }

    /// Build trade overlay for a chart: DARWIN deals as arrows + open position lines.
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

        // Parse MQL5 time string "YYYY.MM.DD HH:MM:SS" to epoch ms
        let parse_time = |s: &str| -> i64 {
            // "2024.10.08 16:47:19" → chrono parse
            let s = s.replace('.', "-");
            chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                .map(|dt| dt.and_utc().timestamp_millis())
                .unwrap_or(0)
        };

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

        // Collect deals from all DARWIN accounts matching this symbol
        use std::collections::HashMap;
        let bare_upper = bare_sym.to_uppercase();
        let mut marker_map: HashMap<(usize, bool, i64), (f64, u32, String)> = HashMap::new(); // (bar_idx, is_buy, price_cents) → (total_vol, count, ticker)

        if self.show_darwin_positions {
            for det in &self.bg.account_details {
                // Check closed positions (have SL/TP)
                for pos in &det.closed_positions {
                    if !symbol_matches_no_alloc(&pos.symbol, &bare_upper) {
                        continue;
                    }
                    let ticker = det.ticker.clone();
                    // Entry arrow
                    let ts = parse_time(&pos.open_time);
                    if let Some(bar_idx) = find_bar(ts) {
                        let is_buy = pos.pos_type == "buy";
                        let price_key = (pos.open_price * 100000.0) as i64;
                        let entry = marker_map.entry((bar_idx, is_buy, price_key)).or_insert((
                            0.0,
                            0,
                            String::new(),
                        ));
                        entry.0 += pos.volume;
                        entry.1 += 1;
                        if !entry.2.contains(&ticker) {
                            if !entry.2.is_empty() {
                                entry.2.push_str(", ");
                            }
                            entry.2.push_str(&ticker);
                        }
                    }
                    // Exit arrow (opposite direction)
                    if !pos.close_time.is_empty() {
                        let ts = parse_time(&pos.close_time);
                        if let Some(bar_idx) = find_bar(ts) {
                            let is_buy = pos.pos_type != "buy";
                            let price_key = (pos.close_price * 100000.0) as i64;
                            let entry = marker_map.entry((bar_idx, is_buy, price_key)).or_insert((
                                0.0,
                                0,
                                String::new(),
                            ));
                            entry.0 += pos.volume;
                            entry.1 += 1;
                            if !entry.2.contains(&ticker) {
                                if !entry.2.is_empty() {
                                    entry.2.push_str(", ");
                                }
                                entry.2.push_str(&ticker);
                            }
                        }
                    }
                }
                // Check recent deals
                for deal in &det.recent_deals {
                    if !symbol_matches_no_alloc(&deal.symbol, &bare_upper) {
                        continue;
                    }
                    if deal.direction.is_empty() {
                        continue;
                    } // skip balance entries
                    let ts = parse_time(&deal.time);
                    if let Some(bar_idx) = find_bar(ts) {
                        let is_buy = deal.deal_type == "buy";
                        let price_key = (deal.price * 100000.0) as i64;
                        let entry = marker_map.entry((bar_idx, is_buy, price_key)).or_insert((
                            0.0,
                            0,
                            String::new(),
                        ));
                        entry.0 += deal.volume;
                        entry.1 += 1;
                        if !entry.2.contains(&det.ticker) {
                            if !entry.2.is_empty() {
                                entry.2.push_str(", ");
                            }
                            entry.2.push_str(&det.ticker);
                        }
                    }
                }

                // Open position lines (entry, SL, TP)
                for pos in &det.open_positions {
                    if !symbol_matches_no_alloc(&pos.symbol, &bare_upper) {
                        continue;
                    }
                    let is_buy = pos.side == "buy";
                    overlay.position_lines.push(PositionLine {
                        price: pos.avg_price,
                        volume: pos.total_volume,
                        is_buy,
                        line_type: 0, // entry
                    });
                }
            }
        } // show_darwin_positions

        // Also check portfolio-level open positions (aggregated across all DARWINs)
        if self.show_darwin_positions {
            for pos in &self.bg.open_positions {
                if !symbol_matches_no_alloc(&pos.symbol, &bare_upper) {
                    continue;
                }
                let is_buy = pos.side == "buy";
                // Only add if not already covered by per-account positions
                let already = overlay
                    .position_lines
                    .iter()
                    .any(|pl| (pl.price - pos.avg_price).abs() < 0.0001 && pl.is_buy == is_buy);
                if !already {
                    overlay.position_lines.push(PositionLine {
                        price: pos.avg_price,
                        volume: pos.total_volume,
                        is_buy,
                        line_type: 0,
                    });
                }
            }
        }

        // Broker fills — add to marker map before conversion. Alpaca currently
        // enters through `recent_fills`; Kraken keeps a full REST + private-WS
        // trade deque. Both paths feed the same chart arrows as DARWIN deals.
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

        // Live broker position lines (Alpaca + tastytrade + Kraken).
        // Kraken spot crypto balances are inventory rather than broker
        // `PositionInfo` rows, but the chart still needs a visible holding
        // entry line when cost basis is known.
        let alpaca_iter: Box<dyn Iterator<Item = &PositionInfo>> = if self.show_alpaca_positions {
            Box::new(self.live_positions.iter())
        } else {
            Box::new(std::iter::empty())
        };
        let tt_iter: Box<dyn Iterator<Item = &PositionInfo>> = if self.show_tt_positions {
            Box::new(self.tt_positions.iter())
        } else {
            Box::new(std::iter::empty())
        };
        let kr_iter: Box<dyn Iterator<Item = &PositionInfo>> = if self.show_kr_positions {
            Box::new(self.kr_positions.iter())
        } else {
            Box::new(std::iter::empty())
        };
        let all_broker_positions = alpaca_iter.chain(tt_iter).chain(kr_iter);
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
            overlay.position_lines.push(PositionLine {
                price: pos.avg_entry_price,
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

        // Deduplicate position lines (aggregate same price+type)
        {
            let mut agg: HashMap<(i64, u8), (f64, bool)> = HashMap::new();
            for pl in &overlay.position_lines {
                let key = ((pl.price * 100000.0) as i64, pl.line_type);
                let entry = agg.entry(key).or_insert((0.0, pl.is_buy));
                entry.0 += pl.volume;
            }
            overlay.position_lines = agg
                .into_iter()
                .map(|((pk, lt), (vol, is_buy))| PositionLine {
                    price: pk as f64 / 100000.0,
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
}
