use super::*;
use crate::app::app_runtime_support::{
    should_auto_start_background_scope_scrape, should_auto_start_kraken_fundamentals_scrape,
};
use typhoon_engine::broker::kraken::KrakenEquityMarket;

impl TyphooNApp {
    pub(super) fn handle_kraken_equity_universe(
        &mut self,
        markets: Vec<KrakenEquityMarket>,
    ) -> bool {
        if !self.kraken_enabled {
            return false;
        }

        // Symbols the iapi catalog marks as not overnight-tradeable
        // (Some(false)). Unknown/None defaults to overnight-enabled, so
        // only the explicit opt-outs land here.
        self.kraken_equity_no_overnight = markets
            .iter()
            .filter(|market| market.overnight_trading == Some(false))
            .map(|market| market.symbol.trim_end_matches(".EQ").to_ascii_uppercase())
            .filter(|symbol| !symbol.is_empty())
            .collect();

        // WS-tokenized subset (real `{SYM}x/USD` WS pairs) — scopes the
        // WS OHLC snapshot sweep. The full catalog below still drives
        // the Alpaca/Yahoo breadth lanes and the Merged Sync Status row.
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
        self.kraken_equity_tokenized_symbols = tokenized;

        let mut symbols: Vec<String> = markets
            .into_iter()
            .filter(|market| {
                market.tradable
                    && market.status.as_deref().unwrap_or("active") != "disabled"
                    && market.instrument_status.as_deref().unwrap_or("enabled") != "disabled"
            })
            .map(|market| market.symbol.trim_end_matches(".EQ").to_ascii_uppercase())
            .filter(|symbol| !symbol.is_empty())
            .collect();
        symbols.sort();
        symbols.dedup();
        self.kraken_equity_universe_symbols = symbols;
        self.kraken_equity_universe_requested = true;
        self.kraken_equity_universe_retry_after_ts = 0;
        self.bg_rev = self.bg_rev.wrapping_add(1);
        self.log.push_back(LogEntry::info(format!(
            "Kraken equities universe loaded: {} tradable symbols ({} WS-tokenized)",
            self.kraken_equity_universe_symbols.len(),
            self.kraken_equity_tokenized_symbols.len()
        )));
        self.maybe_start_kraken_ws_ohlc();

        self.start_deferred_scope_scrapes_after_kraken_universe();
        true
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
        self.kraken_balances = balances;
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
