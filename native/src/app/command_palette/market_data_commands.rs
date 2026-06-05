use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_market_data_command(&mut self, cmd_upper: &str) -> bool {
        match cmd_upper {
            "BARDATA" | "FETCH_ALL" | "FULL_HISTORY" => {
                // Download ALL available bars for ALL symbols from ALL connected brokers
                // Collects: chart tab symbols, watchlist symbols, DARWIN position symbols, Alpaca positions
                let all_tfs =
                    self.filtered_sync_timeframes(["1Day", "1Week", "1Hour", "4Hour", "1Month"]);
                let mut symbols: std::collections::HashSet<String> =
                    std::collections::HashSet::new();

                // Chart tab symbols
                for chart in &self.charts {
                    let bare = chart.symbol.split(':').last().unwrap_or("").to_string();
                    if !bare.is_empty() {
                        symbols.insert(bare);
                    }
                }
                // Watchlist symbols
                for sym in &self.user_watchlist {
                    if !sym.is_empty() {
                        symbols.insert(sym.clone());
                    }
                }
                // DARWIN/MT5 position symbols
                for pos in &self.bg.open_positions {
                    let sym = pos.symbol.replace('/', "");
                    if !sym.is_empty() {
                        symbols.insert(sym);
                    }
                }
                // Alpaca position symbols
                for pos in &self.live_positions {
                    if !pos.symbol.is_empty() {
                        symbols.insert(pos.symbol.clone());
                    }
                }
                // tastytrade position symbols
                for pos in &self.tt_positions {
                    if !pos.symbol.is_empty() {
                        symbols.insert(pos.symbol.clone());
                    }
                }
                // Full Alpaca broker universe (12K+ symbols)
                for (sym, _name, _class) in &self.all_broker_assets {
                    symbols.insert(sym.replace('/', "").to_uppercase());
                }
                // Kraken tradeable pairs
                for (pair, _name) in &self.kraken_pairs {
                    symbols.insert(pair.clone());
                }
                // Kraken Futures instruments
                for symbol in &self.kraken_futures_symbols {
                    symbols.insert(symbol.clone());
                }

                if symbols.is_empty() {
                    self.log.push_back(LogEntry::warn(
                        "BARDATA: no symbols to fetch — open charts or add to watchlist first",
                    ));
                } else {
                    let crypto_bases = [
                        "BTC", "ETH", "SOL", "DOGE", "XRP", "ADA", "LTC", "LINK", "AVAX", "DOT",
                        "XMR", "ZEC", "DASH", "UNI", "AAVE", "MATIC", "SHIB", "ATOM", "ALGO",
                        "FTM", "NEAR", "APE", "ARB",
                    ];

                    // Build set of already-cached symbol:TF combos to skip redundant fetches
                    let mut cached_keys: std::collections::HashSet<String> =
                        std::collections::HashSet::new();
                    for (key, bars, _ts) in &self.bg.detailed_stats {
                        if *bars > 0 {
                            // any cached data = don't re-download full history
                            // Normalize: extract bare symbol + TF from cache key
                            let parts: Vec<&str> = key.split(':').collect();
                            if parts.len() >= 2 {
                                let sym_part =
                                    parts[parts.len() - 2].replace('/', "").to_uppercase();
                                let tf_part = parts[parts.len() - 1];
                                cached_keys.insert(format!("{}:{}", sym_part, tf_part));
                            }
                        }
                    }

                    // Partition: uncached first, then partially cached
                    let mut uncached_syms = Vec::new();
                    let mut cached_syms = Vec::new();
                    for sym in &symbols {
                        let su = sym.to_uppercase();
                        let has_any = all_tfs
                            .iter()
                            .any(|tf| cached_keys.contains(&format!("{}:{}", su, tf)));
                        if has_any {
                            cached_syms.push(sym.clone());
                        } else {
                            uncached_syms.push(sym.clone());
                        }
                    }

                    let mut fetched_count = 0;
                    let mut skipped_count = 0;
                    // Process uncached symbols first (highest priority)
                    for sym in uncached_syms.iter().chain(cached_syms.iter()) {
                        let su = sym.to_uppercase();
                        let is_crypto = crypto_bases
                            .iter()
                            .any(|b| su.starts_with(b) && su.ends_with("USD"));
                        let is_kraken_futures =
                            typhoon_engine::core::kraken_futures::is_futures_symbol(&su);

                        // Find which TFs are missing for this symbol
                        let missing_tfs: Vec<String> = all_tfs
                            .iter()
                            .filter(|tf| !cached_keys.contains(&format!("{}:{}", su, tf)))
                            .cloned()
                            .collect();

                        if missing_tfs.is_empty() {
                            skipped_count += 1;
                            continue; // fully cached, skip entirely
                        }

                        if is_kraken_futures {
                            if self.kraken_scrape_futures {
                                for tf in &missing_tfs {
                                    if self.queue_kraken_futures_fetch(&su, tf) {
                                        fetched_count += 1;
                                    }
                                }
                            } else {
                                skipped_count += 1;
                                continue;
                            }
                        } else if is_crypto {
                            // Crypto: use Kraken public market data.
                            // Normalize: remove slashes, uppercase (BTC/USD → BTCUSD)
                            let clean_sym = sym.replace('/', "").to_uppercase();
                            if self.kraken_spot_symbol_scrape_enabled(&clean_sym) {
                                for tf in &missing_tfs {
                                    if self.queue_kraken_fetch(&clean_sym, tf) {
                                        fetched_count += 1;
                                    }
                                }
                            }
                        } else if self.broker_connected {
                            // Stocks/Forex/CFDs: use Alpaca (AlpacaFetchBars, with MT5 priority + full-history first fetch)
                            for tf in &missing_tfs {
                                if self.queue_alpaca_fetch(&sym, tf) {
                                    fetched_count += 1;
                                }
                            }
                        }

                        // tastytrade: bars + option chain (if connected and not already cached)
                        if self.tt_connected {
                            for tf in &missing_tfs {
                                if self.queue_tastytrade_fetch(&sym, tf) {
                                    fetched_count += 1;
                                }
                            }
                            let _ = self.broker_tx.send(BrokerCmd::TastytradeOptionChain {
                                symbol: sym.clone(),
                            });
                        }
                    }

                    // Update progress tracking and open window
                    self.bardata_total = symbols.len();
                    self.bardata_queued = fetched_count;
                    self.bardata_skipped = skipped_count;
                    self.bardata_completed = 0;
                    self.bardata_log.clear();
                    for line in [
                        format!("BARDATA: total symbols: {}", symbols.len()),
                        format!("BARDATA: queued for download: {}", fetched_count),
                        format!("BARDATA: already cached (skipped): {}", skipped_count),
                        format!(
                            "BARDATA: uncached priority symbols: {}",
                            uncached_syms.len()
                        ),
                    ] {
                        self.bardata_log.push_back(line.clone());
                        self.log.push_back(LogEntry::info(line));
                    }
                    self.show_bardata = true;
                    self.bardata_active = true;
                }
            }
            "INDICES" | "WORLD_INDICES" => {
                self.show_world_indices = true;
                let symbols = vec![
                    "DIA", "SPY", "QQQ", "IWM", "EFA", "EEM", "VGK", "EWJ", "FXI", "EWZ", "GLD",
                    "SLV", "USO", "TLT", "UUP", "BTCUSD",
                ]
                .into_iter()
                .map(String::from)
                .collect();
                let _ = self
                    .broker_tx
                    .send(BrokerCmd::GetWatchlistQuotes { symbols });
                self.log
                    .push_back(LogEntry::info("Fetching world indices quotes..."));
            }
            "CRYPTO50" | "CRYPTO_TOP50" => {
                self.show_crypto_top50 = true;
                let _ = self.broker_tx.send(BrokerCmd::FetchCryptoTop50);
                self.log
                    .push_back(LogEntry::info("Fetching CoinGecko top 50..."));
            }
            "FOREX" | "FOREX_MATRIX" => {
                self.show_forex_matrix = true;
                let symbols = vec![
                    "EURUSD", "GBPUSD", "USDJPY", "USDCHF", "AUDUSD", "NZDUSD", "USDCAD", "EURGBP",
                    "EURJPY", "GBPJPY",
                ]
                .into_iter()
                .map(String::from)
                .collect();
                let _ = self
                    .broker_tx
                    .send(BrokerCmd::GetWatchlistQuotes { symbols });
                self.log
                    .push_back(LogEntry::info("Fetching forex pairs..."));
            }
            "KRAKEN" => {
                self.show_settings = true;
                self.log.push_back(LogEntry::info(
                    "Open Settings to configure Kraken API credentials",
                ));
            }
            "KRAKEN_TRADES" | "KRAKENTRADES" | "KRAKEN_HISTORY" => {
                if !self.kraken_enabled {
                    self.log
                        .push_back(LogEntry::warn("Kraken is disabled in Settings"));
                } else {
                    self.show_kraken_trade_history = true;
                    let _ = self.broker_tx.send(BrokerCmd::KrakenFetchTrades);
                    self.log
                        .push_back(LogEntry::info("Kraken: refreshing trade history"));
                }
            }
            "KRAKEN_ORDERS" | "KRAKENORDERS" | "KRAKEN_OPEN_ORDERS" => {
                if !self.kraken_enabled {
                    self.log
                        .push_back(LogEntry::warn("Kraken is disabled in Settings"));
                } else {
                    self.show_kraken_open_orders = true;
                    let _ = self.broker_tx.send(BrokerCmd::KrakenFetchOpenOrders);
                    self.log
                        .push_back(LogEntry::info("Kraken: refreshing open orders"));
                }
            }
            "KRAKEN_FUTURES" | "KRAKENFUTURES" => {
                if !self.kraken_enabled {
                    self.log
                        .push_back(LogEntry::warn("Kraken is disabled in Settings"));
                } else {
                    let _ = self.broker_tx.send(BrokerCmd::KrakenFuturesGetInstruments);
                    self.kraken_futures_requested = true;
                    self.log.push_back(LogEntry::info(
                        "Kraken Futures: loading public instrument universe",
                    ));
                }
            }
            "PREV_LEVELS" => self.show_prev_levels = !self.show_prev_levels,
            // Trading
            _ => return false,
        }
        true
    }
}
