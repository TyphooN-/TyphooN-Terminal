use super::*;
use crate::app::app_runtime_support::{
    should_auto_start_background_scope_scrape, should_auto_start_kraken_fundamentals_scrape,
};
use typhoon_engine::broker::kraken::{KrakenBroker, KrakenEquityMarket};

/// Pre-computed Kraken equities universe bundle, produced off the render thread by
/// `compute_kraken_universe_digest` and applied by `tick_kraken_universe_digest`.
pub(crate) struct KrakenUniverseDigest {
    pub no_overnight: std::collections::HashSet<String>,
    pub tokenized: Vec<String>,
    pub symbols: Vec<String>,
    pub universe_set: std::collections::HashSet<String>,
    pub names: std::collections::HashMap<String, String>,
    pub regulatory_alerts: Option<
        std::collections::HashMap<
            String,
            Vec<typhoon_engine::core::regulatory_alerts::RegulatoryAlert>,
        >,
    >,
}

/// Build the universe bundle from the raw iapi catalog. Pure CPU (string trims,
/// uppercasing, sort/dedup over ~13k rows) plus the one-time Reg SHO refresh —
/// run on a worker, never the render thread. Mirrors the old synchronous handler.
fn compute_kraken_universe_digest(
    markets: Vec<KrakenEquityMarket>,
    cache: Option<&SqliteCache>,
) -> KrakenUniverseDigest {
    // Symbols the iapi catalog marks as not overnight-tradeable (Some(false)).
    // Unknown/None defaults to overnight-enabled, so only explicit opt-outs land here.
    let no_overnight: std::collections::HashSet<String> = markets
        .iter()
        .filter(|market| market.overnight_trading == Some(false))
        .map(|market| market.symbol.trim_end_matches(".EQ").to_ascii_uppercase())
        .filter(|symbol| !symbol.is_empty())
        .collect();

    // WS-tokenized subset (real `{SYM}x/USD` WS pairs) — scopes the WS OHLC sweep.
    let mut tokenized: Vec<String> = markets
        .iter()
        .filter(|market| {
            market.tokenized
                && market.tradable
                && market.status.as_deref().unwrap_or("active") != "disabled"
                && market.instrument_status.as_deref().unwrap_or("enabled") != "disabled"
        })
        .map(|market| market.symbol.trim_end_matches(".EQ").to_ascii_uppercase())
        .filter(|symbol| !symbol.is_empty())
        .collect();
    tokenized.sort();
    tokenized.dedup();

    let mut names: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut symbols: Vec<String> = markets
        .into_iter()
        .filter(|market| {
            market.tradable
                && market.status.as_deref().unwrap_or("active") != "disabled"
                && market.instrument_status.as_deref().unwrap_or("enabled") != "disabled"
        })
        .map(|market| {
            let bare = market.symbol.trim_end_matches(".EQ").to_ascii_uppercase();
            if let Some(n) = market.name.as_ref() {
                if !n.trim().is_empty() {
                    names.insert(bare.clone(), n.trim().to_string());
                }
            }
            bare
        })
        .filter(|symbol| !symbol.is_empty())
        .collect();
    symbols.sort();
    symbols.dedup();
    let universe_set: std::collections::HashSet<String> = symbols.iter().cloned().collect();

    // Refresh the Reg SHO list now that xStock symbols are available, then rebuild
    // the in-memory map. CRITICAL: the network fetch runs with NO DB lock held —
    // `refresh_regsho_threshold_alerts(&conn)` held the write connection across the
    // HTTP round-trip, stalling every bar-sync writer for its duration. Here the
    // lock is taken only for the quick cached-as_of read, the (conditional) write,
    // and the final map read; the fetch happens between them, unlocked.
    let regulatory_alerts = cache.and_then(|cache| {
        use typhoon_engine::core::regulatory_alerts as ra;
        // Quick read: what as_of do we already have? (lock held for this stmt only)
        let cached_as_of = {
            let conn = cache.connection().ok()?;
            ra::get_latest_regsho_as_of(&conn).ok().flatten()
        };
        // Network fetch — NO DB lock held. Build a throwaway current-thread runtime
        // for the one async call. On any failure we skip the write but still rebuild
        // the map from whatever is already cached (matches the old resilience).
        let fetched = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .ok()
            .and_then(|rt| rt.block_on(ra::fetch_regsho_threshold_entries()).ok());
        if let Some((remote_as_of, rows)) = fetched {
            // Smart refresh: only write when the remote file is newer (lock held
            // only for the DELETE+INSERT, not the fetch above).
            if cached_as_of.as_deref() != Some(remote_as_of.as_str()) {
                if let Ok(conn) = cache.connection() {
                    let _ = ra::replace_regsho_threshold_alerts(&conn, &remote_as_of, &rows);
                }
            }
        }
        // Rebuild the in-memory map from the DB (lock held for this read only).
        let conn = cache.connection().ok()?;
        let alerts = ra::get_regulatory_alerts(&conn).ok()?;
        Some(ra::regulatory_alert_map(&alerts))
    });

    KrakenUniverseDigest {
        no_overnight,
        tokenized,
        symbols,
        universe_set,
        names,
        regulatory_alerts,
    }
}

