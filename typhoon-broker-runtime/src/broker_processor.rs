use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};

use typhoon_engine::broker::protocol::{BrokerCmd, BrokerMsg, OrderBroker};
use typhoon_engine::core::cache::SqliteCache;

use crate::account_pool::{AlpacaAccountPool, KrakenAccountPool};
use crate::resources::BrokerRuntimeResources;
use crate::{
    ai_chat, alpaca_account_data, alpaca_order_ops, alpaca_ws_commands, bar_fetch_commands,
    connection_commands, external_feeds, fundamentals_commands, kraken_market_commands,
    kraken_order_ops, kraken_ws_commands, market_data_commands, matrix_commands, misc_commands,
    news, research_compute, research_fetch, storage, symbol_search, watchlist_quotes,
};

pub fn spawn_broker_message_processor(
    broker_cmd_rx: tokio::sync::mpsc::UnboundedReceiver<BrokerCmd>,
    broker_msg_tx: tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
    importing_flag: Arc<AtomicBool>,
    rt_handle: tokio::runtime::Handle,
    shared_cache: Arc<RwLock<Option<Arc<SqliteCache>>>>,
) {
    // Spawn broker message processor
    let broker_msg_tx_clone = broker_msg_tx.clone();
    let importing_flag_broker = importing_flag.clone();
    let shared_cache_broker = shared_cache.clone();
    rt_handle.spawn(async move {
        let mut cmd_rx = broker_cmd_rx;
        // Multi-account pools (ADR-130): the primary account serves trading /
        // account-data commands; every data-sync-enabled account joins the
        // historical bar-fetch rotation.
        let mut alpaca_pool = AlpacaAccountPool::default();
        let mut kraken_pool = KrakenAccountPool::default();
        // Control sender for the Alpaca market-data WS (push the live subscription
        // set). Held across commands so the single connection is reused.
        let mut alpaca_quote_control: Option<tokio::sync::mpsc::Sender<Vec<String>>> = None;
        // Trade-updates WS forwarder for the current primary account; aborted
        // and restarted on a primary switch so fills of the old account stop
        // overwriting the new account's state.
        let mut alpaca_trade_stream_task: Option<tokio::task::JoinHandle<()>> = None;
        let mut kraken_ws_broker: Option<typhoon_engine::broker::kraken::KrakenBroker> = None;
        // Private ownTrades/openOrders WS reader for the current Kraken
        // primary; aborted and restarted on a primary switch so the stream
        // re-authenticates to the new account (ADR-130).
        let mut kraken_private_ws_task: Option<tokio::task::JoinHandle<()>> = None;
        // Pre-acquire and per-endpoint spacing are now owned by the
        // engine-side `iapi_limiter` (token bucket + escalating backoff,
        // shared across all iapi endpoints). The handler below just
        // delegates to it instead of maintaining its own gate state.
        let importing_flag = importing_flag_broker;
        let runtime_resources = BrokerRuntimeResources::new();
        let mut alpaca_fetch_permits = runtime_resources.alpaca_fetch_permits;
        let yahoo_chart_fetch_permits = runtime_resources.yahoo_chart_fetch_permits;
        let kraken_fetch_permits = runtime_resources.kraken_fetch_permits;
        // Kraken Securities/iapi history is slower and can include synchronous cache work.
        // Keep it off the broker command loop and cap it separately so broad equities
        // sync cannot starve UI-visible broker messages (SEC scanner, order state, etc.).
        let kraken_equity_fetch_permits = runtime_resources.kraken_equity_fetch_permits;
        let kraken_public_client = runtime_resources.kraken_public_client;
        let fallback_bar_client = runtime_resources.fallback_bar_client;
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                cmd @ (BrokerCmd::Connect { .. } | BrokerCmd::ConfigureAlpacaSync { .. }) => {
                    connection_commands::handle_connection_command(
                        cmd,
                        &mut alpaca_pool,
                        &mut kraken_pool,
                        &mut kraken_ws_broker,
                        &mut alpaca_fetch_permits,
                        &broker_msg_tx_clone,
                    )
                    .await;
                }
                cmd @ BrokerCmd::SetPrimaryAccount { .. } => {
                    let is_alpaca = matches!(
                        cmd,
                        BrokerCmd::SetPrimaryAccount {
                            broker: OrderBroker::Alpaca,
                            ..
                        }
                    );
                    let prior_kraken_primary =
                        kraken_pool.primary_id().map(|id| id.to_string());
                    connection_commands::handle_connection_command(
                        cmd,
                        &mut alpaca_pool,
                        &mut kraken_pool,
                        &mut kraken_ws_broker,
                        &mut alpaca_fetch_permits,
                        &broker_msg_tx_clone,
                    )
                    .await;
                    if is_alpaca {
                        // Trade stream must follow the new primary; the old
                        // account's stream would otherwise keep re-emitting the
                        // old positions/orders on every fill.
                        if let Some(task) = alpaca_trade_stream_task.take() {
                            task.abort();
                        }
                        alpaca_trade_stream_task = alpaca_ws_commands::handle_alpaca_ws_command(
                            BrokerCmd::AlpacaStartTradeStream,
                            alpaca_pool.primary_broker().cloned(),
                            &broker_msg_tx_clone,
                        )
                        .await;
                    } else if kraken_pool.primary_id().map(|id| id.to_string())
                        != prior_kraken_primary
                    {
                        // The private ownTrades/openOrders WS must follow the
                        // new Kraken primary too (ADR-130 follow-on-switch):
                        // abort the old reader and re-authenticate with the
                        // rebuilt WS-token broker. Only restart if a private
                        // WS was running — starting it remains an explicit
                        // action on connect.
                        if let Some(task) = kraken_private_ws_task.take() {
                            task.abort();
                            match kraken_ws_broker.as_ref() {
                                Some(kb) => {
                                    match kraken_ws_commands::spawn_kraken_private_ws_reader(
                                        kb,
                                        &broker_msg_tx_clone,
                                    )
                                    .await
                                    {
                                        Ok(handle) => {
                                            kraken_private_ws_task = Some(handle);
                                            let _ = broker_msg_tx_clone.send(
                                                BrokerMsg::OrderResult(
                                                    "Kraken private WS re-authenticated to the new primary account"
                                                        .into(),
                                                ),
                                            );
                                        }
                                        Err(e) => {
                                            let _ = broker_msg_tx_clone.send(BrokerMsg::Error(
                                                format!(
                                                    "Kraken private WS restart for new primary failed: {}",
                                                    e
                                                ),
                                            ));
                                        }
                                    }
                                }
                                None => {
                                    let _ = broker_msg_tx_clone.send(BrokerMsg::Error(
                                        "Kraken private WS stopped — no WS credentials for the new primary"
                                            .into(),
                                    ));
                                }
                            }
                        }
                    }
                }
                BrokerCmd::SetOrderMirroring {
                    enabled,
                    target_ids,
                } => {
                    let n_targets = target_ids.len();
                    alpaca_pool.set_mirror_orders(enabled, target_ids);
                    let msg = if alpaca_pool.mirror_orders() {
                        format!(
                            "TradeCopy live mirroring ENABLED — app-placed Alpaca orders replicate to {} opted-in account(s)",
                            n_targets
                        )
                    } else if enabled {
                        "TradeCopy live mirroring stays OFF — no target accounts opted in".to_string()
                    } else {
                        "TradeCopy live mirroring disabled".to_string()
                    };
                    let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(msg));
                }
                BrokerCmd::KrakenTradeCopy {
                    source_id,
                    target_ids,
                    flatten_extra,
                } => {
                    kraken_order_ops::handle_kraken_trade_copy(
                        source_id,
                        target_ids,
                        flatten_extra,
                        &kraken_pool,
                        &broker_msg_tx_clone,
                    )
                    .await;
                }
                BrokerCmd::AlpacaTradeCopy {
                    source_id,
                    target_ids,
                    flatten_extra,
                } => {
                    alpaca_order_ops::handle_alpaca_trade_copy(
                        source_id,
                        target_ids,
                        flatten_extra,
                        &alpaca_pool,
                        &broker_msg_tx_clone,
                    )
                    .await;
                }
                cmd @ BrokerCmd::MarkUnresolvable { .. } => {
                    misc_commands::handle_misc_command(
                        cmd,
                        alpaca_pool.primary_broker(),
                        &broker_msg_tx_clone,
                    )
                    .await;
                }
                cmd @ (BrokerCmd::GetAccount | BrokerCmd::GetOrders | BrokerCmd::GetOrderHistory { .. }) => {
                    alpaca_account_data::handle_alpaca_account_data_command(
                        cmd,
                        alpaca_pool.primary_broker(),
                        &broker_msg_tx_clone,
                    )
                    .await;
                }
                BrokerCmd::GetPositions => {
                    alpaca_account_data::fetch_and_send_all_account_positions(
                        &alpaca_pool,
                        &broker_msg_tx_clone,
                    )
                    .await;
                }
                cmd @ (BrokerCmd::CloseAll
                | BrokerCmd::ClosePosition { .. }
                | BrokerCmd::AlpacaClosePositionPercent { .. }) => {
                    alpaca_order_ops::handle_alpaca_order_command(
                        cmd,
                        &alpaca_pool,
                        &broker_msg_tx_clone,
                    )
                    .await;
                }
                cmd @ BrokerCmd::AlpacaStartTradeStream => {
                    if let Some(task) = alpaca_trade_stream_task.take() {
                        task.abort();
                    }
                    alpaca_trade_stream_task = alpaca_ws_commands::handle_alpaca_ws_command(
                        cmd,
                        alpaca_pool.primary_broker().cloned(),
                        &broker_msg_tx_clone,
                    )
                    .await;
                }
                BrokerCmd::AlpacaStreamQuotes { symbols } => {
                    // Start the single market-data WS on first use, then keep
                    // pushing the live subscription set to it.
                    if alpaca_quote_control.is_none() {
                        if let Some(b) = alpaca_pool.primary_broker().cloned() {
                            alpaca_quote_control = alpaca_ws_commands::start_alpaca_quote_stream(
                                b,
                                &broker_msg_tx_clone,
                            )
                            .await;
                        }
                    }
                    if let Some(ctrl) = alpaca_quote_control.as_ref() {
                        if ctrl.send(symbols).await.is_err() {
                            alpaca_quote_control = None; // task gone — allow a restart
                        }
                    }
                }
                cmd @ (BrokerCmd::SecScrape { .. } | BrokerCmd::FinnhubNews { .. }) => {
                    news::handle_news_command(
                        cmd,
                        broker_msg_tx_clone.clone(),
                        shared_cache_broker.clone(),
                    );
                }
                cmd @ BrokerCmd::GetQuote { .. } => {
                    misc_commands::handle_misc_command(cmd, alpaca_pool.primary_broker(), &broker_msg_tx_clone)
                        .await;
                }
                BrokerCmd::GetWatchlistQuotes { symbols } => {
                    watchlist_quotes::spawn_watchlist_quotes_task(
                        symbols,
                        alpaca_pool.primary_broker().cloned(),
                        broker_msg_tx_clone.clone(),
                        shared_cache_broker.clone(),
                    );
                }
                cmd @ BrokerCmd::GetMarketClock => {
                    misc_commands::handle_misc_command(cmd, alpaca_pool.primary_broker(), &broker_msg_tx_clone)
                        .await;
                }
                cmd @ (BrokerCmd::GetActivities { .. }
                | BrokerCmd::GetTopMovers
                | BrokerCmd::GetAllAssets) => {
                    alpaca_account_data::handle_alpaca_account_data_command(
                        cmd,
                        alpaca_pool.primary_broker(),
                        &broker_msg_tx_clone,
                    )
                    .await;
                }
                BrokerCmd::SearchSymbols { query } => {
                    symbol_search::handle_symbol_search_command(
                        query,
                        alpaca_pool.primary_broker(),
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
                | BrokerCmd::UpdateWatchlist { .. }
                | BrokerCmd::AddWatchlistSymbol { .. }
                | BrokerCmd::RemoveWatchlistSymbol { .. }
                | BrokerCmd::DeleteWatchlist { .. }
                | BrokerCmd::GetOptionsChain { .. }) => {
                    market_data_commands::handle_market_data_command(
                        cmd,
                        alpaca_pool.primary_broker(),
                        kraken_pool.primary_broker(),
                        &shared_cache_broker,
                        &broker_msg_tx_clone,
                    )
                    .await;
                }
                cmd @ (BrokerCmd::AlpacaMarketOrder { .. }
                | BrokerCmd::AlpacaMarketOrderNotional { .. }
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
                        &alpaca_pool,
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
                        kraken_pool.primary_broker(),
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
                cmd @ (BrokerCmd::FetchCompanyProfile { .. }
                | BrokerCmd::FetchStockPeers { .. }
                | BrokerCmd::FetchEarningsHistory { .. }
                | BrokerCmd::FetchIpoCalendar { .. }
                | BrokerCmd::FetchPressReleases { .. }
                | BrokerCmd::FetchSocialSentiment { .. }
                | BrokerCmd::FetchStockTwitsSentiment { .. }
                | BrokerCmd::FetchRedditMentions { .. }
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
                | BrokerCmd::FetchBetaSnapshot { .. }) => {
                    research_fetch::handle_research_fetch_command(cmd, broker_msg_tx_clone.clone());
                }
                cmd @ (BrokerCmd::ComputeDdmSnapshot { .. }
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
                | BrokerCmd::ComputeKupiecPofSnapshot { .. }) => {
                    research_compute::handle_research_compute_command(
                        cmd,
                        broker_msg_tx_clone.clone(),
                        shared_cache_broker.clone(),
                    );
                }
                cmd @ (BrokerCmd::IngestResearchArticles { .. }
                | BrokerCmd::FetchNewsMulti { .. }
                | BrokerCmd::LoadCachedNews { .. }
                | BrokerCmd::HydrateNewsArticle { .. }
                | BrokerCmd::SearchNews { .. }
                | BrokerCmd::NewsScrapeSymbols { .. }) => {
                    news::handle_news_command(
                        cmd,
                        broker_msg_tx_clone.clone(),
                        shared_cache_broker.clone(),
                    );
                }
                cmd @ BrokerCmd::NewsScrapeAll { .. } => {
                    news::handle_news_scrape_all_command(
                        cmd,
                        alpaca_pool.primary_broker(),
                        &broker_msg_tx_clone,
                        shared_cache_broker.clone(),
                    )
                    .await;
                }
                cmd @ BrokerCmd::KrakenConnect { .. } => {
                    connection_commands::handle_connection_command(
                        cmd,
                        &mut alpaca_pool,
                        &mut kraken_pool,
                        &mut kraken_ws_broker,
                        &mut alpaca_fetch_permits,
                        &broker_msg_tx_clone,
                    )
                    .await;
                }
                cmd @ (BrokerCmd::KrakenGetBalance
                | BrokerCmd::KrakenGetPositions
                | BrokerCmd::KrakenPlaceOrder { .. }
                | BrokerCmd::KrakenPlaceOrderAdvanced { .. }
                | BrokerCmd::KrakenClosePosition { .. }
                | BrokerCmd::KrakenCancelOrder { .. }
                | BrokerCmd::KrakenCancelAll
                | BrokerCmd::KrakenFetchTrades
                | BrokerCmd::KrakenFetchOpenOrders) => {
                    kraken_order_ops::handle_kraken_account_order_command(
                        cmd,
                        kraken_pool.primary_broker(),
                        &broker_msg_tx_clone,
                    )
                    .await;
                }
                cmd @ (BrokerCmd::KrakenFetchEquityTicker { .. }
                | BrokerCmd::KrakenFetchEquityHistory { .. }
                | BrokerCmd::YahooChartFetchBars { .. }
                | BrokerCmd::KrakenFetchEquityUniverse) => {
                    kraken_market_commands::handle_kraken_market_command(
                        cmd,
                        kraken_pool.primary_broker(),
                        &broker_msg_tx_clone,
                        shared_cache_broker.clone(),
                        kraken_equity_fetch_permits.clone(),
                        yahoo_chart_fetch_permits.clone(),
                        kraken_public_client.clone(),
                        fallback_bar_client.clone(),
                    )
                    .await;
                }
                cmd @ (BrokerCmd::KrakenStartPrivateWs
                | BrokerCmd::KrakenStartOhlcStreamers { .. }
                | BrokerCmd::KrakenOhlcSnapshotSweep { .. }
                | BrokerCmd::KrakenStartOrderbookWs { .. }
                | BrokerCmd::KrakenStartTickerWs { .. }
                | BrokerCmd::KrakenStartLevel3Ws { .. }) => {
                    kraken_ws_commands::handle_kraken_ws_command(
                        cmd,
                        kraken_pool.primary_broker(),
                        kraken_ws_broker.as_ref(),
                        &mut kraken_private_ws_task,
                        &broker_msg_tx_clone,
                        shared_cache_broker.clone(),
                        kraken_public_client.clone(),
                    )
                    .await;
                }
                cmd @ (BrokerCmd::KrakenCloseAll | BrokerCmd::KrakenGetPairs) => {
                    kraken_order_ops::handle_kraken_account_order_command(
                        cmd,
                        kraken_pool.primary_broker(),
                        &broker_msg_tx_clone,
                    )
                    .await;
                }
                cmd @ BrokerCmd::KrakenFuturesGetInstruments => {
                    kraken_market_commands::handle_kraken_market_command(
                        cmd,
                        kraken_pool.primary_broker(),
                        &broker_msg_tx_clone,
                        shared_cache_broker.clone(),
                        kraken_equity_fetch_permits.clone(),
                        yahoo_chart_fetch_permits.clone(),
                        kraken_public_client.clone(),
                        fallback_bar_client.clone(),
                    )
                    .await;
                }
                cmd @ (BrokerCmd::FundamentalsScrape { .. }
                | BrokerCmd::FundamentalsScrapeOne { .. }
                | BrokerCmd::ResearchScrape { .. }) => {
                    fundamentals_commands::handle_fundamentals_command(
                        cmd,
                        alpaca_pool.primary_broker(),
                        &broker_msg_tx_clone,
                        shared_cache_broker.clone(),
                    )
                    .await;
                }
                cmd @ (BrokerCmd::CompactStorage { .. } | BrokerCmd::ScanUnusualVolume { .. }) => {
                    storage::handle_storage_command(
                        cmd,
                        broker_msg_tx_clone.clone(),
                        importing_flag.clone(),
                        shared_cache_broker.clone(),
                    );
                }
                cmd @ (BrokerCmd::AlpacaFetchBars { .. }
                | BrokerCmd::AlpacaFetchBarsBatch { .. }
                | BrokerCmd::FetchAllBars { .. }
                | BrokerCmd::KrakenBackfill { .. }
                | BrokerCmd::KrakenFuturesBackfill { .. }) => {
                    bar_fetch_commands::handle_bar_fetch_command(
                        cmd,
                        &alpaca_pool,
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
                cmd @ (BrokerCmd::FredFetch { .. }
                | BrokerCmd::FetchEconCalendar { .. }
                | BrokerCmd::FetchCongressTrades
                | BrokerCmd::SendNotification { .. }) => {
                    external_feeds::handle_external_feed_command(cmd, broker_msg_tx_clone.clone())
                        .await;
                }
            }
        }
    });
}
