use super::*;

const SEC_CACHE_HEAVY_SYNC_MIN_REBUILD_INTERVAL: std::time::Duration =
    std::time::Duration::from_secs(30);

/// Rows pulled per ticker for an on-demand filing-history search.
///
/// Sized to cover a symbol's full history rather than a page of it: the
/// heaviest filer in the live corpus has ~25k rows, but a typical large-cap
/// runs under 1k for its entire life (SMCI: 975 since inception, 398 in the
/// last three years). 2000 answers "the past 1-3 years" for essentially every
/// symbol while keeping a search to ~1MB.
const SEC_HISTORY_PER_TICKER_ROWS: usize = 2_000;

pub(super) fn refresh_arc_slice_cache<T, K, F>(
    cached: &mut std::sync::Arc<[T]>,
    cached_key: &mut Option<K>,
    next_key: K,
    build: F,
) -> bool
where
    K: PartialEq,
    F: FnOnce() -> Vec<T>,
{
    if cached_key.as_ref() == Some(&next_key) {
        return false;
    }

    *cached = build().into();
    *cached_key = Some(next_key);
    true
}

pub(super) fn position_symbol_membership_signature(positions: &[PositionInfo]) -> u64 {
    use std::hash::{Hash, Hasher};

    let mut symbols: Vec<&str> = positions
        .iter()
        .map(|position| position.symbol.as_str())
        .collect();
    symbols.sort_unstable();
    symbols.dedup();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    symbols.hash(&mut hasher);
    hasher.finish()
}

/// Hash of every **user-driven** control on the SEC Filings tab.
///
/// The rebuild gate treats a change here as "the user just touched a control",
/// which is allowed through even while a broad EDGAR scrape or heavy sync is
/// running — otherwise the scanner's own controls look dead. Scope is one of
/// those controls and was missing: it fed the *data* key (marking the cache
/// changed) but not this one, so flipping Scope mid-scrape hit the early-return
/// and left the previous scope's list on screen. That read as "All and Kraken
/// show nothing" while a scrape was in flight.
pub(super) fn sec_filings_controls_key(
    scope: EventSource,
    sec_filters: &[bool; 7],
    search_query: &str,
    sort_column: usize,
    sort_ascending: bool,
) -> u64 {
    use std::hash::{Hash, Hasher};

    let mut h = std::collections::hash_map::DefaultHasher::new();
    scope.hash(&mut h);
    sec_filters.hash(&mut h);
    search_query.hash(&mut h);
    sort_column.hash(&mut h);
    sort_ascending.hash(&mut h);
    h.finish()
}

/// Scope identity for the SEC data caches (tab counts, filings, insiders,
/// timeline).
///
/// Those caches are filtered by the resolved **symbol set**, not by the enum,
/// and the same enum resolves to different sets over a session: Alpaca and
/// Kraken scope both start out as positions-only and widen when the broker
/// asset catalog lands. Keying on the enum alone pinned the filtered result to
/// whatever the set happened to be the first time that scope was rendered.
pub(super) fn sec_scope_identity_key(scope: EventSource, membership_signature: u64) -> u64 {
    use std::hash::{Hash, Hasher};

    let mut h = std::collections::hash_map::DefaultHasher::new();
    scope.hash(&mut h);
    membership_signature.hash(&mut h);
    h.finish()
}

pub(super) fn broker_scope_membership_signature(
    scope: EventSource,
    alpaca_positions_rev: u64,
    kraken_positions_rev: u64,
    kraken_catalog_rev: u64,
    alpaca_catalog_rev: u64,
) -> u64 {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    match scope {
        EventSource::All => return 0,
        // Alpaca scope is catalog + positions (see `alpaca_scope_symbols`), so
        // the signature has to move when the asset list lands — otherwise the
        // cached scope set stays positions-only for the rest of the session.
        EventSource::Alpaca => {
            alpaca_positions_rev.hash(&mut hasher);
            alpaca_catalog_rev.hash(&mut hasher);
        }
        EventSource::Kraken => {
            kraken_positions_rev.hash(&mut hasher);
            kraken_catalog_rev.hash(&mut hasher);
        }
        EventSource::Positions => {
            alpaca_positions_rev.hash(&mut hasher);
            kraken_positions_rev.hash(&mut hasher);
        }
    }
    hasher.finish()
}

impl TyphooNApp {
    pub(super) fn broker_scope_membership_signature(&self) -> u64 {
        broker_scope_membership_signature(
            self.broker_scope,
            self.alpaca_position_membership_rev,
            self.kraken_position_membership_rev,
            self.kraken_scope_catalog_rev,
            self.alpaca_scope_catalog_rev,
        )
    }

    /// Re-resolve `cached_scope_syms` if anything feeding the scope set moved.
    /// O(1) when nothing changed — the key compare is the entire cost, so this
    /// is cheap to call again at a point of use.
    ///
    /// It has to be callable from `ui()`, not just the `logic()` pump, because
    /// `broker_scope` is mutated *during* `ui()` — the menu-bar Scope chip, the
    /// scope window, the scrape-status window, the command palette. A consumer
    /// that only ever saw the `logic()` refresh reads a symbol set belonging to
    /// the scope it was showing one frame ago.
    ///
    /// Returns the key the current set was resolved under, so caches derived
    /// from it (the scoped-fundamentals snapshot) key off the same value and
    /// cannot drift from the set they were built from.
    pub(super) fn refresh_broker_scope_cache(&mut self) -> (u64, EventSource, u64) {
        let scope_key = (
            self.bg_rev,
            self.broker_scope,
            self.broker_scope_membership_signature(),
        );
        if self.cached_scope_key != Some(scope_key) {
            self.cached_scope_syms = self.broker_scope_symbols();
            self.cached_scope_key = Some(scope_key);
        }
        scope_key
    }