fn kraken_positions_with_balance_equities(
    mut positions: Vec<PositionInfo>,
    balances: &[(String, f64)],
) -> Vec<PositionInfo> {
    // Kraken xStocks arrive as cash-account balances (`WOK.EQ`), not REST
    // OpenPositions rows. If periodic Balance polls only refresh
    // `kraken_balances`, the right-panel position row stays timestamp-stale
    // and can keep old quantities after live fills. Treat every fresh balance
    // snapshot as the authoritative xStock position source while preserving
    // any non-balance positions already reported by OpenPositions.
    positions.retain(|pos| !pos.asset_id.starts_with("equity_balance:"));
    positions.extend(KrakenBroker::equity_position_summaries_from_balances(
        balances,
    ));
    positions.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    positions
}

impl TyphooNApp {
    pub(super) fn request_missing_kraken_catalogs(&mut self) {
        if self.cache_loaded
            && self.kraken_enabled
            && self.kraken_any_spot_scrape_enabled()
            && self.kraken_pairs.is_empty()
            && !self.kraken_pairs_requested
        {
            let _ = self.broker_tx.send(BrokerCmd::KrakenGetPairs);
            self.kraken_pairs_requested = true;
        }
        let now_ts = chrono::Utc::now().timestamp();
        if self.cache_loaded
            && self.kraken_enabled
            && self.kraken_scrape_xstocks
            && self.kraken_equity_universe_symbols.is_empty()
            && (!self.kraken_equity_universe_requested
                || now_ts >= self.kraken_equity_universe_retry_after_ts)
        {
            let _ = self.broker_tx.send(BrokerCmd::KrakenFetchEquityUniverse);
            self.kraken_equity_universe_requested = true;
            self.kraken_equity_universe_retry_after_ts = now_ts + 120;
        }
        if self.cache_loaded
            && self.kraken_enabled
            && self.kraken_scrape_futures
            && self.kraken_futures_symbols.is_empty()
            && !self.kraken_futures_requested
        {
            let _ = self.broker_tx.send(BrokerCmd::KrakenFuturesGetInstruments);
            self.kraken_futures_requested = true;
        }
    }

