use super::*;

pub(super) fn build_unresolvable_fetch_key_index(
    pairs: &std::collections::HashMap<String, UnresolvablePair>,
) -> std::collections::HashMap<String, std::collections::HashSet<String>> {
    let mut index = std::collections::HashMap::new();
    for entry in pairs.values() {
        let Some(tf) = normalize_sync_timeframe_key(&entry.timeframe) else {
            continue;
        };
        let symbol = normalize_market_data_symbol(&entry.symbol).replace('/', "");
        if symbol.is_empty() {
            continue;
        }
        index
            .entry(entry.broker.to_ascii_lowercase())
            .or_insert_with(std::collections::HashSet::new)
            .insert(alpaca_fetch_key(&symbol, tf));
    }
    index
}

impl TyphooNApp {
    pub(super) fn rebuild_unresolvable_fetch_key_index(&mut self) {
        self.unresolvable_fetch_keys_by_broker =
            build_unresolvable_fetch_key_index(&self.unresolvable_pairs);
    }

    pub(super) fn alpaca_retry_backoff_secs(retry_count: u32) -> i64 {
        match retry_count {
            0 | 1 => 15,
            2 => 30,
            3 => 60,
            4 => 180,
            _ => 900,
        }
    }

    /// Load the persisted retry queue from cache KV on first tick.
    pub(super) fn alpaca_retry_load(&mut self) {
        if let Some(ref cache) = self.cache {
            if let Ok(Some(json)) = cache.get_kv("alpaca:retry_queue") {
                if let Ok(queue) = serde_json::from_str::<Vec<AlpacaRetry>>(&json) {
                    self.alpaca_retry_queue = queue;
                }
            }
        }
        self.alpaca_retry_loaded = true;
    }

    pub(super) fn alpaca_retry_save(&self) {
        if let Some(ref cache) = self.cache {
            let json =
                serde_json::to_string(&self.alpaca_retry_queue).unwrap_or_else(|_| "[]".into());
            let _ = cache.put_kv("alpaca:retry_queue", &json);
        }
    }

    pub(super) fn alpaca_no_data_load(&mut self) {
        if let Some(ref cache) = self.cache {
            if let Ok(Some(json)) = cache.get_kv("alpaca:no_data_pairs") {
                if let Some(entries) = deserialize_alpaca_no_data_pairs(&json) {
                    self.alpaca_no_data_pairs = entries
                        .into_iter()
                        .map(|entry| (alpaca_fetch_key(&entry.symbol, &entry.timeframe), entry))
                        .collect();
                } else {
                    tracing::warn!("alpaca:no_data_pairs contained unreadable persisted data");
                }
            }
        }
        self.alpaca_no_data_loaded = true;
    }

    pub(super) fn alpaca_no_data_save(&self) {
        if let Some(ref cache) = self.cache {
            let mut entries: Vec<AlpacaNoDataPair> =
                self.alpaca_no_data_pairs.values().cloned().collect();
            entries.sort_by(|a, b| {
                a.symbol.cmp(&b.symbol).then(
                    sync_timeframe_sort_key(&a.timeframe)
                        .cmp(&sync_timeframe_sort_key(&b.timeframe)),
                )
            });
            let json = serde_json::to_string(&entries).unwrap_or_else(|_| "[]".into());
            let _ = cache.put_kv("alpaca:no_data_pairs", &json);
        }
    }

    pub(super) fn alpaca_no_data_mark(
        &mut self,
        symbol: &str,
        timeframe: &str,
        reason: &str,
    ) -> bool {
        if !self.alpaca_no_data_loaded {
            self.alpaca_no_data_load();
        }
        let timeframe = normalize_sync_timeframe_key(timeframe)
            .unwrap_or(timeframe)
            .to_string();
        let symbol = normalize_market_data_symbol(symbol).replace('/', "");
        let key = alpaca_fetch_key(&symbol, &timeframe);
        let entry = AlpacaNoDataPair {
            symbol,
            timeframe,
            marked_at: chrono::Utc::now().timestamp(),
            reason: reason.to_string(),
        };
        let changed = match self.alpaca_no_data_pairs.get(&key) {
            Some(existing) => existing.reason != entry.reason,
            None => true,
        };
        self.alpaca_no_data_pairs.insert(key, entry);
        self.alpaca_no_data_save();
        changed
    }

    pub(super) fn alpaca_no_data_drain(&mut self, symbol: &str, timeframe: &str) {
        if !self.alpaca_no_data_loaded {
            self.alpaca_no_data_load();
        }
        let before = self.alpaca_no_data_pairs.len();
        self.alpaca_no_data_pairs
            .remove(&alpaca_fetch_key(symbol, timeframe));
        if self.alpaca_no_data_pairs.len() != before {
            self.alpaca_no_data_save();
        }
    }

    pub(super) fn alpaca_no_data_clear_all(&mut self) {
        if !self.alpaca_no_data_loaded {
            self.alpaca_no_data_load();
        }
        if self.alpaca_no_data_pairs.is_empty() {
            return;
        }
        self.alpaca_no_data_pairs.clear();
        self.alpaca_no_data_save();
    }

    pub(super) fn unresolvable_load(&mut self) {
        if let Some(ref cache) = self.cache {
            if let Ok(Some(json)) = cache.get_kv("broker:unresolvable_pairs") {
                match serde_json::from_str::<Vec<UnresolvablePair>>(&json) {
                    Ok(entries) => {
                        self.unresolvable_pairs = entries
                            .into_iter()
                            .map(|entry| {
                                let key = unresolvable_pair_key(
                                    &entry.broker,
                                    &entry.symbol,
                                    &entry.timeframe,
                                );
                                (key, entry)
                            })
                            .collect();
                        self.rebuild_unresolvable_fetch_key_index();
                    }
                    Err(e) => tracing::warn!(
                        "broker:unresolvable_pairs contained unreadable persisted data: {e}"
                    ),
                }
            }
        }
    }

    pub(super) fn unresolvable_save(&self) {
        if let Some(ref cache) = self.cache {
            let mut entries: Vec<UnresolvablePair> =
                self.unresolvable_pairs.values().cloned().collect();
            entries.sort_by(|a, b| {
                a.broker.cmp(&b.broker).then(a.symbol.cmp(&b.symbol)).then(
                    sync_timeframe_sort_key(&a.timeframe)
                        .cmp(&sync_timeframe_sort_key(&b.timeframe)),
                )
            });
            let json = serde_json::to_string(&entries).unwrap_or_else(|_| "[]".into());
            let _ = cache.put_kv("broker:unresolvable_pairs", &json);
        }
    }

    pub(super) fn unresolvable_mark(
        &mut self,
        broker: &str,
        symbol: &str,
        timeframe: &str,
        reason: &str,
    ) -> bool {
        let timeframe = normalize_sync_timeframe_key(timeframe)
            .unwrap_or(timeframe)
            .to_string();
        let symbol = normalize_market_data_symbol(symbol).replace('/', "");
        let broker = broker.to_ascii_lowercase();
        let key = unresolvable_pair_key(&broker, &symbol, &timeframe);
        let entry = UnresolvablePair {
            broker,
            symbol,
            timeframe,
            reason: reason.to_string(),
            ts: chrono::Utc::now().timestamp(),
        };
        let changed = self
            .unresolvable_pairs
            .get(&key)
            .is_none_or(|existing| existing.reason != entry.reason);
        self.unresolvable_fetch_keys_by_broker
            .entry(entry.broker.clone())
            .or_default()
            .insert(alpaca_fetch_key(&entry.symbol, &entry.timeframe));
        self.unresolvable_pairs.insert(key, entry);
        self.unresolvable_save();
        changed
    }

    pub(super) fn unresolvable_clear_all(&mut self) {
        if self.unresolvable_pairs.is_empty() {
            return;
        }
        self.unresolvable_pairs.clear();
        self.unresolvable_fetch_keys_by_broker.clear();
        self.unresolvable_save();
    }

    pub(super) fn alpaca_backfill_complete_load(&mut self) {
        if let Some(ref cache) = self.cache {
            if let Ok(Some(json)) = cache.get_kv("alpaca:backfill_complete_pairs") {
                if let Ok(entries) = serde_json::from_str::<Vec<AlpacaBackfillCompletePair>>(&json)
                {
                    self.alpaca_backfill_complete_pairs = entries
                        .into_iter()
                        .map(|entry| (alpaca_fetch_key(&entry.symbol, &entry.timeframe), entry))
                        .collect();
                }
            }
        }
        self.alpaca_backfill_complete_loaded = true;
        self.alpaca_backfill_complete_dirty_since = None;
    }

    pub(super) fn alpaca_backfill_complete_save(&self) {
        if let Some(ref cache) = self.cache {
            let mut entries: Vec<AlpacaBackfillCompletePair> = self
                .alpaca_backfill_complete_pairs
                .values()
                .cloned()
                .collect();
            entries.sort_by(|a, b| {
                a.symbol.cmp(&b.symbol).then(
                    sync_timeframe_sort_key(&a.timeframe)
                        .cmp(&sync_timeframe_sort_key(&b.timeframe)),
                )
            });
            let json = serde_json::to_string(&entries).unwrap_or_else(|_| "[]".into());
            let _ = cache.put_kv("alpaca:backfill_complete_pairs", &json);
        }
    }

    pub(super) fn alpaca_backfill_complete_mark(
        &mut self,
        symbol: &str,
        timeframe: &str,
        bar_count: usize,
        target_bars: usize,
    ) -> bool {
        if !self.alpaca_backfill_complete_loaded {
            self.alpaca_backfill_complete_load();
        }
        let timeframe = normalize_sync_timeframe_key(timeframe)
            .unwrap_or(timeframe)
            .to_string();
        let symbol = normalize_market_data_symbol(symbol).replace('/', "");
        let key = alpaca_fetch_key(&symbol, &timeframe);
        let entry = AlpacaBackfillCompletePair {
            symbol,
            timeframe,
            marked_at: chrono::Utc::now().timestamp(),
            bar_count: bar_count as i64,
            target_bars: target_bars as i64,
        };
        let changed = match self.alpaca_backfill_complete_pairs.get(&key) {
            Some(existing) => {
                existing.bar_count != entry.bar_count || existing.target_bars != entry.target_bars
            }
            None => true,
        };
        if changed {
            self.alpaca_backfill_complete_pairs.insert(key, entry);
            if self.alpaca_backfill_complete_dirty_since.is_none() {
                self.alpaca_backfill_complete_dirty_since = Some(std::time::Instant::now());
            }
        }
        changed
    }