    pub(super) fn dark_visuals() -> egui::Visuals {
        let mut v = egui::Visuals::dark();
        // ── TOTAL AESTHETIC OVERHAUL: square, compact, dark like Godel Terminal ──
        v.panel_fill = egui::Color32::from_rgb(0, 0, 0);
        v.window_fill = egui::Color32::from_rgb(10, 10, 18); // very dark blue-black
        v.extreme_bg_color = egui::Color32::from_rgb(0, 0, 0);
        v.faint_bg_color = egui::Color32::from_rgb(8, 8, 14);
        // Widget colors — dark blue inputs, minimal contrast
        v.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(8, 8, 14);
        v.widgets.noninteractive.fg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(180, 180, 190));
        v.widgets.noninteractive.bg_stroke =
            egui::Stroke::new(0.5, egui::Color32::from_rgb(30, 30, 40));
        v.widgets.inactive.bg_fill = egui::Color32::from_rgb(15, 20, 35); // dark blue input bg
        v.widgets.inactive.bg_stroke = egui::Stroke::new(0.5, egui::Color32::from_rgb(40, 45, 60));
        v.widgets.hovered.bg_fill = egui::Color32::from_rgb(20, 30, 55);
        v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 70, 100));
        v.widgets.active.bg_fill = egui::Color32::from_rgb(15, 40, 80);
        v.selection.bg_fill = egui::Color32::from_rgb(15, 40, 80);
        v.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 140, 255));
        // Windows — SQUARE corners, thin border, minimal shadow
        v.window_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 42, 54));
        v.window_shadow = egui::Shadow {
            offset: [1, 2],
            blur: 4,
            spread: 0,
            color: egui::Color32::from_rgba_premultiplied(0, 0, 0, 120),
        };
        v.window_corner_radius = egui::CornerRadius::same(0); // SQUARE
        v.menu_corner_radius = egui::CornerRadius::same(0); // SQUARE
        // Separator
        v.widgets.noninteractive.corner_radius = egui::CornerRadius::same(0);
        v
    }

    /// Write to KV cache only if content changed AND at least 30s since last write.
    /// Reduces KV timestamp churn.
    /// Key is `&'static str` so hashmap inserts don't allocate a new String per call.
    pub(super) fn put_kv_dedup(&mut self, key: &'static str, json: &str) {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        json.hash(&mut h);
        let hash = h.finish();
        let prev_hash = self.kv_write_hashes.get(key).copied().unwrap_or(0);
        if hash == prev_hash {
            return;
        } // content unchanged — skip entirely
        // Content changed — but throttle writes to at most every 30s per key
        let now = std::time::Instant::now();
        let last_write = self.kv_write_times.get(key).copied();
        if let Some(t) = last_write {
            if now.duration_since(t).as_secs() < 30 {
                return;
            } // too soon — skip
        }
        self.kv_write_hashes.insert(key, hash);
        self.kv_write_times.insert(key, now);
        if let Some(cache) = self.cache.clone() {
            let json = json.to_string();
            self.rt_handle.spawn_blocking(move || {
                let _ = cache.put_kv(key, &json);
            });
        }
    }

    /// Get all "active" symbols: chart tabs + open positions from ticked brokers + watchlist.
    /// Broker scope symbol set for fundamental filtering.
    /// Returns None for `EventSource::All` (no filter applied). Otherwise returns the
    /// uppercase bare-ticker set for the selected broker.
    pub(super) fn broker_scope_symbols(&self) -> Option<std::collections::HashSet<String>> {
        match self.broker_scope {
            EventSource::All => None,
            // Alpaca's tradable universe, mirroring how Kraken scope is the whole
            // Kraken catalog. This used to be open positions only, which made
            // Alpaca a strict subset of the separate `Positions` scope, asymmetric
            // with Kraken, and empty whenever the account was flat — the reason a
            // scoped SEC/news scrape reported "Scope Alpaca has no symbols".
            EventSource::Alpaca => Some(self.alpaca_scope_symbols()),
            EventSource::Kraken => Some(self.kraken_scope_symbols()),
            EventSource::Positions => {
                // All symbols with open positions across any broker
                let mut syms = std::collections::HashSet::new();
                for p in &self.live_positions {
                    syms.insert(p.symbol.replace('/', "").to_uppercase());
                }
                for p in &self.kr_positions {
                    syms.insert(p.symbol.replace('/', "").to_uppercase());
                }
                Some(syms)
            }
        }
    }

    pub(super) fn sec_scrape_scope_symbols(&self) -> Vec<String> {
        match self.broker_scope {
            EventSource::All => {
                // Scope ALL means the whole tradable equity universe, not the
                // current chart/news focus set. Kraken xStocks arrive after
                // startup via KrakenEquityUniverse; until then return empty so
                // auto-scrape defers instead of launching a misleading 4-symbol
                // scrape that prevents the real universe scrape from starting.
                if self.kraken_enabled
                    && self.kraken_scrape_xstocks
                    && self.kraken_equity_universe_symbols.is_empty()
                {
                    return Vec::new();
                }

                // Manual/broad ALL still needs to honor active trading context first.
                // The previous path threw everything into a HashSet and sorted A→Z,
                // so active names like WOK could sit behind thousands of A/B/C... symbols
                // while the UI appeared to be scraping recent SEC filings correctly.
                let priority = self.active_news_scrape_symbols();
                let mut broad = std::collections::HashSet::new();
                broad.extend(self.kraken_equity_universe_symbols.iter().cloned());
                for (sym, _name, class) in &self.all_broker_assets {
                    if class == "us_equity" {
                        broad.insert(sym.clone());
                    }
                }
                normalize_sec_scrape_symbols_priority_order(priority, broad)
            }
            EventSource::Kraken => kraken_sec_scrape_scope_symbols(
                &self.kr_positions,
                &self.kraken_equity_universe_symbols,
                &self.kraken_pairs,
                &self.kraken_futures_symbols,
            ),
            // Alpaca's tradable equity catalog, active context first — the same
            // shape as the ALL and Kraken branches. Falling through to the `_`
            // arm meant a scoped scrape targeted open positions only, so a flat
            // account produced "Scope Alpaca has no symbols" and no scrape.
            EventSource::Alpaca => {
                let priority = self.active_news_scrape_symbols();
                let broad: std::collections::HashSet<String> = self
                    .all_broker_assets
                    .iter()
                    .filter(|(_sym, _name, class)| class == "us_equity")
                    .map(|(sym, _name, _class)| sym.clone())
                    .collect();
                normalize_sec_scrape_symbols_priority_order(priority, broad)
            }
            _ => {
                let raw = self.broker_scope_symbols().unwrap_or_default();
                normalize_sec_scrape_symbols_priority_order(std::collections::HashSet::new(), raw)
            }
        }
    }

    pub(super) fn active_news_scrape_symbols(&self) -> std::collections::HashSet<String> {
        let mut syms = self.news_focus_symbols();
        syms.extend(self.mtf_grid_news_symbols());
        for p in &self.live_positions {
            let s = p.symbol.trim().to_ascii_uppercase();
            if !s.is_empty() {
                syms.insert(s);
            }
        }
        for p in &self.kr_positions {
            let s = p.symbol.trim().to_ascii_uppercase();
            if !s.is_empty() {
                syms.insert(s.trim_end_matches(".EQ").to_string());
            }
        }
        syms
    }

    pub(super) fn news_scrape_scope_symbols(&self) -> Vec<String> {
        let mut raw = std::collections::HashSet::new();
        match self.broker_scope {
            EventSource::All => {
                // News Scope ALL is an explicit full-universe scrape. Do not
                // reuse the active/watchlist/MTF focus set here; that regression
                // silently turned ALL into a tiny active scrape.
                if self.kraken_enabled
                    && self.kraken_scrape_xstocks
                    && self.kraken_equity_universe_symbols.is_empty()
                {
                    return Vec::new();
                }
                raw.extend(self.kraken_equity_universe_symbols.iter().cloned());
                for (sym, _name, class) in &self.all_broker_assets {
                    if class == "us_equity" {
                        raw.insert(sym.clone());
                    }
                }
                raw.extend(self.active_news_scrape_symbols());
            }
            // Alpaca's tradable equity catalog, mirroring the same branch in
            // `sec_scrape_scope_symbols`. Falling through to `_` made Scope
            // Alpaca mean "the handful of symbols already on screen", so the
            // background sweep could never reach the broad universe for anyone
            // not parked on Scope ALL.
            EventSource::Alpaca => {
                for (sym, _name, class) in &self.all_broker_assets {
                    if class == "us_equity" {
                        raw.insert(sym.clone());
                    }
                }
                raw.extend(self.active_news_scrape_symbols());
            }
            // Kraken's tradable universe: xStock equities plus spot/FX pairs.
            // Crypto belongs here in a way it does not for SEC filings — the
            // news pipeline has CryptoPanic/CoinDesk providers and dedups
            // fetches by base asset, so the ~875 pairs collapse to far fewer
            // requests. Defer while the equity universe is still loading, the
            // same way ALL does, rather than committing to an active-only sweep.
            EventSource::Kraken => {
                if self.kraken_enabled
                    && self.kraken_scrape_xstocks
                    && self.kraken_equity_universe_symbols.is_empty()
                {
                    return Vec::new();
                }
                raw.extend(self.kraken_equity_universe_symbols.iter().cloned());
                for (pair, display) in &self.kraken_pairs {
                    let display_or_pair = if display.trim().is_empty() {
                        pair.as_str()
                    } else {
                        display.as_str()
                    };
                    raw.insert(display_or_pair.to_string());
                }
                raw.extend(self.active_news_scrape_symbols());
            }
            // Positions scope is "what I hold", which the active set already
            // covers (it unions both brokers' open positions).
            _ => raw.extend(self.active_news_scrape_symbols()),
        }
        let mut syms: Vec<String> = raw
            .into_iter()
            .map(|sym| {
                normalize_market_data_symbol(&sym)
                    .replace('/', "")
                    .to_ascii_uppercase()
            })
            .filter(|sym| !sym.is_empty())
            .collect();
        syms.sort_unstable();
        syms.dedup();
        syms
    }

    /// Alpaca scope membership: the tradable US-equity catalog plus anything
    /// currently held. Positions are unioned in so a held symbol stays in scope
    /// even before `all_broker_assets` has loaded (or if the asset list omits
    /// it); before that fetch lands this degrades to positions-only, which is
    /// the previous behaviour rather than an empty scope.
    pub(super) fn alpaca_scope_symbols(&self) -> std::collections::HashSet<String> {
        let mut syms: std::collections::HashSet<String> = self
            .all_broker_assets
            .iter()
            .filter(|(_sym, _name, class)| class == "us_equity")
            .map(|(sym, _name, _class)| sym.replace('/', "").to_uppercase())
            .filter(|sym| !sym.is_empty())
            .collect();
        for p in &self.live_positions {
            let symbol = p.symbol.replace('/', "").to_uppercase();
            if !symbol.is_empty() {
                syms.insert(symbol);
            }
        }
        syms
    }

    pub(super) fn kraken_scope_symbols(&self) -> std::collections::HashSet<String> {
        kraken_scope_membership_symbols(
            &self.kr_positions,
            &self.kraken_equity_universe_symbols,
            &self.kraken_pairs,
            &self.kraken_futures_symbols,
        )
    }

    /// Fundamentals filtered by the current `broker_scope`. Returns a Vec of refs
    /// (cheap — just pointers). Uses per-frame cached scope HashSet.
    /// PERF: f.symbol is already uppercase (guaranteed by parse_yahoo_data), so we
    /// skip the redundant to_uppercase allocation per record.
    pub(super) fn scoped_fundamentals(
        &self,
    ) -> Vec<&typhoon_engine::core::fundamentals::Fundamentals> {
        match &self.cached_scope_syms {
            None => self.bg.all_fundamentals.iter().collect(),
            Some(set) => self
                .bg
                .all_fundamentals
                .iter()
                .filter(|f| set.contains(f.symbol.as_str()))
                .collect(),
        }
    }

    /// Owned-Vec variant for APIs that require `&[Fundamentals]`.
    /// When scope=All, returns a clone of the full list (no filter work).
    pub(super) fn scoped_fundamentals_owned(
        &self,
    ) -> Vec<typhoon_engine::core::fundamentals::Fundamentals> {
        match &self.cached_scope_syms {
            None => self.bg.all_fundamentals.clone(),
            Some(set) => self
                .bg
                .all_fundamentals
                .iter()
                .filter(|f| set.contains(f.symbol.as_str()))
                .cloned()
                .collect(),
        }
    }

    /// Fire a per-ticker filing-history query when the search box changes.
    ///
    /// Cheap in steady state: parsing a short string and comparing it against
    /// what is already loaded / in flight. Only a genuinely new symbol search
    /// sends a command.
    pub(super) fn dispatch_sec_history_query(&mut self) {
        let tickers = sec_history_query_tickers(&self.sec_search_query);
        if tickers.is_empty() {
            // Query cleared or is free text — drop the history so the view falls
            // back to the recent snapshot instead of showing a stale symbol.
            if !self.sec_history_tickers.is_empty() {
                self.sec_history_filings.clear();
                self.sec_history_tickers.clear();
            }
            self.sec_history_inflight = None;
            return;
        }
        if self.sec_history_tickers == tickers
            || self.sec_history_inflight.as_ref() == Some(&tickers)
        {
            return;
        }
        self.sec_history_inflight = Some(tickers.clone());
        let _ = self.broker_tx.send(BrokerCmd::SecFilingHistory {
            db_path: cache_db_path(),
            tickers,
            limit: SEC_HISTORY_PER_TICKER_ROWS,
        });
    }

    /// Rebuild SEC window caches if any of the keyed state has changed.
    /// Called once per frame when the SEC window is open. Caches are:
    ///   - tab counts (scoped filings, alerts, insider trades) — keyed on (bg_rev, scope)
    ///   - filings indices (dedup+filter+sort) — keyed on (bg_rev, scope, filters, query, sort)
    ///   - insider aggregation + cluster detection — keyed on (bg_rev, scope, query)
    ///   - timeline (by-month grouping) — keyed on (bg_rev, scope)
    /// O(1) steady state: the O(N) work only runs when the user changes a filter,
    /// types in the search box, or new bg data lands.
    pub(super) fn rebuild_sec_caches(&mut self) {
        use std::hash::{Hash, Hasher};

        // Every cache below filters on the resolved scope *set*
        // (`cached_scope_syms`), which the `logic()` pump refreshes once per
        // frame — before `ui()` runs. The Scope chip lives in the menu bar,
        // which `ui()` draws before this window, so on the frame the user
        // cycles Scope the enum is already the new value while the set is still
        // the previous scope's. The caches were then built with the *old*
        // filter and stored under the *new* scope's key, so the following
        // frame — when the pump had caught up — found a matching key and never
        // rebuilt, latching the one-frame lag permanently. Cycling
        // Kraken → All rendered All through Kraken's ~159 xStock tickers
        // against an unscoped recent-filings snapshot: zero rows. Alpaca and
        // Kraken looked fine only because they inherited the wider preceding
        // scope's filter.
        let _ = self.refresh_broker_scope_cache();
        // Symbol searches are answered from SQLite, not from the capped
        // snapshot. Dispatch happens here (once per changed query) so the rows
        // are already in hand by the time the cache below rebuilds.
        self.dispatch_sec_history_query();

        let sec_data_key = {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            self.bg.sec_filings.len().hash(&mut h);
            self.bg.sec_alerts.len().hash(&mut h);
            self.bg.insider_trades.len().hash(&mut h);
            self.bg.sec_content_stats.hash(&mut h);
            // On-demand history swaps the row source out from under every cache
            // below, so it has to move the data key or a landed query renders
            // nothing until some other input happens to change.
            self.sec_history_filings.len().hash(&mut h);
            self.sec_history_tickers.hash(&mut h);
            if let Some(first) = self.bg.sec_filings.first() {
                first.id.hash(&mut h);
                first.filing_date.hash(&mut h);
                first.accession_number.hash(&mut h);
            }
            if let Some(last) = self.bg.sec_filings.last() {
                last.id.hash(&mut h);
                last.filing_date.hash(&mut h);
                last.accession_number.hash(&mut h);
            }
            h.finish()
        };
        let scope = self.broker_scope;
        // Scope identity for the data caches: the enum plus the membership
        // signature, so a scope whose symbol set widens (broker catalog lands
        // behind a positions-only start) re-filters instead of holding the
        // narrow first result.
        let scope_identity =
            sec_scope_identity_key(scope, self.broker_scope_membership_signature());

        // Tab counts — (SEC data, scope)
        let counts_key = {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            sec_data_key.hash(&mut h);
            scope_identity.hash(&mut h);
            h.finish()
        };
        let filings_key = {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            sec_data_key.hash(&mut h);
            scope_identity.hash(&mut h);
            self.sec_filters.hash(&mut h);
            self.sec_search_query.hash(&mut h);
            self.sec_sort.column.hash(&mut h);
            self.sec_sort.ascending.hash(&mut h);
            h.finish()
        };
        let filings_controls_key = sec_filings_controls_key(
            scope,
            &self.sec_filters,
            &self.sec_search_query,
            self.sec_sort.column,
            self.sec_sort.ascending,
        );
        let insiders_key = {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            sec_data_key.hash(&mut h);
            scope_identity.hash(&mut h);
            self.sec_search_query.hash(&mut h);
            h.finish()
        };
        let timeline_key = counts_key; // same key as tab counts
        let cache_cold = self.sec_cache_tab_counts_key.is_none()
            || self.sec_cache_filings_key.is_none()
            || self.sec_cache_insiders_key.is_none()
            || self.sec_cache_timeline_key.is_none();
        let cache_changed = self.sec_cache_tab_counts_key != Some(counts_key)
            || self.sec_cache_filings_key != Some(filings_key)
            || self.sec_cache_insiders_key != Some(insiders_key)
            || self.sec_cache_timeline_key != Some(timeline_key);
        let visible_filings_controls_changed =
            self.sec_tab == 0 && self.sec_cache_filings_controls_key != Some(filings_controls_key);
        if cache_changed
            && self.heavy_sync_in_progress
            && self.scrape_sec_running
            && !visible_filings_controls_changed
        {
            // During the broad EDGAR scrape the background thread can publish huge
            // filing/insider snapshots. Rebuilding the visible SEC tab cache on
            // egui has already produced 10s+ chart freezes. Keep the last cache
            // while scraping. User-driven filing filter/search/sort changes are
            // allowed through; otherwise the scanner controls look broken.
            return;
        }
        if cache_changed
            && !cache_cold
            && self.heavy_sync_in_progress
            && !visible_filings_controls_changed
            && self.sec_cache_last_rebuild.elapsed() < SEC_CACHE_HEAVY_SYNC_MIN_REBUILD_INTERVAL
        {
            return;
        }
        if cache_changed {
            self.sec_cache_last_rebuild = std::time::Instant::now();
        }
        if self.sec_cache_tab_counts_key != Some(counts_key) {
            let count_rows: &[typhoon_engine::core::sec_filing::SecFiling] =
                if self.sec_history_filings.is_empty() {
                    &self.bg.sec_filings
                } else {
                    &self.sec_history_filings
                };
            let (scoped, insider_total) = match &self.cached_scope_syms {
                None => (
                    count_rows.len(),
                    self.bg.insider_trades.values().map(|v| v.len()).sum(),
                ),
                Some(set) => (
                    count_rows
                        .iter()
                        .filter(|f| set.contains(f.ticker.as_str()))
                        .count(),
                    self.bg
                        .insider_trades
                        .iter()
                        .filter(|(k, _)| set.contains(k.as_str()))
                        .map(|(_, v)| v.len())
                        .sum(),
                ),
            };
            self.sec_cache_tab_counts = (scoped, self.bg.sec_alerts.len(), insider_total);
            self.sec_cache_tab_counts_key = Some(counts_key);
        }

        // Filings tab — (SEC data, scope, filters, query, sort column, sort direction).
        // Do not rebuild hidden-tab caches while the broad SEC scraper is inserting rows;
        // hidden O(N) work is exactly what makes the UI feel stuck.
        if self.sec_tab == 0 && self.sec_cache_filings_key != Some(filings_key) {
            let filter_types: &[&str] = &["4", "13F", "DEF 14A", "S-1", "10-K", "10-Q", "8-K"];
            // Symbol-only search: uppercase query once, compare against ticker (stored upper).
            let search_upper = self.sec_search_query.trim().to_uppercase();
            let has_search = !search_upper.is_empty();
            // Symbol searches read the DB answer; everything else reads the
            // capped recent snapshot. Field-level borrows so the cache
            // assignment below still type-checks.
            let filings: &[typhoon_engine::core::sec_filing::SecFiling] =
                if self.sec_history_filings.is_empty() {
                    &self.bg.sec_filings
                } else {
                    &self.sec_history_filings
                };

            // Search now spans (ticker, company, sector, industry). Build a small
            // fundamentals lookup so the sector/industry hit is O(1) per row.
            let search_fund_map: std::collections::HashMap<&str, (String, String)> = if has_search {
                self.bg
                    .all_fundamentals
                    .iter()
                    .map(|f| {
                        (
                            f.symbol.as_str(),
                            (f.sector.to_uppercase(), f.industry.to_uppercase()),
                        )
                    })
                    .collect()
            } else {
                std::collections::HashMap::new()
            };
            let mut seen: std::collections::HashSet<(&str, &str, &str)> =
                std::collections::HashSet::with_capacity(filings.len());
            let mut idxs: Vec<usize> = Vec::with_capacity(filings.len());
            for (idx, f) in filings.iter().enumerate() {
                // Dedup by (date, ticker, form_type) — tuple key, no per-row format!() alloc.
                let key = (
                    f.filing_date.as_str(),
                    f.ticker.as_str(),
                    f.form_type.as_str(),
                );
                if !seen.insert(key) {
                    continue;
                }
                if !sec_filing_form_matches_filters(&f.form_type, &self.sec_filters, filter_types) {
                    continue;
                }
                if let Some(ref set) = self.cached_scope_syms {
                    if !set.contains(f.ticker.as_str()) {
                        continue;
                    }
                }
                if has_search {
                    let ticker_hit = f.ticker.contains(search_upper.as_str());
                    let company_hit = f
                        .company_name
                        .to_uppercase()
                        .contains(search_upper.as_str());
                    let (sector_hit, industry_hit) = match search_fund_map.get(f.ticker.as_str()) {
                        Some((sec, ind)) => (
                            sec.contains(search_upper.as_str()),
                            ind.contains(search_upper.as_str()),
                        ),
                        None => (false, false),
                    };
                    if !(ticker_hit || company_hit || sector_hit || industry_hit) {
                        continue;
                    }
                }
                idxs.push(idx);
            }
            // Sort by selected column — avoid borrowing self.sec_sort inside closure.
            // Sector / industry columns (6,7) read from the fundamentals map; build
            // it once before the sort closure so the lookup is O(1) per compare.
            let col = self.sec_sort.column;
            let asc = self.sec_sort.ascending;
            if !(col == 0 && !asc) {
                let needs_fund_lookup = matches!(col, 6 | 7);
                let fund_map: std::collections::HashMap<&str, (&str, &str)> = if needs_fund_lookup {
                    self.bg
                        .all_fundamentals
                        .iter()
                        .map(|f| (f.symbol.as_str(), (f.sector.as_str(), f.industry.as_str())))
                        .collect()
                } else {
                    std::collections::HashMap::new()
                };
                idxs.sort_by(|&a, &b| {
                    let fa = &filings[a];
                    let fb = &filings[b];
                    let ord = match col {
                        0 => fa.filing_date.cmp(&fb.filing_date),
                        1 => fa.ticker.cmp(&fb.ticker),
                        2 => fa.form_type.cmp(&fb.form_type),
                        3 => fa.category.cmp(&fb.category),
                        4 => fa.company_name.cmp(&fb.company_name),
                        5 => fa.accession_number.cmp(&fb.accession_number),
                        6 => {
                            let sa = fund_map.get(fa.ticker.as_str()).map(|v| v.0).unwrap_or("");
                            let sb = fund_map.get(fb.ticker.as_str()).map(|v| v.0).unwrap_or("");
                            sa.cmp(sb)
                        }
                        7 => {
                            let ia = fund_map.get(fa.ticker.as_str()).map(|v| v.1).unwrap_or("");
                            let ib = fund_map.get(fb.ticker.as_str()).map(|v| v.1).unwrap_or("");
                            ia.cmp(ib)
                        }
                        _ => std::cmp::Ordering::Equal,
                    };
                    if asc { ord } else { ord.reverse() }
                });
            }
            self.sec_cache_filings = idxs;
            self.sec_cache_filings_key = Some(filings_key);
            self.sec_cache_filings_controls_key = Some(filings_controls_key);
        }

        // Insiders tab — (SEC data, scope, query)
        if self.sec_tab == 2 && self.sec_cache_insiders_key != Some(insiders_key) {
            let search_upper = self.sec_search_query.trim().to_uppercase();
            let has_search = !search_upper.is_empty();

            let mut rows: Vec<(String, usize, String)> = Vec::new(); // (ticker, idx, date)
            for (ticker, trades) in &self.bg.insider_trades {
                if let Some(ref set) = self.cached_scope_syms {
                    if !set.contains(ticker.as_str()) {
                        continue;
                    }
                }
                if has_search && !ticker.contains(search_upper.as_str()) {
                    continue;
                }
                for (i, trade) in trades.iter().enumerate() {
                    rows.push((ticker.clone(), i, trade.transaction_date.clone()));
                }
            }
            // Sort newest first.
            rows.sort_by(|a, b| b.2.cmp(&a.2));

            // Cluster detection: 3+ trades for same symbol within 14 days.
            let mut by_sym: std::collections::HashMap<String, Vec<&str>> =
                std::collections::HashMap::new();
            for (ticker, _, date) in &rows {
                by_sym
                    .entry(ticker.clone())
                    .or_default()
                    .push(date.as_str());
            }
            let mut clusters: Vec<(String, usize)> = Vec::new();
            for (ticker, dates) in &by_sym {
                if dates.len() < 3 {
                    continue;
                }
                let mut sorted_dates: Vec<&str> = dates.clone();
                sorted_dates.sort();
                let mut is_cluster = false;
                for window in sorted_dates.windows(3) {
                    if let (Some(&first), Some(&last)) = (window.first(), window.last()) {
                        if last.len() >= 10 && first.len() >= 10 {
                            let d1: i64 = first[..10].replace('-', "").parse().unwrap_or(0);
                            let d2: i64 = last[..10].replace('-', "").parse().unwrap_or(0);
                            if (d2 - d1).abs() <= 14 {
                                is_cluster = true;
                                break;
                            }
                        }
                    }
                }
                if is_cluster {
                    clusters.push((ticker.clone(), dates.len()));
                }
            }

            self.sec_cache_insiders = rows.into_iter().map(|(t, i, _)| (t, i)).collect();
            self.sec_cache_insiders_clusters = clusters;
            self.sec_cache_insiders_key = Some(insiders_key);
        }

        // Timeline tab — (SEC data, scope)
        if self.sec_tab == 3 && self.sec_cache_timeline_key != Some(timeline_key) {
            let mut by_month: std::collections::BTreeMap<String, Vec<usize>> =
                std::collections::BTreeMap::new();
            for (idx, f) in self.bg.sec_filings.iter().enumerate() {
                if let Some(ref set) = self.cached_scope_syms {
                    if !set.contains(f.ticker.as_str()) {
                        continue;
                    }
                }
                let month = if f.filing_date.len() >= 7 {
                    f.filing_date[..7].to_string()
                } else {
                    f.filing_date.clone()
                };
                by_month.entry(month).or_default().push(idx);
            }
            let mut out: Vec<(String, usize, String)> = Vec::with_capacity(by_month.len());
            for (month, idxs) in by_month.iter().rev() {
                let count = idxs.len();
                let mut types: std::collections::HashMap<&str, usize> =
                    std::collections::HashMap::new();
                for &i in idxs {
                    *types
                        .entry(self.bg.sec_filings[i].form_type.as_str())
                        .or_default() += 1;
                }
                let mut type_vec: Vec<String> =
                    types.iter().map(|(t, c)| format!("{}:{}", t, c)).collect();
                type_vec.sort();
                out.push((month.clone(), count, type_vec.join(" ")));
            }
            self.sec_cache_timeline = out;
            self.sec_cache_timeline_key = Some(timeline_key);
        }
    }

    /// Short label for the current broker scope — used in window headers.
    /// Build an iCalendar (RFC 5545) payload for the current Event Calendar filter.
    /// Events are emitted as all-day VEVENTs — we only store date strings, not precise times.
    /// UX3: Apply a deferred symbol action from a context menu.
    /// Applied after the render closure exits to avoid borrow conflicts.
    pub(super) fn apply_symbol_action(&mut self, action: SymbolAction) {
        match action {
            SymbolAction::None => {}
            SymbolAction::OpenChart(sym) => {
                let sym = normalize_market_data_symbol(&sym);
                let target = sym.to_uppercase();
                if let Some(idx) = self
                    .charts
                    .iter()
                    .position(|c| c.symbol.to_uppercase().contains(&target))
                {
                    self.active_tab = idx;
                } else {
                    let tf = self
                        .charts
                        .get(self.active_tab)
                        .map(|c| c.timeframe)
                        .unwrap_or(Timeframe::D1);
                    let chart = ChartState::new(&sym, tf);
                    self.charts.push(chart);
                    self.active_tab = self.charts.len() - 1;
                    self.rebuild_chart_live_index();
                    // Defer load to the paced loader (ADR-098): opening a chart must not
                    // block the render thread on a heavy symbol's full-history load.
                    self.queue_chart_reload(self.active_tab);
                }
                // Immediate catch-up for crypto — Kraken fills gaps after outages/account loss.
                // Rotation would get to this symbol eventually, but opening a chart
                // should heal within seconds, not ~10 minutes.
                let bare = normalize_market_data_symbol(&sym);
                if Self::demand_is_crypto(&bare) {
                    let tfs = self.enabled_standard_sync_timeframes();
                    for tf in &tfs {
                        self.queue_alpaca_fetch(&bare, tf);
                    }
                    let kr_tfs = self.filtered_sync_timeframes([
                        "1Day", "1Hour", "4Hour", "15Min", "30Min", "5Min",
                    ]);
                    if self.kraken_spot_symbol_scrape_enabled(&bare) {
                        for tf in kr_tfs {
                            self.queue_kraken_fetch(&bare, &tf);
                        }
                    }
                }
            }
            SymbolAction::OpenChartTf(sym, tf) => {
                let sym = normalize_market_data_symbol(&sym);
                let target = sym.to_uppercase();
                // Reuse an existing tab already showing this symbol at this TF;
                // otherwise open a fresh tab at the requested TF.
                if let Some(idx) = self
                    .charts
                    .iter()
                    .position(|c| c.timeframe == tf && c.symbol.to_uppercase().contains(&target))
                {
                    self.active_tab = idx;
                } else {
                    let chart = ChartState::new(&sym, tf);
                    self.charts.push(chart);
                    self.active_tab = self.charts.len() - 1;
                    self.rebuild_chart_live_index();
                    // Defer load to the paced loader (ADR-098): opening a chart must not
                    // block the render thread on a heavy symbol's full-history load.
                    self.queue_chart_reload(self.active_tab);
                }
                // Fill bars if the cache was cold for this symbol/TF.
                self.queue_open_symbol_sync_all_timeframes(&sym);
            }
            SymbolAction::AddWatchlist(sym) => {
                let sym_u = sym.to_uppercase();
                if !self.user_watchlist_set.contains(&sym_u) {
                    self.user_watchlist.push(sym_u.clone());
                    self.user_watchlist_set.insert(sym_u.clone());
                    // Force the cache-fallback re-scan and request a fresh quote so
                    // the newly added symbol fills in without waiting for the next
                    // rotation tick.
                    self.watchlist_cache_tried = false;
                    let _ = self.broker_tx.send(BrokerCmd::GetWatchlistQuotes {
                        symbols: self.user_watchlist.clone(),
                    });
                    self.log
                        .push_back(LogEntry::info(format!("Added {} to watchlist", sym_u)));
                }
            }
            SymbolAction::ShowFundamentals => self.show_fundamentals = true,
            SymbolAction::ShowSec(sym) => {
                self.show_sec = true;
                self.sec_search_query = sym;
            }
            SymbolAction::ShowInsider => self.show_insider = true,
        }
    }

    /// Compatible with Google Calendar, Apple Calendar, Outlook, Thunderbird.
    pub(super) fn build_events_ics(
        rows: &[EventRow],
        source_filter: EventSource,
        show_earnings: bool,
        show_exdiv: bool,
        show_divpay: bool,
    ) -> String {
        let mut out = String::new();
        out.push_str("BEGIN:VCALENDAR\r\n");
        out.push_str("VERSION:2.0\r\n");
        out.push_str("PRODID:-//TyphooN Terminal//Event Calendar//EN\r\n");
        out.push_str("CALSCALE:GREGORIAN\r\n");
        out.push_str("METHOD:PUBLISH\r\n");
        out.push_str("X-WR-CALNAME:TyphooN Event Calendar\r\n");

        let escape = |s: &str| -> String {
            s.replace('\\', "\\\\")
                .replace(';', "\\;")
                .replace(',', "\\,")
                .replace('\n', "\\n")
        };

        let now = chrono::Utc::now();
        let dtstamp = now.format("%Y%m%dT%H%M%SZ").to_string();

        for r in rows {
            let src_ok = match source_filter {
                EventSource::All => r.in_alpaca || r.in_kraken,
                EventSource::Alpaca => r.in_alpaca,
                EventSource::Kraken => r.in_kraken,
                EventSource::Positions => r.in_alpaca || r.in_kraken,
            };
            let kind_ok = match r.kind {
                EventKind::Earnings => show_earnings,
                EventKind::ExDividend => show_exdiv,
                EventKind::DividendPayment => show_divpay,
            };
            if !src_ok || !kind_ok {
                continue;
            }

            // Parse date to YYYYMMDD for DTSTART;VALUE=DATE
            let date_compact = match chrono::NaiveDate::parse_from_str(&r.date, "%Y-%m-%d") {
                Ok(d) => d.format("%Y%m%d").to_string(),
                Err(_) => continue, // skip un-parseable rows
            };
            let next_day = chrono::NaiveDate::parse_from_str(&r.date, "%Y-%m-%d")
                .ok()
                .and_then(|d| d.succ_opt())
                .map(|d| d.format("%Y%m%d").to_string())
                .unwrap_or_else(|| date_compact.clone());

            let uid = format!(
                "{}-{}-{}@typhoon-terminal",
                r.symbol,
                date_compact,
                r.kind.label()
            );
            // Escape once at the final emit site — avoid double-escaping when fields get concatenated.
            let summary_raw = format!("{} — {} ({})", r.symbol, r.kind.label(), r.company);
            let description_raw = if r.detail.is_empty() {
                format!("{} — {}", r.symbol, r.kind.label())
            } else {
                format!("{}\n{}", r.kind.label(), r.detail)
            };

            out.push_str("BEGIN:VEVENT\r\n");
            out.push_str(&format!("UID:{}\r\n", escape(&uid)));
            out.push_str(&format!("DTSTAMP:{}\r\n", dtstamp));
            out.push_str(&format!("DTSTART;VALUE=DATE:{}\r\n", date_compact));
            out.push_str(&format!("DTEND;VALUE=DATE:{}\r\n", next_day));
            out.push_str(&format!("SUMMARY:{}\r\n", escape(&summary_raw)));
            out.push_str(&format!("DESCRIPTION:{}\r\n", escape(&description_raw)));
            out.push_str("END:VEVENT\r\n");
        }

        out.push_str("END:VCALENDAR\r\n");
        out
    }

    pub(super) fn broker_scope_label(&self) -> &'static str {
        match self.broker_scope {
            EventSource::All => "All",
            EventSource::Alpaca => "Alpaca",
            EventSource::Kraken => "Kraken",
            EventSource::Positions => "Positions",
        }
    }

    /// Format an AI session (`Vec<(is_user, message)>`) into a markdown
    /// transcript suitable for both on-disk export and Matrix chat posting.
    /// Associated (non-&mut self) so it can be called inside an egui window
    /// closure where `&mut self.show_xxx` is already borrowed — the caller
    /// then dispatches to the &mut-self save/send helpers after the window
    /// closes.
    pub(super) fn format_ai_transcript(
        history: &[(bool, String)],
        provider_label: &str,
        assistant_label: &str,
        session_id: Option<&str>,
    ) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        let _ = writeln!(
            out,
            "# TyphooN Terminal — {provider_label} session transcript"
        );
        let _ = writeln!(
            out,
            "Exported: {}",
            chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ")
        );
        if let Some(id) = session_id {
            if !id.is_empty() {
                let _ = writeln!(out, "Session: {id}");
            }
        }
        let _ = writeln!(out, "Messages: {}", history.len());
        let _ = writeln!(out);
        for (is_user, msg) in history {
            let speaker = if *is_user { "You" } else { assistant_label };
            let _ = writeln!(out, "**{speaker}:**");
            let _ = writeln!(out);
            let _ = writeln!(out, "{msg}");
            let _ = writeln!(out);
        }
        out
    }

    /// Open a native save dialog and write an AI-session transcript to disk
    /// as a timestamped markdown file. No-op on dialog cancel.
    pub(super) fn save_ai_session_to_file(&mut self, provider: &str, transcript: &str) {
        let default_name = format!(
            "{}_session_{}.md",
            provider,
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        );
        let picked = rfd::FileDialog::new()
            .add_filter("Markdown", &["md"])
            .add_filter("Text", &["txt"])
            .set_file_name(&default_name)
            .set_title(&format!("Export {provider} session"))
            .save_file();
        if let Some(path) = picked {
            match std::fs::write(&path, transcript) {
                Ok(()) => self.log.push_back(LogEntry::info(format!(
                    "{provider} session exported: {} ({} bytes)",
                    path.display(),
                    transcript.len()
                ))),
                Err(e) => self.log.push_back(LogEntry::err(format!(
                    "{provider} session export failed: {e}"
                ))),
            }
        }
    }

    /// Post an AI-session transcript to the configured Matrix community
    /// room. Mirrors the Community-Chat window's gating — an empty,
    /// "pending", or "none" access token short-circuits with a log note
    /// instead of failing silently.
    pub(super) fn send_ai_session_to_matrix(&mut self, provider: &str, transcript: &str) {
        let tok = self.matrix_access_token.as_str();
        if tok.is_empty() || tok == "pending" || tok == "none" {
            self.log.push_back(LogEntry::warn(format!(
                "Matrix: can't send {provider} session — no access token (open Community Chat → Settings to log in)"
            )));
            return;
        }
        if self.matrix_room.is_empty() {
            self.log.push_back(LogEntry::warn(format!(
                "Matrix: can't send {provider} session — no room configured"
            )));
            return;
        }
        // Matrix homeservers commonly cap a single message payload around
        // 65 KB; long sessions get truncated with a marker so the user
        // isn't silently dropped.
        const MAX_LEN: usize = 60_000;
        let body = if transcript.len() > MAX_LEN {
            let head = &transcript[..MAX_LEN];
            format!("{head}\n\n… [truncated for Matrix; use the Save button for the full session]")
        } else {
            transcript.to_string()
        };
        let body_len = body.len();
        let _ = self.broker_tx.send(BrokerCmd::MatrixSendMessage {
            room_id: self.matrix_room.clone(),
            access_token: self.matrix_access_token.clone(),
            body,
        });
        self.log.push_back(LogEntry::info(format!(
            "{provider} session sent to Matrix ({body_len} bytes)"
        )));
    }

    /// Walk the on-disk screenshot directories (`~/Pictures` and `/tmp`
    /// fallback — matching the save path in the render loop) and refresh
    /// `screenshots_list` with all `typhoon_chart_*.webp` files sorted
    /// newest-first. Cheap enough to call on window-open + every 10s.
    pub(super) fn scan_screenshots(&mut self) {
        let mut found: Vec<(std::path::PathBuf, i64, u64)> = Vec::new();
        let mut dirs: Vec<std::path::PathBuf> = Vec::new();
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(std::path::PathBuf::from(home).join("Pictures"));
        }
        dirs.push(std::path::PathBuf::from("/tmp"));
        for dir in &dirs {
            let rd = match std::fs::read_dir(dir) {
                Ok(r) => r,
                Err(_) => continue,
            };
            for entry in rd.flatten() {
                let path = entry.path();
                let name = match path.file_name().and_then(|s| s.to_str()) {
                    Some(s) => s,
                    None => continue,
                };
                if !name.starts_with("typhoon_chart_") || !name.ends_with(".webp") {
                    continue;
                }
                let meta = match entry.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                let mtime = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                found.push((path, mtime, meta.len()));
            }
        }
        found.sort_by(|a, b| b.1.cmp(&a.1));
        self.screenshots_list = found;
        self.screenshots_sorted_indices.invalidate();
        self.screenshots_last_refresh = chrono::Utc::now().timestamp();
    }
}