    pub(super) fn refresh_active_crypto_chart_if_due(&mut self, now_instant: std::time::Instant) {
        // Periodic crypto bar refresh (~60s).
        // Uses Kraken (free, no auth) as primary source, Alpaca as fallback
        if now_instant.duration_since(self.periodic_crypto_last_refresh)
            >= std::time::Duration::from_secs(60)
            && self.cache_loaded
        {
            self.periodic_crypto_last_refresh = now_instant;
            if let Some(chart) = self.charts.get(self.active_tab) {
                let sym = chart.symbol.clone();
                let bare = sym.split(':').last().unwrap_or(&sym).to_string();
                let crypto_bases = [
                    "BTC", "ETH", "SOL", "DOGE", "XRP", "ADA", "LTC", "LINK", "AVAX", "DOT", "XMR",
                    "ZEC", "DASH",
                ];
                let su = bare.to_uppercase();
                let is_crypto = crypto_bases
                    .iter()
                    .any(|b| su.starts_with(b) && su.ends_with("USD"));
                if is_crypto {
                    let tf_label = chart.timeframe.cache_suffix().to_string(); // "1Month" not "MN1"
                    if self.sync_timeframe_enabled(&tf_label) {
                        let tf_minutes = chart.timeframe.minutes();
                        // Fetch from Kraken (free, no auth, works on weekends)
                        // Fetch chart's TF + all lower TFs for gap fill and forming bar synthesis
                        let mut timeframes = vec![tf_label.clone()];
                        let all_tfs = [
                            "1Week", "1Day", "4Hour", "1Hour", "30Min", "15Min", "5Min", "1Min",
                        ];
                        for ltf in &all_tfs {
                            let ltf_min: u32 = match *ltf {
                                "1Week" => 10080,
                                "1Day" => 1440,
                                "4Hour" => 240,
                                "1Hour" => 60,
                                "30Min" => 30,
                                "15Min" => 15,
                                "5Min" => 5,
                                _ => 1,
                            };
                            if ltf_min < tf_minutes && tf_label != *ltf {
                                timeframes.push(ltf.to_string());
                                break; // just the next lower TF for forming bar (Kraken has rate limits)
                            }
                        }
                        // Server/standalone: queue through the scheduler path so periodic chart
                        // refreshes respect pending slots, no-data tombstones, and persisted
                        // backfill-complete markers instead of forcing full-history attempts.
                        let timeframes =
                            self.filtered_sync_timeframes(timeframes.iter().map(|tf| tf.as_str()));
                        if self.kraken_spot_symbol_scrape_enabled(&bare) {
                            for tf in timeframes {
                                self.queue_kraken_fetch(&bare, &tf);
                            }
                        }
                    }
                }
            }
        }
    }

    pub(super) fn handle_kraken_equity_universe(
        &mut self,
        markets: Vec<KrakenEquityMarket>,
    ) -> bool {
        if !self.kraken_enabled {
            return false;
        }
        // Mark requested immediately so the universe isn't re-requested while the
        // digest runs on the worker. The re-request gate also checks
        // `universe_symbols.is_empty()` (still true until the digest applies), so we
        // must push the retry timer out rather than zero it — zeroing here would let
        // the gate fire a spurious re-fetch during the in-flight window. If the
        // worker never delivers, the 120s timer is a self-heal fallback.
        self.kraken_equity_universe_requested = true;
        self.kraken_equity_universe_retry_after_ts = chrono::Utc::now().timestamp() + 120;

        // Digest the full ~13k-symbol catalog (filter/map/sort/dedup) AND the Reg
        // SHO refresh (a network/DB `block_on`) OFF the render thread — together
        // they ran ~290ms inside the broker-message drain. The worker produces a
        // bundle; `tick_kraken_universe_digest` applies it cheaply (O(1) moves).
        // A new universe message simply replaces the receiver — last apply wins.
        let (tx, rx) = std::sync::mpsc::channel();
        self.kraken_universe_digest_rx = Some(rx);
        let cache = self.cache.clone();
        let rt_handle = self.rt_handle.clone();
        rt_handle.spawn_blocking(move || {
            let digest = compute_kraken_universe_digest(markets, cache.as_deref());
            let _ = tx.send(digest);
        });

        // Refill is signalled when the digest is applied, not now.
        false
    }

    /// Apply a completed off-thread Kraken universe digest (see
    /// `handle_kraken_equity_universe`). Cheap: moves the prebuilt collections into
    /// place, bumps `bg_rev`, and kicks the same follow-ups the synchronous handler
    /// used to run inline (WS OHLC start, deferred scope scrapes, sync-slot refill).
    pub(crate) fn tick_kraken_universe_digest(&mut self) {
        let Some(rx) = self.kraken_universe_digest_rx.as_ref() else {
            return;
        };
        let digest = match rx.try_recv() {
            Ok(digest) => digest,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.kraken_universe_digest_rx = None;
                return;
            }
        };
        self.kraken_universe_digest_rx = None;

