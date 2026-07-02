use super::*;
use crate::app::app_runtime_support::{
    broker_msg_kind, is_routine_news_progress, json_result_card_from_text,
    should_emit_alpaca_retry_queue_log,
};

impl TyphooNApp {
    pub(crate) fn tick_broker_messages(
        &mut self,
        ctx: &egui::Context,
        now_instant: std::time::Instant,
    ) -> (f64, f64, std::time::Instant, usize) {
        let perf_pre_broker_ms;
        let perf_broker_drain_ms;
        let perf_after_broker_started;
        // ── poll async broker messages ───────────────────────────────────
        perf_pre_broker_ms = now_instant.elapsed().as_secs_f64() * 1000.0;
        // Cap drain per frame so a flood of messages can't stall the render thread.
        // Anything left over waits for next frame; we repaint immediately in that case.
        let mut msgs_drained = 0usize;
        let broker_drain_max = 48;
        let broker_drain_started = std::time::Instant::now();
        let broker_drain_budget = if self.heavy_sync_in_progress {
            std::time::Duration::from_millis(4)
        } else {
            std::time::Duration::from_millis(8)
        };
        let mut market_data_refill_requested = false;
        while msgs_drained < broker_drain_max
            && broker_drain_started.elapsed() < broker_drain_budget
            && let Ok(msg) = self.broker_rx.try_recv()
        {
            msgs_drained += 1;
            let msg_kind = broker_msg_kind(&msg);
            let msg_started = std::time::Instant::now();
            match msg {
                BrokerMsg::Connected(s) => {
                    self.handle_broker_connected(s);
                }
                BrokerMsg::KrakenTrades(trades) => {
                    self.handle_kraken_trades(trades);
                }
                BrokerMsg::KrakenLiveTrade(trade) => {
                    self.handle_kraken_live_trade(trade);
                }
                BrokerMsg::KrakenOpenOrders(orders) => {
                    self.handle_kraken_open_orders(orders);
                }
                BrokerMsg::KrakenWsStatus { status, message } => {
                    self.handle_kraken_ws_status(status, message);
                }
                BrokerMsg::KrakenOrderbookUpdate(text) => {
                    let was_empty = self.orderbook_result.is_empty();
                    self.orderbook_result = text.clone();
                    self.show_orderbook_window = true;
                    if was_empty {
                        self.log
                            .push_back(LogEntry::info("Kraken orderbook WS: live depth streaming"));
                    }

                    // Full book depth binning: extract top levels from L2/L3 snapshot and push to matching charts
                    // so the chart depth profile overlay shows binned volume-at-price from the live book (not just top).
                    // MTF parity: applies to all open charts including MTF Grid for the bare symbol.
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                        let sym = v["symbol"].as_str().unwrap_or("");
                        let bare = bare_symbol_from_key(sym)
                            .replace('/', "")
                            .trim_end_matches(".EQ")
                            .trim_end_matches(".eq")
                            .to_ascii_uppercase();
                        if let Some(idxs) = self.chart_by_bare.get(&bare) {
                            let bids: Vec<(f64, f64)> = v["bids"].as_array().map(|arr| {
                                arr.iter().filter_map(|l| {
                                    let p = l["price"].as_f64().or_else(|| l["limit_price"].as_f64()).unwrap_or(0.0);
                                    let s = l["size"].as_f64().or_else(|| l["order_qty"].as_f64()).unwrap_or(0.0);
                                    if p > 0.0 && s > 0.0 { Some((p, s)) } else { None }
                                }).take(25).collect()
                            }).unwrap_or_default();
                            let asks: Vec<(f64, f64)> = v["asks"].as_array().map(|arr| {
                                arr.iter().filter_map(|l| {
                                    let p = l["price"].as_f64().or_else(|| l["limit_price"].as_f64()).unwrap_or(0.0);
                                    let s = l["size"].as_f64().or_else(|| l["order_qty"].as_f64()).unwrap_or(0.0);
                                    if p > 0.0 && s > 0.0 { Some((p, s)) } else { None }
                                }).take(25).collect()
                            }).unwrap_or_default();
                            for &i in idxs {
                                if let Some(chart) = self.charts.get_mut(i) {
                                    chart.live_depth_bids = bids.clone();
                                    chart.live_depth_asks = asks.clone();
                                }
                            }
                        }
                    }
                }
                BrokerMsg::KrakenBookQuoteTick { symbol, bid, ask, bid_size, ask_size } => {
                    self.handle_kraken_book_quote_tick(symbol, bid, ask, bid_size, ask_size);
                }
                BrokerMsg::KrakenWsTicker(t) => {
                    self.handle_kraken_ws_ticker(t);
                }
                BrokerMsg::KrakenWsBarsCommitted { fresh } => {
                    self.handle_kraken_ws_bars_committed(fresh);
                }
                BrokerMsg::KrakenWsOhlcStatus {
                    interval_min,
                    kind,
                    detail,
                } => {
                    self.handle_kraken_ws_ohlc_status(interval_min, kind, detail);
                }
                BrokerMsg::KrakenWsOhlcSnapshotSweepSettled {
                    interval_min,
                    pair_count,
                    error,
                } => {
                    self.handle_kraken_ws_ohlc_snapshot_sweep_settled(
                        interval_min,
                        pair_count,
                        error,
                    );
                }
                BrokerMsg::Error(e) => {
                    let now = chrono::Utc::now().timestamp();
                    self.handle_broker_error(e, now);
                }
                BrokerMsg::Unresolvable {
                    broker,
                    symbol,
                    timeframe,
                    reason,
                } => {
                    self.unresolvable_mark(&broker, &symbol, &timeframe, &reason);
                }
                BrokerMsg::Account(acct) => {
                    self.handle_alpaca_account(acct);
                }
                BrokerMsg::AccountRoster { broker, accounts } => {
                    self.handle_account_roster(broker, accounts);
                }
                BrokerMsg::Positions(pos) => {
                    self.handle_alpaca_positions(pos);
                }
                BrokerMsg::AllAssets(assets) => {
                    self.handle_alpaca_all_assets(assets);
                }
                BrokerMsg::RecentFills(fills) => {
                    self.handle_alpaca_recent_fills(fills);
                }
                BrokerMsg::BarsSynced(changed) => {
                    // Reload every open chart to pick up newly-synced bars. This is
                    // intentionally not gated on MTF mode: inactive top tabs should
                    // precompute too, so switching tabs is never the load trigger.
                    if changed > 0 {
                        for i in 0..self.charts.len() {
                            self.queue_chart_reload(i);
                        }
                    }
                }
                BrokerMsg::KrakenPositions(pos) => {
                    self.handle_kraken_positions(pos);
                }
                BrokerMsg::Orders(orders) => {
                    self.handle_alpaca_orders(orders);
                }
                BrokerMsg::OrderResult(msg) => {
                    self.handle_order_result(msg);
                }
                msg @ (BrokerMsg::SecScrapeResult(_)
                | BrokerMsg::FilingContent(_)
                | BrokerMsg::FinnhubNewsResult(_)) => {
                    self.handle_news_sec_result_msg(msg);
                }
                BrokerMsg::KrakenEquityUniverse(markets) => {
                    market_data_refill_requested |= self.handle_kraken_equity_universe(markets);
                }
                BrokerMsg::KrakenEquityQuote(ticker) => {
                    self.handle_kraken_equity_quote(ticker);
                }
                BrokerMsg::KrakenEquityBars {
                    symbol,
                    timeframe,
                    count,
                } => {
                    market_data_refill_requested |=
                        self.handle_kraken_equity_bars(symbol, timeframe, count);
                }
                BrokerMsg::KrakenEquityHistoryError {
                    symbol,
                    timeframe,
                    error,
                } => {
                    market_data_refill_requested |=
                        self.handle_kraken_equity_history_error(symbol, timeframe, error);
                }
                BrokerMsg::Quote(symbol, bid, ask, last) => {
                    self.handle_broker_quote(symbol, bid, ask, last);
                }
                BrokerMsg::AlpacaQuote(q) => {
                    self.handle_alpaca_quote(q);
                }
                BrokerMsg::AlpacaMarketDataFeed(feed) => {
                    self.alpaca_market_data_feed = Some(feed);
                    // Successful (re)connect: clear any prior limit backoff
                    self.alpaca_sub_limit_hit_at = None;
                }
                BrokerMsg::WatchlistQuotes(rows) => {
                    self.handle_watchlist_quotes(rows);
                }
                BrokerMsg::KrakenBalances(balances) => {
                    self.handle_kraken_balances(balances);
                }
                BrokerMsg::KrakenPairs(pairs) => {
                    self.handle_kraken_pairs(pairs);
                }
                BrokerMsg::KrakenFuturesInstruments(symbols) => {
                    self.handle_kraken_futures_instruments(symbols);
                }
                msg @ (BrokerMsg::CryptoTop50(_)
                | BrokerMsg::FredData(_, _)
                | BrokerMsg::EconCalendarData(_)
                | BrokerMsg::CongressData(_)) => {
                    self.handle_macro_alt_data_msg(msg);
                }
                msg @ (BrokerMsg::CompanyProfile(_)
                | BrokerMsg::StockPeers(_, _)
                | BrokerMsg::EarningsHistory(_, _)
                | BrokerMsg::IpoCalendar(_)
                | BrokerMsg::PressReleases(_, _)
                | BrokerMsg::SocialSentiment(_, _)
                | BrokerMsg::StockTwitsSentiment(_, _)
                | BrokerMsg::TranscriptList(_, _)
                | BrokerMsg::TranscriptBody(_)
                | BrokerMsg::CommoditiesQuotes(_)
                | BrokerMsg::DividendHistory(_, _)
                | BrokerMsg::EarningsEstimates(_, _)
                | BrokerMsg::RatingChanges(_, _)
                | BrokerMsg::TreasuryYields(_)
                | BrokerMsg::FinancialStatementsMsg(_, _)
                | BrokerMsg::Executives(_, _)
                | BrokerMsg::CotReports(_)
                | BrokerMsg::StockSplitsMsg(_, _)
                | BrokerMsg::EtfHoldingsMsg(_, _)
                | BrokerMsg::AnalystRecsMsg(_, _)
                | BrokerMsg::PriceTargetMsg(_, _)
                | BrokerMsg::EsgScoresMsg(_, _)
                | BrokerMsg::IndexMembersMsg(_, _)
                | BrokerMsg::InsiderTradesMsg(_, _)
                | BrokerMsg::InstitutionalHoldersMsg(_, _)
                | BrokerMsg::SharesFloatMsg(_, _)
                | BrokerMsg::HistoricalPriceMsg(_, _)
                | BrokerMsg::EarningsSurpriseMsg(_, _)) => {
                    self.handle_research_core_msg(msg);
                }
                msg @ (BrokerMsg::WorldIndicesMsg(_)
                | BrokerMsg::MarketMoversMsg(_)
                | BrokerMsg::SectorPerformanceMsg(_)
                | BrokerMsg::WaccSnapshotMsg(_, _)
                | BrokerMsg::CurrencyRatesMsg(_)
                | BrokerMsg::BetaSnapshotMsg(_, _)
                | BrokerMsg::DdmSnapshotMsg(_, _)
                | BrokerMsg::RelativeValuationMsg(_, _)
                | BrokerMsg::FigiSnapshotMsg(_, _)
                | BrokerMsg::HraSnapshotMsg(_, _)
                | BrokerMsg::DcfSnapshotMsg(_, _)
                | BrokerMsg::SvmSnapshotMsg(_, _)
                | BrokerMsg::OptionsChainMsg(_, _)
                | BrokerMsg::IvolSnapshotMsg(_, _)
                | BrokerMsg::SeasonalitySnapshotMsg(_, _)
                | BrokerMsg::CorrelationMatrixMsg(_, _)
                | BrokerMsg::TotalReturnSnapshotMsg(_, _)
                | BrokerMsg::TechnicalsSnapshotMsg(_, _)
                | BrokerMsg::VolSkewSnapshotMsg(_, _)) => {
                    self.handle_research_macro_valuation_msg(msg);
                }
                msg @ (BrokerMsg::LeverageSnapshotMsg(_, _)
                | BrokerMsg::AccrualsSnapshotMsg(_, _)
                | BrokerMsg::RealizedVolSnapshotMsg(_, _)
                | BrokerMsg::FcfYieldSnapshotMsg(_, _)
                | BrokerMsg::ShortInterestSnapshotMsg(_, _)
                | BrokerMsg::AltmanZSnapshotMsg(_, _)
                | BrokerMsg::PiotroskiSnapshotMsg(_, _)
                | BrokerMsg::OhlcVolSnapshotMsg(_, _)
                | BrokerMsg::EpsBeatSnapshotMsg(_, _)
                | BrokerMsg::PriceTargetDispersionSnapshotMsg(_, _)
                | BrokerMsg::InsiderActivitySnapshotMsg(_, _)
                | BrokerMsg::DivgSnapshotMsg(_, _)
                | BrokerMsg::EarmSnapshotMsg(_, _)
                | BrokerMsg::SectorRotationSnapshotMsg(_, _)
                | BrokerMsg::UpdmSnapshotMsg(_, _)
                | BrokerMsg::MomentumSnapshotMsg(_, _)
                | BrokerMsg::LiquiditySnapshotMsg(_, _)
                | BrokerMsg::BreakoutSnapshotMsg(_, _)
                | BrokerMsg::CashCycleSnapshotMsg(_, _)
                | BrokerMsg::CreditSnapshotMsg(_, _)
                | BrokerMsg::GrowmSnapshotMsg(_, _)
                | BrokerMsg::FlowSnapshotMsg(_, _)
                | BrokerMsg::RegimeSnapshotMsg(_, _)
                | BrokerMsg::RelvolSnapshotMsg(_, _)
                | BrokerMsg::MarginsSnapshotMsg(_, _)
                | BrokerMsg::ValSnapshotMsg(_, _)
                | BrokerMsg::QualSnapshotMsg(_, _)
                | BrokerMsg::RiskSnapshotMsg(_, _)
                | BrokerMsg::InsstrkSnapshotMsg(_, _)
                | BrokerMsg::CovgSnapshotMsg(_, _)
                | BrokerMsg::VrkSnapshotMsg(_, _)
                | BrokerMsg::QrkSnapshotMsg(_, _)
                | BrokerMsg::RrkSnapshotMsg(_, _)
                | BrokerMsg::RelepsgrSnapshotMsg(_, _)
                | BrokerMsg::PeadSnapshotMsg(_, _)) => {
                    self.handle_research_snapshot_msg(msg);
                }
                msg @ (BrokerMsg::SizefSnapshotMsg(_, _)
                | BrokerMsg::MomfSnapshotMsg(_, _)
                | BrokerMsg::PeadrankSnapshotMsg(_, _)
                | BrokerMsg::FqmSnapshotMsg(_, _)
                | BrokerMsg::RevrankSnapshotMsg(_, _)
                | BrokerMsg::LevrankSnapshotMsg(_, _)
                | BrokerMsg::OperankSnapshotMsg(_, _)
                | BrokerMsg::FqmrankSnapshotMsg(_, _)
                | BrokerMsg::LiqrankSnapshotMsg(_, _)
                | BrokerMsg::SurpstkSnapshotMsg(_, _)
                | BrokerMsg::DvdrankSnapshotMsg(_, _)
                | BrokerMsg::EarmrankSnapshotMsg(_, _)
                | BrokerMsg::UpdgrankSnapshotMsg(_, _)
                | BrokerMsg::GySnapshotMsg(_, _)
                | BrokerMsg::DesSnapshotMsg(_, _)
                | BrokerMsg::DvdyieldrankSnapshotMsg(_, _)
                | BrokerMsg::ShrankSnapshotMsg(_, _)
                | BrokerMsg::ShortrankDeltaSnapshotMsg(_, _)
                | BrokerMsg::InsiderconcSnapshotMsg(_, _)
                | BrokerMsg::AtrannSnapshotMsg(_, _)
                | BrokerMsg::DdhistSnapshotMsg(_, _)
                | BrokerMsg::PriceperfSnapshotMsg(_, _)
                | BrokerMsg::MomrankMultiSnapshotMsg(_, _)
                | BrokerMsg::BetarankSnapshotMsg(_, _)
                | BrokerMsg::PegrankSnapshotMsg(_, _)
                | BrokerMsg::FhighlowSnapshotMsg(_, _)
                | BrokerMsg::RvconeSnapshotMsg(_, _)
                | BrokerMsg::CalpbSnapshotMsg(_, _)
                | BrokerMsg::CorrstkSnapshotMsg(_, _)
                | BrokerMsg::TlrankSnapshotMsg(_, _)
                | BrokerMsg::CorrrankSnapshotMsg(_, _)
                | BrokerMsg::OperankDeltaSnapshotMsg(_, _)
                | BrokerMsg::DivaccSnapshotMsg(_, _)
                | BrokerMsg::EpsaccSnapshotMsg(_, _)
                | BrokerMsg::VrpSnapshotMsg(_, _)
                | BrokerMsg::RetskewSnapshotMsg(_, _)
                | BrokerMsg::RetkurtSnapshotMsg(_, _)
                | BrokerMsg::TailrSnapshotMsg(_, _)
                | BrokerMsg::RunlenSnapshotMsg(_, _)
                | BrokerMsg::DayrangeSnapshotMsg(_, _)
                | BrokerMsg::AutocorSnapshotMsg(_, _)
                | BrokerMsg::HurstSnapshotMsg(_, _)
                | BrokerMsg::HitrateSnapshotMsg(_, _)
                | BrokerMsg::GlasymSnapshotMsg(_, _)
                | BrokerMsg::VolratioSnapshotMsg(_, _)
                | BrokerMsg::DrawupSnapshotMsg(_, _)
                | BrokerMsg::GapstatsSnapshotMsg(_, _)
                | BrokerMsg::VolclusterSnapshotMsg(_, _)
                | BrokerMsg::CloseplcSnapshotMsg(_, _)
                | BrokerMsg::MrhlSnapshotMsg(_, _)
                | BrokerMsg::DownvolSnapshotMsg(_, _)
                | BrokerMsg::SharprSnapshotMsg(_, _)
                | BrokerMsg::EffratioSnapshotMsg(_, _)
                | BrokerMsg::WickbiasSnapshotMsg(_, _)
                | BrokerMsg::VolofvolSnapshotMsg(_, _)) => {
                    self.handle_research_rank_snapshot_msg(msg);
                }
                msg @ (BrokerMsg::CalmarSnapshotMsg(_, _)
                | BrokerMsg::UlcerSnapshotMsg(_, _)
                | BrokerMsg::VarratioSnapshotMsg(_, _)
                | BrokerMsg::AmihudSnapshotMsg(_, _)
                | BrokerMsg::JbnormSnapshotMsg(_, _)
                | BrokerMsg::OmegaSnapshotMsg(_, _)
                | BrokerMsg::DfaSnapshotMsg(_, _)
                | BrokerMsg::BurkeSnapshotMsg(_, _)
                | BrokerMsg::MonthseasSnapshotMsg(_, _)
                | BrokerMsg::RollsprdSnapshotMsg(_, _)
                | BrokerMsg::ParkinsonSnapshotMsg(_, _)
                | BrokerMsg::GkvolSnapshotMsg(_, _)
                | BrokerMsg::RsvolSnapshotMsg(_, _)
                | BrokerMsg::CvarSnapshotMsg(_, _)
                | BrokerMsg::DoweffectSnapshotMsg(_, _)
                | BrokerMsg::SterlingSnapshotMsg(_, _)
                | BrokerMsg::KellyfSnapshotMsg(_, _)
                | BrokerMsg::LjungbSnapshotMsg(_, _)
                | BrokerMsg::RunstestSnapshotMsg(_, _)
                | BrokerMsg::ZeroretSnapshotMsg(_, _)
                | BrokerMsg::PsrSnapshotMsg(_, _)
                | BrokerMsg::AdfSnapshotMsg(_, _)
                | BrokerMsg::MnkendallSnapshotMsg(_, _)
                | BrokerMsg::BipowerSnapshotMsg(_, _)
                | BrokerMsg::DddurSnapshotMsg(_, _)
                | BrokerMsg::HilltailSnapshotMsg(_, _)
                | BrokerMsg::ArchlmSnapshotMsg(_, _)
                | BrokerMsg::PainratioSnapshotMsg(_, _)
                | BrokerMsg::CusumSnapshotMsg(_, _)
                | BrokerMsg::CfvarSnapshotMsg(_, _)
                | BrokerMsg::EntropySnapshotMsg(_, _)
                | BrokerMsg::RachevSnapshotMsg(_, _)
                | BrokerMsg::GprSnapshotMsg(_, _)
                | BrokerMsg::PacfSnapshotMsg(_, _)
                | BrokerMsg::ApenSnapshotMsg(_, _)
                | BrokerMsg::UprSnapshotMsg(_, _)
                | BrokerMsg::LevereffSnapshotMsg(_, _)
                | BrokerMsg::DrawdarSnapshotMsg(_, _)
                | BrokerMsg::VarhalfSnapshotMsg(_, _)
                | BrokerMsg::GiniSnapshotMsg(_, _)
                | BrokerMsg::SampenSnapshotMsg(_, _)
                | BrokerMsg::PermenSnapshotMsg(_, _)
                | BrokerMsg::RecfactSnapshotMsg(_, _)
                | BrokerMsg::KpssSnapshotMsg(_, _)
                | BrokerMsg::SpecentSnapshotMsg(_, _)
                | BrokerMsg::RobvolSnapshotMsg(_, _)
                | BrokerMsg::RenyientSnapshotMsg(_, _)
                | BrokerMsg::RetquantSnapshotMsg(_, _)
                | BrokerMsg::MsentSnapshotMsg(_, _)
                | BrokerMsg::EwmavolSnapshotMsg(_, _)
                | BrokerMsg::KsnormSnapshotMsg(_, _)
                | BrokerMsg::AdtestSnapshotMsg(_, _)
                | BrokerMsg::LmomSnapshotMsg(_, _)
                | BrokerMsg::KylelamSnapshotMsg(_, _)
                | BrokerMsg::PeakoverSnapshotMsg(_, _)
                | BrokerMsg::HiguchiSnapshotMsg(_, _)
                | BrokerMsg::PickandsSnapshotMsg(_, _)
                | BrokerMsg::Kappa3SnapshotMsg(_, _)
                | BrokerMsg::LyapunovSnapshotMsg(_, _)
                | BrokerMsg::RankacSnapshotMsg(_, _)
                | BrokerMsg::BnsjumpSnapshotMsg(_, _)
                | BrokerMsg::PprootSnapshotMsg(_, _)
                | BrokerMsg::MfdfaSnapshotMsg(_, _)
                | BrokerMsg::HillksSnapshotMsg(_, _)
                | BrokerMsg::TsiSnapshotMsg(_, _)
                | BrokerMsg::Garch11SnapshotMsg(_, _)
                | BrokerMsg::SadfSnapshotMsg(_, _)
                | BrokerMsg::CordimSnapshotMsg(_, _)
                | BrokerMsg::SkspecSnapshotMsg(_, _)
                | BrokerMsg::AutomiSnapshotMsg(_, _)) => {
                    self.handle_research_quant_snapshot_msg(msg);
                }
                msg @ (BrokerMsg::DurbinWatsonSnapshotMsg(_, _)
                | BrokerMsg::BdsTestSnapshotMsg(_, _)
                | BrokerMsg::BreuschPaganSnapshotMsg(_, _)
                | BrokerMsg::TurnPtsSnapshotMsg(_, _)
                | BrokerMsg::PeriodogramSnapshotMsg(_, _)
                | BrokerMsg::McLeodLiSnapshotMsg(_, _)
                | BrokerMsg::OuFitSnapshotMsg(_, _)
                | BrokerMsg::GphSnapshotMsg(_, _)
                | BrokerMsg::BurgSpecSnapshotMsg(_, _)
                | BrokerMsg::KendallTauSnapshotMsg(_, _)
                | BrokerMsg::SqueezeSnapshotMsg(_, _)
                | BrokerMsg::SqueezeRankSnapshotMsg(_, _)
                | BrokerMsg::SqueezeWatchlistLoaded(_)
                | BrokerMsg::BbsqueezeSnapshotMsg(_, _)
                | BrokerMsg::DonchianSnapshotMsg(_, _)
                | BrokerMsg::KamaSnapshotMsg(_, _)
                | BrokerMsg::IchimokuSnapshotMsg(_, _)
                | BrokerMsg::SupertrendSnapshotMsg(_, _)
                | BrokerMsg::KeltnerSnapshotMsg(_, _)
                | BrokerMsg::FisherSnapshotMsg(_, _)
                | BrokerMsg::AroonSnapshotMsg(_, _)
                | BrokerMsg::AdxSnapshotMsg(_, _)
                | BrokerMsg::CciSnapshotMsg(_, _)
                | BrokerMsg::CmfSnapshotMsg(_, _)
                | BrokerMsg::MfiSnapshotMsg(_, _)
                | BrokerMsg::PsarSnapshotMsg(_, _)
                | BrokerMsg::VortexSnapshotMsg(_, _)
                | BrokerMsg::ChopSnapshotMsg(_, _)
                | BrokerMsg::ObvSnapshotMsg(_, _)
                | BrokerMsg::TrixSnapshotMsg(_, _)
                | BrokerMsg::HmaSnapshotMsg(_, _)
                | BrokerMsg::PpoSnapshotMsg(_, _)
                | BrokerMsg::DpoSnapshotMsg(_, _)
                | BrokerMsg::KstSnapshotMsg(_, _)
                | BrokerMsg::UltoscSnapshotMsg(_, _)
                | BrokerMsg::WillrSnapshotMsg(_, _)
                | BrokerMsg::MassSnapshotMsg(_, _)
                | BrokerMsg::ChaikoscSnapshotMsg(_, _)
                | BrokerMsg::KlingerSnapshotMsg(_, _)
                | BrokerMsg::StochRsiSnapshotMsg(_, _)
                | BrokerMsg::AwesomeSnapshotMsg(_, _)
                | BrokerMsg::EfiSnapshotMsg(_, _)
                | BrokerMsg::EmvSnapshotMsg(_, _)
                | BrokerMsg::NviSnapshotMsg(_, _)
                | BrokerMsg::PviSnapshotMsg(_, _)
                | BrokerMsg::CoppockSnapshotMsg(_, _)
                | BrokerMsg::CmoSnapshotMsg(_, _)
                | BrokerMsg::QstickSnapshotMsg(_, _)
                | BrokerMsg::DisparitySnapshotMsg(_, _)
                | BrokerMsg::BopSnapshotMsg(_, _)
                | BrokerMsg::SchaffSnapshotMsg(_, _)
                | BrokerMsg::StochSnapshotMsg(_, _)
                | BrokerMsg::MacdSnapshotMsg(_, _)
                | BrokerMsg::VwapSnapshotMsg(_, _)
                | BrokerMsg::McgdSnapshotMsg(_, _)
                | BrokerMsg::RwiSnapshotMsg(_, _)) => {
                    self.handle_indicator_snapshot_msg(msg);
                }
                msg @ (BrokerMsg::DemaSnapshotMsg(_, _)
                | BrokerMsg::TemaSnapshotMsg(_, _)
                | BrokerMsg::LinregSnapshotMsg(_, _)
                | BrokerMsg::PivotsSnapshotMsg(_, _)
                | BrokerMsg::HeikinSnapshotMsg(_, _)
                | BrokerMsg::AlmaSnapshotMsg(_, _)
                | BrokerMsg::ZlemaSnapshotMsg(_, _)
                | BrokerMsg::ElderRaySnapshotMsg(_, _)
                | BrokerMsg::TsfSnapshotMsg(_, _)
                | BrokerMsg::RviSnapshotMsg(_, _)
                | BrokerMsg::TrimaSnapshotMsg(_, _)
                | BrokerMsg::T3SnapshotMsg(_, _)
                | BrokerMsg::VidyaSnapshotMsg(_, _)
                | BrokerMsg::SmiSnapshotMsg(_, _)
                | BrokerMsg::PvtSnapshotMsg(_, _)
                | BrokerMsg::AcSnapshotMsg(_, _)
                | BrokerMsg::ChvolSnapshotMsg(_, _)
                | BrokerMsg::BbwidthSnapshotMsg(_, _)
                | BrokerMsg::ElderImpSnapshotMsg(_, _)
                | BrokerMsg::RmiSnapshotMsg(_, _)
                | BrokerMsg::SymbolExpirationsMsg(_, _)
                | BrokerMsg::SmmaSnapshotMsg(_, _)
                | BrokerMsg::AlligatorSnapshotMsg(_, _)
                | BrokerMsg::CrsiSnapshotMsg(_, _)
                | BrokerMsg::SebSnapshotMsg(_, _)
                | BrokerMsg::ImiSnapshotMsg(_, _)
                | BrokerMsg::GmmaSnapshotMsg(_, _)
                | BrokerMsg::MaenvSnapshotMsg(_, _)
                | BrokerMsg::AdlSnapshotMsg(_, _)
                | BrokerMsg::VhfSnapshotMsg(_, _)
                | BrokerMsg::VrocSnapshotMsg(_, _)
                | BrokerMsg::KdjSnapshotMsg(_, _)
                | BrokerMsg::QqeSnapshotMsg(_, _)
                | BrokerMsg::PmoSnapshotMsg(_, _)
                | BrokerMsg::CfoSnapshotMsg(_, _)
                | BrokerMsg::TmfSnapshotMsg(_, _)
                | BrokerMsg::FractalsSnapshotMsg(_, _)
                | BrokerMsg::IftRsiSnapshotMsg(_, _)
                | BrokerMsg::MamaSnapshotMsg(_, _)
                | BrokerMsg::CogSnapshotMsg(_, _)
                | BrokerMsg::DidiSnapshotMsg(_, _)
                | BrokerMsg::DemarkerSnapshotMsg(_, _)
                | BrokerMsg::GatorSnapshotMsg(_, _)
                | BrokerMsg::BwMfiSnapshotMsg(_, _)
                | BrokerMsg::VwmaSnapshotMsg(_, _)
                | BrokerMsg::StddevSnapshotMsg(_, _)
                | BrokerMsg::WmaSnapshotMsg(_, _)
                | BrokerMsg::RainbowSnapshotMsg(_, _)
                | BrokerMsg::MesaSineSnapshotMsg(_, _)
                | BrokerMsg::FramaSnapshotMsg(_, _)
                | BrokerMsg::IbsSnapshotMsg(_, _)
                | BrokerMsg::LaguerreRsiSnapshotMsg(_, _)
                | BrokerMsg::ZigzagSnapshotMsg(_, _)
                | BrokerMsg::PgoSnapshotMsg(_, _)
                | BrokerMsg::HtTrendlineSnapshotMsg(_, _)
                | BrokerMsg::MidpointSnapshotMsg(_, _)
                | BrokerMsg::MassIndexSnapshotMsg(_, _)
                | BrokerMsg::NatrSnapshotMsg(_, _)
                | BrokerMsg::TtmSqueezeSnapshotMsg(_, _)
                | BrokerMsg::ForceIndexSnapshotMsg(_, _)
                | BrokerMsg::TrangeSnapshotMsg(_, _)
                | BrokerMsg::LinearregSlopeSnapshotMsg(_, _)
                | BrokerMsg::HtDcperiodSnapshotMsg(_, _)
                | BrokerMsg::HtTrendmodeSnapshotMsg(_, _)
                | BrokerMsg::AccbandsSnapshotMsg(_, _)
                | BrokerMsg::StochfSnapshotMsg(_, _)
                | BrokerMsg::LinearregSnapshotMsg(_, _)
                | BrokerMsg::LinearregAngleSnapshotMsg(_, _)
                | BrokerMsg::HtDcphaseSnapshotMsg(_, _)
                | BrokerMsg::HtSineSnapshotMsg(_, _)
                | BrokerMsg::HtPhasorSnapshotMsg(_, _)
                | BrokerMsg::MidpriceSnapshotMsg(_, _)
                | BrokerMsg::ApoSnapshotMsg(_, _)
                | BrokerMsg::MomSnapshotMsg(_, _)
                | BrokerMsg::SarextSnapshotMsg(_, _)
                | BrokerMsg::AdxrSnapshotMsg(_, _)
                | BrokerMsg::AvgpriceSnapshotMsg(_, _)
                | BrokerMsg::MedpriceSnapshotMsg(_, _)
                | BrokerMsg::TypPriceSnapshotMsg(_, _)
                | BrokerMsg::WclPriceSnapshotMsg(_, _)
                | BrokerMsg::VarianceSnapshotMsg(_, _)
                | BrokerMsg::PlusDiSnapshotMsg(_, _)
                | BrokerMsg::MinusDiSnapshotMsg(_, _)
                | BrokerMsg::PlusDmSnapshotMsg(_, _)
                | BrokerMsg::MinusDmSnapshotMsg(_, _)
                | BrokerMsg::DxSnapshotMsg(_, _)
                | BrokerMsg::RocSnapshotMsg(_, _)
                | BrokerMsg::RocpSnapshotMsg(_, _)
                | BrokerMsg::RocrSnapshotMsg(_, _)
                | BrokerMsg::Rocr100SnapshotMsg(_, _)
                | BrokerMsg::CorrelSnapshotMsg(_, _)
                | BrokerMsg::MinSnapshotMsg(_, _)
                | BrokerMsg::MaxSnapshotMsg(_, _)
                | BrokerMsg::MinMaxSnapshotMsg(_, _)
                | BrokerMsg::MinIndexSnapshotMsg(_, _)
                | BrokerMsg::MaxIndexSnapshotMsg(_, _)
                | BrokerMsg::BbandsSnapshotMsg(_, _)
                | BrokerMsg::AdSnapshotMsg(_, _)
                | BrokerMsg::AdoscSnapshotMsg(_, _)
                | BrokerMsg::SumSnapshotMsg(_, _)
                | BrokerMsg::LinearRegInterceptSnapshotMsg(_, _)
                | BrokerMsg::AroonoscSnapshotMsg(_, _)
                | BrokerMsg::MinMaxIndexSnapshotMsg(_, _)
                | BrokerMsg::MacdextSnapshotMsg(_, _)
                | BrokerMsg::MacdfixSnapshotMsg(_, _)
                | BrokerMsg::MavpSnapshotMsg(_, _)
                | BrokerMsg::CdlDojiSnapshotMsg(_, _)
                | BrokerMsg::CdlHammerSnapshotMsg(_, _)
                | BrokerMsg::CdlShootingStarSnapshotMsg(_, _)
                | BrokerMsg::CdlEngulfingSnapshotMsg(_, _)
                | BrokerMsg::CdlHaramiSnapshotMsg(_, _)
                | BrokerMsg::CdlMorningStarSnapshotMsg(_, _)
                | BrokerMsg::CdlEveningStarSnapshotMsg(_, _)
                | BrokerMsg::CdlThreeBlackCrowsSnapshotMsg(_, _)
                | BrokerMsg::CdlThreeWhiteSoldiersSnapshotMsg(_, _)
                | BrokerMsg::CdlDarkCloudCoverSnapshotMsg(_, _)
                | BrokerMsg::CdlPiercingSnapshotMsg(_, _)
                | BrokerMsg::CdlDragonflyDojiSnapshotMsg(_, _)
                | BrokerMsg::CdlGravestoneDojiSnapshotMsg(_, _)
                | BrokerMsg::CdlHangingManSnapshotMsg(_, _)
                | BrokerMsg::CdlInvertedHammerSnapshotMsg(_, _)
                | BrokerMsg::CdlHaramiCrossSnapshotMsg(_, _)
                | BrokerMsg::CdlLongLeggedDojiSnapshotMsg(_, _)
                | BrokerMsg::CdlMarubozuSnapshotMsg(_, _)
                | BrokerMsg::CdlSpinningTopSnapshotMsg(_, _)
                | BrokerMsg::CdlTristarSnapshotMsg(_, _)
                | BrokerMsg::CdlDojiStarSnapshotMsg(_, _)
                | BrokerMsg::CdlMorningDojiStarSnapshotMsg(_, _)
                | BrokerMsg::CdlEveningDojiStarSnapshotMsg(_, _)
                | BrokerMsg::CdlAbandonedBabySnapshotMsg(_, _)
                | BrokerMsg::CdlThreeInsideSnapshotMsg(_, _)
                | BrokerMsg::CdlBeltHoldSnapshotMsg(_, _)
                | BrokerMsg::CdlClosingMarubozuSnapshotMsg(_, _)
                | BrokerMsg::CdlHighWaveSnapshotMsg(_, _)
                | BrokerMsg::CdlLongLineSnapshotMsg(_, _)
                | BrokerMsg::CdlShortLineSnapshotMsg(_, _)
                | BrokerMsg::CdlCounterattackSnapshotMsg(_, _)
                | BrokerMsg::CdlHomingPigeonSnapshotMsg(_, _)
                | BrokerMsg::CdlInNeckSnapshotMsg(_, _)
                | BrokerMsg::CdlOnNeckSnapshotMsg(_, _)
                | BrokerMsg::CdlThrustingSnapshotMsg(_, _)
                | BrokerMsg::CdlTwoCrowsSnapshotMsg(_, _)
                | BrokerMsg::CdlThreeLineStrikeSnapshotMsg(_, _)
                | BrokerMsg::CdlThreeOutsideSnapshotMsg(_, _)
                | BrokerMsg::CdlMatchingLowSnapshotMsg(_, _)
                | BrokerMsg::CdlSeparatingLinesSnapshotMsg(_, _)
                | BrokerMsg::CdlStickSandwichSnapshotMsg(_, _)
                | BrokerMsg::CdlRickshawManSnapshotMsg(_, _)
                | BrokerMsg::CdlTakuriSnapshotMsg(_, _)
                | BrokerMsg::CdlThreeStarsInSouthSnapshotMsg(_, _)
                | BrokerMsg::CdlIdenticalThreeCrowsSnapshotMsg(_, _)
                | BrokerMsg::CdlKickingSnapshotMsg(_, _)
                | BrokerMsg::CdlKickingByLengthSnapshotMsg(_, _)
                | BrokerMsg::CdlLadderBottomSnapshotMsg(_, _)
                | BrokerMsg::CdlUniqueThreeRiverSnapshotMsg(_, _)
                | BrokerMsg::CdlAdvanceBlockSnapshotMsg(_, _)
                | BrokerMsg::CdlBreakawaySnapshotMsg(_, _)
                | BrokerMsg::CdlGapSideSideWhiteSnapshotMsg(_, _)
                | BrokerMsg::CdlUpsideGapTwoCrowsSnapshotMsg(_, _)
                | BrokerMsg::CdlXSideGapThreeMethodsSnapshotMsg(_, _)
                | BrokerMsg::CdlConcealBabySwallowSnapshotMsg(_, _)
                | BrokerMsg::CdlHikkakeSnapshotMsg(_, _)
                | BrokerMsg::CdlHikkakeModSnapshotMsg(_, _)
                | BrokerMsg::CdlMatHoldSnapshotMsg(_, _)
                | BrokerMsg::CdlRiseFallThreeMethodsSnapshotMsg(_, _)
                | BrokerMsg::CdlStalledPatternSnapshotMsg(_, _)
                | BrokerMsg::CdlTasukiGapSnapshotMsg(_, _)
                | BrokerMsg::ModSharpeSnapshotMsg(_, _)
                | BrokerMsg::HsiehTestSnapshotMsg(_, _)
                | BrokerMsg::ChowBreakSnapshotMsg(_, _)
                | BrokerMsg::DriftBurstSnapshotMsg(_, _)
                | BrokerMsg::HlvClustSnapshotMsg(_, _)
                | BrokerMsg::YangZhangSnapshotMsg(_, _)
                | BrokerMsg::KuiperSnapshotMsg(_, _)
                | BrokerMsg::DagostinoSnapshotMsg(_, _)
                | BrokerMsg::BaiPerronSnapshotMsg(_, _)
                | BrokerMsg::KupiecPofSnapshotMsg(_, _)) => {
                    self.handle_extended_indicator_snapshot_msg(msg);
                }
                msg @ (BrokerMsg::IngestResearchResult { .. }
                | BrokerMsg::NewsArticlesLoaded { .. }
                | BrokerMsg::NewsDbTotal(_)) => {
                    self.handle_news_ingest_msg(msg);
                }
                msg @ (BrokerMsg::UnusualVolumeResults(_) | BrokerMsg::MarketClock(_)) => {
                    self.handle_misc_broker_msg(msg);
                }
                BrokerMsg::JsonResult(label, text) => {
                    if label == "Kraken WS" {
                        tracing::debug!(
                            "Suppressed raw Kraken private WebSocket payload from UI log ({} bytes)",
                            text.len()
                        );
                        continue;
                    }
                    if label == "Account Activities" {
                        tracing::debug!(
                            "Suppressed raw account activities from UI log ({} bytes)",
                            text.len()
                        );
                        continue;
                    }
                    // Route structured results to their windows; log everything
                    if label.starts_with("Analyst:") {
                        self.analyst_result = text.clone();
                        self.show_analyst = true;
                    } else if label.starts_with("PriceTarget:") {
                        // Append price target to analyst window
                        self.analyst_result.push_str("\n---PRICE_TARGET---\n");
                        self.analyst_result.push_str(&text);
                        self.show_analyst = true;
                    } else if label.starts_with("Holders:") {
                        self.holders_result = text.clone();
                        self.show_holders = true;
                    } else if label.starts_with("Orderbook:") {
                        self.orderbook_result = text.clone();
                        self.show_orderbook_window = true;
                    } else if label == "FearGreed" {
                        // Parse "value|label" format
                        let parts: Vec<&str> = text.splitn(2, '|').collect();
                        if parts.len() == 2 {
                            self.fear_greed_value = parts[0].parse::<u32>().unwrap_or(50);
                            self.fear_greed_label = parts[1].to_string();
                        }
                    } else if label == "AiChat" {
                        self.maybe_queue_ingest_from_ai_response("ai_chat", &text);
                        self.ai_chat_history.push((false, text.clone()));
                        let sid = Self::ensure_session_id(&mut self.ai_chat_session_id);
                        let model = self.ai_model.clone();
                        let history = self.ai_chat_history.clone();
                        self.persist_ai_turn("ai_chat", &sid, None, &history, &model);
                    } else if label == "RedditWSB" {
                        if let Ok(posts) =
                            serde_json::from_str::<Vec<(String, String, u64, u64)>>(&text)
                        {
                            self.reddit_posts = posts;
                        }
                    } else if label == "MatrixMessages" {
                        if let Ok(msgs) =
                            serde_json::from_str::<Vec<(String, String, String)>>(&text)
                        {
                            self.matrix_messages = msgs;
                        }
                    } else if label == "MatrixJoined" {
                        self.log
                            .push_back(LogEntry::info("Matrix: joined community room"));
                    } else if label == "MatrixSent" {
                        // Re-fetch messages after sending
                        let _ = self.broker_tx.send(BrokerCmd::MatrixFetchMessages {
                            room_id: self.matrix_room.clone(),
                            access_token: self.matrix_access_token.clone(),
                        });
                    } else if let Some((card, summary)) = json_result_card_from_text(&label, &text)
                    {
                        self.result_card = Some((card, std::time::Instant::now()));
                        self.log.push_back(LogEntry::info(summary));
                        continue;
                    }
                    self.log
                        .push_back(LogEntry::info(format!("{}:\n{}", label, text)));
                }
                BrokerMsg::FundamentalsProgress(ref msg) => {
                    self.scrape_fund_last_msg = msg.clone();
                    // Parse progress from messages like "Scraped X: OK (5/100)" or "complete: X OK, Y failed..."
                    if msg.contains("stock tickers") || msg.contains("tickers found") {
                        self.scrape_fund_running = true;
                        self.scrape_fund_ok = 0;
                        self.scrape_fund_fail = 0;
                        self.scrape_fund_skipped = 0;
                        if let Some(n) =
                            msg.split_whitespace().find_map(|w| w.parse::<usize>().ok())
                        {
                            self.scrape_fund_total = self.scrape_fund_total.max(n);
                        }
                    } else if msg.contains(": OK") {
                        self.scrape_fund_ok += 1;
                    } else if msg.contains(": FAIL") {
                        self.scrape_fund_fail += 1;
                    } else if msg.contains("complete")
                        || msg.contains("Aborting")
                        || msg.starts_with("Fundamentals progress:")
                    {
                        if !msg.starts_with("Fundamentals progress:") {
                            self.scrape_fund_running = false;
                        }
                        // Parse final/progress counts from "X OK, Y failed, Z skipped ... out of N"
                        let parts: Vec<&str> = msg.split_whitespace().collect();
                        for (i, w) in parts.iter().enumerate() {
                            if *w == "OK," {
                                if let Some(n) = parts
                                    .get(i.wrapping_sub(1))
                                    .and_then(|s| s.parse::<usize>().ok())
                                {
                                    self.scrape_fund_ok = n;
                                }
                            }
                            if *w == "failed," {
                                if let Some(n) = parts
                                    .get(i.wrapping_sub(1))
                                    .and_then(|s| s.parse::<usize>().ok())
                                {
                                    self.scrape_fund_fail = n;
                                }
                            }
                            if *w == "skipped" {
                                if let Some(n) = parts
                                    .get(i.wrapping_sub(1))
                                    .and_then(|s| s.parse::<usize>().ok())
                                {
                                    self.scrape_fund_skipped = n;
                                }
                            }
                        }
                    }
                    if msg.starts_with("Fundamentals progress:") || is_routine_news_progress(msg) {
                        tracing::debug!("{}", msg);
                    } else {
                        self.log.push_back(LogEntry::info(msg.clone()));
                    }
                }
                BrokerMsg::SymbolSuggestions(results) => {
                    // Merge broker search results into autocomplete (if dropdown still visible)
                    // Normalize: remove slash from crypto (BTC/USD → BTCUSD) to avoid duplicates
                    if self.symbol_ac_visible {
                        for (sym, name, class) in results {
                            let normalized = sym.replace('/', "");
                            let existing = self.symbol_suggestions.iter_mut().find(|(s, _, _)| {
                                s.replace('/', "").eq_ignore_ascii_case(&normalized)
                            });
                            match existing {
                                // Already present from a local source. Local cache/universe
                                // entries carry an empty company name (e.g. WOK from
                                // cached_active_symbols); the broker search result *does*
                                // resolve the name ("WORK Medical Technology…"), so fill in
                                // the blanks instead of dropping the richer result.
                                Some((_, ex_name, ex_class)) => {
                                    if ex_name.trim().is_empty() && !name.trim().is_empty() {
                                        *ex_name = name;
                                    }
                                    if ex_class.trim().is_empty() && !class.trim().is_empty() {
                                        *ex_class = class;
                                    }
                                }
                                None => self.symbol_suggestions.push((normalized, name, class)),
                            }
                        }
                        self.symbol_suggestions.truncate(20);
                    }
                }
                msg @ (BrokerMsg::BarsFetched { .. }
                | BrokerMsg::AlpacaFetchSettled { .. }
                | BrokerMsg::KrakenFetchSettled { .. }
                | BrokerMsg::KrakenBackfillComplete { .. }
                | BrokerMsg::KrakenFuturesFetchSettled { .. }
                | BrokerMsg::KrakenFuturesBackfillComplete { .. }) => {
                    market_data_refill_requested |= self.handle_market_data_fetch_result_msg(msg);
                }
                BrokerMsg::AlpacaRateLimitObserved { historical_rpm } => {
                    if historical_rpm > 0 && self.alpaca_historical_rpm_observed != historical_rpm {
                        self.alpaca_historical_rpm_observed = historical_rpm;
                        let capacity = self.alpaca_sync_capacity();
                        self.push_alpaca_sync_runtime_config();
                        self.log.push_back(LogEntry::info(format!(
                            "Alpaca sync speed: detected {} req/min historical tier — {} workers, queue {}, batch {}",
                            historical_rpm,
                            capacity.fetch_permits,
                            capacity.queue_window,
                            capacity.batch_size
                        )));
                    }
                }
                BrokerMsg::AlpacaRetryEnqueue {
                    symbol,
                    timeframe,
                    reason,
                } => {
                    self.alpaca_retry_enqueue(&symbol, &timeframe, &reason);
                    let queue_len = self.alpaca_retry_queue.len();
                    tracing::debug!(
                        "Alpaca {} {}: queued for retry ({}) — {} in queue",
                        symbol,
                        timeframe,
                        reason,
                        queue_len
                    );
                    if should_emit_alpaca_retry_queue_log(queue_len) {
                        self.log.push_back(LogEntry::info(format!(
                            "Alpaca retry queue: {} symbols awaiting targeted probes (latest: {} {} — {})",
                            queue_len, symbol, timeframe, reason
                        )));
                    }
                }
                BrokerMsg::AlpacaNoData {
                    symbol,
                    timeframe,
                    reason,
                } => {
                    self.alpaca_retry_drain(&symbol, &timeframe);
                    let changed = self.alpaca_no_data_mark(&symbol, &timeframe, &reason);
                    let marker_count = self.alpaca_no_data_pairs.len();
                    let prefix = if changed {
                        "marked no-data"
                    } else {
                        "still no-data"
                    };
                    tracing::debug!(
                        "Alpaca {} {}: {} — automated sync will skip it ({} marked)",
                        symbol,
                        timeframe,
                        prefix,
                        marker_count
                    );
                    if changed && marker_count.is_multiple_of(100) {
                        self.log.push_back(LogEntry::warn(format!(
                            "Alpaca no-data milestone: {} provider-unavailable pairs tombstoned",
                            marker_count
                        )));
                    }
                }
                BrokerMsg::AlpacaBackfillComplete {
                    symbol,
                    timeframe,
                    bar_count,
                    target_bars,
                } => {
                    let changed = self.alpaca_backfill_complete_mark(
                        &symbol,
                        &timeframe,
                        bar_count,
                        target_bars,
                    );
                    if changed {
                        let marker_count = self.alpaca_backfill_complete_pairs.len();
                        tracing::debug!(
                            "Alpaca {} {}: marked backfill-complete at {}/{} bars ({} marked)",
                            symbol,
                            timeframe,
                            bar_count,
                            target_bars,
                            marker_count
                        );
                        if marker_count.is_multiple_of(100) {
                            self.log.push_back(LogEntry::info(format!(
                                "Alpaca backfill milestone: {} pairs at provider-window saturation",
                                marker_count
                            )));
                        }
                    }
                }
            }
            let msg_elapsed = msg_started.elapsed();
            if msg_elapsed > std::time::Duration::from_millis(25) {
                tracing::warn!(
                    "BrokerMsg::{msg_kind} handling took {:.2}ms on UI thread",
                    msg_elapsed.as_secs_f64() * 1000.0
                );
            }
        }
        if market_data_refill_requested {
            self.refill_market_data_sync_slots();
        }
        perf_broker_drain_ms = broker_drain_started.elapsed().as_secs_f64() * 1000.0;
        perf_after_broker_started = std::time::Instant::now();
        // If we hit the drain cap there are more messages waiting — repaint
        // immediately to process the next batch rather than waiting on the idle tick.
        if msgs_drained >= broker_drain_max || broker_drain_started.elapsed() >= broker_drain_budget
        {
            // Throttle live Kraken WS forming-bar updates to ~10 fps.
            // Full immediate repaint is only needed for closed bars or user action.
            // The forming_bar_dirty flag on ChartState is the signal from the WS path.
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
        (
            perf_pre_broker_ms,
            perf_broker_drain_ms,
            perf_after_broker_started,
            msgs_drained,
        )
    }
}