fn sec_filing_form_matches_filters(
    form_type: &str,
    filters: &[bool; 7],
    filter_types: &[&str],
) -> bool {
    if filters.iter().all(|&enabled| enabled) {
        return true;
    }
    if filters.iter().all(|&enabled| !enabled) {
        return false;
    }
    filters.iter().enumerate().any(|(i, &enabled)| {
        if !enabled {
            return false;
        }
        let expected = filter_types.get(i).copied().unwrap_or("");
        if expected == "4" {
            form_type == "4"
        } else {
            form_type.contains(expected)
        }
    })
}

fn normalize_sec_scrape_symbol(sym: &str) -> Option<String> {
    let mut sym = sym.trim().to_uppercase();
    if sym.is_empty() || sym.starts_with("__") || sym.contains('/') {
        return None;
    }
    // Kraken xStocks positions can arrive as WOK.EQ before the full
    // Kraken equities catalog is loaded. SEC has the underlying equity ticker,
    // not the venue-qualified synthetic symbol. Filtering before stripping .EQ
    // silently dropped those symbols and made the SEC scrape report 0 tickers.
    if let Some(stripped) = sym.strip_suffix(".EQ") {
        sym = stripped.to_string();
    } else if let Some(stripped) = sym.strip_suffix(".X") {
        sym = stripped.to_string();
    }
    if typhoon_engine::core::news::is_crypto_symbol(&sym) {
        return None;
    }
    if sym.is_empty()
        || sym.len() > 5
        || sym.starts_with("__")
        || !sym.chars().all(|c| c.is_ascii_alphabetic())
    {
        return None;
    }
    Some(sym)
}