        self.kraken_equity_no_overnight = digest.no_overnight;
        self.kraken_equity_tokenized_symbols = digest.tokenized;
        self.kraken_equity_universe_set = digest.universe_set;
        self.kraken_equity_universe_symbols = digest.symbols;
        self.kraken_equity_names = digest.names;
        self.rebuild_chart_company_name_catalog();
        if let Some(map) = digest.regulatory_alerts {
            self.bg.regulatory_alerts_by_symbol = map;
        }
        // Universe is live now → clear the retry timer (matches the old handler's
        // post-load state; the gate is already false via non-empty symbols).
        self.kraken_equity_universe_retry_after_ts = 0;
        self.bg_rev = self.bg_rev.wrapping_add(1);
        self.log.push_back(LogEntry::info(format!(
            "Kraken equities universe loaded: {} tradable symbols ({} WS-tokenized)",
            self.kraken_equity_universe_symbols.len(),
            self.kraken_equity_tokenized_symbols.len()
        )));
        self.maybe_start_kraken_ws_ohlc();
        self.start_deferred_scope_scrapes_after_kraken_universe();
        // The synchronous handler returned `true` so the drain refilled sync slots;
        // do it here now that the universe symbols are live.
        self.refill_market_data_sync_slots();
    }

    pub(super) fn handle_kraken_equity_bars(
        &mut self,
        symbol: String,
        timeframe: String,
        count: usize,
    ) -> bool {
        let symbol = symbol
            .replace('/', "")
            .trim_end_matches(".EQ")
            .to_ascii_uppercase();
        let timeframe = normalize_sync_timeframe_key(&timeframe)
            .unwrap_or(timeframe.as_str())
            .to_string();
        let pending_key = format!("equity:{symbol}:{timeframe}");
        self.pending_kraken_fetches
            .retain(|key| key != &pending_key);
        if count == 0 {
            self.unresolvable_mark(
                "kraken-equities",
                &symbol,
                &timeframe,
                "Kraken internal equities history returned no bars",
            );
            tracing::debug!("Kraken equities: no bars for {} {}", symbol, timeframe);
        } else {
            self.note_cached_sync_success("kraken-equities", &symbol, &timeframe, count);
            tracing::debug!(
                "Kraken equities: cached {} bars for {} {}",
                count,
                symbol,
                timeframe
            );
        }
        true
    }

    pub(super) fn handle_kraken_equity_history_error(
        &mut self,
        symbol: String,
        timeframe: String,
        error: String,
    ) -> bool {
        let symbol = symbol
            .replace('/', "")
            .trim_end_matches(".EQ")
            .to_ascii_uppercase();
        let timeframe = normalize_sync_timeframe_key(&timeframe)
            .unwrap_or(timeframe.as_str())
            .to_string();
        let pending_key = format!("equity:{symbol}:{timeframe}");
        self.pending_kraken_fetches
            .retain(|key| key != &pending_key);
        let iapi_rl_prefix = typhoon_engine::broker::kraken::IAPI_RATE_LIMITED_ERR_PREFIX;
        if error.contains("No data") || error.contains("no data") {
            self.unresolvable_mark("kraken-equities", &symbol, &timeframe, &error);
            tracing::debug!("Kraken equities: no bars for {} {}", symbol, timeframe);
        } else if error.starts_with(iapi_rl_prefix) {
            // Engine-side iapi gate already short-circuited the round-trip; this
            // branch fires once per queued fetch as the broker thread drains them.
            // Arm the queue-side pause to stop NEW dispatches and silence the
            // per-fetch errors — the first 429 produced a single tracing::warn
            // at the engine.
            let now = chrono::Utc::now().timestamp();
            let pause = typhoon_engine::broker::kraken::iapi_rate_limited_for_secs().unwrap_or(60);
            if now + pause > self.kraken_equities_sync_pause_until_ts {
                self.kraken_equities_sync_pause_until_ts = now + pause;
                self.kraken_equities_sync_pause_reason = error.clone();
            }
            tracing::debug!(
                "Kraken equities: {} {} skipped — iapi back-off ({}s left)",
                symbol,
                timeframe,
                pause
            );
        } else if error.contains("HTTP 500") && error.contains("Internal error") {
            // Per-symbol Kraken iapi hiccup: do not pause the entire equities lane.
            self.mark_fetch_queued("kraken-equities", &symbol, &timeframe);
            tracing::debug!(
                "Kraken equities: {} {} skipped — iapi HTTP 500/Internal error (per-symbol cooldown)",
                symbol,
                timeframe
            );
        } else {
            self.log.push_back(LogEntry::err(error));
        }
        true
    }

    pub(super) fn handle_kraken_balances(&mut self, balances: Vec<(String, f64)>) {
        if !self.kraken_enabled {
            return;
        }
        let had_balance_equity_positions = self
            .kr_positions
            .iter()
            .any(|pos| pos.asset_id.starts_with("equity_balance:"));
        self.kraken_balance_assets_by_display = balances
            .iter()
            .filter(|(asset, qty)| {
                qty.is_finite() && *qty > 0.0 && !Self::kraken_is_cash_balance_asset(asset)
            })
            .map(|(asset, _)| {
                Self::kraken_display_asset(asset)
                    .trim_end_matches(".EQ")
                    .to_ascii_uppercase()
            })
            .collect();
        self.kraken_balances = balances;
        let next_positions = kraken_positions_with_balance_equities(
            std::mem::take(&mut self.kr_positions),
            &self.kraken_balances,
        );
        let has_balance_equity_positions = next_positions
            .iter()
            .any(|pos| pos.asset_id.starts_with("equity_balance:"));
        if had_balance_equity_positions || has_balance_equity_positions {
            self.positions_last_update_ts = chrono::Utc::now().timestamp();
            self.kr_positions = next_positions;
            self.kr_positions_by_symbol = self
                .kr_positions
                .iter()
                .map(|p| {
                    let key = bare_symbol_from_key(&p.symbol)
                        .replace("/", "")
                        .trim_end_matches(".EQ")
                        .trim_end_matches(".eq")
                        .to_ascii_uppercase();
                    (key, p.clone())
                })
                .collect();
            self.kr_position_asset_tails = self
                .kr_positions
                .iter()
                .flat_map(|p| {
                    let mut ks = vec![];
                    let k = bare_symbol_from_key(&p.symbol)
                        .replace("/", "")
                        .trim_end_matches(".EQ")
                        .trim_end_matches(".eq")
                        .to_ascii_uppercase();
                    ks.push(k.clone());
                    if !p.asset_id.is_empty() {
                        let a = bare_symbol_from_key(&p.asset_id)
                            .replace("/", "")
                            .trim_end_matches(".EQ")
                            .trim_end_matches(".eq")
                            .to_ascii_uppercase();
                        if a != k {
                            ks.push(a);
                        }
                    }
                    ks
                })
                .collect();
            if let Ok(json) = serde_json::to_string(&self.kr_positions) {
                self.put_kv_dedup("broker:kr_positions", &json);
            }
        } else {
            self.kr_positions = next_positions;
            self.kr_positions_by_symbol = self
                .kr_positions
                .iter()
                .map(|p| {
                    let key = bare_symbol_from_key(&p.symbol)
                        .replace("/", "")
                        .trim_end_matches(".EQ")
                        .trim_end_matches(".eq")
                        .to_ascii_uppercase();
                    (key, p.clone())
                })
                .collect();
            self.kr_position_asset_tails = self
                .kr_positions
                .iter()
                .flat_map(|p| {
                    let mut ks = vec![];
                    let k = bare_symbol_from_key(&p.symbol)
                        .replace("/", "")
                        .trim_end_matches(".EQ")
                        .trim_end_matches(".eq")
                        .to_ascii_uppercase();
                    ks.push(k.clone());
                    if !p.asset_id.is_empty() {
                        let a = bare_symbol_from_key(&p.asset_id)
                            .replace("/", "")
                            .trim_end_matches(".EQ")
                            .trim_end_matches(".eq")
                            .to_ascii_uppercase();
                        if a != k {
                            ks.push(a);
                        }
                    }
                    ks
                })
                .collect();
        }
        self.refresh_kraken_position_costs();
        for c in &mut self.charts {
            c.cached_trade_overlay_frame = 0;
        }
        let active_tf = self
            .charts
            .get(self.active_tab)
            .map(|chart| chart.timeframe.cache_suffix())
            .unwrap_or("1Day");
        let mut queued = 0usize;
        let balance_pairs: Vec<(String, bool)> = self
            .kraken_balances
            .iter()
            .filter(|(asset, qty)| {
                qty.is_finite() && *qty > 0.0 && !Self::kraken_is_cash_balance_asset(asset)
            })
            .map(|(asset, _)| {
                (
                    Self::kraken_spot_pair_for_balance_asset(asset),
                    Self::kraken_display_asset(asset).ends_with(".EQ"),
                )
            })
            .collect();
        for (pair, is_equity) in balance_pairs {
            if is_equity {
                self.dispatch_kraken_equity_ticker(&pair);
                let mut queued_equity_tf = false;
                queued_equity_tf |= self.queue_kraken_equity_fetch(&pair, active_tf);
                queued_equity_tf |= self.queue_alpaca_fetch(&pair, active_tf);
                if queued_equity_tf {
                    queued += 1;
                }
                if active_tf != "1Day" {
                    let mut queued_equity_day = false;
                    queued_equity_day |= self.queue_kraken_equity_fetch(&pair, "1Day");
                    queued_equity_day |= self.queue_alpaca_fetch(&pair, "1Day");
                    if queued_equity_day {
                        queued += 1;
                    }
                }
                continue;
            }
            if self.queue_kraken_fetch(&pair, active_tf) {
                queued += 1;
            }
            if active_tf != "1Day" && self.queue_kraken_fetch(&pair, "1Day") {
                queued += 1;
            }
        }
        if std::time::Instant::now().duration_since(self.kraken_trades_last_fetch)
            >= std::time::Duration::from_secs(KRAKEN_TRADES_REST_REFRESH_SECS)
        {
            let _ = self.broker_tx.send(BrokerCmd::KrakenFetchTrades);
        }
        if queued > 0 {
            self.log.push_back(LogEntry::info(format!(
                "Kraken: {} assets with balance; queued {} owned-symbol bar fetches",
                self.kraken_balances.len(),
                queued
            )));
        } else {
            tracing::debug!(
                "Kraken balances tick: {} assets, 0 fetches queued (all up-to-date)",
                self.kraken_balances.len()
            );
        }
    }

    pub(super) fn handle_kraken_pairs(&mut self, pairs: Vec<(String, String)>) {
        self.log.push_back(LogEntry::info(format!(
            "Kraken: {} tradeable pairs loaded",
            pairs.len()
        )));
        self.kraken_pairs_requested = true;
        self.kraken_pairs = pairs;
        self.kraken_pairs_normalized.clear();
        self.kraken_pairs_normalized
            .reserve(self.kraken_pairs.len() * 2);
        self.kraken_equity_pair_by_base.clear();
        for (pair_name, display_name) in &self.kraken_pairs {
            let pair_norm = typhoon_engine::core::kraken::normalize_pair_symbol(pair_name);
            if !pair_norm.is_empty() {
                self.kraken_pairs_normalized
                    .insert(pair_norm.to_ascii_uppercase());
            }
            let display_norm = typhoon_engine::core::kraken::normalize_pair_symbol(display_name);
            if !display_norm.is_empty() {
                self.kraken_pairs_normalized
                    .insert(display_norm.to_ascii_uppercase());
            }
            // Build O(1) base -> candidate for kraken_resolved_equity_pair
            let candidate = if display_name.trim().is_empty() {
                pair_name
            } else {
                display_name
            };
            let base = TyphooNApp::kraken_pair_base_ticker(candidate);
            if !base.is_empty() {
                self.kraken_equity_pair_by_base
                    .insert(base, candidate.to_string());
            }
        }
        self.refill_market_data_sync_slots();
        self.maybe_start_kraken_ws_ohlc();
    }

    pub(super) fn handle_kraken_futures_instruments(&mut self, symbols: Vec<String>) {
        self.log.push_back(LogEntry::info(format!(
            "Kraken Futures: {} tradeable instruments loaded",
            symbols.len()
        )));
        self.kraken_futures_requested = true;
        self.kraken_futures_symbols = symbols;
        self.refill_market_data_sync_slots();
    }

    fn start_deferred_scope_scrapes_after_kraken_universe(&mut self) {
        if self.auto_sec_scrape_deferred && !self.scrape_sec_running {
            let symbols = self.sec_scrape_scope_symbols();
            let symbol_count = symbols.len();
            if should_auto_start_background_scope_scrape(self.broker_scope, symbol_count) {
                let db_path = cache_db_path();
                let _ = self
                    .broker_tx
                    .send(BrokerCmd::SecScrape { db_path, symbols });
                self.auto_sec_scrape_deferred = false;
                self.scrape_sec_running = true;
                self.scrape_sec_last_msg = format!(
                    "scraping Scope {} ({} symbols)...",
                    self.broker_scope_label(),
                    symbol_count
                );
                self.log.push_back(LogEntry::info(format!(
                    "SEC EDGAR deferred scrape started for Scope {} ({} symbols)...",
                    self.broker_scope_label(),
                    symbol_count
                )));
            } else if symbol_count > 0 {
                self.auto_sec_scrape_deferred = false;
                self.log.push_back(LogEntry::info(format!(
                    "SEC EDGAR deferred auto-scrape skipped for broad Scope {} ({} symbols); use manual SEC scrape for full-universe backfill",
                    self.broker_scope_label(),
                    symbol_count
                )));
            }
        }

        if self.auto_fundamentals_deferred && !self.auto_fundamentals_started {
            if !should_auto_start_kraken_fundamentals_scrape(
                self.kraken_equity_universe_symbols.len(),
            ) {
                self.auto_fundamentals_deferred = false;
                self.auto_fundamentals_started = false;
                self.log.push_back(LogEntry::info(format!(
                    "Fundamentals deferred auto-scrape skipped for broad Kraken xStocks universe ({} symbols); use manual Fundamentals scrape for full-universe backfill",
                    self.kraken_equity_universe_symbols.len()
                )));
            } else {
                let db_path = cache_db_path();
                let _ = self.broker_tx.send(BrokerCmd::FundamentalsScrape {
                    db_path,
                    use_alpaca: self.fund_source_alpaca,
                    use_kraken: self.fund_source_kraken,
                    kraken_equity_symbols: self.kraken_equity_universe_symbols.clone(),
                    force: false,
                });
                self.auto_fundamentals_deferred = false;
                self.auto_fundamentals_started = true;
                self.log.push_back(LogEntry::info(
                    "Fundamentals deferred scrape started for selected source universes...",
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn position(symbol: &str, asset_id: &str, qty: f64) -> PositionInfo {
        PositionInfo {
            symbol: symbol.to_string(),
            qty,
            qty_available: qty,
            side: "long".to_string(),
            avg_entry_price: 0.0,
            market_value: 0.0,
            unrealized_pl: 0.0,
            asset_class: "stock".to_string(),
            asset_id: asset_id.to_string(),
        }
    }

    #[test]
    fn kraken_balance_snapshot_replaces_stale_xstock_positions() {
        let existing = vec![
            position("WOK", "equity_balance:WOK.EQ", 8174.0),
            position("BTCUSD", "margin:btc", 0.25),
        ];
        let balances = vec![("WOK.EQ".to_string(), 8123.0), ("ZUSD".to_string(), 705.0)];

        let merged = kraken_positions_with_balance_equities(existing, &balances);

        assert_eq!(merged.len(), 2);
        assert!(merged.iter().any(|pos| pos.symbol == "BTCUSD"));
        let wok = merged.iter().find(|pos| pos.symbol == "WOK").unwrap();
        assert_eq!(wok.qty, 8123.0);
        assert_eq!(wok.asset_id, "equity_balance:WOK.EQ");
    }

    #[test]
    fn kraken_balance_snapshot_removes_closed_xstock_positions() {
        let existing = vec![
            position("WOK", "equity_balance:WOK.EQ", 8174.0),
            position("BTCUSD", "margin:btc", 0.25),
        ];
        let balances = vec![("ZUSD".to_string(), 705.0)];

        let merged = kraken_positions_with_balance_equities(existing, &balances);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].symbol, "BTCUSD");
    }
}
