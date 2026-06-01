use super::*;

const SEC_CACHE_HEAVY_SYNC_MIN_REBUILD_INTERVAL: std::time::Duration =
    std::time::Duration::from_millis(750);

impl TyphooNApp {
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
    /// Reduces KV timestamp churn → less LAN sync traffic.
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
        if let Some(ref cache) = self.cache {
            let _ = cache.put_kv(key, json);
        }
        self.kv_write_hashes.insert(key, hash);
        self.kv_write_times.insert(key, now);
    }

    /// Get all "active" symbols: chart tabs + open positions from ticked brokers + watchlist.
    /// Broker scope symbol set for fundamental filtering.
    /// Returns None for `EventSource::All` (no filter applied). Otherwise returns the
    /// uppercase bare-ticker set for the selected broker.
    pub(super) fn broker_scope_symbols(&self) -> Option<std::collections::HashSet<String>> {
        match self.broker_scope {
            EventSource::All => None,
            EventSource::Alpaca => Some(
                self.live_positions
                    .iter()
                    .map(|p| p.symbol.replace('/', "").to_uppercase())
                    .collect(),
            ),
            EventSource::Tasty => Some(
                self.tt_positions
                    .iter()
                    .map(|p| p.symbol.replace('/', "").to_uppercase())
                    .collect(),
            ),
            EventSource::Kraken => Some(self.kraken_scope_symbols()),
            EventSource::Darwinex => {
                if self.darwinex_radar_data.is_empty() {
                    // Specs not loaded yet — don't filter (show all fundamentals)
                    None
                } else {
                    Some(
                        self.darwinex_radar_data
                            .iter()
                            .filter(|(_, _, _, trade_mode, _, _, _, _, _)| *trade_mode != 0)
                            .map(|(sym, _, _, _, _, _, _, _, _)| {
                                sym.split('.').next().unwrap_or(sym.as_str()).to_uppercase()
                            })
                            .collect(),
                    )
                }
            }
            EventSource::Positions => {
                // All symbols with open positions across any broker
                let mut syms = std::collections::HashSet::new();
                for p in &self.live_positions {
                    syms.insert(p.symbol.replace('/', "").to_uppercase());
                }
                for p in &self.tt_positions {
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
        let mut syms = match self.broker_scope_symbols() {
            Some(scope) => scope,
            None => {
                let mut syms = std::collections::HashSet::new();
                if self.fund_source_mt5 && self.darwinex_enabled {
                    if self.darwinex_radar_data.is_empty() {
                        syms.extend(self.bg.all_fundamentals.iter().map(|f| f.symbol.clone()));
                    } else {
                        syms.extend(
                            self.darwinex_radar_data
                                .iter()
                                .filter(|(_, _, _, trade_mode, _, _, _, _, _)| *trade_mode != 0)
                                .map(|(sym, _, _, _, _, _, _, _, _)| {
                                    sym.split('.').next().unwrap_or(sym.as_str()).to_uppercase()
                                }),
                        );
                    }
                }
                if self.fund_source_alpaca && self.alpaca_enabled {
                    syms.extend(
                        self.live_positions
                            .iter()
                            .map(|p| p.symbol.replace('/', "").to_uppercase()),
                    );
                }
                if self.fund_source_tastytrade && self.tastytrade_enabled {
                    syms.extend(
                        self.tt_positions
                            .iter()
                            .map(|p| p.symbol.replace('/', "").to_uppercase()),
                    );
                }
                if self.fund_source_kraken && self.kraken_enabled {
                    syms.extend(self.kraken_scope_symbols());
                }
                syms
            }
        };
        syms.retain(|sym| sec_scrape_scope_symbol_allowed(sym));
        let mut syms: Vec<String> = syms.into_iter().collect();
        syms.sort_unstable();
        syms.dedup();
        syms
    }

    pub(super) fn enabled_news_scope_symbols(&self) -> std::collections::HashSet<String> {
        if let Some(scope) = self.broker_scope_symbols() {
            return scope;
        }

        let mut syms = std::collections::HashSet::new();
        if self.fund_source_mt5 {
            if self.darwinex_radar_data.is_empty() {
                syms.extend(self.bg.all_fundamentals.iter().map(|f| f.symbol.clone()));
            } else {
                syms.extend(
                    self.darwinex_radar_data
                        .iter()
                        .filter(|(_, _, _, trade_mode, _, _, _, _, _)| *trade_mode != 0)
                        .map(|(sym, _, _, _, _, _, _, _, _)| {
                            sym.split('.').next().unwrap_or(sym.as_str()).to_uppercase()
                        }),
                );
            }
        }
        if self.fund_source_alpaca {
            syms.extend(
                self.live_positions
                    .iter()
                    .map(|p| p.symbol.replace('/', "").to_uppercase()),
            );
        }
        if self.fund_source_tastytrade {
            syms.extend(
                self.tt_positions
                    .iter()
                    .map(|p| p.symbol.replace('/', "").to_uppercase()),
            );
        }
        if self.fund_source_kraken {
            syms.extend(self.kraken_scope_symbols());
        }
        if syms.is_empty() {
            syms.extend(self.bg.all_fundamentals.iter().map(|f| f.symbol.clone()));
        }
        syms
    }

    pub(super) fn kraken_scope_symbols(&self) -> std::collections::HashSet<String> {
        let mut syms = std::collections::HashSet::new();
        for p in &self.kr_positions {
            let symbol = normalize_market_data_symbol(&p.symbol)
                .replace('/', "")
                .to_uppercase();
            if !symbol.is_empty() {
                syms.insert(symbol);
            }
        }
        for (pair, display) in &self.kraken_pairs {
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
        for symbol in &self.kraken_futures_symbols {
            let symbol = typhoon_engine::core::kraken_futures::normalize_futures_symbol(symbol)
                .replace('/', "")
                .to_uppercase();
            if !symbol.is_empty() {
                syms.insert(symbol);
            }
        }
        syms
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

        let bg_rev = self.bg_rev;
        let scope = self.broker_scope;

        // Tab counts — (bg_rev, scope)
        let counts_key = {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            bg_rev.hash(&mut h);
            scope.hash(&mut h);
            h.finish()
        };
        let filings_key = {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            bg_rev.hash(&mut h);
            scope.hash(&mut h);
            self.sec_filters.hash(&mut h);
            self.sec_search_query.hash(&mut h);
            self.sec_sort.column.hash(&mut h);
            self.sec_sort.ascending.hash(&mut h);
            h.finish()
        };
        let insiders_key = {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            bg_rev.hash(&mut h);
            scope.hash(&mut h);
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
        if cache_changed
            && !cache_cold
            && self.heavy_sync_in_progress
            && self.sec_cache_last_rebuild.elapsed() < SEC_CACHE_HEAVY_SYNC_MIN_REBUILD_INTERVAL
        {
            return;
        }
        if cache_changed {
            self.sec_cache_last_rebuild = std::time::Instant::now();
        }
        if self.sec_cache_tab_counts_key != Some(counts_key) {
            let (scoped, insider_total) = match &self.cached_scope_syms {
                None => (
                    self.bg.sec_filings.len(),
                    self.bg.insider_trades.values().map(|v| v.len()).sum(),
                ),
                Some(set) => (
                    self.bg
                        .sec_filings
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

        // Filings tab — (bg_rev, scope, filters, query, sort column, sort direction)
        if self.sec_cache_filings_key != Some(filings_key) {
            let filter_types: &[&str] = &["4", "13F", "DEF 14A", "S-1", "10-K", "10-Q", "8-K"];
            let all_on = self.sec_filters.iter().all(|&v| v);
            let none_on = self.sec_filters.iter().all(|&v| !v);
            let show_all = all_on || none_on;
            // Symbol-only search: uppercase query once, compare against ticker (stored upper).
            let search_upper = self.sec_search_query.trim().to_uppercase();
            let has_search = !search_upper.is_empty();
            let filings = &self.bg.sec_filings;

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
            let mut seen: std::collections::HashSet<(String, String, String)> =
                std::collections::HashSet::with_capacity(filings.len());
            let mut idxs: Vec<usize> = Vec::with_capacity(filings.len());
            for (idx, f) in filings.iter().enumerate() {
                // Dedup by (date, ticker, form_type) — tuple key, no per-row format!() alloc.
                let key = (f.filing_date.clone(), f.ticker.clone(), f.form_type.clone());
                if !seen.insert(key) {
                    continue;
                }
                if !show_all {
                    let pass = self.sec_filters.iter().enumerate().any(|(i, &en)| {
                        if !en {
                            return false;
                        }
                        let ft = filter_types.get(i).unwrap_or(&"");
                        if *ft == "4" {
                            f.form_type == "4"
                        } else {
                            f.form_type.contains(ft)
                        }
                    });
                    if !pass {
                        continue;
                    }
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
            self.sec_cache_filings = idxs;
            self.sec_cache_filings_key = Some(filings_key);
        }

        // Insiders tab — (bg_rev, scope, query)
        if self.sec_cache_insiders_key != Some(insiders_key) {
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

        // Timeline tab — (bg_rev, scope)
        if self.sec_cache_timeline_key != Some(timeline_key) {
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
                    let mut chart = ChartState::new(&sym, tf);
                    if let Some(ref cache) = self.cache.clone() {
                        let mut gpu = self.gpu_indicators.take();
                        chart.try_load(Arc::as_ref(cache), &mut self.log, gpu.as_mut());
                        self.gpu_indicators = gpu;
                    }
                    self.charts.push(chart);
                    self.active_tab = self.charts.len() - 1;
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
            SymbolAction::AddWatchlist(sym) => {
                if !self
                    .user_watchlist
                    .iter()
                    .any(|s| s.eq_ignore_ascii_case(&sym))
                {
                    self.user_watchlist.push(sym.clone());
                    self.log
                        .push_back(LogEntry::info(format!("Added {} to watchlist", sym)));
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
                EventSource::All => r.in_alpaca || r.in_darwinex || r.in_tasty || r.in_kraken,
                EventSource::Alpaca => r.in_alpaca,
                EventSource::Darwinex => r.in_darwinex,
                EventSource::Tasty => r.in_tasty,
                EventSource::Kraken => r.in_kraken,
                EventSource::Positions => r.in_alpaca || r.in_darwinex || r.in_tasty || r.in_kraken,
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
            EventSource::All => "ALL",
            EventSource::Alpaca => "ALPACA",
            EventSource::Darwinex => "DARWINEX",
            EventSource::Tasty => "TASTY",
            EventSource::Kraken => "KRAKEN",
            EventSource::Positions => "POSITIONS",
        }
    }

    /// Emit the user's current open positions for `sym_upper` across all
    /// connected brokers (Alpaca `live_positions` + Darwinex/MT5-like
    /// `tt_positions`). Returns an empty string when there is no exposure,
    /// so the caller can unconditionally append. Derives current price from
    /// `market_value / qty.abs()` rather than querying a separate quote
    /// stream — the broker has already marked the position to market.
    pub(super) fn user_position_section(&self, sym_upper: &str) -> String {
        use std::fmt::Write as _;
        let matches_sym =
            |p: &PositionInfo| p.symbol.eq_ignore_ascii_case(sym_upper) && p.qty != 0.0;
        let alpaca: Vec<&PositionInfo> = self
            .live_positions
            .iter()
            .filter(|p| matches_sym(p))
            .collect();
        let tt: Vec<&PositionInfo> = self
            .tt_positions
            .iter()
            .filter(|p| matches_sym(p))
            .collect();
        let kr: Vec<&PositionInfo> = self
            .kr_positions
            .iter()
            .filter(|p| matches_sym(p))
            .collect();
        if alpaca.is_empty() && tt.is_empty() && kr.is_empty() {
            return String::new();
        }

        let mut out = String::new();
        let _ = writeln!(out, "### Current user position in {sym_upper}");
        let _ = writeln!(
            out,
            "*The user holds the following open position(s) in this symbol. When answering questions like \"what do you think of my position?\" treat this as the primary context.*"
        );
        let _ = writeln!(out);

        let emit_lot = |out: &mut String, broker: &str, p: &PositionInfo| {
            let side_upper = if p.side.eq_ignore_ascii_case("short") || p.qty < 0.0 {
                "SHORT"
            } else {
                "LONG"
            };
            let abs_qty = p.qty.abs();
            let current_price = if abs_qty > 0.0 {
                p.market_value.abs() / abs_qty
            } else {
                0.0
            };
            let cost_basis = p.avg_entry_price * abs_qty;
            let unreal_pct = if cost_basis > 0.0 {
                (p.unrealized_pl / cost_basis) * 100.0
            } else {
                0.0
            };
            let sign = if p.unrealized_pl >= 0.0 { "+" } else { "" };
            let _ = writeln!(
                out,
                "- **{broker}** — {side_upper} {abs_qty:.4} @ avg {avg:.4} (current ~{cur:.4}); market value {mv:.2}; unrealized {sign}{pnl:.2} ({sign}{pct:.2}%)",
                side_upper = side_upper,
                abs_qty = abs_qty,
                avg = p.avg_entry_price,
                cur = current_price,
                mv = p.market_value,
                sign = sign,
                pnl = p.unrealized_pl,
                pct = unreal_pct,
            );
        };
        for p in &alpaca {
            emit_lot(&mut out, "Alpaca", p);
        }
        for p in &tt {
            emit_lot(&mut out, "tastytrade", p);
        }
        for p in &kr {
            emit_lot(&mut out, "Kraken", p);
        }
        let _ = writeln!(out);
        out
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
        self.screenshots_last_refresh = chrono::Utc::now().timestamp();
    }
}

fn sec_scrape_scope_symbol_allowed(sym: &str) -> bool {
    let sym = sym.trim();
    !sym.is_empty()
        && sym.len() <= 5
        && !sym.starts_with("__")
        && !sym.contains('/')
        && sym.chars().all(|c| c.is_ascii_alphabetic())
}