fn normalize_sec_scrape_symbols_priority_order(
    priority: std::collections::HashSet<String>,
    broad: std::collections::HashSet<String>,
) -> Vec<String> {
    let priority_norm: std::collections::HashSet<String> = priority
        .iter()
        .filter_map(|sym| normalize_sec_scrape_symbol(sym))
        .collect();
    let mut syms: Vec<String> = priority
        .into_iter()
        .chain(broad)
        .filter_map(|sym| normalize_sec_scrape_symbol(&sym))
        .collect();
    syms.sort_by(
        |a, b| match (priority_norm.contains(a), priority_norm.contains(b)) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.cmp(b),
        },
    );
    syms.dedup();
    syms
}

/// Parse the SEC scanner's search box into tickers to pull history for.
///
/// Only symbol-shaped input qualifies: the box also accepts company / sector /
/// industry text, and firing a per-ticker DB query for "technology" would be a
/// guaranteed miss. Comma or whitespace separated, so `SMCI, NVDA` pulls both.
/// Capped because each ticker is its own query and its own snapshot.
pub(super) fn sec_history_query_tickers(query: &str) -> Vec<String> {
    const MAX_TICKERS: usize = 8;

    // Every token must look like a ticker, not merely some of them. Accepting
    // the ticker-shaped subset meant "WESTERN DIGITAL CORP" dispatched a query
    // for CORP — a guaranteed miss that would then *replace* the snapshot rows
    // the company-name filter was about to match, turning a working search into
    // an empty table.
    let mut out: Vec<String> = Vec::new();
    for tok in query.split([',', ' ', '\t', ';']) {
        // SEC tickers are 1-5 alphanumerics, sometimes class-suffixed (BRK.B).
        let tok = tok.trim().to_ascii_uppercase();
        if tok.is_empty() {
            continue;
        }
        let ticker_shaped = tok.len() <= 6
            && tok
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
            && tok.chars().any(|c| c.is_ascii_alphabetic());
        if !ticker_shaped {
            return Vec::new();
        }
        if !out.contains(&tok) {
            out.push(tok);
        }
        if out.len() >= MAX_TICKERS {
            break;
        }
    }
    out.sort();
    out
}

