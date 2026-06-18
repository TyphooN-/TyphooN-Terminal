use super::*;

mod ai_chat;
mod alpaca_account_data;
mod alpaca_order_ops;
mod bar_fetch_commands;
mod external_feeds;
mod fundamentals_commands;
mod kraken_market_commands;
mod kraken_order_ops;
mod kraken_ws_commands;
mod market_data_commands;
mod matrix_commands;
mod news;
mod research_compute;
mod research_fetch;
mod storage;
mod symbol_search;
mod watchlist_quotes;

pub(super) fn spawn_broker_message_processor(
    broker_cmd_rx: tokio::sync::mpsc::UnboundedReceiver<BrokerCmd>,
    broker_msg_tx: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    importing_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    rt_handle: tokio::runtime::Handle,
    shared_cache: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
) {
    // Spawn broker message processor
    let broker_msg_tx_clone = broker_msg_tx.clone();
    let importing_flag_broker = importing_flag.clone();
    let shared_cache_broker = shared_cache.clone();
    rt_handle.spawn(async move {
        let mut cmd_rx = broker_cmd_rx;
        let mut broker: Option<AlpacaBroker> = None;
        let mut kraken_broker: Option<typhoon_engine::broker::kraken::KrakenBroker> = None;
        let mut kraken_ws_broker: Option<typhoon_engine::broker::kraken::KrakenBroker> = None;
        // Pre-acquire and per-endpoint spacing are now owned by the
        // engine-side `iapi_limiter` (token bucket + escalating backoff,
        // shared across all iapi endpoints). The handler below just
        // delegates to it instead of maintaining its own gate state.
        let importing_flag = importing_flag_broker;
        let mut alpaca_fetch_permits = Arc::new(tokio::sync::Semaphore::new(4));
        let yahoo_chart_fetch_permits = Arc::new(tokio::sync::Semaphore::new(4));
        let kraken_fetch_permits =
            Arc::new(tokio::sync::Semaphore::new(KRAKEN_PUBLIC_FETCH_PERMITS));
        // Kraken Securities/iapi history is slower and can include synchronous cache work.
        // Keep it off the broker command loop and cap it separately so broad equities
        // sync cannot starve UI-visible broker messages (SEC scanner, order state, etc.).
        let kraken_equity_fetch_permits =
            Arc::new(tokio::sync::Semaphore::new(KRAKEN_EQUITIES_FETCH_PERMITS));
        let kraken_public_client = reqwest::Client::builder()
            .user_agent("TyphooN-Terminal/1.0")
            .pool_max_idle_per_host(KRAKEN_PUBLIC_FETCH_PERMITS * 2)
            .build()
            .unwrap_or_default();
        let fallback_bar_client = reqwest::Client::builder()
            .user_agent("TyphooN-Terminal/1.0")
            .pool_max_idle_per_host(8)
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .unwrap_or_default();
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                BrokerCmd::Connect {
                    api_key,
                    secret,
                    paper,
                    bar_requests_per_minute,
                    fetch_permits,
                } => {
                    alpaca_fetch_permits =
                        Arc::new(tokio::sync::Semaphore::new(fetch_permits.max(1)));
                    let b = AlpacaBroker::new(
                        api_key,
                        secret,
                        paper,
                        bar_requests_per_minute.max(ALPACA_DEFAULT_HISTORICAL_RPM),
                    );
                    match b.get_account().await {
                        Ok(acct) => {
                            let _ = broker_msg_tx_clone.send(BrokerMsg::Connected(format!(
                                "Connected: ${:.2} equity, ${:.2} buying power",
                                acct.equity, acct.buying_power
                            )));
                            let _ = broker_msg_tx_clone.send(BrokerMsg::Account(acct));
                            b.warm_data_connection().await;
                            broker = Some(b);
                        }
                        Err(e) => {
                            let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!("Connection failed: {}", e)));
                        }
                    }
                }
                BrokerCmd::ConfigureAlpacaSync {
                    bar_requests_per_minute,
                    fetch_permits,
                } => {
                    alpaca_fetch_permits =
                        Arc::new(tokio::sync::Semaphore::new(fetch_permits.max(1)));
                    if let Some(ref b) = broker {
                        b.set_bar_requests_per_minute_hint(
                            bar_requests_per_minute.max(ALPACA_DEFAULT_HISTORICAL_RPM),
                        )
                        .await;
                    }
                }
                BrokerCmd::MarkUnresolvable {
                    broker,
                    symbol,
                    timeframe,
                    reason,
                } => {
                    let _ = broker_msg_tx_clone.send(BrokerMsg::Unresolvable {
                        broker,
                        symbol,
                        timeframe,
                        reason,
                    });
                }
                cmd @ (BrokerCmd::GetAccount
                | BrokerCmd::GetPositions
                | BrokerCmd::GetOrders
                | BrokerCmd::GetOrderHistory { .. }) => {
                    alpaca_account_data::handle_alpaca_account_data_command(
                        cmd,
                        broker.as_ref(),
                        &broker_msg_tx_clone,
                    )
                    .await;
                }
                cmd @ (BrokerCmd::CloseAll | BrokerCmd::ClosePosition { .. }) => {
                    alpaca_order_ops::handle_alpaca_order_command(
                        cmd,
                        broker.as_ref(),
                        &broker_msg_tx_clone,
                    )
                    .await;
                }
                cmd @ (BrokerCmd::SecScrape { .. } | BrokerCmd::FinnhubNews { .. }) => {
                    news::handle_news_command(
                        cmd,
                        broker_msg_tx_clone.clone(),
                        shared_cache_broker.clone(),
                    );
                }
                BrokerCmd::GetQuote { symbol } => {
                    if let Some(ref b) = broker {
                        match b.get_latest_quote(&symbol).await {
                            Ok(q) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Quote(symbol, q.bid, q.ask, (q.bid + q.ask) / 2.0)); }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                        }
                    }
                }
                BrokerCmd::GetWatchlistQuotes { symbols } => {
                    watchlist_quotes::spawn_watchlist_quotes_task(
                        symbols,
                        broker.clone(),
                        broker_msg_tx_clone.clone(),
                        shared_cache_broker.clone(),
                    );
                }
                BrokerCmd::GetMarketClock => {
                    // US-equity/xStock session status is sourced from Alpaca's market clock.
                    // Kraken crypto pairs are shown separately as 24/7 in the toolbar.
                    if let Some(ref b) = broker {
                        match b.get_market_clock().await {
                            Ok(v) => {
                                let is_open = v["is_open"].as_bool().unwrap_or(false);
                                let next_open = v["next_open"].as_str().unwrap_or("—");
                                let next_close = v["next_close"].as_str().unwrap_or("—");

                                let next_open_utc = chrono::DateTime::parse_from_rfc3339(next_open)
                                    .ok()
                                    .map(|dt| dt.with_timezone(&chrono::Utc));
                                let next_close_utc = chrono::DateTime::parse_from_rfc3339(next_close)
                                    .ok()
                                    .map(|dt| dt.with_timezone(&chrono::Utc));

                                let msg = crate::app::app_runtime_support::us_equities_session_status_at(
                                    chrono::Utc::now(),
                                    is_open,
                                    next_open_utc,
                                    next_close_utc,
                                );
                                let _ = broker_msg_tx_clone.send(BrokerMsg::MarketClock(msg));
                            }
                            Err(e) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e));
                            }
                        }
                    }
                }
                cmd @ (BrokerCmd::GetActivities { .. }
                | BrokerCmd::GetTopMovers
                | BrokerCmd::GetAllAssets) => {
                    alpaca_account_data::handle_alpaca_account_data_command(
                        cmd,
                        broker.as_ref(),
                        &broker_msg_tx_clone,
                    )
                    .await;
                }
                BrokerCmd::SearchSymbols { query } => {
                    symbol_search::handle_symbol_search_command(
                        query,
                        broker.as_ref(),
                        &broker_msg_tx_clone,
                    )
                    .await;
                }

                cmd @ (BrokerCmd::GetFundamentals { .. }
                | BrokerCmd::GetHolders { .. }
                | BrokerCmd::GetAnalyst { .. }
                | BrokerCmd::GetOrderbook { .. }
                | BrokerCmd::GetMostActive
                | BrokerCmd::GetPortfolioHistory { .. }
                | BrokerCmd::GetPriceTarget { .. }
                | BrokerCmd::GetShortInterest { .. }
                | BrokerCmd::GetCorporateActions { .. }
                | BrokerCmd::GetWatchlists
                | BrokerCmd::CreateWatchlist { .. }
                | BrokerCmd::GetOptionsChain { .. }) => {
                    market_data_commands::handle_market_data_command(
                        cmd,
                        broker.as_ref(),
                        kraken_broker.as_ref(),
                        &shared_cache_broker,
                        &broker_msg_tx_clone,
                    )
                    .await;
                }
                cmd @ (BrokerCmd::AlpacaMarketOrder { .. }
                | BrokerCmd::AlpacaLimitOrder { .. }
                | BrokerCmd::AlpacaStopOrder { .. }
                | BrokerCmd::AlpacaBracketOrder { .. }
                | BrokerCmd::AlpacaCancelOrder { .. }
                | BrokerCmd::AlpacaOcoOrder { .. }
                | BrokerCmd::AlpacaModifyOrder { .. }
                | BrokerCmd::AlpacaSyncExits { .. }
                | BrokerCmd::AlpacaTrailingStop { .. }
                | BrokerCmd::AlpacaStopLimitOrder { .. }) => {
                    alpaca_order_ops::handle_alpaca_order_command(
                        cmd,
                        broker.as_ref(),
                        &broker_msg_tx_clone,
                    )
                    .await;
                }
                cmd @ BrokerCmd::AiChat { .. } => {
                    ai_chat::handle_ai_chat_command(
                        cmd,
                        broker_msg_tx_clone.clone(),
                        shared_cache_broker.clone(),
                    );
                }
                cmd @ (BrokerCmd::MatrixJoinRoom { .. }
                | BrokerCmd::MatrixFetchMessages { .. }
                | BrokerCmd::MatrixSendImage { .. }
                | BrokerCmd::MatrixSendMessage { .. }) => {
                    matrix_commands::handle_matrix_command(cmd, broker_msg_tx_clone.clone());
                }
                cmd @ BrokerCmd::KrakenSyncExits { .. } => {
                    kraken_order_ops::handle_kraken_order_command(
                        cmd,
                        kraken_broker.as_ref(),
                        &broker_msg_tx_clone,
                    )
                    .await;
                }
                cmd @ (BrokerCmd::FetchFearGreed
                | BrokerCmd::FetchRedditWSB
                | BrokerCmd::FetchCryptoTop50) => {
                    external_feeds::handle_external_feed_command(cmd, broker_msg_tx_clone.clone())
                        .await;
                }
                cmd @ (
                    BrokerCmd::FetchCompanyProfile { .. }
                    | BrokerCmd::FetchStockPeers { .. }
                    | BrokerCmd::FetchEarningsHistory { .. }
                    | BrokerCmd::FetchIpoCalendar { .. }
                    | BrokerCmd::FetchPressReleases { .. }
                    | BrokerCmd::FetchSocialSentiment { .. }
                    | BrokerCmd::FetchTranscriptList { .. }
                    | BrokerCmd::FetchTranscriptBody { .. }
                    | BrokerCmd::FetchCommoditiesQuotes
                    | BrokerCmd::FetchDividendHistory { .. }
                    | BrokerCmd::FetchEarningsEstimates { .. }
                    | BrokerCmd::FetchRatingChanges { .. }
                    | BrokerCmd::FetchTreasuryYields
                    | BrokerCmd::FetchFinancialStatements { .. }
                    | BrokerCmd::FetchExecutives { .. }
                    | BrokerCmd::FetchCotReports
                    | BrokerCmd::FetchStockSplits { .. }
                    | BrokerCmd::FetchEtfHoldings { .. }
                    | BrokerCmd::FetchAnalystRecs { .. }
                    | BrokerCmd::FetchPriceTarget { .. }
                    | BrokerCmd::FetchEsgScores { .. }
                    | BrokerCmd::FetchIndexMembers { .. }
                    | BrokerCmd::FetchInsiderTrades { .. }
                    | BrokerCmd::FetchInstitutionalHolders { .. }
                    | BrokerCmd::FetchSharesFloat { .. }
                    | BrokerCmd::FetchHistoricalPrice { .. }
                    | BrokerCmd::FetchEarningsSurprises { .. }
                    | BrokerCmd::FetchWorldIndices
                    | BrokerCmd::FetchMarketMovers { .. }
                    | BrokerCmd::FetchSectorPerformance { .. }
                    | BrokerCmd::FetchWaccSnapshot { .. }
                    | BrokerCmd::FetchCurrencyRates
                    | BrokerCmd::FetchBetaSnapshot { .. }
                ) => {
                    research_fetch::handle_research_fetch_command(
                        cmd,
                        broker_msg_tx_clone.clone(),
                    );
                }
                cmd @ (
                    BrokerCmd::ComputeDdmSnapshot { .. }
                    | BrokerCmd::ComputeRelativeValuation { .. }
                    | BrokerCmd::FetchFigiIdentifiers { .. }
                    | BrokerCmd::FetchHraSnapshot { .. }
                    | BrokerCmd::ComputeDcfSnapshot { .. }
                    | BrokerCmd::ComputeSvmSnapshot { .. }
                    | BrokerCmd::FetchOptionsChain { .. }
                    | BrokerCmd::ComputeIvolSnapshot { .. }
                    | BrokerCmd::ComputeSeasonalitySnapshot { .. }
                    | BrokerCmd::ComputeCorrelationMatrix { .. }
                    | BrokerCmd::ComputeTotalReturnSnapshot { .. }
                    | BrokerCmd::ComputeTechnicalsSnapshot { .. }
                    | BrokerCmd::ComputeVolSkewSnapshot { .. }
                    | BrokerCmd::ComputeLeverageSnapshot { .. }
                    | BrokerCmd::ComputeAccrualsSnapshot { .. }
                    | BrokerCmd::ComputeRealizedVolSnapshot { .. }
                    | BrokerCmd::ComputeFcfYieldSnapshot { .. }
                    | BrokerCmd::ComputeShortInterestSnapshot { .. }
                    | BrokerCmd::ComputeAltmanZSnapshot { .. }
                    | BrokerCmd::ComputePiotroskiSnapshot { .. }
                    | BrokerCmd::ComputeOhlcVolSnapshot { .. }
                    | BrokerCmd::ComputeEpsBeatSnapshot { .. }
                    | BrokerCmd::ComputePriceTargetDispersionSnapshot { .. }
                    | BrokerCmd::ComputeInsiderActivitySnapshot { .. }
                    | BrokerCmd::ComputeDivgSnapshot { .. }
                    | BrokerCmd::ComputeEarmSnapshot { .. }
                    | BrokerCmd::ComputeSectorRotationSnapshot { .. }
                    | BrokerCmd::ComputeUpdmSnapshot { .. }
                    | BrokerCmd::ComputeMomentumSnapshot { .. }
                    | BrokerCmd::ComputeLiquiditySnapshot { .. }
                    | BrokerCmd::ComputeBreakoutSnapshot { .. }
                    | BrokerCmd::ComputeCashCycleSnapshot { .. }
                    | BrokerCmd::ComputeCreditSnapshot { .. }
                    | BrokerCmd::ComputeGrowmSnapshot { .. }
                    | BrokerCmd::ComputeFlowSnapshot { .. }
                    | BrokerCmd::ComputeRegimeSnapshot { .. }
                    | BrokerCmd::ComputeRelvolSnapshot { .. }
                    | BrokerCmd::ComputeMarginsSnapshot { .. }
                    | BrokerCmd::ComputeValSnapshot { .. }
                    | BrokerCmd::ComputeQualSnapshot { .. }
                    | BrokerCmd::ComputeRiskSnapshot { .. }
                    | BrokerCmd::ComputeInsstrkSnapshot { .. }
                    | BrokerCmd::ComputeCovgSnapshot { .. }
                    | BrokerCmd::ComputeVrkSnapshot { .. }
                    | BrokerCmd::ComputeQrkSnapshot { .. }
                    | BrokerCmd::ComputeRrkSnapshot { .. }
                    | BrokerCmd::ComputeRelepsgrSnapshot { .. }
                    | BrokerCmd::ComputePeadSnapshot { .. }
                    | BrokerCmd::ComputeSizefSnapshot { .. }
                    | BrokerCmd::ComputeMomfSnapshot { .. }
                    | BrokerCmd::ComputePeadrankSnapshot { .. }
                    | BrokerCmd::ComputeFqmSnapshot { .. }
                    | BrokerCmd::ComputeRevrankSnapshot { .. }
                    | BrokerCmd::ComputeLevrankSnapshot { .. }
                    | BrokerCmd::ComputeOperankSnapshot { .. }
                    | BrokerCmd::ComputeFqmrankSnapshot { .. }
                    | BrokerCmd::ComputeLiqrankSnapshot { .. }
                    | BrokerCmd::ComputeSurpstkSnapshot { .. }
                    | BrokerCmd::ComputeDvdrankSnapshot { .. }
                    | BrokerCmd::ComputeEarmrankSnapshot { .. }
                    | BrokerCmd::ComputeUpdgrankSnapshot { .. }
                    | BrokerCmd::ComputeGySnapshot { .. }
                    | BrokerCmd::ComputeDesSnapshot { .. }
                    | BrokerCmd::ComputeDvdyieldrankSnapshot { .. }
                    | BrokerCmd::ComputeShrankSnapshot { .. }
                    | BrokerCmd::ComputeShortrankDeltaSnapshot { .. }
                    | BrokerCmd::ComputeInsiderconcSnapshot { .. }
                    | BrokerCmd::ComputeAtrannSnapshot { .. }
                    | BrokerCmd::ComputeDdhistSnapshot { .. }
                    | BrokerCmd::ComputePriceperfSnapshot { .. }
                    | BrokerCmd::ComputeMomrankMultiSnapshot { .. }
                    | BrokerCmd::ComputeBetarankSnapshot { .. }
                    | BrokerCmd::ComputePegrankSnapshot { .. }
                    | BrokerCmd::ComputeFhighlowSnapshot { .. }
                    | BrokerCmd::ComputeRvconeSnapshot { .. }
                    | BrokerCmd::ComputeCalpbSnapshot { .. }
                    | BrokerCmd::ComputeCorrstkSnapshot { .. }
                    | BrokerCmd::ComputeTlrankSnapshot { .. }
                    | BrokerCmd::ComputeCorrrankSnapshot { .. }
                    | BrokerCmd::ComputeOperankDeltaSnapshot { .. }
                    | BrokerCmd::ComputeDivaccSnapshot { .. }
                    | BrokerCmd::ComputeEpsaccSnapshot { .. }
                    | BrokerCmd::ComputeVrpSnapshot { .. }
                    | BrokerCmd::ComputeRetskewSnapshot { .. }
                    | BrokerCmd::ComputeRetkurtSnapshot { .. }
                    | BrokerCmd::ComputeTailrSnapshot { .. }
                    | BrokerCmd::ComputeRunlenSnapshot { .. }
                    | BrokerCmd::ComputeDayrangeSnapshot { .. }
                    | BrokerCmd::ComputeAutocorSnapshot { .. }
                    | BrokerCmd::ComputeHurstSnapshot { .. }
                    | BrokerCmd::ComputeHitrateSnapshot { .. }
                    | BrokerCmd::ComputeGlasymSnapshot { .. }
                    | BrokerCmd::ComputeVolratioSnapshot { .. }
                    | BrokerCmd::ComputeDrawupSnapshot { .. }
                    | BrokerCmd::ComputeGapstatsSnapshot { .. }
                    | BrokerCmd::ComputeVolclusterSnapshot { .. }
                    | BrokerCmd::ComputeCloseplcSnapshot { .. }
                    | BrokerCmd::ComputeMrhlSnapshot { .. }
                    | BrokerCmd::ComputeDownvolSnapshot { .. }
                    | BrokerCmd::ComputeSharprSnapshot { .. }
                    | BrokerCmd::ComputeEffratioSnapshot { .. }
                    | BrokerCmd::ComputeWickbiasSnapshot { .. }
                    | BrokerCmd::ComputeVolofvolSnapshot { .. }
                    | BrokerCmd::ComputeCalmarSnapshot { .. }
                    | BrokerCmd::ComputeUlcerSnapshot { .. }
                    | BrokerCmd::ComputeVarratioSnapshot { .. }
                    | BrokerCmd::ComputeAmihudSnapshot { .. }
                    | BrokerCmd::ComputeJbnormSnapshot { .. }
                    | BrokerCmd::ComputeOmegaSnapshot { .. }
                    | BrokerCmd::ComputeDfaSnapshot { .. }
                    | BrokerCmd::ComputeBurkeSnapshot { .. }
                    | BrokerCmd::ComputeMonthseasSnapshot { .. }
                    | BrokerCmd::ComputeRollsprdSnapshot { .. }
                    | BrokerCmd::ComputeParkinsonSnapshot { .. }
                    | BrokerCmd::ComputeGkvolSnapshot { .. }
                    | BrokerCmd::ComputeRsvolSnapshot { .. }
                    | BrokerCmd::ComputeCvarSnapshot { .. }
                    | BrokerCmd::ComputeDoweffectSnapshot { .. }
                    | BrokerCmd::ComputeSterlingSnapshot { .. }
                    | BrokerCmd::ComputeKellyfSnapshot { .. }
                    | BrokerCmd::ComputeLjungbSnapshot { .. }
                    | BrokerCmd::ComputeRunstestSnapshot { .. }
                    | BrokerCmd::ComputeZeroretSnapshot { .. }
                    | BrokerCmd::ComputePsrSnapshot { .. }
                    | BrokerCmd::ComputeAdfSnapshot { .. }
                    | BrokerCmd::ComputeMnkendallSnapshot { .. }
                    | BrokerCmd::ComputeBipowerSnapshot { .. }
                    | BrokerCmd::ComputeDddurSnapshot { .. }
                    | BrokerCmd::ComputeHilltailSnapshot { .. }
                    | BrokerCmd::ComputeArchlmSnapshot { .. }
                    | BrokerCmd::ComputePainratioSnapshot { .. }
                    | BrokerCmd::ComputeCusumSnapshot { .. }
                    | BrokerCmd::ComputeCfvarSnapshot { .. }
                    | BrokerCmd::ComputeEntropySnapshot { .. }
                    | BrokerCmd::ComputeRachevSnapshot { .. }
                    | BrokerCmd::ComputeGprSnapshot { .. }
                    | BrokerCmd::ComputePacfSnapshot { .. }
                    | BrokerCmd::ComputeApenSnapshot { .. }
                    | BrokerCmd::ComputeUprSnapshot { .. }
                    | BrokerCmd::ComputeLevereffSnapshot { .. }
                    | BrokerCmd::ComputeDrawdarSnapshot { .. }
                    | BrokerCmd::ComputeVarhalfSnapshot { .. }
                    | BrokerCmd::ComputeGiniSnapshot { .. }
                    | BrokerCmd::ComputeSampenSnapshot { .. }
                    | BrokerCmd::ComputePermenSnapshot { .. }
                    | BrokerCmd::ComputeRecfactSnapshot { .. }
                    | BrokerCmd::ComputeKpssSnapshot { .. }
                    | BrokerCmd::ComputeSpecentSnapshot { .. }
                    | BrokerCmd::ComputeRobvolSnapshot { .. }
                    | BrokerCmd::ComputeRenyientSnapshot { .. }
                    | BrokerCmd::ComputeRetquantSnapshot { .. }
                    | BrokerCmd::ComputeMsentSnapshot { .. }
                    | BrokerCmd::ComputeEwmavolSnapshot { .. }
                    | BrokerCmd::ComputeKsnormSnapshot { .. }
                    | BrokerCmd::ComputeAdtestSnapshot { .. }
                    | BrokerCmd::ComputeLmomSnapshot { .. }
                    | BrokerCmd::ComputeKylelamSnapshot { .. }
                    | BrokerCmd::ComputePeakoverSnapshot { .. }
                    | BrokerCmd::ComputeHiguchiSnapshot { .. }
                    | BrokerCmd::ComputePickandsSnapshot { .. }
                    | BrokerCmd::ComputeKappa3Snapshot { .. }
                    | BrokerCmd::ComputeLyapunovSnapshot { .. }
                    | BrokerCmd::ComputeRankacSnapshot { .. }
                    | BrokerCmd::ComputeBnsjumpSnapshot { .. }
                    | BrokerCmd::ComputePprootSnapshot { .. }
                    | BrokerCmd::ComputeMfdfaSnapshot { .. }
                    | BrokerCmd::ComputeHillksSnapshot { .. }
                    | BrokerCmd::ComputeTsiSnapshot { .. }
                    | BrokerCmd::ComputeGarch11Snapshot { .. }
                    | BrokerCmd::ComputeSadfSnapshot { .. }
                    | BrokerCmd::ComputeCordimSnapshot { .. }
                    | BrokerCmd::ComputeSkspecSnapshot { .. }
                    | BrokerCmd::ComputeAutomiSnapshot { .. }
                    | BrokerCmd::ComputeDurbinWatsonSnapshot { .. }
                    | BrokerCmd::ComputeBdsTestSnapshot { .. }
                    | BrokerCmd::ComputeBreuschPaganSnapshot { .. }
                    | BrokerCmd::ComputeTurnPtsSnapshot { .. }
                    | BrokerCmd::ComputePeriodogramSnapshot { .. }
                    | BrokerCmd::ComputeMcLeodLiSnapshot { .. }
                    | BrokerCmd::ComputeOuFitSnapshot { .. }
                    | BrokerCmd::ComputeGphSnapshot { .. }
                    | BrokerCmd::ComputeBurgSpecSnapshot { .. }
                    | BrokerCmd::ComputeKendallTauSnapshot { .. }
                    | BrokerCmd::ComputeSqueezeSnapshot { .. }
                    | BrokerCmd::ComputeSqueezeRankSnapshot { .. }
                    | BrokerCmd::RefreshSqueezeWatchlist { .. }
                    | BrokerCmd::ComputeBbsqueezeSnapshot { .. }
                    | BrokerCmd::ComputeDonchianSnapshot { .. }
                    | BrokerCmd::ComputeKamaSnapshot { .. }
                    | BrokerCmd::ComputeIchimokuSnapshot { .. }
                    | BrokerCmd::ComputeSupertrendSnapshot { .. }
                    | BrokerCmd::ComputeKeltnerSnapshot { .. }
                    | BrokerCmd::ComputeFisherSnapshot { .. }
                    | BrokerCmd::ComputeAroonSnapshot { .. }
                    | BrokerCmd::ComputeAdxSnapshot { .. }
                    | BrokerCmd::ComputeCciSnapshot { .. }
                    | BrokerCmd::ComputeCmfSnapshot { .. }
                    | BrokerCmd::ComputeMfiSnapshot { .. }
                    | BrokerCmd::ComputePsarSnapshot { .. }
                    | BrokerCmd::ComputeVortexSnapshot { .. }
                    | BrokerCmd::ComputeChopSnapshot { .. }
                    | BrokerCmd::ComputeObvSnapshot { .. }
                    | BrokerCmd::ComputeTrixSnapshot { .. }
                    | BrokerCmd::ComputeHmaSnapshot { .. }
                    | BrokerCmd::ComputePpoSnapshot { .. }
                    | BrokerCmd::ComputeDpoSnapshot { .. }
                    | BrokerCmd::ComputeKstSnapshot { .. }
                    | BrokerCmd::ComputeUltoscSnapshot { .. }
                    | BrokerCmd::ComputeWillrSnapshot { .. }
                    | BrokerCmd::ComputeMassSnapshot { .. }
                    | BrokerCmd::ComputeChaikoscSnapshot { .. }
                    | BrokerCmd::ComputeKlingerSnapshot { .. }
                    | BrokerCmd::ComputeStochRsiSnapshot { .. }
                    | BrokerCmd::ComputeAwesomeSnapshot { .. }
                    | BrokerCmd::ComputeEfiSnapshot { .. }
                    | BrokerCmd::ComputeEmvSnapshot { .. }
                    | BrokerCmd::ComputeNviSnapshot { .. }
                    | BrokerCmd::ComputePviSnapshot { .. }
                    | BrokerCmd::ComputeCoppockSnapshot { .. }
                    | BrokerCmd::ComputeCmoSnapshot { .. }
                    | BrokerCmd::ComputeQstickSnapshot { .. }
                    | BrokerCmd::ComputeDisparitySnapshot { .. }
                    | BrokerCmd::ComputeBopSnapshot { .. }
                    | BrokerCmd::ComputeSchaffSnapshot { .. }
                    | BrokerCmd::ComputeStochSnapshot { .. }
                    | BrokerCmd::ComputeMacdSnapshot { .. }
                    | BrokerCmd::ComputeVwapSnapshot { .. }
                    | BrokerCmd::ComputeMcgdSnapshot { .. }
                    | BrokerCmd::ComputeRwiSnapshot { .. }
                    | BrokerCmd::ComputeDemaSnapshot { .. }
                    | BrokerCmd::ComputeTemaSnapshot { .. }
                    | BrokerCmd::ComputeLinregSnapshot { .. }
                    | BrokerCmd::ComputePivotsSnapshot { .. }
                    | BrokerCmd::ComputeHeikinSnapshot { .. }
                    | BrokerCmd::ComputeAlmaSnapshot { .. }
                    | BrokerCmd::ComputeZlemaSnapshot { .. }
                    | BrokerCmd::ComputeElderRaySnapshot { .. }
                    | BrokerCmd::ComputeTsfSnapshot { .. }
                    | BrokerCmd::ComputeRviSnapshot { .. }
                    | BrokerCmd::ComputeTrimaSnapshot { .. }
                    | BrokerCmd::ComputeT3Snapshot { .. }
                    | BrokerCmd::ComputeVidyaSnapshot { .. }
                    | BrokerCmd::ComputeSmiSnapshot { .. }
                    | BrokerCmd::ComputePvtSnapshot { .. }
                    | BrokerCmd::ComputeAcSnapshot { .. }
                    | BrokerCmd::ComputeChvolSnapshot { .. }
                    | BrokerCmd::ComputeBbwidthSnapshot { .. }
                    | BrokerCmd::ComputeElderImpSnapshot { .. }
                    | BrokerCmd::ComputeRmiSnapshot { .. }
                    | BrokerCmd::ComputeSymbolExpirations { .. }
                    | BrokerCmd::ComputeSmmaSnapshot { .. }
                    | BrokerCmd::ComputeAlligatorSnapshot { .. }
                    | BrokerCmd::ComputeCrsiSnapshot { .. }
                    | BrokerCmd::ComputeSebSnapshot { .. }
                    | BrokerCmd::ComputeImiSnapshot { .. }
                    | BrokerCmd::ComputeGmmaSnapshot { .. }
                    | BrokerCmd::ComputeMaenvSnapshot { .. }
                    | BrokerCmd::ComputeAdlSnapshot { .. }
                    | BrokerCmd::ComputeVhfSnapshot { .. }
                    | BrokerCmd::ComputeVrocSnapshot { .. }
                    | BrokerCmd::ComputeKdjSnapshot { .. }
                    | BrokerCmd::ComputeQqeSnapshot { .. }
                    | BrokerCmd::ComputePmoSnapshot { .. }
                    | BrokerCmd::ComputeCfoSnapshot { .. }
                    | BrokerCmd::ComputeTmfSnapshot { .. }
                    | BrokerCmd::ComputeFractalsSnapshot { .. }
                    | BrokerCmd::ComputeIftRsiSnapshot { .. }
                    | BrokerCmd::ComputeMamaSnapshot { .. }
                    | BrokerCmd::ComputeCogSnapshot { .. }
                    | BrokerCmd::ComputeDidiSnapshot { .. }
                    | BrokerCmd::ComputeDemarkerSnapshot { .. }
                    | BrokerCmd::ComputeGatorSnapshot { .. }
                    | BrokerCmd::ComputeBwMfiSnapshot { .. }
                    | BrokerCmd::ComputeVwmaSnapshot { .. }
                    | BrokerCmd::ComputeStddevSnapshot { .. }
                    | BrokerCmd::ComputeWmaSnapshot { .. }
                    | BrokerCmd::ComputeRainbowSnapshot { .. }
                    | BrokerCmd::ComputeMesaSineSnapshot { .. }
                    | BrokerCmd::ComputeFramaSnapshot { .. }
                    | BrokerCmd::ComputeIbsSnapshot { .. }
                    | BrokerCmd::ComputeLaguerreRsiSnapshot { .. }
                    | BrokerCmd::ComputeZigzagSnapshot { .. }
                    | BrokerCmd::ComputePgoSnapshot { .. }
                    | BrokerCmd::ComputeHtTrendlineSnapshot { .. }
                    | BrokerCmd::ComputeMidpointSnapshot { .. }
                    | BrokerCmd::ComputeMassIndexSnapshot { .. }
                    | BrokerCmd::ComputeNatrSnapshot { .. }
                    | BrokerCmd::ComputeTtmSqueezeSnapshot { .. }
                    | BrokerCmd::ComputeForceIndexSnapshot { .. }
                    | BrokerCmd::ComputeTrangeSnapshot { .. }
                    | BrokerCmd::ComputeLinearregSlopeSnapshot { .. }
                    | BrokerCmd::ComputeHtDcperiodSnapshot { .. }
                    | BrokerCmd::ComputeHtTrendmodeSnapshot { .. }
                    | BrokerCmd::ComputeAccbandsSnapshot { .. }
                    | BrokerCmd::ComputeStochfSnapshot { .. }
                    | BrokerCmd::ComputeLinearregSnapshot { .. }
                    | BrokerCmd::ComputeLinearregAngleSnapshot { .. }
                    | BrokerCmd::ComputeHtDcphaseSnapshot { .. }
                    | BrokerCmd::ComputeHtSineSnapshot { .. }
                    | BrokerCmd::ComputeHtPhasorSnapshot { .. }
                    | BrokerCmd::ComputeMidpriceSnapshot { .. }
                    | BrokerCmd::ComputeApoSnapshot { .. }
                    | BrokerCmd::ComputeMomSnapshot { .. }
                    | BrokerCmd::ComputeSarextSnapshot { .. }
                    | BrokerCmd::ComputeAdxrSnapshot { .. }
                    | BrokerCmd::ComputeAvgpriceSnapshot { .. }
                    | BrokerCmd::ComputeMedpriceSnapshot { .. }
                    | BrokerCmd::ComputeTypPriceSnapshot { .. }
                    | BrokerCmd::ComputeWclPriceSnapshot { .. }
                    | BrokerCmd::ComputeVarianceSnapshot { .. }
                    | BrokerCmd::ComputePlusDiSnapshot { .. }
                    | BrokerCmd::ComputeMinusDiSnapshot { .. }
                    | BrokerCmd::ComputePlusDmSnapshot { .. }
                    | BrokerCmd::ComputeMinusDmSnapshot { .. }
                    | BrokerCmd::ComputeDxSnapshot { .. }
                    | BrokerCmd::ComputeRocSnapshot { .. }
                    | BrokerCmd::ComputeRocpSnapshot { .. }
                    | BrokerCmd::ComputeRocrSnapshot { .. }
                    | BrokerCmd::ComputeRocr100Snapshot { .. }
                    | BrokerCmd::ComputeCorrelSnapshot { .. }
                    | BrokerCmd::ComputeMinSnapshot { .. }
                    | BrokerCmd::ComputeMaxSnapshot { .. }
                    | BrokerCmd::ComputeMinMaxSnapshot { .. }
                    | BrokerCmd::ComputeMinIndexSnapshot { .. }
                    | BrokerCmd::ComputeMaxIndexSnapshot { .. }
                    | BrokerCmd::ComputeBbandsSnapshot { .. }
                    | BrokerCmd::ComputeAdSnapshot { .. }
                    | BrokerCmd::ComputeAdoscSnapshot { .. }
                    | BrokerCmd::ComputeSumSnapshot { .. }
                    | BrokerCmd::ComputeLinearRegInterceptSnapshot { .. }
                    | BrokerCmd::ComputeAroonoscSnapshot { .. }
                    | BrokerCmd::ComputeMinMaxIndexSnapshot { .. }
                    | BrokerCmd::ComputeMacdextSnapshot { .. }
                    | BrokerCmd::ComputeMacdfixSnapshot { .. }
                    | BrokerCmd::ComputeMavpSnapshot { .. }
                    | BrokerCmd::ComputeCdlDojiSnapshot { .. }
                    | BrokerCmd::ComputeCdlHammerSnapshot { .. }
                    | BrokerCmd::ComputeCdlShootingStarSnapshot { .. }
                    | BrokerCmd::ComputeCdlEngulfingSnapshot { .. }
                    | BrokerCmd::ComputeCdlHaramiSnapshot { .. }
                    | BrokerCmd::ComputeCdlMorningStarSnapshot { .. }
                    | BrokerCmd::ComputeCdlEveningStarSnapshot { .. }
                    | BrokerCmd::ComputeCdlThreeBlackCrowsSnapshot { .. }
                    | BrokerCmd::ComputeCdlThreeWhiteSoldiersSnapshot { .. }
                    | BrokerCmd::ComputeCdlDarkCloudCoverSnapshot { .. }
                    | BrokerCmd::ComputeCdlPiercingSnapshot { .. }
                    | BrokerCmd::ComputeCdlDragonflyDojiSnapshot { .. }
                    | BrokerCmd::ComputeCdlGravestoneDojiSnapshot { .. }
                    | BrokerCmd::ComputeCdlHangingManSnapshot { .. }
                    | BrokerCmd::ComputeCdlInvertedHammerSnapshot { .. }
                    | BrokerCmd::ComputeCdlHaramiCrossSnapshot { .. }
                    | BrokerCmd::ComputeCdlLongLeggedDojiSnapshot { .. }
                    | BrokerCmd::ComputeCdlMarubozuSnapshot { .. }
                    | BrokerCmd::ComputeCdlSpinningTopSnapshot { .. }
                    | BrokerCmd::ComputeCdlTristarSnapshot { .. }
                    | BrokerCmd::ComputeCdlDojiStarSnapshot { .. }
                    | BrokerCmd::ComputeCdlMorningDojiStarSnapshot { .. }
                    | BrokerCmd::ComputeCdlEveningDojiStarSnapshot { .. }
                    | BrokerCmd::ComputeCdlAbandonedBabySnapshot { .. }
                    | BrokerCmd::ComputeCdlThreeInsideSnapshot { .. }
                    | BrokerCmd::ComputeCdlBeltHoldSnapshot { .. }
                    | BrokerCmd::ComputeCdlClosingMarubozuSnapshot { .. }
                    | BrokerCmd::ComputeCdlHighWaveSnapshot { .. }
                    | BrokerCmd::ComputeCdlLongLineSnapshot { .. }
                    | BrokerCmd::ComputeCdlShortLineSnapshot { .. }
                    | BrokerCmd::ComputeCdlCounterattackSnapshot { .. }
                    | BrokerCmd::ComputeCdlHomingPigeonSnapshot { .. }
                    | BrokerCmd::ComputeCdlInNeckSnapshot { .. }
                    | BrokerCmd::ComputeCdlOnNeckSnapshot { .. }
                    | BrokerCmd::ComputeCdlThrustingSnapshot { .. }
                    | BrokerCmd::ComputeCdlTwoCrowsSnapshot { .. }
                    | BrokerCmd::ComputeCdlThreeLineStrikeSnapshot { .. }
                    | BrokerCmd::ComputeCdlThreeOutsideSnapshot { .. }
                    | BrokerCmd::ComputeCdlMatchingLowSnapshot { .. }
                    | BrokerCmd::ComputeCdlSeparatingLinesSnapshot { .. }
                    | BrokerCmd::ComputeCdlStickSandwichSnapshot { .. }
                    | BrokerCmd::ComputeCdlRickshawManSnapshot { .. }
                    | BrokerCmd::ComputeCdlTakuriSnapshot { .. }
                    | BrokerCmd::ComputeCdlThreeStarsInSouthSnapshot { .. }
                    | BrokerCmd::ComputeCdlIdenticalThreeCrowsSnapshot { .. }
                    | BrokerCmd::ComputeCdlKickingSnapshot { .. }
                    | BrokerCmd::ComputeCdlKickingByLengthSnapshot { .. }
                    | BrokerCmd::ComputeCdlLadderBottomSnapshot { .. }
                    | BrokerCmd::ComputeCdlUniqueThreeRiverSnapshot { .. }
                    | BrokerCmd::ComputeCdlAdvanceBlockSnapshot { .. }
                    | BrokerCmd::ComputeCdlBreakawaySnapshot { .. }
                    | BrokerCmd::ComputeCdlGapSideSideWhiteSnapshot { .. }
                    | BrokerCmd::ComputeCdlUpsideGapTwoCrowsSnapshot { .. }
                    | BrokerCmd::ComputeCdlXSideGapThreeMethodsSnapshot { .. }
                    | BrokerCmd::ComputeCdlConcealBabySwallowSnapshot { .. }
                    | BrokerCmd::ComputeCdlHikkakeSnapshot { .. }
                    | BrokerCmd::ComputeCdlHikkakeModSnapshot { .. }
                    | BrokerCmd::ComputeCdlMatHoldSnapshot { .. }
                    | BrokerCmd::ComputeCdlRiseFallThreeMethodsSnapshot { .. }
                    | BrokerCmd::ComputeCdlStalledPatternSnapshot { .. }
                    | BrokerCmd::ComputeCdlTasukiGapSnapshot { .. }
                    | BrokerCmd::ComputeModSharpeSnapshot { .. }
                    | BrokerCmd::ComputeHsiehTestSnapshot { .. }
                    | BrokerCmd::ComputeChowBreakSnapshot { .. }
                    | BrokerCmd::ComputeDriftBurstSnapshot { .. }
                    | BrokerCmd::ComputeHlvClustSnapshot { .. }
                    | BrokerCmd::ComputeYangZhangSnapshot { .. }
                    | BrokerCmd::ComputeKuiperSnapshot { .. }
                    | BrokerCmd::ComputeDagostinoSnapshot { .. }
                    | BrokerCmd::ComputeBaiPerronSnapshot { .. }
                    | BrokerCmd::ComputeKupiecPofSnapshot { .. }
                ) => {
                    research_compute::handle_research_compute_command(
                        cmd,
                        broker_msg_tx_clone.clone(),
                        shared_cache_broker.clone(),
                    );
                }
                cmd @ (
                    BrokerCmd::IngestResearchArticles { .. }
                    | BrokerCmd::FetchNewsMulti { .. }
                    | BrokerCmd::LoadCachedNews { .. }
                    | BrokerCmd::HydrateNewsArticle { .. }
                    | BrokerCmd::SearchNews { .. }
                    | BrokerCmd::NewsScrapeSymbols { .. }
                ) => {
                    news::handle_news_command(
                        cmd,
                        broker_msg_tx_clone.clone(),
                        shared_cache_broker.clone(),
                    );
                }
                cmd @ BrokerCmd::NewsScrapeAll { .. } => {
                    news::handle_news_scrape_all_command(
                        cmd,
                        broker.as_ref(),
                        &broker_msg_tx_clone,
                        shared_cache_broker.clone(),
                    )
                    .await;
                }
                BrokerCmd::KrakenConnect {
                    api_key,
                    api_secret,
                    ws_api_key,
                    ws_api_secret,
                } => {
                    use typhoon_engine::broker::kraken::KrakenBroker;
                    let msg_tx = broker_msg_tx_clone.clone();
                    let rest_ready =
                        !api_key.trim().is_empty() && !api_secret.trim().is_empty();
                    let ws_override_ready =
                        !ws_api_key.trim().is_empty() && !ws_api_secret.trim().is_empty();
                    let ws_creds = if ws_override_ready {
                        Some((ws_api_key.clone(), ws_api_secret.clone(), "WebSocket"))
                    } else if rest_ready {
                        Some((api_key.clone(), api_secret.clone(), "REST"))
                    } else {
                        None
                    };
                    let mut ws_status: Option<String> = None;
                    if let Some((ws_key, ws_secret, label)) = ws_creds {
                        let ws_kb = KrakenBroker::new(ws_key, ws_secret);
                        ws_status = Some(match ws_kb.get_websockets_token_string().await {
                            Ok(_token) => format!("WS auth ready via {} key", label),
                            Err(e) => format!("WS auth unavailable via {} key: {}", label, e),
                        });
                    }
                    if !rest_ready {
                        let suffix = ws_status
                            .as_ref()
                            .map(|status| format!(" ({})", status))
                            .unwrap_or_default();
                        let _ = msg_tx.send(BrokerMsg::Error(format!(
                            "Kraken REST key required for account/trading{}",
                            suffix
                        )));
                        continue;
                    }
                    let rest_api_key = api_key.clone();
                    let rest_api_secret = api_secret.clone();
                    let kb = KrakenBroker::new(api_key, api_secret);
                    match kb.get_balance().await {
                        Ok(balances) => {
                            let mut bal_vec: Vec<(String, f64)> = balances.into_iter()
                                .filter(|(_, v)| *v > 0.0)
                                .collect();
                            bal_vec.sort_by(|a, b| a.0.cmp(&b.0));
                            let summary: String = bal_vec.iter()
                                .map(|(a, v)| format!("{}: {:.8}", a, v))
                                .collect::<Vec<_>>().join(", ");
                            let ws_suffix = ws_status
                                .as_ref()
                                .map(|status| format!(" · {}", status))
                                .unwrap_or_else(|| " · WS auth not configured".to_string());
                            let _ = msg_tx.send(BrokerMsg::Connected(format!(
                                "Kraken connected — {} assets ({}){}",
                                bal_vec.len(), summary, ws_suffix
                            )));
                            let mut pos = kb.get_position_summaries().await.unwrap_or_default();
                            pos.extend(KrakenBroker::equity_position_summaries_from_balances(
                                &bal_vec,
                            ));
                            pos.sort_by(|a, b| a.symbol.cmp(&b.symbol));
                            let _ = msg_tx.send(BrokerMsg::KrakenBalances(bal_vec));
                            let _ = msg_tx.send(BrokerMsg::KrakenPositions(pos));
                            if let Ok(pairs) = kb.get_tradeable_pairs().await {
                                let _ = msg_tx.send(BrokerMsg::KrakenPairs(pairs));
                            }
                            kraken_ws_broker = Some(if ws_override_ready {
                                KrakenBroker::new(ws_api_key, ws_api_secret)
                            } else {
                                KrakenBroker::new(rest_api_key, rest_api_secret)
                            });
                            kraken_broker = Some(kb);
                        }
                        Err(e) => {
                            let _ = msg_tx.send(BrokerMsg::Error(format!("Kraken auth failed: {}", e)));
                        }
                    }
                }
                cmd @ (
                    BrokerCmd::KrakenGetBalance
                    | BrokerCmd::KrakenGetPositions
                    | BrokerCmd::KrakenPlaceOrder { .. }
                    | BrokerCmd::KrakenPlaceOrderAdvanced { .. }
                    | BrokerCmd::KrakenClosePosition { .. }
                    | BrokerCmd::KrakenCancelOrder { .. }
                    | BrokerCmd::KrakenCancelAll
                    | BrokerCmd::KrakenFetchTrades
                    | BrokerCmd::KrakenFetchOpenOrders
                ) => {
                    kraken_order_ops::handle_kraken_account_order_command(
                        cmd,
                        kraken_broker.as_ref(),
                        &broker_msg_tx_clone,
                    )
                    .await;
                }
                cmd @ (
                    BrokerCmd::KrakenFetchEquityTicker { .. }
                    | BrokerCmd::KrakenFetchEquityHistory { .. }
                    | BrokerCmd::YahooChartFetchBars { .. }
                    | BrokerCmd::KrakenFetchEquityUniverse
                ) => {
                    kraken_market_commands::handle_kraken_market_command(
                        cmd,
                        kraken_broker.as_ref(),
                        &broker_msg_tx_clone,
                        shared_cache_broker.clone(),
                        kraken_equity_fetch_permits.clone(),
                        yahoo_chart_fetch_permits.clone(),
                        kraken_public_client.clone(),
                        fallback_bar_client.clone(),
                    )
                    .await;
                }
                cmd @ (
                    BrokerCmd::KrakenStartPrivateWs
                    | BrokerCmd::KrakenStartOhlcStreamers { .. }
                    | BrokerCmd::KrakenOhlcSnapshotSweep { .. }
                    | BrokerCmd::KrakenStartOrderbookWs { .. }
                ) => {
                    kraken_ws_commands::handle_kraken_ws_command(
                        cmd,
                        kraken_broker.as_ref(),
                        kraken_ws_broker.as_ref(),
                        &broker_msg_tx_clone,
                        shared_cache_broker.clone(),
                        kraken_public_client.clone(),
                    )
                    .await;
                }
                cmd @ (BrokerCmd::KrakenCloseAll | BrokerCmd::KrakenGetPairs) => {
                    kraken_order_ops::handle_kraken_account_order_command(
                        cmd,
                        kraken_broker.as_ref(),
                        &broker_msg_tx_clone,
                    )
                    .await;
                }
                cmd @ BrokerCmd::KrakenFuturesGetInstruments => {
                    kraken_market_commands::handle_kraken_market_command(
                        cmd,
                        kraken_broker.as_ref(),
                        &broker_msg_tx_clone,
                        shared_cache_broker.clone(),
                        kraken_equity_fetch_permits.clone(),
                        yahoo_chart_fetch_permits.clone(),
                        kraken_public_client.clone(),
                        fallback_bar_client.clone(),
                    )
                    .await;
                }
                cmd @ (
                    BrokerCmd::FundamentalsScrape { .. }
                    | BrokerCmd::FundamentalsScrapeOne { .. }
                    | BrokerCmd::ResearchScrape { .. }
                ) => {
                    fundamentals_commands::handle_fundamentals_command(
                        cmd,
                        broker.as_ref(),
                        &broker_msg_tx_clone,
                        shared_cache_broker.clone(),
                    )
                    .await;
                }
                cmd @ (
                    BrokerCmd::CompactStorage { .. }
                    | BrokerCmd::ScanUnusualVolume { .. }
                ) => {
                    storage::handle_storage_command(
                        cmd,
                        broker_msg_tx_clone.clone(),
                        importing_flag.clone(),
                        shared_cache_broker.clone(),
                    );
                }
                cmd @ (
                    BrokerCmd::AlpacaFetchBars { .. }
                    | BrokerCmd::AlpacaFetchBarsBatch { .. }
                    | BrokerCmd::FetchAllBars { .. }
                    | BrokerCmd::KrakenBackfill { .. }
                    | BrokerCmd::KrakenFuturesBackfill { .. }
                ) => {
                    bar_fetch_commands::handle_bar_fetch_command(
                        cmd,
                        broker.as_ref(),
                        &broker_msg_tx_clone,
                        shared_cache_broker.clone(),
                        alpaca_fetch_permits.clone(),
                        kraken_fetch_permits.clone(),
                        kraken_public_client.clone(),
                    )
                    .await;
                }
                cmd @ (BrokerCmd::FetchFilingContent { .. }
                | BrokerCmd::IgnoreNewsArticle { .. }) => {
                    news::handle_news_maintenance_command(
                        cmd,
                        &broker_msg_tx_clone,
                        shared_cache_broker.clone(),
                    )
                    .await;
                }
                cmd @ (
                    BrokerCmd::FredFetch { .. }
                    | BrokerCmd::FetchEconCalendar { .. }
                    | BrokerCmd::FetchCongressTrades
                    | BrokerCmd::SendNotification { .. }
                ) => {
                    external_feeds::handle_external_feed_command(cmd, broker_msg_tx_clone.clone())
                        .await;
                }
            }
        }
    });
}