    pub(super) fn flush_alpaca_backfill_complete_marks(&mut self, force: bool) {
        let Some(dirty_since) = self.alpaca_backfill_complete_dirty_since else {
            return;
        };
        if !force
            && std::time::Instant::now().saturating_duration_since(dirty_since)
                < std::time::Duration::from_secs(2)
        {
            return;
        }
        self.alpaca_backfill_complete_save();
        self.alpaca_backfill_complete_dirty_since = None;
    }

    pub(super) fn load_backfill_complete_pairs_from_kv(
        &self,
        kv_key: &str,
    ) -> std::collections::HashMap<String, AlpacaBackfillCompletePair> {
        let Some(ref cache) = self.cache else {
            return std::collections::HashMap::new();
        };
        let Ok(Some(json)) = cache.get_kv(kv_key) else {
            return std::collections::HashMap::new();
        };
        serde_json::from_str::<Vec<AlpacaBackfillCompletePair>>(&json)
            .map(|entries| {
                entries
                    .into_iter()
                    .map(|entry| (alpaca_fetch_key(&entry.symbol, &entry.timeframe), entry))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(super) fn save_backfill_complete_pairs_to_kv(
        &self,
        kv_key: &str,
        pairs: &std::collections::HashMap<String, AlpacaBackfillCompletePair>,
    ) {
        if let Some(ref cache) = self.cache {
            let mut entries: Vec<AlpacaBackfillCompletePair> = pairs.values().cloned().collect();
            entries.sort_by(|a, b| {
                a.symbol.cmp(&b.symbol).then(
                    sync_timeframe_sort_key(&a.timeframe)
                        .cmp(&sync_timeframe_sort_key(&b.timeframe)),
                )
            });
            let json = serde_json::to_string(&entries).unwrap_or_else(|_| "[]".into());
            let _ = cache.put_kv(kv_key, &json);
        }
    }

    pub(super) fn kraken_backfill_complete_load(&mut self) {
        self.kraken_backfill_complete_pairs =
            self.load_backfill_complete_pairs_from_kv("kraken:backfill_complete_pairs");
        self.kraken_backfill_complete_loaded = true;
        self.kraken_backfill_complete_dirty_since = None;
    }

    pub(super) fn kraken_futures_backfill_complete_load(&mut self) {
        self.kraken_futures_backfill_complete_pairs =
            self.load_backfill_complete_pairs_from_kv("kraken-futures:backfill_complete_pairs");
        self.kraken_futures_backfill_complete_loaded = true;
        self.kraken_futures_backfill_complete_dirty_since = None;
    }

    pub(super) fn tastytrade_backfill_complete_load(&mut self) {
        self.tastytrade_backfill_complete_pairs =
            self.load_backfill_complete_pairs_from_kv("tastytrade:backfill_complete_pairs");
        self.tastytrade_backfill_complete_loaded = true;
        self.tastytrade_backfill_complete_dirty_since = None;
    }

    pub(super) fn kraken_backfill_complete_mark(
        &mut self,
        symbol: &str,
        timeframe: &str,
        bar_count: usize,
        target_bars: usize,
    ) -> bool {
        if !self.kraken_backfill_complete_loaded {
            self.kraken_backfill_complete_load();
        }
        let timeframe = normalize_sync_timeframe_key(timeframe)
            .unwrap_or(timeframe)
            .to_string();
        let symbol = typhoon_engine::core::kraken::normalize_pair_symbol(symbol);
        let key = alpaca_fetch_key(&symbol, &timeframe);
        let entry = AlpacaBackfillCompletePair {
            symbol,
            timeframe,
            marked_at: chrono::Utc::now().timestamp(),
            bar_count: bar_count as i64,
            target_bars: target_bars as i64,
        };
        let changed = match self.kraken_backfill_complete_pairs.get(&key) {
            Some(existing) => {
                existing.bar_count != entry.bar_count || existing.target_bars != entry.target_bars
            }
            None => true,
        };
        if changed {
            self.kraken_backfill_complete_pairs.insert(key, entry);
            if self.kraken_backfill_complete_dirty_since.is_none() {
                self.kraken_backfill_complete_dirty_since = Some(std::time::Instant::now());
            }
        }
        changed
    }

    pub(super) fn kraken_futures_backfill_complete_mark(
        &mut self,
        symbol: &str,
        timeframe: &str,
        bar_count: usize,
        target_bars: usize,
    ) -> bool {
        if !self.kraken_futures_backfill_complete_loaded {
            self.kraken_futures_backfill_complete_load();
        }
        let timeframe = normalize_sync_timeframe_key(timeframe)
            .unwrap_or(timeframe)
            .to_string();
        let symbol = typhoon_engine::core::kraken_futures::normalize_futures_symbol(symbol);
        let key = alpaca_fetch_key(&symbol, &timeframe);
        let entry = AlpacaBackfillCompletePair {
            symbol,
            timeframe,
            marked_at: chrono::Utc::now().timestamp(),
            bar_count: bar_count as i64,
            target_bars: target_bars as i64,
        };
        let changed = match self.kraken_futures_backfill_complete_pairs.get(&key) {
            Some(existing) => {
                existing.bar_count != entry.bar_count || existing.target_bars != entry.target_bars
            }
            None => true,
        };
        if changed {
            self.kraken_futures_backfill_complete_pairs
                .insert(key, entry);
            if self.kraken_futures_backfill_complete_dirty_since.is_none() {
                self.kraken_futures_backfill_complete_dirty_since = Some(std::time::Instant::now());
            }
        }
        changed
    }

    pub(super) fn tastytrade_backfill_complete_mark(
        &mut self,
        symbol: &str,
        timeframe: &str,
        bar_count: usize,
        target_bars: usize,
    ) -> bool {
        if !self.tastytrade_backfill_complete_loaded {
            self.tastytrade_backfill_complete_load();
        }
        let timeframe = normalize_sync_timeframe_key(timeframe)
            .unwrap_or(timeframe)
            .to_string();
        let symbol = normalize_market_data_symbol(symbol);
        let key = alpaca_fetch_key(&symbol.replace('/', ""), &timeframe);
        let entry = AlpacaBackfillCompletePair {
            symbol,
            timeframe,
            marked_at: chrono::Utc::now().timestamp(),
            bar_count: bar_count as i64,
            target_bars: target_bars as i64,
        };
        let changed = match self.tastytrade_backfill_complete_pairs.get(&key) {
            Some(existing) => {
                existing.bar_count != entry.bar_count || existing.target_bars != entry.target_bars
            }
            None => true,
        };
        if changed {
            self.tastytrade_backfill_complete_pairs.insert(key, entry);
            if self.tastytrade_backfill_complete_dirty_since.is_none() {
                self.tastytrade_backfill_complete_dirty_since = Some(std::time::Instant::now());
            }
        }
        changed
    }

    pub(super) fn flush_kraken_backfill_complete_marks(&mut self, force: bool) {
        if let Some(dirty_since) = self.kraken_backfill_complete_dirty_since {
            if force
                || std::time::Instant::now().saturating_duration_since(dirty_since)
                    >= std::time::Duration::from_secs(2)
            {
                self.save_backfill_complete_pairs_to_kv(
                    "kraken:backfill_complete_pairs",
                    &self.kraken_backfill_complete_pairs,
                );
                self.kraken_backfill_complete_dirty_since = None;
            }
        }
        if let Some(dirty_since) = self.kraken_futures_backfill_complete_dirty_since {
            if force
                || std::time::Instant::now().saturating_duration_since(dirty_since)
                    >= std::time::Duration::from_secs(2)
            {
                self.save_backfill_complete_pairs_to_kv(
                    "kraken-futures:backfill_complete_pairs",
                    &self.kraken_futures_backfill_complete_pairs,
                );
                self.kraken_futures_backfill_complete_dirty_since = None;
            }
        }
        if let Some(dirty_since) = self.tastytrade_backfill_complete_dirty_since {
            if force
                || std::time::Instant::now().saturating_duration_since(dirty_since)
                    >= std::time::Duration::from_secs(2)
            {
                self.save_backfill_complete_pairs_to_kv(
                    "tastytrade:backfill_complete_pairs",
                    &self.tastytrade_backfill_complete_pairs,
                );
                self.tastytrade_backfill_complete_dirty_since = None;
            }
        }
    }

    /// Upsert a (symbol, timeframe) pair into the retry queue. Called when
    /// the fetch worker signals `AlpacaRetryEnqueue` for 429/partial/error outcomes.
    pub(super) fn alpaca_retry_enqueue(&mut self, symbol: &str, timeframe: &str, reason: &str) {
        if !self.alpaca_no_data_loaded {
            self.alpaca_no_data_load();
        }
        if self
            .alpaca_no_data_pairs
            .contains_key(&alpaca_fetch_key(symbol, timeframe))
        {
            return;
        }
        let now = chrono::Utc::now().timestamp();
        let partial = reason.contains("partial");
        if let Some(e) = self
            .alpaca_retry_queue
            .iter_mut()
            .find(|e| e.symbol == symbol && e.timeframe == timeframe)
        {
            e.retry_count = e.retry_count.saturating_add(1);
            e.last_attempt = now;
            e.next_attempt = now + Self::alpaca_retry_backoff_secs(e.retry_count);
            e.last_error = reason.to_string();
            if partial {
                e.partial = true;
            }
        } else {
            self.alpaca_retry_queue.push(AlpacaRetry {
                symbol: symbol.to_string(),
                timeframe: timeframe.to_string(),
                last_attempt: now,
                next_attempt: now + Self::alpaca_retry_backoff_secs(1),
                retry_count: 1,
                last_error: reason.to_string(),
                partial,
            });
        }
        self.alpaca_retry_save();
    }

    /// Clear a successful (symbol, timeframe) from the retry queue.
    pub(super) fn alpaca_retry_drain(&mut self, symbol: &str, timeframe: &str) {
        let before = self.alpaca_retry_queue.len();
        self.alpaca_retry_queue
            .retain(|e| !(e.symbol == symbol && e.timeframe == timeframe));
        if self.alpaca_retry_queue.len() != before {
            self.alpaca_retry_save();
        }
    }

    /// Periodic retry-queue tick. Invoked from `update()` at most once per
    /// 10s. Loads persisted state on first call; evicts entries older than 24h
    /// or with 20+ retries; re-dispatches any entry whose `next_attempt` has
    /// passed. Each redispatch bumps `next_attempt` immediately so a slow
    /// response can't cause duplicate requests on the next tick.
    pub(super) fn poll_alpaca_retry_queue(&mut self) {
        if !self.alpaca_retry_loaded {
            self.alpaca_retry_load();
        }
        if !self.alpaca_no_data_loaded {
            self.alpaca_no_data_load();
        }
        let now = chrono::Utc::now().timestamp();
        if now - self.alpaca_retry_last_poll < 10 {
            return;
        }
        self.alpaca_retry_last_poll = now;

        const MAX_AGE_SECS: i64 = 24 * 3600;
        let before = self.alpaca_retry_queue.len();
        self.alpaca_retry_queue
            .retain(|e| now - e.last_attempt <= MAX_AGE_SECS && e.retry_count < 20);
        if self.alpaca_retry_queue.len() != before {
            self.alpaca_retry_save();
        }

        if !self.broker_connected
            || (!self.alpaca_full_bar_sync_enabled && !self.backfill_alpaca_kraken_equities_enabled)
            || self.alpaca_retry_queue.is_empty()
        {
            return;
        }

        let enabled_sync_timeframes = self.enabled_sync_timeframes.clone();
        let retry_len_before = self.alpaca_retry_queue.len();
        self.alpaca_retry_queue.retain(|e| {
            normalize_sync_timeframe_key(&e.timeframe)
                .map(|tf| enabled_sync_timeframes.contains(tf))
                .unwrap_or(false)
        });
        if self.alpaca_retry_queue.len() != retry_len_before {
            self.alpaca_retry_save();
        }
        if self.alpaca_retry_queue.is_empty() {
            return;
        }

        let retry_len_before = self.alpaca_retry_queue.len();
        self.alpaca_retry_queue.retain(|e| {
            !self
                .alpaca_no_data_pairs
                .contains_key(&alpaca_fetch_key(&e.symbol, &e.timeframe))
        });
        if self.alpaca_retry_queue.len() != retry_len_before {
            self.alpaca_retry_save();
        }
        if self.alpaca_retry_queue.is_empty() {
            return;
        }

        let due: Vec<(String, String)> = self
            .alpaca_retry_queue
            .iter()
            .filter(|e| e.next_attempt <= now)
            .map(|e| (e.symbol.clone(), e.timeframe.clone()))
            .collect();
        if due.is_empty() {
            return;
        }

        let mut redispatched = 0usize;
        for (sym, tf) in &due {
            if self.queue_alpaca_fetch(sym, tf) {
                redispatched += 1;
                if let Some(e) = self
                    .alpaca_retry_queue
                    .iter_mut()
                    .find(|e| e.symbol == *sym && e.timeframe == *tf)
                {
                    e.last_attempt = now;
                    e.next_attempt = now + Self::alpaca_retry_backoff_secs(e.retry_count + 1);
                }
            }
        }
        if redispatched == 0 {
            return;
        }
        self.alpaca_retry_save();
        self.log.push_back(LogEntry::info(format!(
            "Alpaca retry: re-dispatched {} symbol(s) ({} in queue)",
            redispatched,
            self.alpaca_retry_queue.len()
        )));
    }

    /// Format a Unix timestamp as a relative staleness label for UI display.
    /// Returns (label, color) so the caller can render with appropriate urgency.
    /// `ts=0` means "never fetched".
    pub(super) fn staleness_badge(&self, ts: i64) -> (String, egui::Color32) {
        if ts == 0 {
            return ("— never".to_string(), AXIS_TEXT);
        }
        let age = chrono::Utc::now().timestamp() - ts;
        if age < 0 {
            // Clock skew — treat as fresh
            return ("fresh".to_string(), egui::Color32::from_rgb(120, 220, 120));
        }
        if age < 30 {
            (format!("{}s", age), egui::Color32::from_rgb(120, 220, 120))
        } else if age < 120 {
            (format!("{}s", age), AXIS_TEXT)
        } else if age < 600 {
            (
                format!("{}m", age / 60),
                egui::Color32::from_rgb(220, 180, 60),
            )
        } else {
            (
                format!("{}m STALE", age / 60),
                egui::Color32::from_rgb(231, 76, 60),
            )
        }
    }

    pub(super) fn active_symbols(&self) -> Vec<String> {
        // PERF: O(1) dedup via HashSet (was O(n²) Vec::contains).
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut syms: Vec<String> = Vec::new();
        let add =
            |s: &str, syms: &mut Vec<String>, seen: &mut std::collections::HashSet<String>| {
                let t = s
                    .split(':')
                    .rev()
                    .nth(1)
                    .or_else(|| s.split(':').last())
                    .unwrap_or(s)
                    .to_uppercase();
                if !t.is_empty() && seen.insert(t.clone()) {
                    syms.push(t);
                }
            };
        // Open chart tabs are always foreground sync targets, not just the
        // currently visible tab. If a chart exists, it should stop showing
        // "waiting for data" before broad-universe backfill gets more slots.
        for c in &self.charts {
            add(&c.symbol, &mut syms, &mut seen);
        }
        // Broker positions are foreground sync targets only while that broker's
        // positions are displayed. If the navbar hides Alpaca/Tasty/Kraken
        // positions, those symbols should stop consuming update slots unless
        // they are also open chart tabs, open orders, or watchlist entries.
        if self.show_alpaca_positions {
            for p in &self.live_positions {
                add(&p.symbol, &mut syms, &mut seen);
            }
        }
        if self.show_tt_positions {
            for p in &self.tt_positions {
                add(&p.symbol, &mut syms, &mut seen);
            }
        }
        if self.show_kr_positions {
            for p in &self.kr_positions {
                add(&p.symbol, &mut syms, &mut seen);
            }
        }
        // Open orders are live exposure even before a fill creates a position.
        for o in &self.live_orders {
            add(&o.symbol, &mut syms, &mut seen);
        }
        for o in &self.kraken_open_orders {
            add(&o.pair, &mut syms, &mut seen);
        }
        // Watchlist
        for s in &self.user_watchlist {
            add(s, &mut syms, &mut seen);
        }
        syms
    }

    /// Build the symbol set the navbar News section is allowed to surface.
    ///
    /// Drives the right-panel news filter: only articles whose primary
    /// symbol or any tagged ticker hits this set are shown. Built once per
    /// render (O(n) over the source collections) so per-article lookups
    /// are O(1) via HashSet::contains. Returns an empty set if the user
    /// has no open charts / positions / orders / holdings / watchlist —
    /// callers treat that as "show everything" rather than "show nothing"
    /// so a fresh app instance with no state attached still renders news.
    pub(super) fn news_focus_symbols(&self) -> std::collections::HashSet<String> {
        // Start from active_symbols(): open chart tabs + alpaca positions +
        // tt positions + kraken positions + user watchlist (deduped).
        let mut set: std::collections::HashSet<String> =
            self.active_symbols().into_iter().collect();

        // Open orders: live exposure that may not have a filled position yet.
        for o in &self.live_orders {
            let s = o.symbol.trim().to_ascii_uppercase();
            if !s.is_empty() {
                set.insert(s);
            }
        }
        for o in &self.kraken_open_orders {
            let s = o.pair.trim().to_ascii_uppercase();
            if !s.is_empty() {
                set.insert(s);
            }
        }

        // Kraken balances: held assets that may not appear as positions
        // (e.g. spot crypto with no open futures contract). Strip the
        // .EQ suffix on tokenized equities so news tagged with the
        // underlying symbol (TSLA vs TSLA.EQ) still matches.
        for (asset, qty) in &self.kraken_balances {
            if !qty.is_finite() || *qty <= 0.0 {
                continue;
            }
            let display = Self::kraken_display_asset(asset);
            if Self::kraken_is_cash_balance_asset(asset) {
                // Fiat cash balances aren't news-worthy on their own.
                continue;
            }
            let base = display.trim_end_matches(".EQ");
            if !base.is_empty() {
                set.insert(base.to_string());
            }
        }

        set
    }

    /// O(1)-per-call check: does this article touch the user's focus set?
    /// `focus.is_empty()` short-circuits to true so an empty focus means
    /// "no filter" (see `news_focus_symbols` docs for the rationale).
    pub(super) fn news_article_in_focus(
        focus: &std::collections::HashSet<String>,
        primary_symbol: &str,
        tickers: &[String],
    ) -> bool {
        if focus.is_empty() {
            return true;
        }
        let primary = primary_symbol.trim().to_ascii_uppercase();
        if !primary.is_empty() && focus.contains(&primary) {
            return true;
        }
        tickers
            .iter()
            .any(|t| focus.contains(&t.trim().to_ascii_uppercase()))
    }

    pub(super) fn active_symbols_cache_key(&self) -> u64 {
        use std::hash::{Hash, Hasher};

        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.mtf_enabled.hash(&mut h);
        self.active_tab.hash(&mut h);
        for c in &self.charts {
            c.symbol.hash(&mut h);
        }
        self.show_alpaca_positions.hash(&mut h);
        self.show_tt_positions.hash(&mut h);
        self.show_kr_positions.hash(&mut h);
        for p in &self.live_positions {
            p.symbol.hash(&mut h);
        }
        for p in &self.tt_positions {
            p.symbol.hash(&mut h);
        }
        for p in &self.kr_positions {
            p.symbol.hash(&mut h);
        }
        for s in &self.user_watchlist {
            s.hash(&mut h);
        }
        h.finish()
    }

    pub(super) fn active_trade_symbol_and_price(&self) -> Option<(String, f64)> {
        let chart = self.charts.get(self.active_tab)?;
        let price = chart.bars.last()?.close;
        let symbol = normalize_market_data_symbol(&chart.symbol);
        if symbol.is_empty() {
            None
        } else {
            Some((symbol, price))
        }
    }

    pub(super) fn sync_trade_line_inputs(&mut self) {
        self.sl_input = self.sl_price.map(format_price).unwrap_or_default();
        self.tp_input = self.tp_price.map(format_price).unwrap_or_default();
    }

    pub(super) fn set_trade_lines(&mut self, sl: Option<f64>, tp: Option<f64>) {
        self.sl_price = sl;
        self.tp_price = tp;
        self.sl_enabled = sl.is_some();
        self.tp_enabled = tp.is_some();
        self.sync_trade_line_inputs();
    }

    pub(super) fn clear_trade_lines(&mut self) {
        self.set_trade_lines(None, None);
    }

    pub(super) fn set_visible_range_trade_lines(
        &mut self,
        is_buy: bool,
    ) -> Result<(f64, f64), String> {
        let (sl, tp) = {
            let chart = self
                .charts
                .get(self.active_tab)
                .ok_or_else(|| "Trade lines: active chart unavailable".to_string())?;
            let (si, ei) = chart.visible_range();
            if ei <= si || chart.bars.is_empty() {
                return Err("Trade lines: no visible bars on chart".to_string());
            }
            let vis = &chart.bars[si..ei];
            let lo = vis.iter().map(|b| b.low).fold(f64::MAX, f64::min);
            let hi = vis.iter().map(|b| b.high).fold(f64::MIN, f64::max);
            if is_buy { (lo, hi) } else { (hi, lo) }
        };
        self.set_trade_lines(Some(sl), Some(tp));
        Ok((sl, tp))
    }

    pub(super) fn infer_quick_trade_side_from_lines(
        &self,
        sl: f64,
        tp: f64,
    ) -> Result<usize, String> {
        if tp > sl {
            Ok(0)
        } else if sl > tp {
            Ok(1)
        } else {
            Err("Open Trade: TP and SL are at the same price".to_string())
        }
    }

    pub(super) fn floor_to_step(value: f64, step: f64) -> f64 {
        if step <= 0.0 {
            value
        } else {
            (value / step).floor() * step
        }
    }

    pub(super) fn build_trade_risk_config(&self) -> Result<risk::RiskConfig, String> {
        let mut cfg = risk::RiskConfig::default();
        cfg.order_mode = match self.risk_mode {
            RiskMode::Standard => risk::OrderMode::Standard,
            RiskMode::Fixed => risk::OrderMode::Fixed,
            RiskMode::Dynamic => risk::OrderMode::Dynamic,
            RiskMode::VaR | RiskMode::KrakenPro => risk::OrderMode::VaR,
        };
        cfg.var_mode = risk::VaRMode::PercentVaR;
        cfg.fixed_orders = 1;
        match self.risk_mode {
            RiskMode::Standard => {
                cfg.risk_pct = self
                    .trade_risk_pct_input
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| "Open Trade: invalid Risk %".to_string())?;
                if cfg.risk_pct <= 0.0 {
                    return Err("Open Trade: Risk % must be > 0".to_string());
                }
            }
            RiskMode::Fixed => {
                cfg.fixed_lots =
                    self.order_qty.trim().parse::<f64>().map_err(|_| {
                        format!("Open Trade: invalid quantity '{}'", self.order_qty)
                    })?;
                if cfg.fixed_lots <= 0.0 {
                    return Err("Open Trade: quantity must be > 0".to_string());
                }
            }
            RiskMode::Dynamic => {
                cfg.min_balance = self
                    .trade_min_balance_input
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| "Open Trade: invalid Min Bal".to_string())?;
                cfg.losses_to_min = self
                    .trade_losses_to_min_input
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| "Open Trade: invalid Losses".to_string())?;
                if cfg.losses_to_min == 0 {
                    return Err("Open Trade: Losses must be > 0".to_string());
                }
            }
            RiskMode::VaR | RiskMode::KrakenPro => {
                cfg.var_risk_pct = self
                    .trade_var_risk_pct_input
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| "Open Trade: invalid VaR %".to_string())?;
                if cfg.var_risk_pct <= 0.0 {
                    return Err("Open Trade: VaR % must be > 0".to_string());
                }
            }
        }
        Ok(cfg)
    }

    pub(super) fn trade_symbol_spec(&self, symbol: &str, last_price: f64) -> risk::SymbolSpec {
        let uses_whole_units = matches!(
            self.order_broker,
            OrderBroker::Tastytrade | OrderBroker::Both
        );
        let upper = symbol.to_ascii_uppercase();
        let known_crypto = self.live_positions.iter().any(|p| {
            p.symbol.eq_ignore_ascii_case(symbol) && p.asset_class.eq_ignore_ascii_case("crypto")
        });
        let is_crypto = matches!(self.order_broker, OrderBroker::Kraken)
            || known_crypto
            || (upper.ends_with("USD") && upper.len() > 5 && !uses_whole_units);
        let tick_size = if last_price >= 1.0 {
            0.01
        } else if last_price >= 0.1 {
            0.0001
        } else {
            0.00001
        };
        let volume_step = if uses_whole_units {
            1.0
        } else if is_crypto {
            0.00000001
        } else {
            0.01
        };
        risk::SymbolSpec {
            symbol: symbol.to_string(),
            tick_size,
            tick_value: tick_size,
            volume_min: volume_step,
            volume_max: 1_000_000.0,
            volume_step,
            contract_size: 1.0,
            margin_rate: 1.0,
        }
    }

    pub(super) fn quick_trade_plan(&self) -> Result<QuickTradePlan, String> {
        let chart = self
            .charts
            .get(self.active_tab)
            .ok_or_else(|| "Open Trade: active chart unavailable".to_string())?;
        let last_price = chart
            .bars
            .last()
            .map(|b| b.close)
            .ok_or_else(|| "Open Trade: active chart needs loaded bars".to_string())?;
        let symbol = normalize_market_data_symbol(&chart.symbol);
        if symbol.is_empty() {
            return Err("Open Trade: active chart has no normalized symbol".to_string());
        }
        let mut sl = self
            .sl_enabled
            .then_some(self.sl_price)
            .flatten()
            .ok_or_else(|| {
                "Open Trade: SL and TP lines must both be placed on the chart".to_string()
            })?;
        let mut tp = self
            .tp_enabled
            .then_some(self.tp_price)
            .flatten()
            .ok_or_else(|| {
                "Open Trade: SL and TP lines must both be placed on the chart".to_string()
            })?;
        let side_idx = self.infer_quick_trade_side_from_lines(sl, tp)?;
        let cfg = self.build_trade_risk_config()?;
        let spec = self.trade_symbol_spec(&symbol, last_price);
        sl = (sl / spec.tick_size).round() * spec.tick_size;
        tp = (tp / spec.tick_size).round() * spec.tick_size;
        let sl_distance = if side_idx == 0 {
            last_price - sl
        } else {
            sl - last_price
        };
        if sl_distance <= 0.0 {
            return Err(
                "Open Trade: SL line must be on the risk side of the current market".to_string(),
            );
        }
        let reward_distance = if side_idx == 0 {
            tp - last_price
        } else {
            last_price - tp
        };
        if reward_distance <= 0.0 {
            return Err(
                "Open Trade: TP line must be on the reward side of the current market".to_string(),
            );
        }

        let account_snapshots = self.selected_trade_account_snapshots();
        let (send_alpaca, send_tt, send_kraken) = self.selected_live_broker_targets();
        let required_snapshots = send_alpaca as usize + send_tt as usize + send_kraken as usize;
        if !matches!(cfg.order_mode, risk::OrderMode::Fixed)
            && account_snapshots.len() < required_snapshots
        {
            return Err("Open Trade: selected broker balances have not loaded yet".to_string());
        }
        let account_snapshot = self.selected_trade_account_floor();
        let balance = account_snapshot
            .map(|acct| {
                if acct.balance > 0.0 {
                    acct.balance
                } else {
                    acct.equity
                }
            })
            .unwrap_or(0.0);
        let equity = account_snapshot.map(|acct| acct.equity).unwrap_or(0.0);
        let has_break_even =
            self.selected_symbol_has_break_even_position(&symbol, side_idx, sl, spec.tick_size);
        if matches!(cfg.order_mode, risk::OrderMode::Dynamic)
            && !has_break_even
            && self.selected_symbol_has_same_side_position(&symbol, side_idx)
        {
            return Err(
                "Open Trade: Dynamic mode will not add another same-side position".to_string(),
            );
        }

        let var_per_lot = if matches!(cfg.order_mode, risk::OrderMode::VaR) {
            let closes: Vec<f64> = chart.bars.iter().map(|b| b.close).collect();
            var::calculate_var(
                &closes,
                1.0,
                spec.tick_value,
                spec.tick_size,
                last_price,
                cfg.var_confidence,
            )
            .map(|res| res.var_dollars)
            .ok_or_else(|| "Open Trade: not enough bar history for VaR sizing".to_string())?
        } else {
            0.0
        };

        let (mut qty, _) = risk::calculate_lots(
            &cfg,
            &spec,
            balance,
            equity,
            sl_distance,
            has_break_even,
            var_per_lot,
        );
        if qty <= 0.0 {
            return Err(format!(
                "Open Trade: {} mode produced zero size",
                self.risk_mode.label()
            ));
        }
        if let Some(acct) = account_snapshot {
            let buying_power = if acct.buying_power > 0.0 {
                acct.buying_power
            } else if acct.balance > 0.0 {
                acct.balance
            } else {
                acct.equity
            };
            let usable_notional = (buying_power * (1.0 - cfg.margin_buffer_pct / 100.0)).max(0.0);
            if usable_notional <= 0.0 {
                return Err("Open Trade: insufficient buying power".to_string());
            }
            let max_qty = Self::floor_to_step(usable_notional / last_price, spec.volume_step)
                .min(spec.volume_max);
            if max_qty < spec.volume_min {
                return Err("Open Trade: insufficient buying power for minimum size".to_string());
            }
            qty = qty.min(max_qty);
        }
        qty = Self::floor_to_step(qty, spec.volume_step);
        if qty < spec.volume_min {
            return Err("Open Trade: computed size is below minimum trade increment".to_string());
        }

        let risk_dollars = sl_distance * qty;
        let reward_dollars = reward_distance * qty;
        let risk_pct = if balance > 0.0 {
            Some(risk_dollars / balance * 100.0)
        } else {
            None
        };
        let rr = if risk_dollars > 0.0 {
            Some(reward_dollars / risk_dollars)
        } else {
            None
        };
        Ok(QuickTradePlan {
            symbol,
            last_price,
            sl,
            tp,
            side_idx,
            qty,
            risk_dollars,
            risk_pct,
            reward_dollars,
            rr,
        })
    }

    pub(super) fn active_trade_symbol(&self) -> Option<String> {
        let chart = self.charts.get(self.active_tab)?;
        let symbol = normalize_market_data_symbol(&chart.symbol);
        if symbol.is_empty() {
            None
        } else {
            Some(symbol)
        }
    }

    pub(super) fn alpaca_order_available(&self) -> bool {
        self.alpaca_enabled && self.broker_connected
    }

    pub(super) fn tastytrade_order_available(&self) -> bool {
        self.tastytrade_enabled && self.tt_connected
    }

    pub(super) fn kraken_order_available(&self) -> bool {
        self.kraken_enabled && self.kraken_connected
    }

    pub(super) fn order_broker_available(&self, broker: OrderBroker) -> bool {
        match broker {
            OrderBroker::Alpaca => self.alpaca_order_available(),
            OrderBroker::Tastytrade => self.tastytrade_order_available(),
            OrderBroker::Kraken => self.kraken_order_available(),
            OrderBroker::Both => self.alpaca_order_available() && self.tastytrade_order_available(),
        }
    }

    pub(super) fn resolve_order_broker(&mut self) {
        if self.order_broker_available(self.order_broker) {
            return;
        }

        self.order_broker = if self.kraken_order_available() {
            OrderBroker::Kraken
        } else if self.alpaca_order_available() {
            OrderBroker::Alpaca
        } else if self.tastytrade_order_available() {
            OrderBroker::Tastytrade
        } else {
            self.order_broker
        };
    }

    pub(super) fn selected_live_broker_targets(&self) -> (bool, bool, bool) {
        let send_alpaca = self.alpaca_order_available()
            && matches!(self.order_broker, OrderBroker::Alpaca | OrderBroker::Both);
        let send_tt = self.tastytrade_order_available()
            && matches!(
                self.order_broker,
                OrderBroker::Tastytrade | OrderBroker::Both
            );
        let send_kraken =
            self.kraken_order_available() && matches!(self.order_broker, OrderBroker::Kraken);
        (send_alpaca, send_tt, send_kraken)
    }

    pub(super) fn alpaca_trade_account_snapshot(&self) -> Option<TradeAccountSnapshot> {
        self.live_account.as_ref().map(|acct| TradeAccountSnapshot {
            broker: "Alpaca",
            balance: if acct.balance > 0.0 {
                acct.balance
            } else {
                acct.cash
            },
            equity: acct.equity,
            buying_power: acct.buying_power,
            margin_used: acct.initial_margin,
        })
    }

    pub(super) fn tastytrade_trade_account_snapshot(&self) -> Option<TradeAccountSnapshot> {
        self.tt_balances.as_ref().map(|bal| TradeAccountSnapshot {
            broker: "tastytrade",
            balance: if bal.cash_balance > 0.0 {
                bal.cash_balance
            } else {
                bal.net_liquidating_value
            },
            equity: bal.net_liquidating_value,
            buying_power: bal.equity_buying_power,
            margin_used: bal.maintenance_requirement,
        })
    }

    pub(super) fn kraken_display_asset(asset: &str) -> String {
        let raw = asset.trim().to_ascii_uppercase();
        match raw.as_str() {
            "XXBT" | "XBT" => "BTC".to_string(),
            "XXDG" | "XDG" => "DOGE".to_string(),
            "ZUSD" => "USD".to_string(),
            "ZEUR" => "EUR".to_string(),
            "ZGBP" => "GBP".to_string(),
            "ZJPY" => "JPY".to_string(),
            other if other.len() == 4 && (other.starts_with('X') || other.starts_with('Z')) => {
                other[1..].to_string()
            }
            other => other.to_string(),
        }
    }

    pub(super) fn kraken_is_cash_balance_asset(asset: &str) -> bool {
        matches!(
            Self::kraken_display_asset(asset).as_str(),
            "USD"
                | "EUR"
                | "GBP"
                | "JPY"
                | "CAD"
                | "AUD"
                | "CHF"
                | "USDT"
                | "USDC"
                | "USDG"
                | "DAI"
                | "PYUSD"
        )
    }

    pub(super) fn kraken_spot_pair_for_balance_asset(asset: &str) -> String {
        let display = Self::kraken_display_asset(asset);
        if let Some(stripped) = display.strip_suffix(".EQ") {
            // Kraken Securities/equity balances are reported as assets (`WOK.EQ`),
            // not Spot OHLC pairs. Keep the underlying ticker bare so the UI does
            // not manufacture `WOKUSD` and collide with stale/non-equity caches.
            stripped.to_string()
        } else {
            format!("{}USD", display)
        }
    }

    pub(super) fn kraken_quote_balance(&self) -> f64 {
        self.kraken_balances
            .iter()
            .filter(|(asset, balance)| {
                *balance > 0.0
                    && matches!(
                        Self::kraken_display_asset(asset).as_str(),
                        "USD" | "USDT" | "USDC"
                    )
            })
            .map(|(_, balance)| *balance)
            .sum()
    }

    pub(super) fn kraken_usd_equivalent_balance(&self) -> f64 {
        self.kraken_balances
            .iter()
            .filter(|(_, balance)| balance.is_finite() && *balance > 0.0)
            .map(|(asset, balance)| {
                let display = Self::kraken_display_asset(asset);
                match display.as_str() {
                    "USD" | "USDT" | "USDC" | "USDG" | "DAI" | "PYUSD" => *balance,
                    _ => self
                        .kraken_usd_price_for_balance_asset(&display)
                        .map(|price| *balance * price)
                        .unwrap_or(0.0),
                }
            })
            .sum()
    }

    pub(super) fn kraken_usd_price_for_balance_asset(&self, display_asset: &str) -> Option<f64> {
        let display = display_asset.trim().to_ascii_uppercase();
        let is_equity_balance = display.ends_with(".EQ");
        let mut candidates = Vec::new();
        if let Some(stripped) = display.strip_suffix(".EQ") {
            candidates.push(stripped.to_string());
            candidates.push(format!("{}USD", stripped));
            candidates.push(format!("{}ZUSD", stripped));
        }
        candidates.push(display.clone());
        candidates.push(format!("{}USD", display));
        candidates.push(format!("{}ZUSD", display));
        candidates.into_iter().find_map(|symbol| {
            let price = if is_equity_balance {
                self.latest_cached_equity_price_for_symbol(&symbol)
            } else {
                self.latest_cached_price_for_symbol(&symbol)
            };
            price.filter(|price| price.is_finite() && *price > 0.0)
        })
    }

    pub(super) fn kraken_base_asset_for_pair(pair: &str) -> String {
        let upper = typhoon_engine::core::kraken::normalize_pair_symbol(pair)
            .replace('/', "")
            .to_ascii_uppercase();
        let stripped = upper
            .strip_suffix("USDT")
            .or_else(|| upper.strip_suffix("USDC"))
            .or_else(|| upper.strip_suffix("USD"))
            .or_else(|| upper.strip_suffix("ZUSD"))
            .unwrap_or(upper.as_str());
        stripped.strip_suffix(".EQ").unwrap_or(stripped).to_string()
    }

    fn latest_cached_price_for_symbol_from_sources(
        &self,
        symbol: &str,
        sources: &[&str],
    ) -> Option<f64> {
        let cache = self.cache.as_ref()?;
        let timeframes = ["1Min", "5Min", "15Min", "30Min", "1Hour", "4Hour", "1Day"];
        let mut symbols = Vec::new();
        let mut push_symbol = |candidate: String| {
            if !candidate.is_empty() && !symbols.iter().any(|s| s == &candidate) {
                symbols.push(candidate);
            }
        };
        let normalized = typhoon_engine::core::kraken::normalize_pair_symbol(symbol)
            .replace('/', "")
            .to_ascii_uppercase();
        push_symbol(normalized.clone());
        push_symbol(symbol.trim().replace('/', "").to_ascii_uppercase());
        let base = Self::kraken_base_asset_for_pair(&normalized);
        if !base.is_empty() && base != normalized {
            push_symbol(base.clone());
            push_symbol(format!("{}USD", base));
            push_symbol(format!("{}ZUSD", base));
        } else if !normalized.ends_with("USD")
            && !normalized.ends_with("USDT")
            && !normalized.ends_with("USDC")
        {
            push_symbol(format!("{}USD", normalized));
            push_symbol(format!("{}ZUSD", normalized));
        }
        if let Some(eq) = normalized.strip_suffix(".EQ") {
            push_symbol(eq.to_string());
            push_symbol(format!("{}USD", eq));
            push_symbol(format!("{}ZUSD", eq));
        }
        for tf in timeframes {
            for source in sources {
                for candidate in &symbols {
                    for key in chart_source_cache_keys(source, candidate, tf) {
                        let Ok(Some(raw)) = cache.get_bars_raw(&key) else {
                            continue;
                        };
                        if let Some((_, _, _, _, close, _)) =
                            raw.iter().rev().find(|(ts, _, _, _, close, _)| {
                                *ts > 0 && *close > 0.0 && close.is_finite()
                            })
                        {
                            return Some(*close);
                        }
                    }
                }
            }
        }
        None
    }

    pub(super) fn latest_cached_price_for_symbol(&self, symbol: &str) -> Option<f64> {
        self.latest_cached_price_for_symbol_from_sources(
            symbol,
            &[
                "kraken",
                "kraken-futures",
                "tastytrade",
                "alpaca",
                "mt5",
                "default",
            ],
        )
    }

    pub(super) fn latest_cached_equity_price_for_symbol(&self, symbol: &str) -> Option<f64> {
        let cache = self.cache.as_ref()?;
        let timeframes = [
            "quote", "1Min", "5Min", "15Min", "30Min", "1Hour", "4Hour", "1Day",
        ];
        let sources = ["kraken-equities", "tastytrade", "alpaca", "default", "mt5"];
        let mut symbols = Vec::new();
        let mut push_symbol = |candidate: String| {
            let candidate = candidate.trim().replace('/', "").to_ascii_uppercase();
            if !candidate.is_empty() && !symbols.iter().any(|s| s == &candidate) {
                symbols.push(candidate);
            }
        };
        let normalized = typhoon_engine::core::kraken::normalize_pair_symbol(symbol)
            .replace('/', "")
            .to_ascii_uppercase();
        let no_eq = normalized.strip_suffix(".EQ").unwrap_or(&normalized);
        let base = Self::kraken_base_asset_for_pair(no_eq);
        // Equities must use the plain underlying ticker. Do not probe `{TICKER}USD`;
        // that is exactly how WOK picked up a bogus/stale synthetic price.
        push_symbol(base);
        push_symbol(no_eq.to_string());
        if let Some(stripped) = no_eq.strip_suffix("USD") {
            push_symbol(stripped.to_string());
        }
        for tf in timeframes {
            for source in sources {
                for candidate in &symbols {
                    let mut keys = vec![format!("{source}:{candidate}:{tf}")];
                    if source == "alpaca" {
                        keys.push(format!("paper_TyphooN:{candidate}:{tf}"));
                        keys.push(format!("alpaca_paper_TyphooN:{candidate}:{tf}"));
                    }
                    for key in keys {
                        let Ok(Some(raw)) = cache.get_bars_raw(&key) else {
                            continue;
                        };
                        if let Some((_, _, _, _, close, _)) =
                            raw.iter().rev().find(|(ts, _, _, _, close, _)| {
                                *ts > 0 && *close > 0.0 && close.is_finite()
                            })
                        {
                            return Some(*close);
                        }
                    }
                }
            }
        }
        None
    }

    pub(super) fn kraken_balance_avg_price(&self, asset: &str) -> Option<f64> {
        self.kraken_cost_basis_for_base_asset(&Self::kraken_display_asset(asset))
            .and_then(|basis| basis.avg_price())
    }

    pub(super) fn kraken_position_avg_price(&self, symbol: &str) -> Option<f64> {
        self.kraken_cost_basis_for_base_asset(&Self::kraken_base_asset_for_pair(symbol))
            .and_then(|basis| basis.avg_price())
    }

    pub(super) fn kraken_asset_keys_match(left: &str, right: &str) -> bool {
        let normalize = |s: &str| {
            s.trim()
                .to_ascii_uppercase()
                .replace('/', "")
                .replace(".EQ", "")
        };
        left.eq_ignore_ascii_case(right) || normalize(left) == normalize(right)
    }

    pub(super) fn kraken_spot_balance_for_pair(&self, pair: &str) -> Option<(String, f64)> {
        let base = Self::kraken_base_asset_for_pair(pair);
        self.kraken_balances
            .iter()
            .filter(|(asset, balance)| *balance > 0.0 && !Self::kraken_is_cash_balance_asset(asset))
            .find_map(|(asset, balance)| {
                let display = Self::kraken_display_asset(asset);
                (Self::kraken_asset_keys_match(&display, &base)
                    || Self::kraken_asset_keys_match(asset, &base))
                .then(|| (asset.clone(), *balance))
            })
    }

    pub(super) fn kraken_trade_key(trade: &typhoon_engine::broker::kraken::KrakenTrade) -> String {
        if !trade.trade_id.is_empty() {
            trade.trade_id.clone()
        } else {
            format!(
                "{}:{}:{:.9}:{:.12}:{:.12}",
                trade.ordertxid, trade.pair, trade.time, trade.vol, trade.price
            )
        }
    }

    pub(super) fn rebuild_kraken_trade_indexes(&mut self) {
        self.kraken_trade_keys.clear();
        for trade in &self.kraken_trades {
            self.kraken_trade_keys.insert(Self::kraken_trade_key(trade));
        }
        self.rebuild_kraken_cost_basis();
    }

    pub(super) fn insert_kraken_live_trade(
        &mut self,
        trade: typhoon_engine::broker::kraken::KrakenTrade,
    ) -> bool {
        let key = Self::kraken_trade_key(&trade);
        if !self.kraken_trade_keys.insert(key) {
            return false;
        }
        self.kraken_trades.push_front(trade);
        while self.kraken_trades.len() > KRAKEN_TRADE_HISTORY_CAP {
            if let Some(removed) = self.kraken_trades.pop_back() {
                self.kraken_trade_keys
                    .remove(&Self::kraken_trade_key(&removed));
            }
        }
        self.rebuild_kraken_cost_basis();
        true
    }

    pub(super) fn kraken_cost_basis_for_base_asset(
        &self,
        base: &str,
    ) -> Option<crate::app::KrakenCostBasis> {
        let base = base.trim().to_ascii_uppercase();
        self.kraken_cost_basis
            .iter()
            .find_map(|(key, basis)| Self::kraken_asset_keys_match(key, &base).then_some(*basis))
    }

    pub(super) fn refresh_kraken_position_costs(&mut self) {
        // `updates` is built from `kr_positions` in order, so the previous code did
        // an O(n²) `updates.iter().find` per position to re-pair them. Drop the
        // symbol key entirely and zip the two slices in lockstep — same data, O(n).
        let updates: Vec<(Option<f64>, Option<f64>)> = self
            .kr_positions
            .iter()
            .map(|pos| {
                let base = Self::kraken_base_asset_for_pair(&pos.symbol);
                let avg = self
                    .kraken_cost_basis_for_base_asset(&base)
                    .and_then(|basis| basis.avg_price());
                let current = if pos.asset_id.starts_with("equity_balance:")
                    || pos.asset_class.eq_ignore_ascii_case("stock")
                {
                    self.latest_cached_equity_price_for_symbol(&pos.symbol)
                } else {
                    self.latest_cached_price_for_symbol(&pos.symbol)
                };
                (avg, current)
            })
            .collect();

        for (pos, (avg, current)) in self.kr_positions.iter_mut().zip(updates.into_iter()) {
            if let Some(avg) = avg {
                pos.avg_entry_price = avg;
            }
            if let Some(current) = current {
                pos.market_value = pos.qty * current;
                let dir = if pos.side == "short" { -1.0 } else { 1.0 };
                let basis = if pos.avg_entry_price > 0.0 {
                    pos.avg_entry_price
                } else {
                    current
                };
                pos.unrealized_pl = (current - basis) * pos.qty * dir;
            }
        }
    }

    pub(super) fn rebuild_kraken_cost_basis(&mut self) {
        let mut trades: Vec<_> = self.kraken_trades.iter().collect();
        trades.sort_by(|a, b| a.time.total_cmp(&b.time));

        let mut by_base: std::collections::HashMap<String, crate::app::KrakenCostBasis> =
            std::collections::HashMap::new();
        for trade in trades {
            if trade.vol <= 0.0 || !trade.vol.is_finite() {
                continue;
            }
            let pair_norm = typhoon_engine::core::kraken::normalize_pair_symbol(&trade.pair);
            let trade_base = Self::kraken_base_asset_for_pair(&pair_norm);
            if trade_base.is_empty() || Self::kraken_is_cash_balance_asset(&trade_base) {
                continue;
            }

            let side = trade.side.to_ascii_lowercase();
            let basis = by_base.entry(trade_base).or_default();
            if side == "buy" {
                basis.qty += trade.vol;
                basis.cost += trade.cost.max(0.0) + trade.fee.max(0.0);
            } else if side == "sell" && basis.qty > 0.0 {
                let reduce_qty = trade.vol.min(basis.qty);
                let avg = basis.cost / basis.qty;
                basis.qty -= reduce_qty;
                basis.cost -= avg * reduce_qty;
                if basis.qty <= 1e-12 {
                    basis.qty = 0.0;
                    basis.cost = 0.0;
                }
            }
        }
        by_base.retain(|_, basis| basis.qty > 0.0 && basis.cost > 0.0);

        let held_assets: Vec<String> = self
            .kraken_balances
            .iter()
            .filter(|(asset, balance)| *balance > 0.0 && !Self::kraken_is_cash_balance_asset(asset))
            .map(|(asset, _)| Self::kraken_display_asset(asset))
            .collect();
        if !held_assets.is_empty() {
            by_base.retain(|base, _| {
                held_assets
                    .iter()
                    .any(|held| Self::kraken_asset_keys_match(base, held))
            });
        }

        self.kraken_cost_basis = by_base;
    }

    pub(super) fn render_kraken_spot_buy_controls(&mut self, ui: &mut egui::Ui) {
        let Some((pair, last_price)) = self.active_trade_symbol_and_price() else {
            return;
        };
        if !self.kraken_connected || last_price <= 0.0 {
            return;
        }

        let quote_balance = self.kraken_quote_balance().max(0.0);
        let max_qty = Self::floor_to_step(quote_balance / last_price, 0.00000001);
        let base_asset = Self::kraken_base_asset_for_pair(&pair);

        ui.separator();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Mode").color(AXIS_TEXT).small());
            ui.label(egui::RichText::new("KrakenPro").color(UP).small().strong());
            ui.label(
                egui::RichText::new(format!("${quote_balance:.2}"))
                    .color(AXIS_TEXT)
                    .small(),
            );
        });

        let pct_before = self.kraken_spot_buy_pct;
        ui.add(
            egui::Slider::new(&mut self.kraken_spot_buy_pct, 0.0..=100.0)
                .text("% cash")
                .suffix("%"),
        );
        if (self.kraken_spot_buy_pct - pct_before).abs() > f32::EPSILON {
            self.kraken_spot_buy_qty = max_qty * (self.kraken_spot_buy_pct as f64 / 100.0);
        }

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Qty").color(AXIS_TEXT).small());
            let qty_before = self.kraken_spot_buy_qty;
            let qty_resp = ui.add(
                egui::DragValue::new(&mut self.kraken_spot_buy_qty)
                    .range(0.0..=max_qty)
                    .speed((max_qty / 200.0).max(0.00000001))
                    .max_decimals(8),
            );
            ui.label(egui::RichText::new(&base_asset).monospace().small());
            if qty_resp.changed() || (self.kraken_spot_buy_qty - qty_before).abs() > f64::EPSILON {
                self.kraken_spot_buy_qty = self.kraken_spot_buy_qty.clamp(0.0, max_qty);
                self.kraken_spot_buy_pct = if max_qty > 0.0 {
                    ((self.kraken_spot_buy_qty / max_qty) * 100.0) as f32
                } else {
                    0.0
                };
            }
        });

        ui.horizontal(|ui| {
            for pct in [25.0_f32, 50.0, 75.0, 100.0] {
                if ui.small_button(format!("{pct:.0}%")).clicked() {
                    self.kraken_spot_buy_pct = pct;
                    self.kraken_spot_buy_qty = max_qty * (pct as f64 / 100.0);
                }
            }
        });

        let qty = Self::floor_to_step(self.kraken_spot_buy_qty, 0.00000001);
        let notional = qty * last_price;
        let can_submit = qty > 0.0 && quote_balance > 0.0 && notional <= quote_balance;
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    can_submit,
                    egui::Button::new(format!("Buy {base_asset}")).fill(BTN_GREEN),
                )
                .clicked()
            {
                let _ = self.broker_tx.send(BrokerCmd::KrakenPlaceOrder {
                    pair: pair.clone(),
                    side: "buy".to_string(),
                    order_type: "market".to_string(),
                    volume: qty,
                    price: None,
                    leverage: None,
                });
                self.log.push_back(LogEntry::info(format!(
                    "KrakenPro: queued market buy {:.8} {} ({})",
                    qty, base_asset, pair
                )));
            }
            ui.label(
                egui::RichText::new(format!("≈ ${notional:.2}"))
                    .color(AXIS_TEXT)
                    .small(),
            );
        });
    }

    pub(super) fn open_kraken_spot_sell_dialog(&mut self, asset: String, available: f64) {
        self.kraken_spot_sell_pair = Self::kraken_spot_pair_for_balance_asset(&asset);
        self.kraken_spot_sell_asset = Self::kraken_display_asset(&asset);
        self.kraken_spot_sell_available = available.max(0.0);
        self.kraken_spot_sell_qty = self.kraken_spot_sell_available;
        self.kraken_spot_sell_pct = 100.0;
        self.show_kraken_spot_sell_dialog = true;
    }

    pub(super) fn render_kraken_spot_sell_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_kraken_spot_sell_dialog {
            return;
        }

        let mut open = self.show_kraken_spot_sell_dialog;
        let mut close_after_submit = false;
        egui::Window::new(format!("Sell {} on Kraken", self.kraken_spot_sell_asset))
            .open(&mut open)
            .default_size([460.0, 260.0])
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("Kraken spot balance sell ticket")
                        .strong()
                        .color(AXIS_TEXT),
                );
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Pair:");
                    ui.label(
                        egui::RichText::new(&self.kraken_spot_sell_pair)
                            .monospace()
                            .strong(),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Available balance:");
                    ui.label(
                        egui::RichText::new(format!(
                            "{:.8} {}",
                            self.kraken_spot_sell_available, self.kraken_spot_sell_asset
                        ))
                        .monospace(),
                    );
                });

                let available = self.kraken_spot_sell_available.max(0.0);
                let pct_before = self.kraken_spot_sell_pct;
                ui.add(
                    egui::Slider::new(&mut self.kraken_spot_sell_pct, 0.0..=100.0)
                        .text("% of balance")
                        .suffix("%"),
                );
                if (self.kraken_spot_sell_pct - pct_before).abs() > f32::EPSILON {
                    self.kraken_spot_sell_qty =
                        available * (self.kraken_spot_sell_pct as f64 / 100.0);
                }

                ui.horizontal(|ui| {
                    ui.label("Quantity:");
                    let qty_before = self.kraken_spot_sell_qty;
                    let qty_resp = ui.add(
                        egui::DragValue::new(&mut self.kraken_spot_sell_qty)
                            .range(0.0..=available)
                            .speed((available / 200.0).max(0.00000001))
                            .max_decimals(8),
                    );
                    ui.label(egui::RichText::new(&self.kraken_spot_sell_asset).monospace());
                    if qty_resp.changed()
                        || (self.kraken_spot_sell_qty - qty_before).abs() > f64::EPSILON
                    {
                        self.kraken_spot_sell_qty = self.kraken_spot_sell_qty.clamp(0.0, available);
                        self.kraken_spot_sell_pct = if available > 0.0 {
                            ((self.kraken_spot_sell_qty / available) * 100.0) as f32
                        } else {
                            0.0
                        };
                    }
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    for pct in [25.0_f32, 50.0, 75.0, 100.0] {
                        if ui.button(format!("{pct:.0}%")).clicked() {
                            self.kraken_spot_sell_pct = pct;
                            self.kraken_spot_sell_qty = available * (pct as f64 / 100.0);
                        }
                    }
                });
                ui.separator();

                let can_submit = self.kraken_connected
                    && available > 0.0
                    && self.kraken_spot_sell_qty > 0.0
                    && self.kraken_spot_sell_qty <= available;
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            can_submit,
                            egui::Button::new(format!(
                                "Queue Sell {}",
                                self.kraken_spot_sell_asset
                            ))
                            .fill(egui::Color32::from_rgb(126, 28, 64)),
                        )
                        .on_hover_text(
                            "Submit a Kraken market sell for the selected balance quantity",
                        )
                        .clicked()
                    {
                        let pair = self.kraken_spot_sell_pair.clone();
                        let qty = self.kraken_spot_sell_qty;
                        let asset = self.kraken_spot_sell_asset.clone();
                        let _ = self.broker_tx.send(BrokerCmd::KrakenPlaceOrder {
                            pair: pair.clone(),
                            side: "sell".to_string(),
                            order_type: "market".to_string(),
                            volume: qty,
                            price: None,
                            leverage: None,
                        });
                        self.log.push_back(LogEntry::info(format!(
                            "Kraken: queued market sell {:.8} {} ({})",
                            qty, asset, pair
                        )));
                        close_after_submit = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close_after_submit = true;
                    }
                });
            });

        self.show_kraken_spot_sell_dialog = open && !close_after_submit;
    }

    pub(super) fn kraken_trade_account_snapshot(&self) -> Option<TradeAccountSnapshot> {
        let usd_like = self.kraken_usd_equivalent_balance();
        if usd_like <= 0.0 {
            None
        } else {
            Some(TradeAccountSnapshot {
                broker: "Kraken",
                balance: usd_like,
                equity: usd_like,
                buying_power: usd_like,
                margin_used: 0.0,
            })
        }
    }

    pub(super) fn selected_trade_account_snapshots(&self) -> Vec<TradeAccountSnapshot> {
        let (send_alpaca, send_tt, send_kraken) = self.selected_live_broker_targets();
        let mut snapshots = Vec::new();
        if send_alpaca && let Some(snap) = self.alpaca_trade_account_snapshot() {
            snapshots.push(snap);
        }
        if send_tt && let Some(snap) = self.tastytrade_trade_account_snapshot() {
            snapshots.push(snap);
        }
        if send_kraken && let Some(snap) = self.kraken_trade_account_snapshot() {
            snapshots.push(snap);
        }
        snapshots
    }

    pub(super) fn selected_trade_account_floor(&self) -> Option<TradeAccountSnapshot> {
        let snaps = self.selected_trade_account_snapshots();
        let first = *snaps.first()?;
        if snaps.len() == 1 {
            return Some(first);
        }
        Some(TradeAccountSnapshot {
            broker: "Selected",
            balance: snaps
                .iter()
                .map(|s| s.balance)
                .fold(first.balance, f64::min),
            equity: snaps.iter().map(|s| s.equity).fold(first.equity, f64::min),
            buying_power: snaps
                .iter()
                .map(|s| s.buying_power)
                .fold(first.buying_power, f64::min),
            margin_used: snaps
                .iter()
                .map(|s| s.margin_used)
                .fold(first.margin_used, f64::max),
        })
    }

    pub(super) fn selected_symbol_has_same_side_position(
        &self,
        symbol: &str,
        side_idx: usize,
    ) -> bool {
        let wants_long = side_idx == 0;
        let same_side = |pos: &PositionInfo| {
            pos.symbol.eq_ignore_ascii_case(symbol)
                && if wants_long {
                    pos.side.eq_ignore_ascii_case("long")
                } else {
                    pos.side.eq_ignore_ascii_case("short")
                }
        };
        let (send_alpaca, send_tt, send_kraken) = self.selected_live_broker_targets();
        (send_alpaca && self.live_positions.iter().any(same_side))
            || (send_tt && self.tt_positions.iter().any(same_side))
            || (send_kraken && self.kr_positions.iter().any(same_side))
            || (send_kraken && wants_long && self.kraken_spot_balance_for_pair(symbol).is_some())
    }

    pub(super) fn selected_symbol_has_break_even_position(
        &self,
        symbol: &str,
        side_idx: usize,
        sl: f64,
        tick_size: f64,
    ) -> bool {
        let wants_long = side_idx == 0;
        let at_break_even = |pos: &PositionInfo| {
            pos.symbol.eq_ignore_ascii_case(symbol)
                && if wants_long {
                    pos.side.eq_ignore_ascii_case("long")
                } else {
                    pos.side.eq_ignore_ascii_case("short")
                }
                && (pos.avg_entry_price - sl).abs() <= tick_size * 0.5
        };
        let (send_alpaca, send_tt, send_kraken) = self.selected_live_broker_targets();
        (send_alpaca && self.live_positions.iter().any(at_break_even))
            || (send_tt && self.tt_positions.iter().any(at_break_even))
            || (send_kraken && self.kr_positions.iter().any(at_break_even))
            || (send_kraken
                && wants_long
                && self.kraken_spot_balance_for_pair(symbol).is_some()
                && self
                    .kraken_position_avg_price(symbol)
                    .map(|avg| (avg - sl).abs() <= tick_size * 0.5)
                    .unwrap_or(false))
    }

    pub(super) fn close_all_selected_brokers(&mut self) {
        self.resolve_order_broker();
        let (send_alpaca, send_tt, send_kraken) = self.selected_live_broker_targets();
        if !send_alpaca && !send_tt && !send_kraken {
            self.log.push_back(LogEntry::warn(
                "Close All: no broker connected for selected target",
            ));
            return;
        }
        let Some(symbol) = self.active_trade_symbol() else {
            self.log
                .push_back(LogEntry::warn("Close All: active chart symbol unavailable"));
            return;
        };
        let mut any = false;
        if send_alpaca
            && self
                .live_positions
                .iter()
                .any(|pos| pos.symbol.eq_ignore_ascii_case(&symbol))
        {
            let _ = self.broker_tx.send(BrokerCmd::ClosePosition {
                symbol: symbol.clone(),
                qty: None,
            });
            any = true;
        }
        if send_tt
            && self
                .tt_positions
                .iter()
                .any(|pos| pos.symbol.eq_ignore_ascii_case(&symbol))
        {
            let _ = self.broker_tx.send(BrokerCmd::TastytradeClosePositionQty {
                symbol: symbol.clone(),
                qty: None,
            });
            any = true;
        }
        if send_kraken {
            if self
                .kr_positions
                .iter()
                .any(|pos| pos.symbol.eq_ignore_ascii_case(&symbol))
            {
                let _ = self.broker_tx.send(BrokerCmd::KrakenClosePosition {
                    pair: symbol.clone(),
                    volume: None,
                });
                any = true;
            } else if let Some((asset, available)) = self.kraken_spot_balance_for_pair(&symbol) {
                self.open_kraken_spot_sell_dialog(asset.clone(), available);
                self.log.push_back(LogEntry::info(format!(
                    "Close All: {} is Kraken spot inventory — opened Sell ticket for {}",
                    symbol,
                    Self::kraken_display_asset(&asset)
                )));
                any = true;
            }
        }
        if any {
            self.log.push_back(LogEntry::info(format!(
                "Close All: submitted for {} on selected broker target(s)",
                symbol
            )));
        } else {
            self.log.push_back(LogEntry::warn(format!(
                "Close All: no selected broker position found for {}",
                symbol
            )));
        }
    }

    pub(super) fn submit_quick_trade(&mut self) {
        self.resolve_order_broker();
        if matches!(self.risk_mode, RiskMode::KrakenPro) {
            self.log
                .push_back(LogEntry::warn("KrakenPro selected: use Buy/Sell controls."));
            return;
        }
        let plan = match self.quick_trade_plan() {
            Ok(plan) => plan,
            Err(e) => {
                self.log.push_back(LogEntry::err(e));
                return;
            }
        };
        self.order_symbol = plan.symbol.clone();
        self.order_side = plan.side_idx;
        let side_label = if plan.side_idx == 0 { "BUY" } else { "SELL" };
        let side_str = if plan.side_idx == 0 {
            "buy".to_string()
        } else {
            "sell".to_string()
        };
        let (send_alpaca, send_tt, send_kraken) = self.selected_live_broker_targets();
        if !send_alpaca && !send_tt && !send_kraken {
            self.log.push_back(LogEntry::warn(
                "Open Trade: no broker connected for selected target",
            ));
            return;
        }

        if send_alpaca {
            // Alpaca rejects fractional + GTC, and bracket orders are GTC-only —
            // floor to whole shares so the bracket legs survive submission.
            let alpaca_qty = plan.qty.floor();
            if alpaca_qty < 1.0 {
                self.log.push_back(LogEntry::warn(format!(
                    "Open Trade: Alpaca bracket needs whole shares; {} {} rounds to 0 — skipping Alpaca leg (use Set SL/Set TP after a fractional fill)",
                    plan.qty, plan.symbol
                )));
            } else {
                let _ = self.broker_tx.send(BrokerCmd::AlpacaBracketOrder {
                    symbol: plan.symbol.clone(),
                    qty: alpaca_qty,
                    side: side_str.clone(),
                    stop_loss: plan.sl,
                    take_profit: plan.tp,
                });
                self.log.push_back(LogEntry::info(format!(
                    "Open Trade: market {} {} {} @ {} sl={} tp={} [{}]",
                    side_label,
                    alpaca_qty,
                    plan.symbol,
                    format_price(plan.last_price),
                    format_price(plan.sl),
                    format_price(plan.tp),
                    self.risk_mode.label()
                )));
            }
        }

        if send_tt {
            let action = if plan.side_idx == 0 {
                "Buy to Open".to_string()
            } else {
                "Sell to Open".to_string()
            };
            let _ = self.broker_tx.send(BrokerCmd::TastytradeEquityOrder {
                symbol: plan.symbol.clone(),
                qty: plan.qty.floor() as i64,
                side: action,
                order_type: "Market".to_string(),
                price: None,
            });
            self.log.push_back(LogEntry::info(format!(
                "Open Trade: tastytrade market {} {} {}",
                side_label,
                plan.qty.floor() as i64,
                plan.symbol
            )));
            let _ = self.broker_tx.send(BrokerCmd::TastytradeSyncExits {
                symbol: plan.symbol.clone(),
                sl_price: Some(plan.sl),
                tp_price: Some(plan.tp),
                wait_for_position: true,
                wait_for_qty_at_most: None,
            });
            self.log.push_back(LogEntry::info(format!(
                "Open Trade: tastytrade exit sync queued for {} (sl={} tp={})",
                plan.symbol,
                format_price(plan.sl),
                format_price(plan.tp)
            )));
        }

        if send_kraken {
            let _ = self.broker_tx.send(BrokerCmd::KrakenPlaceOrder {
                pair: plan.symbol.clone(),
                side: side_str,
                order_type: "market".to_string(),
                volume: plan.qty,
                price: None,
                leverage: None,
            });
            self.log.push_back(LogEntry::info(format!(
                "Open Trade: Kraken market {} {} {}",
                side_label, plan.qty, plan.symbol
            )));
            let _ = self.broker_tx.send(BrokerCmd::KrakenSyncExits {
                pair: plan.symbol.clone(),
                sl_price: Some(plan.sl),
                tp_price: Some(plan.tp),
                wait_for_position: true,
                wait_for_qty_at_most: None,
            });
            self.log.push_back(LogEntry::info(format!(
                "Open Trade: Kraken exit sync queued for {} (sl={} tp={})",
                plan.symbol,
                format_price(plan.sl),
                format_price(plan.tp)
            )));
        }
    }

    pub(super) fn sync_current_position_exits(&mut self, reason: &str) {
        let Some((symbol, _)) = self.active_trade_symbol_and_price() else {
            self.log.push_back(LogEntry::warn(format!(
                "{reason}: active chart symbol unavailable"
            )));
            return;
        };
        let sl = self.sl_enabled.then_some(self.sl_price).flatten();
        let tp = self.tp_enabled.then_some(self.tp_price).flatten();
        if sl.is_none() && tp.is_none() {
            self.log.push_back(LogEntry::warn(format!(
                "{reason}: no SL/TP lines enabled — use Buy Lines or Sell Lines first"
            )));
            return;
        }

        let (send_alpaca, send_tt, send_kraken) = self.selected_live_broker_targets();
        if !send_alpaca && !send_tt && !send_kraken {
            self.log.push_back(LogEntry::warn(format!(
                "{reason}: no broker connected for selected target"
            )));
            return;
        }

        if send_alpaca {
            let _ = self.broker_tx.send(BrokerCmd::AlpacaSyncExits {
                symbol: symbol.clone(),
                sl_price: sl,
                tp_price: tp,
                wait_for_qty_at_most: None,
            });
        }
        if send_tt {
            let _ = self.broker_tx.send(BrokerCmd::TastytradeSyncExits {
                symbol: symbol.clone(),
                sl_price: sl,
                tp_price: tp,
                wait_for_position: false,
                wait_for_qty_at_most: None,
            });
        }
        if send_kraken {
            let _ = self.broker_tx.send(BrokerCmd::KrakenSyncExits {
                pair: symbol.clone(),
                sl_price: sl,
                tp_price: tp,
                wait_for_position: false,
                wait_for_qty_at_most: None,
            });
        }

        let sl_text = sl.map(format_price).unwrap_or_else(|| "OFF".to_string());
        let tp_text = tp.map(format_price).unwrap_or_else(|| "OFF".to_string());
        self.log.push_back(LogEntry::info(format!(
            "{reason}: syncing exits for {} (sl={} tp={})",
            symbol, sl_text, tp_text
        )));
    }

    pub(super) fn apply_current_sl_to_positions(&mut self) {
        if self.sl_price.is_none() {
            self.log.push_back(LogEntry::warn(
                "Set SL: no SL line set — use Buy Lines or Sell Lines first",
            ));
            return;
        }
        self.sync_current_position_exits("Set SL");
    }

    pub(super) fn apply_current_tp_to_positions(&mut self) {
        if self.tp_price.is_none() {
            self.log.push_back(LogEntry::warn(
                "Set TP: no TP line set — use Buy Lines or Sell Lines first",
            ));
            return;
        }
        self.sync_current_position_exits("Set TP");
    }
}