/// Kraken scope **membership**: what the filtered views (fundamentals, SEC
/// filings/insiders/timeline) test their rows against.
///
/// The equity universe is the load-bearing part. It used to be absent: xStock
/// equities were derived only from `kraken_pairs` via
/// `kraken_xstock_fundamental_symbol`, which needs a `.EQ` suffix that the
/// **public** AssetPairs feed does not carry (that feed is crypto + spot FX;
/// `.EQ` shows up on private balances). So the set collapsed to crypto pairs
/// and futures, which share no tickers with the equity-keyed fundamentals or
/// filing tables — Kraken scope reported "0 fundamentals in scope" and an empty
/// Filings tab for the whole session, no matter how much the scraper fetched.
///
/// Worse, it disagreed with `kraken_sec_scrape_scope_symbols`, which has always
/// used the equity universe: the scraper pulled filings for Kraken's equities
/// and this filter then discarded every one of them. Both now start from the
/// same universe. `kraken_equity_universe_symbols` arrives bare, uppercase and
/// `.EQ`-stripped, matching `Fundamentals::symbol` and `SecFiling::ticker`.
fn kraken_scope_membership_symbols(
    kr_positions: &[PositionInfo],
    kraken_equity_universe_symbols: &[String],
    kraken_pairs: &[(String, String)],
    kraken_futures_symbols: &[String],
) -> std::collections::HashSet<String> {
    let mut syms = std::collections::HashSet::new();
    for p in kr_positions {
        let symbol = normalize_market_data_symbol(&p.symbol)
            .replace('/', "")
            .to_uppercase();
        if !symbol.is_empty() {
            syms.insert(symbol);
        }
    }
    for symbol in kraken_equity_universe_symbols {
        let symbol = symbol.trim().trim_end_matches(".EQ").to_uppercase();
        if !symbol.is_empty() {
            syms.insert(symbol);
        }
    }
    for (pair, display) in kraken_pairs {
        let display_or_pair = if display.trim().is_empty() {
            pair.as_str()
        } else {
            display.as_str()
        };
        let symbol = typhoon_engine::core::kraken::normalize_pair_symbol(display_or_pair)
            .replace('/', "")
            .to_uppercase();
        if !symbol.is_empty() {
            syms.insert(symbol);
        }
        if let Some(equity) = kraken_xstock_fundamental_symbol(pair, display) {
            syms.insert(equity.to_uppercase());
        }
    }
    for symbol in kraken_futures_symbols {
        let symbol = typhoon_engine::core::kraken_futures::normalize_futures_symbol(symbol)
            .replace('/', "")
            .to_uppercase();
        if !symbol.is_empty() {
            syms.insert(symbol);
        }
    }
    syms
}

fn kraken_sec_scrape_scope_symbols(
    kr_positions: &[PositionInfo],
    kraken_equity_universe_symbols: &[String],
    kraken_pairs: &[(String, String)],
    _kraken_futures_symbols: &[String],
) -> Vec<String> {
    let mut raw = std::collections::HashSet::new();

    for pos in kr_positions {
        let asset_id_upper = pos.asset_id.to_ascii_uppercase();
        let is_equity_position = pos.asset_class.eq_ignore_ascii_case("stock")
            || asset_id_upper.starts_with("EQUITY_BALANCE:")
            || asset_id_upper.contains(".EQ");
        if !is_equity_position {
            continue;
        }
        raw.insert(pos.symbol.clone());
        if let Some(asset) = pos.asset_id.strip_prefix("equity_balance:") {
            raw.insert(asset.to_string());
        }
    }

    raw.extend(kraken_equity_universe_symbols.iter().cloned());

    for (pair, display) in kraken_pairs {
        if let Some(equity) = kraken_xstock_fundamental_symbol(pair, display) {
            raw.insert(equity);
        }
    }

    normalize_sec_scrape_symbols_priority_order(std::collections::HashSet::new(), raw)
}

#[cfg(test)]
#[path = "style_scope/arc_cache_tests.rs"]
mod arc_cache_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sec_scrape_symbol_normalizes_kraken_xstock_suffixes() {
        assert_eq!(normalize_sec_scrape_symbol("WOK.EQ"), Some("WOK".into()));
        assert_eq!(normalize_sec_scrape_symbol("baby.eq"), None);
        assert_eq!(normalize_sec_scrape_symbol("AAPL"), Some("AAPL".into()));
        assert_eq!(normalize_sec_scrape_symbol("BABY"), None);
        assert_eq!(normalize_sec_scrape_symbol("BTC/USD"), None);
        assert_eq!(normalize_sec_scrape_symbol("TOOLONG.EQ"), None);
    }

    #[test]
    fn sec_scrape_scope_keeps_active_symbols_before_broad_universe() {
        let priority = ["WOK.EQ", "POM"].into_iter().map(str::to_string).collect();
        let broad = ["AAPL", "WOK", "ZZZ", "BABY.EQ"]
            .into_iter()
            .map(str::to_string)
            .collect();

        assert_eq!(
            normalize_sec_scrape_symbols_priority_order(priority, broad),
            vec![
                "POM".to_string(),
                "WOK".to_string(),
                "AAPL".to_string(),
                "ZZZ".to_string()
            ]
        );
    }

    #[test]
    fn kraken_sec_scope_uses_equities_not_spot_or_futures_pairs() {
        let positions = vec![
            PositionInfo {
                symbol: "WOK".to_string(),
                qty: 1.0,
                qty_available: 1.0,
                side: "long".to_string(),
                avg_entry_price: 0.0,
                market_value: 0.0,
                unrealized_pl: 0.0,
                asset_class: "stock".to_string(),
                asset_id: "equity_balance:WOK.EQ".to_string(),
            },
            PositionInfo {
                symbol: "BTC/USD".to_string(),
                qty: 1.0,
                qty_available: 1.0,
                side: "long".to_string(),
                avg_entry_price: 0.0,
                market_value: 0.0,
                unrealized_pl: 0.0,
                asset_class: "crypto".to_string(),
                asset_id: "margin:btc".to_string(),
            },
        ];
        let catalog = vec!["BABY.EQ".to_string(), "AAPL".to_string()];
        let spot_pairs = vec![
            ("ABEUR".to_string(), "ABE/EUR".to_string()),
            ("ETHUSD".to_string(), "ETH/USD".to_string()),
            ("HRTX.EQUSD".to_string(), "HRTX.EQ/USD".to_string()),
        ];
        let futures = vec!["PI_XBTUSD".to_string(), "PF_ETHUSD".to_string()];

        assert_eq!(
            kraken_sec_scrape_scope_symbols(&positions, &catalog, &spot_pairs, &futures),
            vec!["AAPL".to_string(), "HRTX".to_string(), "WOK".to_string(),]
        );

        // The membership set must cover everything the scrape targeted, or the
        // scraper fetches filings the filter then throws away. It legitimately
        // holds *more* (crypto/futures pairs are in Kraken's scope for the
        // non-equity views); what it may never do is miss an equity.
        let membership =
            kraken_scope_membership_symbols(&positions, &catalog, &spot_pairs, &futures);
        for equity in kraken_sec_scrape_scope_symbols(&positions, &catalog, &spot_pairs, &futures) {
            assert!(
                membership.contains(&equity),
                "{equity} was scraped under Kraken scope but is not in scope membership"
            );
        }
    }

    #[test]
    fn sec_history_query_parses_symbols_and_ignores_free_text() {
        // A symbol search must reach the DB — the capped snapshot spans weeks
        // and a few hundred tickers of a corpus going back to 1994, so anything
        // outside it was invisible no matter how many filings were stored.
        assert_eq!(sec_history_query_tickers("SMCI"), vec!["SMCI".to_string()]);
        assert_eq!(
            sec_history_query_tickers(" smci "),
            vec!["SMCI".to_string()]
        );
        assert_eq!(
            sec_history_query_tickers("SMCI, NVDA"),
            vec!["NVDA".to_string(), "SMCI".to_string()]
        );
        // Class suffixes are real tickers.
        assert_eq!(sec_history_query_tickers("BRK.B"), vec!["BRK.B".to_string()]);

        // The box also accepts company / sector / industry text. Firing a
        // per-ticker query for those is a guaranteed miss, so they must not
        // trigger one — the snapshot filter still handles them.
        assert!(sec_history_query_tickers("").is_empty());
        assert!(sec_history_query_tickers("technology").is_empty());
        assert!(sec_history_query_tickers("WESTERN DIGITAL CORP").is_empty());
        assert!(sec_history_query_tickers("12345").is_empty());

        // Bounded: each ticker is its own query and its own snapshot.
        let many = "AA BB CC DD EE FF GG HH II JJ KK";
        assert_eq!(sec_history_query_tickers(many).len(), 8);

        // Deduped and stable, so an unchanged query never re-dispatches.
        assert_eq!(
            sec_history_query_tickers("SMCI smci SMCI"),
            vec!["SMCI".to_string()]
        );
    }

    #[test]
    fn kraken_scope_membership_includes_the_equity_universe() {
        // Regression: the universe was absent and xStock equities were derived
        // only from `kraken_pairs`, which needs a `.EQ` suffix the public
        // AssetPairs feed does not carry. Kraken scope collapsed to crypto +
        // futures, sharing no tickers with the equity-keyed fundamentals or
        // filing tables — hence "Broker scope → Kraken (0 fundamentals in
        // scope)" for a whole session.
        let catalog = vec!["BABY.EQ".to_string(), "AAPL".to_string()];
        let spot_pairs = vec![
            ("XXBTZUSD".to_string(), "XBT/USD".to_string()),
            ("ETHUSD".to_string(), "ETH/USD".to_string()),
        ];
        let futures = vec!["PI_XBTUSD".to_string()];

        let syms = kraken_scope_membership_symbols(&[], &catalog, &spot_pairs, &futures);

        // Bare, uppercase, `.EQ`-stripped — the format `Fundamentals::symbol`
        // and `SecFiling::ticker` use, so `set.contains(...)` actually hits.
        assert!(syms.contains("AAPL"));
        assert!(syms.contains("BABY"));
        assert!(!syms.contains("BABY.EQ"));

        // Crypto and futures still belong to Kraken scope for the non-equity views.
        assert!(syms.contains("ETHUSD"));

        // Without the universe the set is equity-free, which is the bug.
        let without_universe = kraken_scope_membership_symbols(&[], &[], &spot_pairs, &futures);
        assert!(!without_universe.contains("AAPL"));
        assert!(!without_universe.contains("BABY"));
    }

    #[test]
    fn sec_filing_form_filters_are_checkbox_exact() {
        let filter_types: &[&str] = &["4", "13F", "DEF 14A", "S-1", "10-K", "10-Q", "8-K"];
        assert!(sec_filing_form_matches_filters(
            "10-Q",
            &[true; 7],
            filter_types
        ));
        assert!(!sec_filing_form_matches_filters(
            "10-Q",
            &[false; 7],
            filter_types
        ));

        let mut form4_only = [false; 7];
        form4_only[0] = true;
        assert!(sec_filing_form_matches_filters(
            "4",
            &form4_only,
            filter_types
        ));
        assert!(!sec_filing_form_matches_filters(
            "10-K",
            &form4_only,
            filter_types
        ));

        let mut proxy_only = [false; 7];
        proxy_only[2] = true;
        assert!(sec_filing_form_matches_filters(
            "DEF 14A",
            &proxy_only,
            filter_types
        ));
        assert!(!sec_filing_form_matches_filters(
            "13F-HR",
            &proxy_only,
            filter_types
        ));
    }
}
