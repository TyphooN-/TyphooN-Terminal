// ADR-127 Phase B: the broker message protocol depends only on typhoon-engine + std (no
// `use super::*` native-state glob), so it can move to engine in Phase C. Everything else
// it carries is referenced by fully-qualified `crate::…` path; these are the only
// bare-name types its variants use.
use crate::broker::alpaca::{AccountInfo, OrderInfo, PositionInfo};
use crate::core::watchlist::WatchlistRow;
use std::path::PathBuf;

/// Broker identity. Doubles as the order-routing target and the primary/assist
/// role selector (see `primary_broker`). New brokers are added here and to the
/// match arms below; nothing else hardcodes a 2-way Alpaca/Kraken split.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrderBroker {
    Alpaca,
    Kraken,
}

impl OrderBroker {
    pub fn label(self) -> &'static str {
        match self {
            OrderBroker::Alpaca => "Alpaca",
            OrderBroker::Kraken => "Kraken",
        }
    }

    /// The equity-merge source tag this broker provides (bridges the identity
    /// enum to the string-keyed merge sources in `chart/equity_merge.rs`).
    pub fn equity_source_tag(self) -> &'static str {
        match self {
            OrderBroker::Alpaca => "alpaca",
            OrderBroker::Kraken => "kraken-equities",
        }
    }

    /// Stable token used for session persistence.
    pub fn as_persist_str(self) -> &'static str {
        match self {
            OrderBroker::Alpaca => "alpaca",
            OrderBroker::Kraken => "kraken",
        }
    }

    pub fn from_persist_str(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "alpaca" => Some(OrderBroker::Alpaca),
            "kraken" => Some(OrderBroker::Kraken),
            _ => None,
        }
    }

    /// Ordered list of brokers to offer in the top-bar Primary switch, limited to
    /// those the user has enabled. Order here is the cycle order. Extends to N
    /// brokers by adding rows.
    pub fn enabled_cycle(alpaca_enabled: bool, kraken_enabled: bool) -> Vec<OrderBroker> {
        let mut out = Vec::new();
        if alpaca_enabled {
            out.push(OrderBroker::Alpaca);
        }
        if kraken_enabled {
            out.push(OrderBroker::Kraken);
        }
        out
    }
}

/// One broker login (API key pair) plus its role flags. A broker module may
/// hold several of these (ADR-130): Alpaca free tier allows 1 live + 3 paper
/// accounts, each with an independent market-data rate limit, so extra
/// accounts multiply historical sync throughput. `trade_enabled` marks the
/// account as a TradeCopy target; `data_sync_enabled` puts it in the bar-sync
/// fan-out rotation.
#[derive(Clone, Debug)]
pub struct BrokerAccountSpec {
    /// Stable id, e.g. "alpaca1".."alpaca4" / "kraken1".."kraken4".
    pub id: String,
    /// User-facing label ("Live", "Paper 1", …).
    pub label: String,
    pub api_key: String,
    pub secret: String,
    /// Alpaca: paper vs live endpoint. Kraken ignores this (single endpoint).
    pub paper: bool,
    pub trade_enabled: bool,
    pub data_sync_enabled: bool,
}

/// Connection/roster state for one account, reported by the broker runtime
/// after (re)connect or primary switch so the UI can render the account list
/// and the top-bar primary cycler.
#[derive(Clone, Debug)]
pub struct AccountRosterEntry {
    pub id: String,
    pub label: String,
    pub paper: bool,
    pub trade_enabled: bool,
    pub data_sync_enabled: bool,
    pub equity: f64,
    pub is_primary: bool,
    pub connected: bool,
    /// Short status detail ("Connected", auth error, …).
    pub detail: String,
}

/// Positions for one connected Alpaca account. The legacy `Positions` message
/// remains the primary-account snapshot; this lets the UI render every account
/// explicitly without guessing from the primary-only list.
#[derive(Clone, Debug)]
pub struct AccountPositions {
    pub account_id: String,
    pub label: String,
    pub is_primary: bool,
    pub positions: Vec<PositionInfo>,
}

pub struct QuickTradePlan {
    pub symbol: String,
    pub last_price: f64,
    pub sl: f64,
    pub tp: f64,
    pub side_idx: usize,
    pub qty: f64,
    pub risk_dollars: f64,
    pub risk_pct: Option<f64>,
    pub reward_dollars: f64,
    pub rr: Option<f64>,
}

#[derive(Clone, Copy, Debug)]
pub struct TradeAccountSnapshot {
    pub broker: &'static str,
    pub balance: f64,
    pub equity: f64,
    pub buying_power: f64,
    pub margin_used: f64,
}

/// Messages sent from UI → async broker task.
#[allow(dead_code)] // All variants are handled in broker task. Some lack dedicated UI buttons but are
// accessible via console commands or research windows.
pub enum BrokerCmd {
    /// Connect every configured Alpaca account. `primary_id` selects the
    /// trading/account-data account; all accounts with `data_sync_enabled`
    /// join the historical bar-fetch rotation (per-account rate limiters, so
    /// N accounts ≈ N× the historical sync budget). `fetch_permits` is the
    /// aggregate worker cap across the pool.
    Connect {
        accounts: Vec<BrokerAccountSpec>,
        primary_id: String,
        bar_requests_per_minute: u32,
        fetch_permits: usize,
    },
    /// Re-point the primary (order-routing + account-data) account for a
    /// broker without reconnecting the pool.
    SetPrimaryAccount {
        broker: OrderBroker,
        account_id: String,
    },
    /// Mirror app-placed Alpaca orders onto explicitly selected accounts
    /// (TradeCopy live mode). Strictly opt-in: `target_ids` is the checked
    /// target set from the TradeCopy window, and an empty set mirrors nothing.
    /// Neither the flag nor the set persists across restarts — copying is
    /// always disabled by default.
    SetOrderMirroring {
        enabled: bool,
        target_ids: Vec<String>,
    },
    /// One-shot TradeCopy: replicate the source account's open positions onto
    /// each target account by submitting market orders for the per-symbol qty
    /// delta. `flatten_extra` also closes target positions the source lacks.
    AlpacaTradeCopy {
        source_id: String,
        target_ids: Vec<String>,
        flatten_extra: bool,
    },
    /// One-shot Kraken TradeCopy (ADR-130): replicate the source account's
    /// **xStock equity-balance positions** (the app's Kraken position
    /// definition minus margin positions) onto each target via spot market
    /// orders on the `{TICKER}x/USD` pair. Margin positions are reported and
    /// skipped — leverage/short semantics cannot be replicated with spot
    /// market orders. Same opt-in target rules as the Alpaca copy.
    KrakenTradeCopy {
        source_id: String,
        target_ids: Vec<String>,
        flatten_extra: bool,
    },
    ConfigureAlpacaSync {
        bar_requests_per_minute: u32,
        fetch_permits: usize,
    },
    GetAccount,
    GetPositions,
    GetOrders,
    /// Open the Alpaca trading WebSocket and stream real-time `trade_updates`
    /// (instant fills/orders) instead of relying solely on REST polling.
    AlpacaStartTradeStream,
    /// Start (first call) or update the Alpaca market-data WebSocket subscription
    /// to exactly `symbols` (positions + watchlist + active chart). The feed
    /// (SIP if entitled, else free IEX) is auto-detected at connect.
    AlpacaStreamQuotes {
        symbols: Vec<String>,
    },
    CloseAll,
    ClosePosition {
        symbol: String,
        qty: Option<f64>,
    },
    ClosePositionForAccount {
        account_id: String,
        symbol: String,
        qty: Option<f64>,
    },
    AlpacaClosePositionPercent {
        symbol: String,
        percentage: f64,
    },
    AlpacaClosePositionPercentForAccount {
        account_id: String,
        symbol: String,
        percentage: f64,
    },
    /// Scrape SEC EDGAR filings for all portfolio symbols.
    SecScrape {
        db_path: PathBuf,
        /// Uppercase equity tickers derived from the current top-level Scope.
        symbols: Vec<String>,
    },
    // scrape_filings_for_ticker available via scrape_all_portfolio_symbols
    /// Fetch Finnhub news for a symbol.
    FinnhubNews {
        symbol: String,
        api_key: String,
    },
    /// Get latest quote for a symbol.
    GetQuote {
        symbol: String,
    },
    /// Get market clock (hours/status).
    GetMarketClock,
    /// Get account activities (fills, transfers).
    GetActivities {
        limit: u32,
    },
    /// Get top movers.
    GetTopMovers,
    /// Search symbols.
    SearchSymbols {
        query: String,
    },
    /// Fetch full tradeable asset list from connected broker.
    GetAllAssets,
    /// Ignore/remove a specific news article for a symbol (user curation of bad matches).
    IgnoreNewsArticle {
        symbol: String,
        url_hash: String,
    },
    /// Get order history.
    GetOrderHistory {
        limit: u32,
    },
    /// Fetch fundamentals for a symbol (SEC EDGAR).
    GetFundamentals {
        ticker: String,
    },
    /// Fetch institutional holders (13F).
    GetHolders {
        ticker: String,
    },
    /// Fetch analyst ratings (Finnhub).
    GetAnalyst {
        symbol: String,
        finnhub_key: String,
    },
    /// Fetch orderbook (Level 2).
    GetOrderbook {
        symbol: String,
    },
    /// Fetch most active stocks by volume.
    GetMostActive,
    /// Fetch portfolio equity history.
    GetPortfolioHistory {
        period: String,
    },
    /// Fetch Finnhub price target for symbol.
    GetPriceTarget {
        symbol: String,
        finnhub_key: String,
    },
    /// Fetch Finnhub short interest for symbol.
    GetShortInterest {
        symbol: String,
        finnhub_key: String,
    },
    /// Fetch corporate actions for symbol.
    GetCorporateActions {
        symbol: String,
    },
    /// Fetch Alpaca watchlists.
    GetWatchlists,
    /// Create Alpaca watchlist.
    CreateWatchlist {
        name: String,
        symbols: Vec<String>,
    },
    /// Replace Alpaca watchlist symbols by id.
    UpdateWatchlist {
        id: String,
        symbols: Vec<String>,
    },
    /// Append one symbol to an Alpaca watchlist.
    AddWatchlistSymbol {
        id: String,
        symbol: String,
    },
    /// Remove one symbol from an Alpaca watchlist.
    RemoveWatchlistSymbol {
        id: String,
        symbol: String,
    },
    /// Delete an Alpaca watchlist by id.
    DeleteWatchlist {
        id: String,
    },
    /// Fetch Alpaca options chain for underlying.
    GetOptionsChain {
        symbol: String,
        expiry: String,
    },
    /// Fetch live bars from Alpaca and store in cache (fallback when cache misses).
    AlpacaFetchBars {
        symbol: String,
        timeframe: String,
        db_path: std::path::PathBuf,
        backfill_complete: bool,
    },
    /// Fetch bars for several stock symbols through Alpaca's batch bars endpoint.
    /// `limit` is the lookback depth in bars: 10_000 requests full provider
    /// history (coverage/backfill chunks); smaller values bound a stale top-up
    /// to the gap actually missing instead of re-pulling every symbol's entire
    /// history on each refresh.
    AlpacaFetchBarsBatch {
        symbols: Vec<String>,
        timeframe: String,
        limit: u32,
    },
    /// Kraken public Spot/xStocks backfill via the public OHLC API.
    KrakenBackfill {
        symbol: String,
        timeframes: Vec<String>,
        db_path: std::path::PathBuf,
        backfill_complete: bool,
    },
    /// Kraken Futures public chart backfill via the public charts API.
    KrakenFuturesBackfill {
        symbol: String,
        timeframes: Vec<String>,
        db_path: std::path::PathBuf,
        backfill_complete: bool,
    },
    /// Recompress cache at target zstd level (e.g. 22 for max compression).
    CompactStorage {
        db_path: PathBuf,
        level: i32,
    },
    /// Scrape fundamentals for stock symbols from selected sources (non-blocking).
    FundamentalsScrape {
        db_path: PathBuf,
        use_alpaca: bool,
        use_kraken: bool,
        kraken_equity_symbols: Vec<String>,
        force: bool,
    },
    /// Scrape fundamentals for a single ticker.
    FundamentalsScrapeOne {
        ticker: String,
        db_path: PathBuf,
    },
    /// Bulk research scrape — profile/peers/earnings/press/sentiment/transcripts.
    ResearchScrape {
        use_alpaca: bool,
        finnhub_key: String,
        fmp_key: String,
    },
    /// Fetch SEC filing document content from EDGAR URL.
    FetchFilingContent {
        url: String,
    },
    /// Fetch ALL available bars from Alpaca (full history, no limit).
    FetchAllBars {
        symbol: String,
        timeframe: String,
    },
    /// Batch quote request for watchlist symbols.
    GetWatchlistQuotes {
        symbols: Vec<String>,
    },
    /// Fetch FRED economic data series.
    FredFetch {
        api_key: String,
    },
    /// Fetch Finnhub economic calendar.
    FetchEconCalendar {
        finnhub_key: String,
    },
    /// Fetch congressional stock trades (House Stock Watcher).
    FetchCongressTrades,
    /// Send notification (Discord/Pushover/ntfy). Runs async.
    SendNotification {
        discord_webhook: String,
        pushover_token: String,
        pushover_user: String,
        ntfy_topic: String,
        message: String,
    },
    /// Scan unusual volume (background, heavy DB reads).
    ScanUnusualVolume {
        keys: Vec<(String, i64)>,
    },
    /// Place market order via Alpaca.
    AlpacaMarketOrder {
        symbol: String,
        qty: f64,
        side: String,
    },
    /// Place market order via Alpaca using dollar notional instead of quantity.
    /// Alpaca documents notional as market/day only and mutually exclusive with qty.
    AlpacaMarketOrderNotional {
        symbol: String,
        notional: f64,
        side: String,
    },
    /// Place limit order via Alpaca.
    AlpacaLimitOrder {
        symbol: String,
        qty: f64,
        side: String,
        limit_price: f64,
    },
    /// Place stop order via Alpaca.
    AlpacaStopOrder {
        symbol: String,
        qty: f64,
        side: String,
        stop_price: f64,
    },
    /// Place bracket order via Alpaca (market entry + TP/SL legs).
    AlpacaBracketOrder {
        symbol: String,
        qty: f64,
        side: String,
        stop_loss: f64,
        take_profit: f64,
    },
    /// Cancel an Alpaca order by ID.
    AlpacaCancelOrder {
        order_id: String,
    },
    /// Place an OCO (one-cancels-other) exit order on Alpaca.
    AlpacaOcoOrder {
        symbol: String,
        qty: f64,
        side: String,
        tp_price: f64,
        sl_price: f64,
    },
    /// Modify an existing Alpaca order (change price/qty on bracket legs).
    AlpacaModifyOrder {
        order_id: String,
        qty: Option<f64>,
        limit_price: Option<f64>,
        stop_price: Option<f64>,
    },
    /// Sync Alpaca exits for current symbol/position.
    AlpacaSyncExits {
        symbol: String,
        sl_price: Option<f64>,
        tp_price: Option<f64>,
        wait_for_qty_at_most: Option<f64>,
    },
    /// Sync Kraken exits for the current open position.
    KrakenSyncExits {
        pair: String,
        sl_price: Option<f64>,
        tp_price: Option<f64>,
        wait_for_position: bool,
        wait_for_qty_at_most: Option<f64>,
    },
    /// AI chat request (Anthropic Claude or OpenAI GPT).
    /// `system` — optional system prompt / research packet.
    /// `model` — optional model override (else the provider default is used).
    AiChat {
        provider: String,
        api_key: String,
        message: String,
        history: Vec<(bool, String)>,
        system: Option<String>,
        model: Option<String>,
    },
    /// Join a Matrix room (idempotent — no-op if already joined).
    MatrixJoinRoom {
        room_id: String,
        access_token: String,
    },
    /// Fetch recent messages from a public Matrix room.
    MatrixFetchMessages {
        room_id: String,
        access_token: String,
    },
    /// Send a message to a Matrix room.
    MatrixSendMessage {
        room_id: String,
        access_token: String,
        body: String,
    },
    /// Upload an image to Matrix and send it as m.image message.
    MatrixSendImage {
        room_id: String,
        access_token: String,
        file_path: std::path::PathBuf,
    },
    /// Place trailing stop order via Alpaca.
    AlpacaTrailingStop {
        symbol: String,
        qty: f64,
        side: String,
        trail_percent: f64,
    },
    /// Place stop-limit order via Alpaca.
    AlpacaStopLimitOrder {
        symbol: String,
        qty: f64,
        side: String,
        stop_price: f64,
        limit_price: f64,
    },
    /// Fetch Crypto Fear & Greed Index (alternative.me, no auth).
    FetchFearGreed,
    /// Fetch Reddit WSB hot posts (no auth).
    FetchRedditWSB,
    /// Fetch CoinGecko top 50 cryptocurrencies by market cap (free, no auth).
    FetchCryptoTop50,
    // Company events, sentiment, transcripts, commodities, and tape research commands
    /// Fetch Finnhub company profile (name, exchange, industry, logo, website, IPO, shares, mcap).
    FetchCompanyProfile {
        symbol: String,
        finnhub_key: String,
    },
    /// Fetch Finnhub /stock/peers — related tickers.
    FetchStockPeers {
        symbol: String,
        finnhub_key: String,
    },
    /// Fetch Finnhub /stock/earnings — actual vs estimate EPS per quarter.
    FetchEarningsHistory {
        symbol: String,
        finnhub_key: String,
    },
    /// Fetch Finnhub /calendar/ipo — upcoming IPOs for given date window.
    FetchIpoCalendar {
        finnhub_key: String,
        days_ahead: i64,
        days_back: i64,
    },
    /// Fetch Finnhub /press-releases for a symbol.
    FetchPressReleases {
        symbol: String,
        finnhub_key: String,
    },
    /// Fetch Finnhub /stock/social-sentiment (Reddit + Twitter).
    FetchSocialSentiment {
        symbol: String,
        finnhub_key: String,
    },
    /// Fetch keyless Reddit mention snapshot across the finance subreddits
    /// (ADR-117 Reddit lane).
    FetchRedditMentions {
        symbol: String,
    },
    /// Fetch StockTwits public symbol stream sentiment snapshot.
    FetchStockTwitsSentiment {
        symbol: String,
    },
    /// Fetch FMP transcript list for a symbol.
    FetchTranscriptList {
        symbol: String,
        fmp_key: String,
    },
    /// Fetch FMP full transcript by quarter/year.
    FetchTranscriptBody {
        symbol: String,
        quarter: i32,
        year: i32,
        fmp_key: String,
    },
    /// Fetch Yahoo quote batch for the GLCO commodities dashboard.
    FetchCommoditiesQuotes,
    // Dividend, earnings-estimate, rating, and treasury research
    /// FMP historical dividend payment schedule for a symbol.
    FetchDividendHistory {
        symbol: String,
        fmp_key: String,
    },
    /// FMP forward EPS + revenue consensus estimates for a symbol.
    FetchEarningsEstimates {
        symbol: String,
        fmp_key: String,
    },
    /// FMP analyst rating change feed (upgrades/downgrades/initiations).
    FetchRatingChanges {
        symbol: String,
        fmp_key: String,
    },
    /// Yahoo batch quote for ^IRX/^FVX/^TNX/^TYX treasury yield ladder.
    FetchTreasuryYields,
    // Financial statements, management, and COT research
    /// FMP full FA bundle (income + balance + cash flow × annual/quarterly).
    FetchFinancialStatements {
        symbol: String,
        fmp_key: String,
    },
    /// Finnhub officers + compensation feed for a symbol.
    FetchExecutives {
        symbol: String,
        finnhub_key: String,
    },
    /// CFTC Socrata — latest weekly Commitments of Traders (legacy futures).
    FetchCotReports,
    // Corporate action, analyst, ESG, ETF, and index research
    /// Historical stock split events for a symbol (FMP when keyed, Yahoo fallback).
    FetchStockSplits {
        symbol: String,
        fmp_key: String,
    },
    /// FMP ETF holdings (constituents) for an ETF ticker.
    FetchEtfHoldings {
        symbol: String,
        fmp_key: String,
    },
    /// Finnhub analyst recommendation bucket trend (monthly).
    FetchAnalystRecs {
        symbol: String,
        finnhub_key: String,
    },
    /// Finnhub consensus price target snapshot (single row).
    FetchPriceTarget {
        symbol: String,
        finnhub_key: String,
    },
    /// FMP ESG (environmental/social/governance) score history.
    FetchEsgScores {
        symbol: String,
        fmp_key: String,
    },
    /// FMP index constituents by index code (SP500/NDX/DJIA).
    FetchIndexMembers {
        index_code: String,
        fmp_key: String,
    },
    // Ownership, float, price-history, and earnings-surprise research
    /// FMP SEC Form-4 insider trades for a symbol.
    FetchInsiderTrades {
        symbol: String,
        fmp_key: String,
    },
    /// FMP 13F-derived institutional holders for a symbol.
    FetchInstitutionalHolders {
        symbol: String,
        fmp_key: String,
    },
    /// FMP shares float + outstanding snapshot for a symbol.
    FetchSharesFloat {
        symbol: String,
        fmp_key: String,
    },
    /// FMP daily historical OHLCV table (bars up to `limit`) for a symbol.
    FetchHistoricalPrice {
        symbol: String,
        fmp_key: String,
        limit: usize,
    },
    /// FMP quarterly earnings surprises (actual vs estimate EPS) for a symbol.
    FetchEarningsSurprises {
        symbol: String,
        fmp_key: String,
    },
    // World index, market mover, sector, and WACC research
    /// Yahoo batch-quote the global equity indices dashboard (no key).
    FetchWorldIndices,
    /// FMP market movers — top gainers/losers/most-active (single bundle).
    FetchMarketMovers {
        fmp_key: String,
    },
    /// FMP intraday sector performance snapshot.
    FetchSectorPerformance {
        fmp_key: String,
    },
    /// FMP profile + key metrics for a symbol, combined into a WACC snapshot.
    /// `risk_free_pct` is passed in so the broker avoids touching non-Send state —
    /// the main thread sources the latest 10Y yield from its in-memory cache.
    FetchWaccSnapshot {
        symbol: String,
        fmp_key: String,
        risk_free_pct: f64,
    },
    // FX, beta, valuation, and identifier research
    /// Yahoo batch-quote the FX majors universe (no key).
    FetchCurrencyRates,
    /// FMP historical price fetch for (symbol, SPY) + OLS beta regression. The
    /// broker handler fetches both histories and computes the rolling windows.
    FetchBetaSnapshot {
        symbol: String,
        fmp_key: String,
    },
    /// Pure compute — read cached DVD + WACC, produce a Gordon Growth DDM
    /// snapshot. `required_return_pct` and `return_source` are sourced by the
    /// main thread from the in-memory WACC state (with a 10% fallback).
    ComputeDdmSnapshot {
        symbol: String,
        required_return_pct: f64,
        return_source: String,
    },
    /// Pure compute — read peer fundamentals from the in-memory cache (handed
    /// in as JSON by the main thread so the handler stays Send-safe) and
    /// produce a relative valuation snapshot.
    ComputeRelativeValuation {
        symbol: String,
        sector: String,
        self_json: String,
        peers_json: String,
    },
    /// POST to OpenFIGI — free instrument identifier mapping service.
    FetchFigiIdentifiers {
        symbol: String,
    },
    // Advanced valuation, options, and implied-volatility research
    /// Compute historical return / risk analysis for a symbol from cached bars.
    /// Broker handler reads cached HP rows via `get_historical_price` then calls
    /// `compute_hra_snapshot` — no network call.
    FetchHraSnapshot {
        symbol: String,
        risk_free_pct: f64,
    },
    /// DCF-on-FCFF fair value. All raw inputs are pre-computed on the main
    /// thread (TTM revenue/FCFF from `QuarterlyFinancial`, balance-sheet items
    /// from `Fundamentals`) and passed by value — the handler is pure compute.
    ComputeDcfSnapshot {
        symbol: String,
        base_revenue: f64,
        base_fcff: f64,
        growth_pct: f64,
        terminal_growth_pct: f64,
        wacc_pct: f64,
        tax_rate_pct: f64,
        projection_years: usize,
        total_debt: f64,
        cash_and_equivalents: f64,
        shares_outstanding: f64,
    },
    /// SVM synthesis — DDM / DCF snapshots are passed as JSON strings;
    /// pre-computed peer-median tuples are passed as JSON too. Pure compute.
    ComputeSvmSnapshot {
        symbol: String,
        current_price: f64,
        ddm_json: String,
        dcf_json: String,
        peer_pe_tuple_json: String, // Option<(f64, f64)>
        peer_ev_tuple_json: String, // Option<(f64, f64, f64, f64, f64)>
        peer_pb_tuple_json: String, // Option<(f64, f64)>
    },
    /// Yahoo options chain — `/v7/finance/options/{symbol}` (no key).
    FetchOptionsChain {
        symbol: String,
    },
    /// IV rank / percentile from stored OMON history. Main thread hands over
    /// the history rows as JSON so the handler stays Send-safe.
    ComputeIvolSnapshot {
        symbol: String,
        current_atm_iv_pct: f64,
        history_json: String,
    },
    // Seasonality, correlation, total-return, technical, and vol-skew research
    /// SEAG — seasonality compute over cached HP bars (pure compute).
    ComputeSeasonalitySnapshot {
        symbol: String,
    },
    /// COR — correlation matrix over cached HP for subject + peers.
    /// Peer bars are passed as JSON (`Vec<(String, Vec<HistoricalPriceRow>)>`)
    /// so the handler stays Send-safe.
    ComputeCorrelationMatrix {
        symbol: String,
        window_days: usize,
        peer_series_json: String,
    },
    /// TRA — total return (price + dividends) from cached HP + DVD.
    ComputeTotalReturnSnapshot {
        symbol: String,
    },
    /// TECH — technical indicators (RSI/MACD/BB/ATR/ADX/Stoch) from cached HP.
    ComputeTechnicalsSnapshot {
        symbol: String,
    },
    /// SKEW — volatility smile/skew compute over cached OMON.
    ComputeVolSkewSnapshot {
        symbol: String,
    },
    // Leverage, accruals, realized-volatility, cash-flow, and short-interest research
    /// LEV — debt leverage & coverage ratios from cached Financials + Fundamentals.
    /// `total_debt`/`cash` come from the main thread (Fundamentals) so the handler
    /// only needs to read research_financials + compute ratios.
    ComputeLeverageSnapshot {
        symbol: String,
        total_debt_fund: f64,
        cash_fund: f64,
    },
    /// ACRL — earnings quality (NI vs FCF) from cached quarterly Financials.
    ComputeAccrualsSnapshot {
        symbol: String,
    },
    /// RVOL — realized vol cone from cached HP bars; optional current ATM IV for regime.
    /// Bars are passed as JSON so the handler stays Send-safe.
    ComputeRealizedVolSnapshot {
        symbol: String,
        current_atm_iv_pct: Option<f64>,
        bars_json: String,
    },
    /// FCFY — FCF yield, payout ratios, dividend sustainability from cached Financials.
    /// Market cap + stock price come from the main thread (Fundamentals).
    ComputeFcfYieldSnapshot {
        symbol: String,
        market_cap: f64,
        stock_price: f64,
    },
    /// SHRT — short interest / days-to-cover from cached SharesFloat + HP bars.
    ComputeShortInterestSnapshot {
        symbol: String,
        shares_out: f64,
        float_shares: f64,
        short_pct_of_float: f64,
        short_ratio_reported: f64,
        bars_json: String,
    },
    // Solvency, quality, volatility-estimator, EPS-beat, and price-target research
    /// ALTZ — classic Altman Z-score; MVE comes from the main thread (Fundamentals).
    ComputeAltmanZSnapshot {
        symbol: String,
        market_value_equity: f64,
    },
    /// PTFS — Piotroski F-score over two annual periods from cached Financials.
    ComputePiotroskiSnapshot {
        symbol: String,
    },
    /// VOLE — OHLC volatility estimators (CtC, Parkinson, GK, RS, YZ) over cached HP bars.
    ComputeOhlcVolSnapshot {
        symbol: String,
        window_days: usize,
        bars_json: String,
    },
    /// EPSB — EPS beat streak / surprise analysis over cached earnings surprise history.
    ComputeEpsBeatSnapshot {
        symbol: String,
    },
    /// PTD — Price target dispersion & implied return; current price comes from the main thread.
    ComputePriceTargetDispersionSnapshot {
        symbol: String,
        current_price: f64,
    },
    // Insider, dividend-growth, earnings-revision, sector-rotation, and upgrade/downgrade research
    /// MNGR — Insider activity bias over cached INS form-4 trades in a lookback window.
    ComputeInsiderActivitySnapshot {
        symbol: String,
        window_days: i32,
    },
    /// DIVG — Dividend growth analysis (CAGR, consistency) over cached DVD history.
    ComputeDivgSnapshot {
        symbol: String,
    },
    /// EARM — Earnings momentum trend from cached FA quarterly statements + EPS surprises.
    ComputeEarmSnapshot {
        symbol: String,
    },
    /// SECTR — Sector rotation strength; symbol_sector read from Fundamentals on main thread.
    ComputeSectorRotationSnapshot {
        symbol: String,
        symbol_sector: String,
    },
    /// UPDM — Upgrade/downgrade momentum from cached RatingChange history.
    ComputeUpdmSnapshot {
        symbol: String,
    },
    // Momentum, liquidity, breakout, cash-cycle, and credit research
    /// MOM — 12-1 month momentum score over cached HP bars.
    ComputeMomentumSnapshot {
        symbol: String,
    },
    /// LIQ — Liquidity profile (ADV, Amihud, spread proxy, turnover); shares_outstanding from main thread.
    ComputeLiquiditySnapshot {
        symbol: String,
        window_days: i32,
        shares_outstanding: f64,
    },
    /// BREAK — Breakout proximity over cached HP bars (52w/60d/20d high/low, consolidation).
    ComputeBreakoutSnapshot {
        symbol: String,
    },
    /// CCRL — Cash conversion cycle (DSO + DIO - DPO) over cached FA statements.
    ComputeCashCycleSnapshot {
        symbol: String,
    },
    /// CREDIT — Unified credit score fusing cached ALTZ + PTFS + LEV + ACRL snapshots.
    ComputeCreditSnapshot {
        symbol: String,
    },
    /// GROWM — GARP composite fusing cached MOM + EARM + DIVG snapshots.
    ComputeGrowmSnapshot {
        symbol: String,
    },
    /// FLOW — Insider + institutional flow score over cached INS trades + HDS holders.
    ComputeFlowSnapshot {
        symbol: String,
        window_days: i32,
    },
    /// REGIME — Market regime classifier fusing cached VOLE + TECH + HRA snapshots.
    ComputeRegimeSnapshot {
        symbol: String,
    },
    /// RELVOL — Relative volume vs trailing averages over cached HP bars.
    ComputeRelvolSnapshot {
        symbol: String,
    },
    /// MARGINS — Margin trajectory (gross/op/net) over cached FA statements.
    ComputeMarginsSnapshot {
        symbol: String,
    },
    /// VAL — Value-factor composite vs sector peers.
    ComputeValSnapshot {
        symbol: String,
    },
    /// QUAL — Quality-factor composite fusing PTFS + MARGINS + ACRL + LEV.
    ComputeQualSnapshot {
        symbol: String,
    },
    /// RISK — Risk-factor composite fusing VOLE + BETA + LIQ + SHRT + ALTZ.
    ComputeRiskSnapshot {
        symbol: String,
    },
    /// INSSTRK — Insider streak detector from cached Form 4 trades.
    ComputeInsstrkSnapshot {
        symbol: String,
        window_days: i32,
    },
    /// COVG — Analyst coverage breadth + churn snapshot.
    ComputeCovgSnapshot {
        symbol: String,
    },
    /// VRK — Value Rank vs sector peers (percentile of VAL composite).
    ComputeVrkSnapshot {
        symbol: String,
    },
    /// QRK — Quality Rank vs sector peers (percentile of QUAL composite).
    ComputeQrkSnapshot {
        symbol: String,
    },
    /// RRK — Risk Rank vs sector peers (inverted: higher percentile = safer).
    ComputeRrkSnapshot {
        symbol: String,
    },
    /// RELEPSGR — Relative 3y EPS CAGR vs sector median.
    ComputeRelepsgrSnapshot {
        symbol: String,
    },
    /// PEAD — Post-earnings-announcement drift from cached surprises + HP bars.
    ComputePeadSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// SIZEF — Size factor rank vs sector (log market cap percentile).
    ComputeSizefSnapshot {
        symbol: String,
    },
    /// MOMF — Momentum factor rank vs sector peers (percentile of MOMENTUM composite).
    ComputeMomfSnapshot {
        symbol: String,
    },
    /// PEADRANK — PEAD drift rank vs sector peers (percentile of 5d avg drift).
    ComputePeadrankSnapshot {
        symbol: String,
    },
    /// FQM — Fundamental quality meter (Piotroski + margins + accruals, no leverage).
    ComputeFqmSnapshot {
        symbol: String,
    },
    /// REVRANK — Relative 3y revenue CAGR vs sector median.
    ComputeRevrankSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// LEVRANK — Leverage rank vs sector peers (D/E percentile, inverted).
    ComputeLevrankSnapshot {
        symbol: String,
    },
    /// OPERANK — Operating quality rank vs sector peers (latest op margin percentile).
    ComputeOperankSnapshot {
        symbol: String,
    },
    /// FQMRANK — Fundamental Quality Meter rank vs sector peers.
    ComputeFqmrankSnapshot {
        symbol: String,
    },
    /// LIQRANK — Liquidity rank vs sector peers (avg daily dollar volume percentile).
    ComputeLiqrankSnapshot {
        symbol: String,
    },
    /// SURPSTK — Earnings surprise streak stat (beats/misses, current streak, labels).
    ComputeSurpstkSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// DVDRANK — Dividend growth rank vs sector peers (3y CAGR percentile).
    ComputeDvdrankSnapshot {
        symbol: String,
    },
    /// EARMRANK — Earnings momentum rank vs sector peers (composite score percentile).
    ComputeEarmrankSnapshot {
        symbol: String,
    },
    /// UPDGRANK — Upgrade/downgrade net-90d rank vs sector peers.
    ComputeUpdgrankSnapshot {
        symbol: String,
    },
    /// GY — Gap yearly stat (253-bar gap census with labels).
    ComputeGySnapshot {
        symbol: String,
    },
    /// DES — Daily event streak stat (up/down/flat day distribution, longest runs).
    ComputeDesSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// DVDYIELDRANK — Dividend yield rank vs sector peers (non-payers filtered).
    ComputeDvdyieldrankSnapshot {
        symbol: String,
    },
    /// SHRANK — Short interest rank vs sector peers (risk-inverted).
    ComputeShrankSnapshot {
        symbol: String,
    },
    /// SHORTRANK_DELTA — 180d short-interest trend rank vs sector peers.
    ComputeShortrankDeltaSnapshot {
        symbol: String,
    },
    /// INSIDERCONC — insider ownership concentration vs sector peers.
    ComputeInsiderconcSnapshot {
        symbol: String,
    },
    /// ATRANN — Annualized Wilder ATR volatility regime.
    ComputeAtrannSnapshot {
        symbol: String,
    },
    /// DDHIST — Drawdown history stat (max dd, longest dd, corrections, current dd).
    ComputeDdhistSnapshot {
        symbol: String,
    },
    /// PRICEPERF — Multi-horizon price performance (1M/3M/6M/YTD/1Y returns).
    ComputePriceperfSnapshot {
        symbol: String,
    },
    /// MOMRANK_MULTI — sector-relative rank of cached PRICEPERF horizons.
    ComputeMomrankMultiSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// BETARANK — Beta rank vs sector peers (risk-inverted: lower beta = safer).
    ComputeBetarankSnapshot {
        symbol: String,
    },
    /// PEGRANK — PEG ratio rank vs sector peers (lower PEG = better value).
    ComputePegrankSnapshot {
        symbol: String,
    },
    /// FHIGHLOW — 52-week high/low distance + proximity band.
    ComputeFhighlowSnapshot {
        symbol: String,
    },
    /// RVCONE — Multi-horizon realized volatility cone (20d/60d/120d/252d).
    ComputeRvconeSnapshot {
        symbol: String,
    },
    /// CALPB — Calendar period breakdowns (MTD/QTD/YTD + prior quarter/year).
    ComputeCalpbSnapshot {
        symbol: String,
    },
    /// CORRSTK — rolling correlation vs SPY and sector ETF benchmark.
    ComputeCorrstkSnapshot {
        symbol: String,
        symbol_sector: String,
        fmp_key: String,
    },
    /// TLRANK — trailing 30-day dollar-volume rank vs sector peers.
    ComputeTlrankSnapshot {
        symbol: String,
    },
    /// CORRRANK — benchmark-linkage rank vs sector peers using cached CORRSTK.
    ComputeCorrrankSnapshot {
        symbol: String,
    },
    /// OPERANK_DELTA — operating-margin trend rank vs sector peers.
    ComputeOperankDeltaSnapshot {
        symbol: String,
    },
    /// DIVACC — dividend growth acceleration from cached dividend history.
    ComputeDivaccSnapshot {
        symbol: String,
    },
    /// EPSACC — EPS acceleration from cached quarterly financials.
    ComputeEpsaccSnapshot {
        symbol: String,
    },
    /// VRP — implied-vs-realized vol premium using cached IVOL + RVCONE.
    ComputeVrpSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// RETSKEW — Return distribution skewness (third standardized moment).
    ComputeRetskewSnapshot {
        symbol: String,
    },
    /// RETKURT — Return distribution excess kurtosis (fourth moment - 3).
    ComputeRetkurtSnapshot {
        symbol: String,
    },
    /// TAILR — Tail ratio (95th pct / |5th pct|).
    ComputeTailrSnapshot {
        symbol: String,
    },
    /// RUNLEN — Up/down day run length stats over trailing 253 sessions.
    ComputeRunlenSnapshot {
        symbol: String,
    },
    /// DAYRANGE — Daily high-low range stats (60d vs 252d baseline).
    ComputeDayrangeSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// AUTOCOR — Autocorrelation of log returns at lags 1/5/10/20.
    ComputeAutocorSnapshot {
        symbol: String,
    },
    /// HURST — Hurst exponent via rescaled-range (R/S) analysis.
    ComputeHurstSnapshot {
        symbol: String,
    },
    /// HITRATE — Multi-horizon win rate (5d/20d/60d/252d).
    ComputeHitrateSnapshot {
        symbol: String,
    },
    /// GLASYM — Gain/loss magnitude asymmetry.
    ComputeGlasymSnapshot {
        symbol: String,
    },
    /// VOLRATIO — Up-day vs down-day volume ratio.
    ComputeVolratioSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// DRAWUP — Upside rally history (mirror of DDHIST).
    ComputeDrawupSnapshot {
        symbol: String,
    },
    /// GAPSTATS — Overnight gap frequency + magnitude.
    ComputeGapstatsSnapshot {
        symbol: String,
    },
    /// VOLCLUSTER — ACF of squared / absolute returns (ARCH signature).
    ComputeVolclusterSnapshot {
        symbol: String,
    },
    /// CLOSEPLC — Close placement within daily range.
    ComputeCloseplcSnapshot {
        symbol: String,
    },
    /// MRHL — AR(1) mean-reversion half-life.
    ComputeMrhlSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// DOWNVOL — Downside deviation + Sortino ratio.
    ComputeDownvolSnapshot {
        symbol: String,
    },
    /// SHARPR — Sharpe ratio snapshot.
    ComputeSharprSnapshot {
        symbol: String,
    },
    /// EFFRATIO — Kaufman's efficiency ratio on closes.
    ComputeEffratioSnapshot {
        symbol: String,
    },
    /// WICKBIAS — Upper vs lower wick asymmetry.
    ComputeWickbiasSnapshot {
        symbol: String,
    },
    /// VOLOFVOL — Stdev of rolling 20-day realized vol.
    ComputeVolofvolSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// CALMAR — Calmar ratio (annualized return / max drawdown).
    ComputeCalmarSnapshot {
        symbol: String,
    },
    /// ULCER — Ulcer index + Martin ratio (UPI).
    ComputeUlcerSnapshot {
        symbol: String,
    },
    /// VARRATIO — Lo-MacKinlay variance ratio.
    ComputeVarratioSnapshot {
        symbol: String,
    },
    /// AMIHUD — Amihud illiquidity ratio.
    ComputeAmihudSnapshot {
        symbol: String,
    },
    /// JBNORM — Jarque-Bera normality test.
    ComputeJbnormSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// OMEGA — Omega ratio at threshold 0.
    ComputeOmegaSnapshot {
        symbol: String,
    },
    /// DFA — Detrended fluctuation analysis (Hurst alternative).
    ComputeDfaSnapshot {
        symbol: String,
    },
    /// BURKE — Burke ratio (sum-of-squared drawdowns adjusted return).
    ComputeBurkeSnapshot {
        symbol: String,
    },
    /// MONTHSEAS — Monthly seasonality hit rate.
    ComputeMonthseasSnapshot {
        symbol: String,
    },
    /// ROLLSPRD — Roll's (1984) implicit bid-ask spread.
    ComputeRollsprdSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// PARKINSON — Parkinson (1980) H-L range-based vol.
    ComputeParkinsonSnapshot {
        symbol: String,
    },
    /// GKVOL — Garman-Klass (1980) OHLC vol.
    ComputeGkvolSnapshot {
        symbol: String,
    },
    /// RSVOL — Rogers-Satchell drift-independent OHLC vol.
    ComputeRsvolSnapshot {
        symbol: String,
    },
    /// CVAR — Conditional VaR (Expected Shortfall) at 5% / 1%.
    ComputeCvarSnapshot {
        symbol: String,
    },
    /// DOWEFFECT — Day-of-week intraday seasonality.
    ComputeDoweffectSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// STERLING — Sterling ratio (mean of N worst drawdowns).
    ComputeSterlingSnapshot {
        symbol: String,
    },
    /// KELLYF — Kelly fraction / optimal leverage.
    ComputeKellyfSnapshot {
        symbol: String,
    },
    /// LJUNGB — Ljung-Box Q-statistic at lag 10.
    ComputeLjungbSnapshot {
        symbol: String,
    },
    /// RUNSTEST — Wald-Wolfowitz runs test.
    ComputeRunstestSnapshot {
        symbol: String,
    },
    /// ZERORET — Zero-return-day fraction (Lesmond-Ogden-Trzcinka).
    ComputeZeroretSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// PSR — Probabilistic Sharpe Ratio (Lopez de Prado 2012).
    ComputePsrSnapshot {
        symbol: String,
    },
    /// ADF — Dickey-Fuller unit-root / stationarity test.
    ComputeAdfSnapshot {
        symbol: String,
    },
    /// MNKENDALL — Mann-Kendall nonparametric trend test.
    ComputeMnkendallSnapshot {
        symbol: String,
    },
    /// BIPOWER — Bipower variation + realized-jump ratio.
    ComputeBipowerSnapshot {
        symbol: String,
    },
    /// DDDUR — Drawdown duration statistics.
    ComputeDddurSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// HILLTAIL — Hill tail-index estimator.
    ComputeHilltailSnapshot {
        symbol: String,
    },
    /// ARCHLM — Engle ARCH Lagrange-multiplier test.
    ComputeArchlmSnapshot {
        symbol: String,
    },
    /// PAINRATIO — Pain index and pain ratio.
    ComputePainratioSnapshot {
        symbol: String,
    },
    /// CUSUM — Brown-Durbin-Evans OLS CUSUM structural-break test.
    ComputeCusumSnapshot {
        symbol: String,
    },
    /// CFVAR — Cornish-Fisher modified Value-at-Risk.
    ComputeCfvarSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// ENTROPY — Shannon entropy of return distribution.
    ComputeEntropySnapshot {
        symbol: String,
    },
    /// RACHEV — Rachev ratio (conditional tail expectation ratio).
    ComputeRachevSnapshot {
        symbol: String,
    },
    /// GPR — Gain-to-Pain Ratio (Schwager).
    ComputeGprSnapshot {
        symbol: String,
    },
    /// PACF — Partial autocorrelation function lags 1-5.
    ComputePacfSnapshot {
        symbol: String,
    },
    /// APEN — Approximate entropy (Pincus 1991).
    ComputeApenSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// UPR — Upside Potential Ratio.
    ComputeUprSnapshot {
        symbol: String,
    },
    /// LEVEREFF — Leverage effect.
    ComputeLevereffSnapshot {
        symbol: String,
    },
    /// DRAWDAR — Drawdown-at-Risk + CDaR.
    ComputeDrawdarSnapshot {
        symbol: String,
    },
    /// VARHALF — Volatility half-life.
    ComputeVarhalfSnapshot {
        symbol: String,
    },
    /// GINI — Gini coefficient of |returns|.
    ComputeGiniSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// SAMPEN — Sample Entropy.
    ComputeSampenSnapshot {
        symbol: String,
    },
    /// PERMEN — Permutation Entropy.
    ComputePermenSnapshot {
        symbol: String,
    },
    /// RECFACT — Recovery Factor.
    ComputeRecfactSnapshot {
        symbol: String,
    },
    /// KPSS — KPSS stationarity test.
    ComputeKpssSnapshot {
        symbol: String,
    },
    /// SPECENT — Spectral Entropy.
    ComputeSpecentSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// ROBVOL — Robust Volatility (MAD+IQR).
    ComputeRobvolSnapshot {
        symbol: String,
    },
    /// RENYIENT — Rényi Entropy (α=2).
    ComputeRenyientSnapshot {
        symbol: String,
    },
    /// RETQUANT — Return Quantile Profile.
    ComputeRetquantSnapshot {
        symbol: String,
    },
    /// MSENT — Multiscale Entropy.
    ComputeMsentSnapshot {
        symbol: String,
    },
    /// EWMAVOL — EWMA Volatility (RiskMetrics λ=0.94).
    ComputeEwmavolSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// KSNORM — Kolmogorov-Smirnov normality test.
    ComputeKsnormSnapshot {
        symbol: String,
    },
    /// ADTEST — Anderson-Darling normality test.
    ComputeAdtestSnapshot {
        symbol: String,
    },
    /// LMOM — Hosking L-moments.
    ComputeLmomSnapshot {
        symbol: String,
    },
    /// KYLELAM — Kyle's price-impact λ.
    ComputeKylelamSnapshot {
        symbol: String,
    },
    /// PEAKOVER — Peaks-Over-Threshold (EVT).
    ComputePeakoverSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// HIGUCHI — Higuchi fractal dimension.
    ComputeHiguchiSnapshot {
        symbol: String,
    },
    /// PICKANDS — Pickands tail-index.
    ComputePickandsSnapshot {
        symbol: String,
    },
    /// KAPPA3 — Kaplan-Knowles Kappa-3 ratio.
    ComputeKappa3Snapshot {
        symbol: String,
    },
    /// LYAPUNOV — Rosenstein largest Lyapunov exponent.
    ComputeLyapunovSnapshot {
        symbol: String,
    },
    /// RANKAC — Spearman rank autocorrelation.
    ComputeRankacSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// BNSJUMP — Barndorff-Nielsen-Shephard 2006 jump-test Z-statistic.
    ComputeBnsjumpSnapshot {
        symbol: String,
    },
    /// PPROOT — Phillips-Perron 1988 unit-root test.
    ComputePprootSnapshot {
        symbol: String,
    },
    /// MFDFA — Multifractal Detrended Fluctuation Analysis.
    ComputeMfdfaSnapshot {
        symbol: String,
    },
    /// HILLKS — KS goodness-of-fit for Hill-tail Pareto model.
    ComputeHillksSnapshot {
        symbol: String,
    },
    /// TSI — Blau 1991 True Strength Index.
    ComputeTsiSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// GARCH11 — Bollerslev 1986 GARCH(1,1) fit.
    ComputeGarch11Snapshot {
        symbol: String,
    },
    /// SADF — Phillips-Wu-Yu 2011 Sup-ADF explosive-root test.
    ComputeSadfSnapshot {
        symbol: String,
    },
    /// CORDIM — Grassberger-Procaccia 1983 correlation dimension D2.
    ComputeCordimSnapshot {
        symbol: String,
    },
    /// SKSPEC — Rolling skewness spectrum / stability.
    ComputeSkspecSnapshot {
        symbol: String,
    },
    /// AUTOMI — Auto-mutual-information (information-theoretic ACF).
    ComputeAutomiSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// DURBINWATSON — d-statistic for first-order residual autocorrelation.
    ComputeDurbinWatsonSnapshot {
        symbol: String,
    },
    /// BDSTEST — Brock-Dechert-Scheinkman iid test on returns.
    ComputeBdsTestSnapshot {
        symbol: String,
    },
    /// BREUSCHPAGAN — LM test for heteroskedasticity of residuals.
    ComputeBreuschPaganSnapshot {
        symbol: String,
    },
    /// TURNPTS — Bartels turning-points test for non-randomness.
    ComputeTurnPtsSnapshot {
        symbol: String,
    },
    /// PERIODOGRAM — Direct-DFT periodogram / dominant-cycle detection.
    ComputePeriodogramSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// MCLEODLI — McLeod-Li portmanteau on squared returns (ARCH detection).
    ComputeMcLeodLiSnapshot {
        symbol: String,
    },
    /// OUFIT — Ornstein-Uhlenbeck mean-reversion fit on log-price.
    ComputeOuFitSnapshot {
        symbol: String,
    },
    /// GPH — Geweke-Porter-Hudak log-periodogram long-memory estimator.
    ComputeGphSnapshot {
        symbol: String,
    },
    /// BURGSPEC — Burg maximum-entropy AR spectral estimator.
    ComputeBurgSpecSnapshot {
        symbol: String,
    },
    /// KENDALLTAU — Kendall's tau lag-1 rank autocorrelation.
    ComputeKendallTauSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// SQUEEZE — composite short-squeeze outlier score per symbol.
    ComputeSqueezeSnapshot {
        symbol: String,
    },
    /// SQUEEZERANK — cross-symbol percentile rank of SQUEEZE composite.
    ComputeSqueezeRankSnapshot {
        symbol: String,
    },
    /// SQUEEZERANK — recompute SQUEEZE composites across all cached symbols
    /// and update SQUEEZERANK for each. Used by the standalone watchlist.
    RefreshSqueezeWatchlist,
    /// BBSQUEEZE — Bollinger-Band width squeeze detector.
    ComputeBbsqueezeSnapshot {
        symbol: String,
    },
    /// DONCHIAN — 20-bar Donchian channel breakout detector.
    ComputeDonchianSnapshot {
        symbol: String,
    },
    /// KAMA — Kaufman adaptive moving average efficiency ratio.
    ComputeKamaSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// ICHIMOKU — Ichimoku Kinko Hyo five-line cloud system.
    ComputeIchimokuSnapshot {
        symbol: String,
    },
    /// SUPERTREND — ATR-based trailing stop trend indicator.
    ComputeSupertrendSnapshot {
        symbol: String,
    },
    /// KELTNER — Keltner Channels (EMA 20 ± 2·ATR 10) with TTM-squeeze flag.
    ComputeKeltnerSnapshot {
        symbol: String,
    },
    /// FISHER — Fisher Transform of normalised price.
    ComputeFisherSnapshot {
        symbol: String,
    },
    /// AROON — Aroon Up/Down/Oscillator.
    ComputeAroonSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// ADX — Wilder's Average Directional Index.
    ComputeAdxSnapshot {
        symbol: String,
    },
    /// CCI — Lambert's Commodity Channel Index.
    ComputeCciSnapshot {
        symbol: String,
    },
    /// CMF — Chaikin Money Flow.
    ComputeCmfSnapshot {
        symbol: String,
    },
    /// MFI — Money Flow Index.
    ComputeMfiSnapshot {
        symbol: String,
    },
    /// PSAR — Wilder's Parabolic Stop-And-Reverse.
    ComputePsarSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// VORTEX — Botes & Siepman directional-movement alternative.
    ComputeVortexSnapshot {
        symbol: String,
    },
    /// CHOP — Choppiness Index.
    ComputeChopSnapshot {
        symbol: String,
    },
    /// OBV — On-Balance Volume (cumulative + 20-bar slope).
    ComputeObvSnapshot {
        symbol: String,
    },
    /// TRIX — triple-EMA momentum oscillator.
    ComputeTrixSnapshot {
        symbol: String,
    },
    /// HMA — Hull Moving Average.
    ComputeHmaSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// PPO — Percentage Price Oscillator (12/26/9).
    ComputePpoSnapshot {
        symbol: String,
    },
    /// DPO — Detrended Price Oscillator (period 20, shift 11).
    ComputeDpoSnapshot {
        symbol: String,
    },
    /// KST — Pring Know Sure Thing (weighted 4-ROC + 9 signal).
    ComputeKstSnapshot {
        symbol: String,
    },
    /// ULTOSC — Williams Ultimate Oscillator (7/14/28, 4/2/1 weights).
    ComputeUltoscSnapshot {
        symbol: String,
    },
    /// WILLR — Williams %R (period 14).
    ComputeWillrSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// MASS — Donald Dorsey Mass Index (EMA9 H-L ratio, 25-bar sum).
    ComputeMassSnapshot {
        symbol: String,
    },
    /// CHAIKOSC — Chaikin Oscillator (EMA3(AD) − EMA10(AD)).
    ComputeChaikoscSnapshot {
        symbol: String,
    },
    /// KLINGER — Klinger Volume Oscillator (34/55/13).
    ComputeKlingerSnapshot {
        symbol: String,
    },
    /// STOCHRSI — Stochastic RSI (14/14/3/3).
    ComputeStochRsiSnapshot {
        symbol: String,
    },
    /// AWESOME — Bill Williams Awesome Oscillator (SMA5 − SMA34 on hl2).
    ComputeAwesomeSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// EFI — Alexander Elder Force Index (EMA13 of volume × close change).
    ComputeEfiSnapshot {
        symbol: String,
    },
    /// EMV — Richard Arms Ease of Movement (SMA14 of midpoint-change / box-ratio).
    ComputeEmvSnapshot {
        symbol: String,
    },
    /// NVI — Dysart/Fosback Negative Volume Index (updates only on down-volume days).
    ComputeNviSnapshot {
        symbol: String,
    },
    /// PVI — Dysart/Fosback Positive Volume Index (updates only on up-volume days).
    ComputePviSnapshot {
        symbol: String,
    },
    /// COPPOCK — E.S.C. Coppock Curve (WMA10 of ROC14 + ROC11).
    ComputeCoppockSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// CMO — Tushar Chande Momentum Oscillator (raw gain/loss spread on [-100, +100]).
    ComputeCmoSnapshot {
        symbol: String,
    },
    /// QSTICK — Tushar Chande Q-Stick (SMA of close−open candle body).
    ComputeQstickSnapshot {
        symbol: String,
    },
    /// DISPARITY — Japanese Disparity Index (% deviation of close from SMA14).
    ComputeDisparitySnapshot {
        symbol: String,
    },
    /// BOP — Igor Livshin Balance of Power (SMA14 of (close−open)/(high−low)).
    ComputeBopSnapshot {
        symbol: String,
    },
    /// SCHAFF — Doug Schaff Trend Cycle (stochastic-of-MACD, double-smoothed, 23/50/10).
    ComputeSchaffSnapshot {
        symbol: String,
    },
    // ── Research moving-average palette aliases ──
    /// STOCH — George Lane Stochastic Oscillator (%K fast + %D smooth, 14/3/3).
    ComputeStochSnapshot {
        symbol: String,
    },
    /// MACD — Gerald Appel Moving Average Convergence Divergence (12/26/9 EMA).
    ComputeMacdSnapshot {
        symbol: String,
    },
    /// VWAP — Volume-Weighted Average Price (rolling 20-bar deviation).
    ComputeVwapSnapshot {
        symbol: String,
    },
    /// MCGD — John McGinley Dynamic (adaptive MA with volatility self-tuning, length 14).
    ComputeMcgdSnapshot {
        symbol: String,
    },
    /// RWI — Michael Poulos Random Walk Index (high/low 14-bar excursion).
    ComputeRwiSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// DEMA — Patrick Mulloy Double Exponential MA (length 20).
    ComputeDemaSnapshot {
        symbol: String,
    },
    /// TEMA — Patrick Mulloy Triple Exponential MA (length 20).
    ComputeTemaSnapshot {
        symbol: String,
    },
    /// LINREG — OLS linear regression on last 20 closes (slope + R² + channel).
    ComputeLinregSnapshot {
        symbol: String,
    },
    /// PIVOTS — classic floor-trader daily pivot levels (PP/R1/R2/S1/S2 from prior bar).
    ComputePivotsSnapshot {
        symbol: String,
    },
    /// HEIKIN — Heikin Ashi candle sentiment tracker with consecutive-run count.
    ComputeHeikinSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// ALMA — Arnaud Legoux Moving Average with Gaussian kernel (length 20).
    ComputeAlmaSnapshot {
        symbol: String,
    },
    /// ZLEMA — Zero-Lag EMA (Ehlers de-lag shift, length 20).
    ComputeZlemaSnapshot {
        symbol: String,
    },
    /// ELDERRAY — Elder Bull/Bear Power (dual-channel trend intensity, EMA13).
    ComputeElderRaySnapshot {
        symbol: String,
    },
    /// TSF — Time Series Forecast (OLS projected one bar forward).
    ComputeTsfSnapshot {
        symbol: String,
    },
    /// RVI — Relative Vigor Index (Ehlers closing-conviction oscillator).
    ComputeRviSnapshot {
        symbol: String,
    },
    /// TRIMA — Triangular Moving Average (SMA-of-SMA, length 20).
    ComputeTrimaSnapshot {
        symbol: String,
    },
    /// T3 — Tillson 1998 T3 composite MA (six-EMA chain, v=0.7).
    ComputeT3Snapshot {
        symbol: String,
    },
    /// VIDYA — Chande 1992 Variable Index Dynamic Average (CMO-adaptive alpha).
    ComputeVidyaSnapshot {
        symbol: String,
    },
    /// SMI — Stochastic Momentum Index (Blau 1993 double-smoothed).
    ComputeSmiSnapshot {
        symbol: String,
    },
    /// PVT — Price Volume Trend (Dysart/Lowry 1966 volume-weighted cumulative).
    ComputePvtSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// AC — Bill Williams Accelerator Oscillator (AO − SMA5(AO)).
    ComputeAcSnapshot {
        symbol: String,
    },
    /// CHVOL — Marc Chaikin's Volatility (ROC₁₀ of EMA₁₀(H−L)).
    ComputeChvolSnapshot {
        symbol: String,
    },
    /// BBWIDTH — Bollinger Bandwidth ((upper−lower)/middle) + 125-bar squeeze percentile.
    ComputeBbwidthSnapshot {
        symbol: String,
    },
    /// ELDERIMP — Dr. Elder's Impulse System (13-EMA slope + MACD-hist slope).
    ComputeElderImpSnapshot {
        symbol: String,
    },
    /// RMI — Roger Altman's Relative Momentum Index (RSI variant on N-bar momentum).
    ComputeRmiSnapshot {
        symbol: String,
    },
    // ── Options Expiration Calendar ──
    /// Tier 2: compute per-symbol expiration snapshot from cached options chain.
    ComputeSymbolExpirations {
        symbol: String,
    },
    // ── Research section ──
    /// SMMA — Wilder's Smoothed MA (α=1/N) with close-vs-SMMA deviation label.
    ComputeSmmaSnapshot {
        symbol: String,
    },
    /// ALLIGATOR — Bill Williams's 3-line shifted SMMA system on median price.
    ComputeAlligatorSnapshot {
        symbol: String,
    },
    /// CRSI — Connors RSI = mean(RSI₃ close, RSI₂ streak, percent_rank ROC₁/100).
    ComputeCrsiSnapshot {
        symbol: String,
    },
    /// SEB — Standard Error Bands (linreg endpoint ± k·SE channels).
    ComputeSebSnapshot {
        symbol: String,
    },
    /// IMI — Chande's Intraday Momentum Index (RSI-style on bar-local close−open).
    ComputeImiSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// GMMA — Guppy Multiple MA: 6 short + 6 long EMA groups with compression/gap labels.
    ComputeGmmaSnapshot {
        symbol: String,
    },
    /// MAENV — Moving Average Envelope (SMA ± pct bands, position label).
    ComputeMaenvSnapshot {
        symbol: String,
    },
    /// ADL — Chaikin Accumulation/Distribution Line (cumulative MFM·V + slope label).
    ComputeAdlSnapshot {
        symbol: String,
    },
    /// VHF — Adam White's Vertical Horizontal Filter (trending vs ranging).
    ComputeVhfSnapshot {
        symbol: String,
    },
    /// VROC — Volume Rate of Change (14-bar ROC of volume, surge/quiet labels).
    ComputeVrocSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// KDJ — Chinese Stochastic variant (%K, %D, J=3K−2D on 9/3 settings).
    ComputeKdjSnapshot {
        symbol: String,
    },
    /// QQE — Quantitative Qualitative Estimation (smoothed RSI with adaptive trailing bands).
    ComputeQqeSnapshot {
        symbol: String,
    },
    /// PMO — Pring's Price Momentum Oscillator (double-smoothed ROC + signal line).
    ComputePmoSnapshot {
        symbol: String,
    },
    /// CFO — Chande's Forecast Oscillator (100·(close − linreg_forecast)/close).
    ComputeCfoSnapshot {
        symbol: String,
    },
    /// TMF — Twiggs Money Flow (EMA-smoothed volume-weighted money flow using true range).
    ComputeTmfSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// FRACTALS — Bill Williams 5-bar peak/trough structural pivot markers.
    ComputeFractalsSnapshot {
        symbol: String,
    },
    /// IFT_RSI — Ehlers Inverse Fisher Transform of RSI (bounded [-1, 1] oscillator).
    ComputeIftRsiSnapshot {
        symbol: String,
    },
    /// MAMA — MESA Adaptive Moving Average (Hilbert-transform phase-adaptive MA + FAMA companion).
    ComputeMamaSnapshot {
        symbol: String,
    },
    /// COG — Ehlers Center of Gravity zero-lag oscillator (recency-weighted centroid).
    ComputeCogSnapshot {
        symbol: String,
    },
    /// DIDI — Didi Aguiar's 3-SMA Brazilian "didi needles" crossover system.
    ComputeDidiSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// DEMARKER — Tom DeMark's DeMarker (bounded [0,1] high/low-range oscillator).
    ComputeDemarkerSnapshot {
        symbol: String,
    },
    /// GATOR — Bill Williams Gator Oscillator (jaw/teeth/lips SMMA-spread life-cycle).
    ComputeGatorSnapshot {
        symbol: String,
    },
    /// BW_MFI — Bill Williams Market Facilitation Index (range-per-volume color classifier).
    ComputeBwMfiSnapshot {
        symbol: String,
    },
    /// VWMA — Volume Weighted Moving Average (volume-weighted SMA) vs plain SMA.
    ComputeVwmaSnapshot {
        symbol: String,
    },
    /// STDDEV — Rolling sample standard deviation of close over N=20 + 60-bar regime.
    ComputeStddevSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// WMA — Weighted Moving Average (linearly-weighted SMA variant).
    ComputeWmaSnapshot {
        symbol: String,
    },
    /// RAINBOW — 10-level recursive SMA rainbow (Mel Widner).
    ComputeRainbowSnapshot {
        symbol: String,
    },
    /// MESA_SINE — Ehlers MESA Sine Wave (cycle phase + lead-sine).
    ComputeMesaSineSnapshot {
        symbol: String,
    },
    /// FRAMA — Fractal Adaptive Moving Average (Ehlers, D-driven α).
    ComputeFramaSnapshot {
        symbol: String,
    },
    /// IBS — Internal Bar Strength ((close−low)/(high−low) with 14-bar SMA).
    ComputeIbsSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// LAGUERRE_RSI — Ehlers 4-stage Laguerre filter bounded oscillator.
    ComputeLaguerreRsiSnapshot {
        symbol: String,
    },
    /// ZIGZAG — %-threshold pivot reversal detector (5% default).
    ComputeZigzagSnapshot {
        symbol: String,
    },
    /// PGO — Mark Johnson's Pretty Good Oscillator ((close−SMA)/EMA(TR)).
    ComputePgoSnapshot {
        symbol: String,
    },
    /// HT_TRENDLINE — Hilbert-period-adaptive WMA trendline.
    ComputeHtTrendlineSnapshot {
        symbol: String,
    },
    /// MIDPOINT — (HHV(14) + LLV(14)) / 2 with close position in range.
    ComputeMidpointSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// MASSINDEX — Dorsey Mass Index: Σ(EMA(H-L,9) / EMA(EMA(H-L,9),9)) over 25 bars.
    ComputeMassIndexSnapshot {
        symbol: String,
    },
    /// NATR — normalized ATR: 100 × Wilder_ATR(14) / close.
    ComputeNatrSnapshot {
        symbol: String,
    },
    /// TTM_SQUEEZE — Carter's BB⊂KC squeeze regime + momentum oscillator.
    ComputeTtmSqueezeSnapshot {
        symbol: String,
    },
    /// FORCE_INDEX — Elder's Force Index: EMA(volume × (close − close_prev), 13).
    ComputeForceIndexSnapshot {
        symbol: String,
    },
    /// TRANGE — single-bar True Range: max(H−L, |H−C_prev|, |L−C_prev|).
    ComputeTrangeSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// LINEARREG_SLOPE — least-squares slope of N-bar linreg on close (TA-Lib).
    ComputeLinearregSlopeSnapshot {
        symbol: String,
    },
    /// HT_DCPERIOD — Ehlers Hilbert dominant cycle period in bars.
    ComputeHtDcperiodSnapshot {
        symbol: String,
    },
    /// HT_TRENDMODE — Ehlers trend-vs-cycle regime flag (0/1) + lock-in duration.
    ComputeHtTrendmodeSnapshot {
        symbol: String,
    },
    /// ACCBANDS — Price Headley Acceleration Bands (H×(1+4·range/sum), SMA-20).
    ComputeAccbandsSnapshot {
        symbol: String,
    },
    /// STOCHF — TA-Lib Fast Stochastic (%K + %D without slow smoothing).
    ComputeStochfSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// LINEARREG — TA-Lib Linear Regression fitted endpoint (14-bar LS).
    ComputeLinearregSnapshot {
        symbol: String,
    },
    /// LINEARREG_ANGLE — atan(slope)·180/π of 14-bar least-squares fit.
    ComputeLinearregAngleSnapshot {
        symbol: String,
    },
    /// HT_DCPHASE — Ehlers Hilbert dominant cycle phase in degrees.
    ComputeHtDcphaseSnapshot {
        symbol: String,
    },
    /// HT_SINE — Ehlers sine + leadsine wave with 45° lead.
    ComputeHtSineSnapshot {
        symbol: String,
    },
    /// HT_PHASOR — Ehlers raw I/Q components + magnitude + phase.
    ComputeHtPhasorSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// MIDPRICE — (HHV(H, 14) + LLV(L, 14)) / 2 range midpoint.
    ComputeMidpriceSnapshot {
        symbol: String,
    },
    /// APO — TA-Lib Absolute Price Oscillator (fast=12, slow=26 EMA diff).
    ComputeApoSnapshot {
        symbol: String,
    },
    /// MOM — raw close − close[n−10] momentum.
    ComputeMomSnapshot {
        symbol: String,
    },
    /// SAREXT — Extended Parabolic SAR with asymmetric long/short AF.
    ComputeSarextSnapshot {
        symbol: String,
    },
    /// ADXR — (ADX + ADX[n−14]) / 2 Average Directional Movement Rating.
    ComputeAdxrSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// AVGPRICE — (open + high + low + close) / 4.
    ComputeAvgpriceSnapshot {
        symbol: String,
    },
    /// MEDPRICE — (high + low) / 2 range midpoint.
    ComputeMedpriceSnapshot {
        symbol: String,
    },
    /// TYPPRICE — (high + low + close) / 3 typical price.
    ComputeTypPriceSnapshot {
        symbol: String,
    },
    /// WCLPRICE — (high + low + 2 × close) / 4 weighted close.
    ComputeWclPriceSnapshot {
        symbol: String,
    },
    /// VARIANCE — population variance of close over 5 bars (TA-Lib default).
    ComputeVarianceSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// PLUS_DI — Wilder's Positive Directional Indicator (period 14).
    ComputePlusDiSnapshot {
        symbol: String,
    },
    /// MINUS_DI — Wilder's Negative Directional Indicator (period 14).
    ComputeMinusDiSnapshot {
        symbol: String,
    },
    /// PLUS_DM — Wilder's raw Positive Directional Movement (period 14).
    ComputePlusDmSnapshot {
        symbol: String,
    },
    /// MINUS_DM — Wilder's raw Negative Directional Movement (period 14).
    ComputeMinusDmSnapshot {
        symbol: String,
    },
    /// DX — Wilder's Directional Movement Index (period 14).
    ComputeDxSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// ROC — raw Rate of Change `close_t − close_{t−n}` (period 10).
    ComputeRocSnapshot {
        symbol: String,
    },
    /// ROCP — Rate of Change Percentage (period 10).
    ComputeRocpSnapshot {
        symbol: String,
    },
    /// ROCR — Rate of Change Ratio (period 10).
    ComputeRocrSnapshot {
        symbol: String,
    },
    /// ROCR100 — Rate of Change Ratio × 100 (period 10).
    ComputeRocr100Snapshot {
        symbol: String,
    },
    /// CORREL — lag-1 autocorrelation of close over 30 bars.
    ComputeCorrelSnapshot {
        symbol: String,
    },
    // ── Research section ──
    /// MIN — rolling-window minimum of close (period 30).
    ComputeMinSnapshot {
        symbol: String,
    },
    /// MAX — rolling-window maximum of close (period 30).
    ComputeMaxSnapshot {
        symbol: String,
    },
    /// MINMAX — both endpoints + range width (period 30).
    ComputeMinMaxSnapshot {
        symbol: String,
    },
    /// MININDEX — recency-in-bars of the window low (period 30).
    ComputeMinIndexSnapshot {
        symbol: String,
    },
    /// MAXINDEX — recency-in-bars of the window high (period 30).
    ComputeMaxIndexSnapshot {
        symbol: String,
    },
    // ── BBANDS / AD / ADOSC / SUM / LINEARREG_INTERCEPT ──
    /// BBANDS — Bollinger Bands (period 20, 2σ).
    ComputeBbandsSnapshot {
        symbol: String,
    },
    /// AD — Chaikin Accumulation/Distribution Line.
    ComputeAdSnapshot {
        symbol: String,
    },
    /// ADOSC — Chaikin A/D Oscillator (3/10).
    ComputeAdoscSnapshot {
        symbol: String,
    },
    /// SUM — rolling sum of close over period 30.
    ComputeSumSnapshot {
        symbol: String,
    },
    /// LINEARREG_INTERCEPT — intercept coefficient of 14-bar linear regression.
    ComputeLinearRegInterceptSnapshot {
        symbol: String,
    },
    // ── AROONOSC / MINMAXINDEX / MACDEXT / MACDFIX / MAVP ──
    /// AROONOSC — Aroon Oscillator (period 14, = aroon_up - aroon_down).
    ComputeAroonoscSnapshot {
        symbol: String,
    },
    /// MINMAXINDEX — combined min+max index recency (period 30).
    ComputeMinMaxIndexSnapshot {
        symbol: String,
    },
    /// MACDEXT — MACD with SMA for fast/slow/signal (12/26/9).
    ComputeMacdextSnapshot {
        symbol: String,
    },
    /// MACDFIX — MACD with hardcoded 12/26 fast/slow + configurable signal.
    ComputeMacdfixSnapshot {
        symbol: String,
    },
    /// MAVP — Moving Average with Variable Period (5..30 linear ramp).
    ComputeMavpSnapshot {
        symbol: String,
    },
    // ── CDLDOJI / CDLHAMMER / CDLSHOOTINGSTAR / CDLENGULFING / CDLHARAMI ──
    /// CDLDOJI — single-bar doji pattern (body ≤ 5% range).
    ComputeCdlDojiSnapshot {
        symbol: String,
    },
    /// CDLHAMMER — single-bar hammer pattern (long lower shadow, small body upper).
    ComputeCdlHammerSnapshot {
        symbol: String,
    },
    /// CDLSHOOTINGSTAR — single-bar shooting star pattern (long upper shadow, small body lower).
    ComputeCdlShootingStarSnapshot {
        symbol: String,
    },
    /// CDLENGULFING — two-bar engulfing pattern (current body engulfs prior, opposite direction).
    ComputeCdlEngulfingSnapshot {
        symbol: String,
    },
    /// CDLHARAMI — two-bar harami/inside-bar pattern (current contained in prior, opposite direction).
    ComputeCdlHaramiSnapshot {
        symbol: String,
    },
    // ── CDLMORNINGSTAR / CDLEVENINGSTAR / CDL3BLACKCROWS / CDL3WHITESOLDIERS / CDLDARKCLOUDCOVER ──
    /// CDLMORNINGSTAR — 3-bar bullish reversal (big red, star, big green).
    ComputeCdlMorningStarSnapshot {
        symbol: String,
    },
    /// CDLEVENINGSTAR — 3-bar bearish reversal (big green, star, big red).
    ComputeCdlEveningStarSnapshot {
        symbol: String,
    },
    /// CDL3BLACKCROWS — 3 consecutive red bars (bearish continuation).
    ComputeCdlThreeBlackCrowsSnapshot {
        symbol: String,
    },
    /// CDL3WHITESOLDIERS — 3 consecutive green bars (bullish continuation).
    ComputeCdlThreeWhiteSoldiersSnapshot {
        symbol: String,
    },
    /// CDLDARKCLOUDCOVER — 2-bar bearish reversal (red opens above prior high, closes below midpoint).
    ComputeCdlDarkCloudCoverSnapshot {
        symbol: String,
    },
    // ── CDLPIERCING / CDLDRAGONFLYDOJI / CDLGRAVESTONEDOJI / CDLHANGINGMAN / CDLINVERTEDHAMMER ──
    /// CDLPIERCING — 2-bar bullish reversal (mirror of DarkCloudCover).
    ComputeCdlPiercingSnapshot {
        symbol: String,
    },
    /// CDLDRAGONFLYDOJI — single-bar doji with long lower shadow (T-shape support).
    ComputeCdlDragonflyDojiSnapshot {
        symbol: String,
    },
    /// CDLGRAVESTONEDOJI — single-bar doji with long upper shadow (inverted-T resistance).
    ComputeCdlGravestoneDojiSnapshot {
        symbol: String,
    },
    /// CDLHANGINGMAN — single-bar bearish reversal at tops (hammer geometry, -100 convention).
    ComputeCdlHangingManSnapshot {
        symbol: String,
    },
    /// CDLINVERTEDHAMMER — single-bar bullish reversal at bottoms (shooting-star geometry, +100 convention).
    ComputeCdlInvertedHammerSnapshot {
        symbol: String,
    },
    // ── CDLHARAMICROSS / CDLLONGLEGGEDDOJI / CDLMARUBOZU / CDLSPINNINGTOP / CDLTRISTAR ──
    /// CDLHARAMICROSS — 2-bar harami where inside bar is a doji (stricter reversal).
    ComputeCdlHaramiCrossSnapshot {
        symbol: String,
    },
    /// CDLLONGLEGGEDDOJI — single-bar doji with both shadows dominant (≥ 30% each).
    ComputeCdlLongLeggedDojiSnapshot {
        symbol: String,
    },
    /// CDLMARUBOZU — single-bar full-body pattern (shadows ≤ 5%, body ≥ 90%).
    ComputeCdlMarubozuSnapshot {
        symbol: String,
    },
    /// CDLSPINNINGTOP — single-bar indecision (body ≤ 30%, both shadows > body).
    ComputeCdlSpinningTopSnapshot {
        symbol: String,
    },
    /// CDLTRISTAR — 3-bar triple-doji reversal (rare high-conviction signal).
    ComputeCdlTristarSnapshot {
        symbol: String,
    },
    // ── CDLDOJISTAR / CDLMORNINGDOJISTAR / CDLEVENINGDOJISTAR / CDLABANDONEDBABY / CDL3INSIDE ──
    /// CDLDOJISTAR — 2-bar reversal precursor: prior body + doji that gaps away from prior close.
    ComputeCdlDojiStarSnapshot {
        symbol: String,
    },
    /// CDLMORNINGDOJISTAR — 3-bar bullish reversal: long red + gap-down doji + strong green close above bar-1 midpoint.
    ComputeCdlMorningDojiStarSnapshot {
        symbol: String,
    },
    /// CDLEVENINGDOJISTAR — 3-bar bearish reversal: long green + gap-up doji + strong red close below bar-1 midpoint.
    ComputeCdlEveningDojiStarSnapshot {
        symbol: String,
    },
    /// CDLABANDONEDBABY — strongest 3-bar star variant: doji isolated by full-shadow gaps on both sides.
    ComputeCdlAbandonedBabySnapshot {
        symbol: String,
    },
    /// CDL3INSIDE — confirmed Harami: Harami bars 1-2 + bar 3 closes past bar 1 body in opposite direction.
    ComputeCdlThreeInsideSnapshot {
        symbol: String,
    },
    // ── CDLBELTHOLD / CDLCLOSINGMARUBOZU / CDLHIGHWAVE / CDLLONGLINE / CDLSHORTLINE ──
    /// CDLBELTHOLD — single-bar long body with virtually no opening shadow.
    ComputeCdlBeltHoldSnapshot {
        symbol: String,
    },
    /// CDLCLOSINGMARUBOZU — single-bar long body with virtually no closing shadow.
    ComputeCdlClosingMarubozuSnapshot {
        symbol: String,
    },
    /// CDLHIGHWAVE — single-bar small body with long upper/lower shadows.
    ComputeCdlHighWaveSnapshot {
        symbol: String,
    },
    /// CDLLONGLINE — single-bar long real body with relatively small shadows.
    ComputeCdlLongLineSnapshot {
        symbol: String,
    },
    /// CDLSHORTLINE — single-bar short real body with relatively small shadows.
    ComputeCdlShortLineSnapshot {
        symbol: String,
    },
    // ── CDLCOUNTERATTACK / CDLHOMINGPIGEON / CDLINNECK / CDLONNECK / CDLTHRUSTING ──
    /// CDLCOUNTERATTACK — opposite-colour long bodies with a directional gap and a close back near the prior close.
    ComputeCdlCounterattackSnapshot {
        symbol: String,
    },
    /// CDLHOMINGPIGEON — bearish harami variant: long red body then smaller red body inside it.
    ComputeCdlHomingPigeonSnapshot {
        symbol: String,
    },
    /// CDLINNECK — long red bar then gap-down green bar closing slightly into the prior body.
    ComputeCdlInNeckSnapshot {
        symbol: String,
    },
    /// CDLONNECK — long red bar then gap-down green bar closing back at or near the prior close.
    ComputeCdlOnNeckSnapshot {
        symbol: String,
    },
    /// CDLTHRUSTING — long red bar then gap-down green bar closing deeper into the prior body, but below the midpoint.
    ComputeCdlThrustingSnapshot {
        symbol: String,
    },
    // ── CDL2CROWS / CDL3LINESTRIKE / CDL3OUTSIDE / CDLMATCHINGLOW ──
    /// CDL2CROWS — long green candle, gap-up red candle, then a second red candle that closes back inside the first body.
    ComputeCdlTwoCrowsSnapshot {
        symbol: String,
    },
    /// CDL3LINESTRIKE — three same-direction candles followed by an opposite strike candle that closes beyond the first open.
    ComputeCdlThreeLineStrikeSnapshot {
        symbol: String,
    },
    /// CDL3OUTSIDE — engulfing reversal confirmed by a third bar in the same direction.
    ComputeCdlThreeOutsideSnapshot {
        symbol: String,
    },
    /// CDLMATCHINGLOW — two red candles closing at nearly the same level.
    ComputeCdlMatchingLowSnapshot {
        symbol: String,
    },
    // ── CDLSEPARATINGLINES / CDLSTICKSANDWICH / CDLRICKSHAWMAN / CDLTAKURI ──
    /// CDLSEPARATINGLINES — opposite-colour candles sharing the same open, with the second resuming trend.
    ComputeCdlSeparatingLinesSnapshot {
        symbol: String,
    },
    /// CDLSTICKSANDWICH — red/green/red sequence with the first and third closes matching.
    ComputeCdlStickSandwichSnapshot {
        symbol: String,
    },
    /// CDLRICKSHAWMAN — centered doji with long shadows on both sides.
    ComputeCdlRickshawManSnapshot {
        symbol: String,
    },
    /// CDLTAKURI — dragonfly-style doji with an especially long lower shadow.
    ComputeCdlTakuriSnapshot {
        symbol: String,
    },
    // ── harder CDL* parity pack ──
    /// CDL3STARSINSOUTH — three descending red candles with progressively contracting downside pressure.
    ComputeCdlThreeStarsInSouthSnapshot {
        symbol: String,
    },
    /// CDLIDENTICAL3CROWS — three long red candles opening near each prior close.
    ComputeCdlIdenticalThreeCrowsSnapshot {
        symbol: String,
    },
    /// CDLKICKING — opposite-colour marubozu candles separated by a clean full-range gap.
    ComputeCdlKickingSnapshot {
        symbol: String,
    },
    /// CDLKICKINGBYLENGTH — kicking pattern whose sign is assigned from the longer body.
    ComputeCdlKickingByLengthSnapshot {
        symbol: String,
    },
    /// CDLLADDERBOTTOM — five-bar bullish reversal with three descending red bars, a rung candle, then breakout.
    ComputeCdlLadderBottomSnapshot {
        symbol: String,
    },
    /// CDLUNIQUE3RIVER — long red candle, lower-shadow red candle, then a small green candle inside the second.
    ComputeCdlUniqueThreeRiverSnapshot {
        symbol: String,
    },
    // ── additional multi-bar CDL* parity pack ──
    /// CDLADVANCEBLOCK — three rising green candles with shrinking progress and longer upper shadows.
    ComputeCdlAdvanceBlockSnapshot {
        symbol: String,
    },
    /// CDLBREAKAWAY — five-bar reversal with an initial directional gap and final retracement back into it.
    ComputeCdlBreakawaySnapshot {
        symbol: String,
    },
    /// CDLGAPSIDESIDEWHITE — two similar green candles that preserve an up-gap or down-gap.
    ComputeCdlGapSideSideWhiteSnapshot {
        symbol: String,
    },
    /// CDLUPSIDEGAP2CROWS — long green candle, gap-up red candle, then another red candle closing into the gap.
    ComputeCdlUpsideGapTwoCrowsSnapshot {
        symbol: String,
    },
    /// CDLXSIDEGAP3METHODS — two same-direction gapped candles followed by an opposite-colour gap-filling candle.
    ComputeCdlXSideGapThreeMethodsSnapshot {
        symbol: String,
    },
    /// CDLCONCEALBABYSWALL — four black candles where the fourth engulfs the third after a gap-down sequence.
    ComputeCdlConcealBabySwallowSnapshot {
        symbol: String,
    },
    // ── stateful CDL* parity pack ──
    /// CDLHIKKAKE — inside-bar trap with a false break to one side.
    ComputeCdlHikkakeSnapshot {
        symbol: String,
    },
    /// CDLHIKKAKEMOD — Hikkake pattern with a separate confirmation bar.
    ComputeCdlHikkakeModSnapshot {
        symbol: String,
    },
    /// CDLMATHOLD — five-bar continuation with a gap-up/down pause and breakout.
    ComputeCdlMatHoldSnapshot {
        symbol: String,
    },
    /// CDLRISEFALL3METHODS — long trend candle, three contained pullback candles, then continuation.
    ComputeCdlRiseFallThreeMethodsSnapshot {
        symbol: String,
    },
    // ── final CDL* parity pack ──
    /// CDLSTALLEDPATTERN — three advancing green candles where the third stalls with a small body and upper shadow.
    ComputeCdlStalledPatternSnapshot {
        symbol: String,
    },
    /// CDLTASUKIGAP — gap continuation pattern with an opposite-colour retracement candle.
    ComputeCdlTasukiGapSnapshot {
        symbol: String,
    },
    // ── (Quant Stats) ──
    /// MODSHARPE — Pezier-White Adjusted Sharpe Ratio (skew/kurt adjusted).
    ComputeModSharpeSnapshot {
        symbol: String,
    },
    /// HSIEHTEST — Hsieh (1989) third-moment nonlinearity test on AR(1) residuals.
    ComputeHsiehTestSnapshot {
        symbol: String,
    },
    /// CHOWBREAK — Chow structural-break F-test at the midpoint of the series.
    ComputeChowBreakSnapshot {
        symbol: String,
    },
    /// DRIFTBURST — Christensen-Oomen-Renò drift-burst kernel-weighted statistic.
    ComputeDriftBurstSnapshot {
        symbol: String,
    },
    /// HLVCLUST — Parkinson high-low volatility clustering Ljung-Box at lag 10.
    ComputeHlvClustSnapshot {
        symbol: String,
    },
    // ── YANGZHANG / KUIPER / DAGOSTINO / BAIPERRON / KUPIECPOF ──
    /// YANGZHANG — Yang-Zhang three-component range volatility estimator.
    ComputeYangZhangSnapshot {
        symbol: String,
    },
    /// KUIPER — Kuiper two-sided empirical CDF goodness-of-fit vs standard normal.
    ComputeKuiperSnapshot {
        symbol: String,
    },
    /// DAGOSTINO — D'Agostino-Pearson K² omnibus normality test.
    ComputeDagostinoSnapshot {
        symbol: String,
    },
    /// BAIPERRON — Bai-Perron sup-F structural-break search over interior [0.15n, 0.85n].
    ComputeBaiPerronSnapshot {
        symbol: String,
    },
    /// KUPIECPOF — Kupiec Proportion-of-Failures VaR exceedance backtest.
    ComputeKupiecPofSnapshot {
        symbol: String,
    },
    // ── web article ingestion ──
    /// Parse an AI agent reply, extract any `===TYPHOON_INGEST===` fenced
    /// blocks, and merge the discovered articles into the per-symbol
    /// `research_web_articles` cache.
    IngestResearchArticles {
        text: String,
        agent_override: String,
    },
    /// Fetch multi-source news for a symbol, cache in SQLite, return cached set.
    /// Routes equity vs crypto sources internally via `news::is_crypto_symbol`.
    FetchNewsMulti {
        symbol: String,
        marketaux_key: String,
        alpha_vantage_key: String,
        fmp_key: String,
        finnhub_key: String,
        cryptopanic_key: String,
    },
    /// Read cached news for a symbol from SQLite without fetching.
    LoadCachedNews {
        symbol: String,
        limit: usize,
    },
    /// Foreground-fetch the body for one article and refresh the cached
    /// news list for the symbol when the body arrives. Triggered by the
    /// news window's on-click handler so a user clicking an unhydrated
    /// article doesn't have to wait for the next background tick.
    HydrateNewsArticle {
        symbol: String,
        url_hash: String,
        url: String,
    },
    /// Full-text search cached news across all symbols.
    SearchNews {
        query: String,
        limit: usize,
    },
    /// Scrape news across all enabled source-universe symbols.
    /// Long-running: hits 3-6 APIs per symbol with rate-limiting sleeps.
    NewsScrapeAll {
        use_alpaca: bool,
        use_kraken: bool,
        marketaux_key: String,
        alpha_vantage_key: String,
        fmp_key: String,
        finnhub_key: String,
        cryptopanic_key: String,
    },
    /// Scrape news for an explicit, already-normalized symbol set. Used by MTF Grid
    /// so multiple timeframe cells for the same ticker fetch once.
    NewsScrapeSymbols {
        symbols: Vec<String>,
        marketaux_key: String,
        alpha_vantage_key: String,
        fmp_key: String,
        finnhub_key: String,
        cryptopanic_key: String,
    },
    /// Connect to Kraken crypto exchange. The four scalar fields are the
    /// primary account (REST + optional WS-token key override, unchanged
    /// wire-compat shape); `extra_accounts` are additional Kraken logins for
    /// account-level primary cycling and future TradeCopy targets. Kraken
    /// market data is public, so extra accounts do not join bar sync.
    KrakenConnect {
        api_key: String,
        api_secret: String,
        ws_api_key: String,
        ws_api_secret: String,
        extra_accounts: Vec<BrokerAccountSpec>,
    },
    /// Get Kraken account balance.
    KrakenGetBalance,
    KrakenGetPositions,
    /// Place an order on Kraken.
    KrakenPlaceOrder {
        pair: String,
        side: String,
        order_type: String,
        volume: f64,
        price: Option<f64>,
        leverage: Option<String>,
    },
    KrakenPlaceOrderAdvanced {
        order: crate::broker::kraken::KrakenOrderRequest,
    },
    /// Cancel a Kraken order by transaction ID.
    KrakenCancelOrder {
        txid: String,
    },
    /// Cancel all open Kraken orders.
    KrakenCancelAll,
    KrakenFetchTrades,
    KrakenFetchOpenOrders,
    KrakenFetchEquityTicker {
        symbol: String,
    },
    KrakenFetchEquityHistory {
        symbol: String,
        timeframe: String,
    },
    YahooChartFetchBars {
        symbol: String,
        timeframe: String,
    },
    KrakenFetchEquityUniverse,
    KrakenStartPrivateWs,
    /// Start the Kraken WS v2 OHLC streamers for the selected intervals.
    /// Subscribes to the provided pairs and merges live bar updates
    /// into the existing `kraken:SYMBOL:TF` cache keys.
    KrakenStartOhlcStreamers {
        pairs: Vec<String>,
        intervals_min: Vec<u32>,
    },
    /// Run one bounded Kraken WS OHLC snapshot sweep batch and then unsubscribe.
    KrakenOhlcSnapshotSweep {
        interval_min: u32,
        pairs: Vec<String>,
    },
    KrakenStartOrderbookWs {
        symbol: String,
        depth: usize,
        publish_dom: bool,
    },
    /// Start Kraken WS v2 ticker (L1) for rich bid/ask + volume + last etc.
    KrakenStartTickerWs {
        symbol: String,
    },
    /// Start Kraken Level 3 (authenticated, per-order book) — rich but limited availability.
    KrakenStartLevel3Ws {
        symbol: String,
    },
    KrakenClosePosition {
        pair: String,
        volume: Option<f64>,
    },
    KrakenCloseAll,
    /// Get all Kraken tradeable asset pairs.
    KrakenGetPairs,
    /// Get all Kraken Futures tradeable instruments.
    KrakenFuturesGetInstruments,
    /// Generic command to tombstone a broker/symbol/timeframe that cannot be resolved for bars.
    MarkUnresolvable {
        broker: String,
        symbol: String,
        timeframe: String,
        reason: String,
    },
}

/// Messages sent from async broker task → UI.
/// Rich L1 quote data from Alpaca market data WS.
#[derive(Debug, Clone)]
pub struct AlpacaQuoteData {
    pub symbol: String,
    pub bid: f64,
    pub ask: f64,
    pub bid_size: f64,
    pub ask_size: f64,
    /// For trade prints, this is the trade price (bid==ask==last).
    pub last: Option<f64>,
}

pub enum BrokerMsg {
    Connected(String),
    Error(String),
    Account(AccountInfo),
    /// Per-broker account roster after (re)connect or a primary switch. The
    /// UI uses it for the top-bar primary cycler and the TradeCopy window.
    AccountRoster {
        broker: OrderBroker,
        accounts: Vec<AccountRosterEntry>,
    },
    Positions(Vec<PositionInfo>),
    AlpacaAccountPositions(Vec<AccountPositions>),
    Orders(Vec<OrderInfo>),
    OrderResult(String),
    /// Real-time Alpaca market-data L1 (rich quote with sizes).
    /// Trade print sets bid==ask==last (sizes may be 0 or trade size).
    AlpacaQuote(AlpacaQuoteData),
    /// Chosen Alpaca market-data feed for the current WS connection.
    /// "sip" (entitled full) or "iex" (free real-time). Used by UI for
    /// feed-aware subscription caps and diagnostics.
    AlpacaMarketDataFeed(String),
    KrakenTrades(Vec<crate::broker::kraken::KrakenTrade>),
    KrakenLiveTrade(crate::broker::kraken::KrakenTrade),
    KrakenOpenOrders(Vec<crate::broker::kraken::KrakenOrder>),
    KrakenWsStatus {
        status: String,
        message: String,
    },
    KrakenOrderbookUpdate(String),
    KrakenBookQuoteTick {
        symbol: String,
        bid: f64,
        ask: f64,
        /// Rich L2 top sizes (from Kraken WS v2 book).
        bid_size: f64,
        ask_size: f64,
    },
    /// Rich L1 from Kraken WS v2 ticker (bid/ask + sizes + last + 24h stats).
    KrakenWsTicker(crate::broker::kraken::KrakenWsTicker),
    /// Bars just committed to cache by the Kraken WS OHLC pipeline.
    /// Each entry is `(typhoon_symbol, tf_label, last_bar_ts_ms)` so the
    /// REST scheduler can mark the (symbol, tf) WS-fresh and skip refetch.
    KrakenWsBarsCommitted {
        fresh: Vec<(String, String, i64)>,
    },
    /// Lifecycle event from one of the WS OHLC streamers (connect /
    /// subscribe / disconnect). Surfaced as a log line.
    KrakenWsOhlcStatus {
        interval_min: u32,
        kind: String,
        detail: String,
    },
    KrakenWsOhlcSnapshotSweepSettled {
        interval_min: u32,
        pair_count: usize,
        error: Option<String>,
    },
    KrakenEquityQuote(crate::broker::kraken::KrakenEquityTicker),
    KrakenEquityBars {
        symbol: String,
        timeframe: String,
        count: usize,
    },
    KrakenEquityHistoryError {
        symbol: String,
        timeframe: String,
        error: String,
    },
    KrakenEquityUniverse(Vec<crate::broker::kraken::KrakenEquityMarket>),
    SecScrapeResult(String),
    FilingContent(String), // fetched SEC filing document text
    FinnhubNewsResult(Vec<(String, String, String)>),
    /// Latest quote data.
    Quote(String, f64, f64, f64), // symbol, bid, ask, last
    /// Market clock status.
    MarketClock(String),
    /// Generic JSON results for various API calls.
    JsonResult(String, String), // (label, formatted text)
    /// Fundamentals scrape progress update.
    FundamentalsProgress(String),
    /// Bars fetched from a broker and stored in cache. The UI only reloads the
    /// active chart when the source/symbol/timeframe actually match.
    BarsFetched {
        source: String,
        symbol: String,
        timeframe: String,
        count: usize,
    },
    /// Alpaca fetch did not complete (429 mid-pagination, empty 429, or transient
    /// network error). The retry worker enqueues this (symbol, timeframe) for
    /// exponential-backoff follow-up — survives app restarts via KV persistence.
    AlpacaRetryEnqueue {
        symbol: String,
        timeframe: String,
        reason: String,
    },
    /// Alpaca definitively returned no bars for this symbol/timeframe.
    /// Persisting this tombstone lets automated sync skip it indefinitely.
    AlpacaNoData {
        symbol: String,
        timeframe: String,
        reason: String,
    },
    /// Alpaca finished a bounded full-history fetch that did not reach the
    /// requested target depth, so repeat Backfill scheduling should stop for
    /// this pair until the cache is missing or stale again.
    AlpacaBackfillComplete {
        symbol: String,
        timeframe: String,
        bar_count: usize,
        target_bars: usize,
    },
    /// Alpaca fetch finished (successfully or not) so UI-side in-flight dedupe
    /// can be released even when no new bars were written.
    AlpacaFetchSettled {
        symbol: String,
        timeframe: String,
        success: bool,
    },
    /// Kraken fetch finished (successfully or not) so UI-side in-flight dedupe
    /// can be released even when no new bars were written.
    KrakenFetchSettled {
        symbol: String,
        timeframe: String,
    },
    Unresolvable {
        broker: String,
        symbol: String,
        timeframe: String,
        reason: String,
    },
    KrakenBackfillComplete {
        symbol: String,
        timeframe: String,
        bar_count: usize,
        target_bars: usize,
    },
    /// Kraken Futures fetch finished (successfully or not) so UI-side in-flight
    /// dedupe can be released even when no new bars were written.
    KrakenFuturesFetchSettled {
        symbol: String,
        timeframe: String,
    },
    KrakenFuturesBackfillComplete {
        symbol: String,
        timeframe: String,
        bar_count: usize,
        target_bars: usize,
    },
    /// Alpaca reported a concrete historical-data RPM via rate-limit headers.
    /// UI uses this to scale fetch concurrency and queue depth to the real plan.
    AlpacaRateLimitObserved {
        historical_rpm: u32,
    },
    /// Symbol search results for autocomplete.
    SymbolSuggestions(Vec<(String, String, String)>), // (symbol, name, asset_class)
    /// Batch watchlist quote data.
    WatchlistQuotes(Vec<WatchlistRow>),
    /// FRED economic data results.
    FredData(Vec<crate::core::fred::FredSeries>, Vec<(String, f64)>),
    /// Economic calendar events (date, country, event, impact, actual).
    EconCalendarData(Vec<(String, String, String, String, String)>),
    /// Congressional stock trades (date, rep, ticker, type, amount, party).
    CongressData(Vec<(String, String, String, String, String, String)>),
    /// Unusual volume scan results (symbol, today_vol, avg_vol, ratio).
    UnusualVolumeResults(Vec<(String, f64, f64, f64)>),
    /// Kraken positions converted to unified format for chart overlay.
    KrakenPositions(Vec<PositionInfo>),
    /// Full asset list from broker (symbol, name, asset_class).
    AllAssets(Vec<(String, String, String)>),
    /// Structured fills for chart overlay (symbol, side, qty, price, time).
    RecentFills(Vec<(String, String, f64, f64, String)>),
    /// Bar sync completed with N keys updated — trigger chart reloads.
    /// Emitted by bulk Alpaca fetches after new bars land.
    BarsSynced(usize),
    /// Crypto Top 50 from CoinGecko (name, price, 24h%, market_cap).
    CryptoTop50(Vec<(String, f64, f64, f64)>),
    /// Kraken account balances (asset, balance).
    KrakenBalances(Vec<(String, f64)>),
    /// Kraken tradeable pairs (pair_name, display_name).
    KrakenPairs(Vec<(String, String)>),
    /// Kraken Futures tradeable instrument symbols.
    KrakenFuturesInstruments(Vec<String>),
    // Company events, sentiment, transcripts, commodities, and tape research results
    /// Finnhub company profile (symbol + profile).
    CompanyProfile(crate::core::research::CompanyProfile),
    /// Finnhub peer list for a symbol.
    StockPeers(String, Vec<String>),
    /// Finnhub earnings history rows (symbol, rows).
    EarningsHistory(String, Vec<crate::core::research::EarningRow>),
    /// Finnhub IPO calendar rows.
    IpoCalendar(Vec<crate::core::research::IpoEvent>),
    /// Finnhub press releases for a symbol.
    PressReleases(String, Vec<crate::core::research::PressRelease>),
    /// Finnhub social sentiment rows for a symbol.
    SocialSentiment(String, Vec<crate::core::research::SocialSentimentRow>),
    /// StockTwits public-stream sentiment snapshot for a symbol.
    StockTwitsSentiment(String, crate::core::research::StockTwitsSentimentSnapshot),
    /// Keyless Reddit mention snapshot for a symbol (ADR-117 Reddit lane).
    RedditMentions(String, crate::core::research::RedditMentionSnapshot),
    /// FMP transcript metadata list.
    TranscriptList(String, Vec<crate::core::research::TranscriptMeta>),
    /// FMP full transcript body.
    TranscriptBody(crate::core::research::Transcript),
    /// Commodities quote batch.
    CommoditiesQuotes(Vec<crate::core::research::CommodityQuote>),
    // ── ──
    /// Dividend payment history for a symbol.
    DividendHistory(String, Vec<crate::core::research::DividendRecord>),
    /// Forward earnings estimates for a symbol.
    EarningsEstimates(String, Vec<crate::core::research::EarningsEstimate>),
    /// Analyst rating change feed for a symbol.
    RatingChanges(String, Vec<crate::core::research::RatingChange>),
    /// Treasury yield curve snapshot.
    TreasuryYields(Vec<crate::core::research::TreasuryYield>),
    // ── ──
    /// Full FA bundle for a symbol (income + balance + cash flow × annual/quarterly).
    FinancialStatementsMsg(String, crate::core::research::FinancialStatements),
    /// Company officers + compensation feed for a symbol.
    Executives(String, Vec<crate::core::research::Executive>),
    /// CFTC COT weekly snapshot.
    CotReports(Vec<crate::core::research::CotReport>),
    // ── ──
    /// Stock split history for a symbol.
    StockSplitsMsg(String, Vec<crate::core::research::StockSplit>),
    /// ETF holdings (constituents) for an ETF ticker.
    EtfHoldingsMsg(String, Vec<crate::core::research::EtfHolding>),
    /// Analyst recommendation buckets (monthly trend) for a symbol.
    AnalystRecsMsg(String, Vec<crate::core::research::AnalystRecommendation>),
    /// Consensus price target snapshot for a symbol.
    PriceTargetMsg(String, crate::core::research::PriceTarget),
    /// ESG score history for a symbol.
    EsgScoresMsg(String, Vec<crate::core::research::EsgScore>),
    /// Index members (constituents) for an index code.
    IndexMembersMsg(String, Vec<crate::core::research::IndexMember>),
    // ── ──
    /// Insider trade filings (Form 4) for a symbol.
    InsiderTradesMsg(String, Vec<crate::core::research::InsiderTrade>),
    /// Institutional holders (13F-derived) for a symbol.
    InstitutionalHoldersMsg(String, Vec<crate::core::research::InstitutionalHolder>),
    /// Shares float + outstanding snapshot for a symbol.
    SharesFloatMsg(String, crate::core::research::SharesFloat),
    /// Historical price table (daily OHLCV rows) for a symbol.
    HistoricalPriceMsg(String, Vec<crate::core::research::HistoricalPriceRow>),
    /// Earnings surprise rows (quarterly EPS actual vs estimate) for a symbol.
    EarningsSurpriseMsg(String, Vec<crate::core::research::EarningsSurprise>),
    // ── ──
    /// World equity index quotes (global WEI dashboard).
    WorldIndicesMsg(Vec<crate::core::research::WorldIndex>),
    /// Market movers bundle (gainers + losers + actives).
    MarketMoversMsg(crate::core::research::MarketMovers),
    /// Sector performance snapshot (GICS sector ETF % change).
    SectorPerformanceMsg(Vec<crate::core::research::SectorPerformance>),
    /// WACC (weighted-average cost of capital) snapshot for a symbol.
    WaccSnapshotMsg(String, crate::core::research::WaccSnapshot),
    // ── ──
    /// World currency rates bundle (FX majors + crosses + EM).
    CurrencyRatesMsg(Vec<crate::core::research::CurrencyRate>),
    /// Rolling beta snapshot (1Y/3Y/5Y vs SPY) for a symbol.
    BetaSnapshotMsg(String, crate::core::research::BetaSnapshot),
    /// Gordon Growth dividend-discount-model snapshot for a symbol.
    DdmSnapshotMsg(String, crate::core::research::DdmSnapshot),
    /// Relative-valuation peer matrix for a symbol.
    RelativeValuationMsg(String, crate::core::research::RelativeValuation),
    /// OpenFIGI identifier mapping for a symbol.
    FigiSnapshotMsg(String, crate::core::research::FigiSnapshot),
    // ── ──
    /// Historical return / risk analysis snapshot for a symbol.
    HraSnapshotMsg(String, crate::core::research::HraSnapshot),
    /// Discounted cash flow (FCFF) fair-value snapshot for a symbol.
    DcfSnapshotMsg(String, crate::core::research::DcfSnapshot),
    /// Stock valuation model synthesis (DDM + DCF + peer multiples).
    SvmSnapshotMsg(String, crate::core::research::SvmSnapshot),
    /// Yahoo options chain snapshot for a symbol.
    OptionsChainMsg(String, crate::core::research::OptionsChainSnapshot),
    /// Implied-vol rank / percentile snapshot for a symbol.
    IvolSnapshotMsg(String, crate::core::research::IvolSnapshot),
    // ── ──
    /// SEAG — monthly/dow seasonality snapshot for a symbol.
    SeasonalitySnapshotMsg(String, crate::core::research::SeasonalitySnapshot),
    /// COR — pairwise correlation matrix snapshot for a symbol.
    CorrelationMatrixMsg(String, crate::core::research::CorrelationMatrix),
    /// TRA — total-return windows (price + dividend yield) for a symbol.
    TotalReturnSnapshotMsg(String, crate::core::research::TotalReturnSnapshot),
    /// TECH — technical indicators snapshot for a symbol.
    TechnicalsSnapshotMsg(String, crate::core::research::TechnicalSnapshot),
    /// SKEW — implied-volatility smile/skew snapshot for a symbol.
    VolSkewSnapshotMsg(String, crate::core::research::VolatilitySkew),
    // ── ──
    /// LEV — debt leverage / coverage ratios snapshot for a symbol.
    LeverageSnapshotMsg(String, crate::core::research::LeverageSnapshot),
    /// ACRL — earnings quality (NI vs FCF) snapshot for a symbol.
    AccrualsSnapshotMsg(String, crate::core::research::AccrualsSnapshot),
    /// RVOL — realized volatility cone snapshot for a symbol.
    RealizedVolSnapshotMsg(String, crate::core::research::RealizedVolSnapshot),
    /// FCFY — FCF yield / payout / dividend sustainability snapshot for a symbol.
    FcfYieldSnapshotMsg(String, crate::core::research::FcfYieldSnapshot),
    /// SHRT — short interest / days-to-cover snapshot for a symbol.
    ShortInterestSnapshotMsg(String, crate::core::research::ShortInterestSnapshot),
    // ── ──
    /// ALTZ — Altman Z-score snapshot for a symbol.
    AltmanZSnapshotMsg(String, crate::core::research::AltmanZSnapshot),
    /// PTFS — Piotroski F-score snapshot for a symbol.
    PiotroskiSnapshotMsg(String, crate::core::research::PiotroskiSnapshot),
    /// VOLE — OHLC volatility estimators snapshot for a symbol.
    OhlcVolSnapshotMsg(String, crate::core::research::OhlcVolSnapshot),
    /// EPSB — EPS beat streak snapshot for a symbol.
    EpsBeatSnapshotMsg(String, crate::core::research::EpsBeatSnapshot),
    /// PTD — Price target dispersion snapshot for a symbol.
    PriceTargetDispersionSnapshotMsg(String, crate::core::research::PriceTargetDispersion),
    // ── ──
    /// MNGR — Insider activity bias snapshot for a symbol.
    InsiderActivitySnapshotMsg(String, crate::core::research::InsiderActivitySnapshot),
    /// DIVG — Dividend growth analysis snapshot for a symbol.
    DivgSnapshotMsg(String, crate::core::research::DivgSnapshot),
    /// EARM — Earnings momentum trend snapshot for a symbol.
    EarmSnapshotMsg(String, crate::core::research::EarmSnapshot),
    /// SECTR — Sector rotation strength snapshot for a symbol.
    SectorRotationSnapshotMsg(String, crate::core::research::SectorRotationSnapshot),
    /// UPDM — Upgrade/downgrade momentum snapshot for a symbol.
    UpdmSnapshotMsg(String, crate::core::research::UpdmSnapshot),
    // ── ──
    /// MOM — 12-1 month momentum snapshot for a symbol.
    MomentumSnapshotMsg(String, crate::core::research::MomentumSnapshot),
    /// LIQ — Liquidity profile snapshot for a symbol.
    LiquiditySnapshotMsg(String, crate::core::research::LiquiditySnapshot),
    /// BREAK — Breakout proximity snapshot for a symbol.
    BreakoutSnapshotMsg(String, crate::core::research::BreakoutSnapshot),
    /// CCRL — Cash conversion cycle snapshot for a symbol.
    CashCycleSnapshotMsg(String, crate::core::research::CashCycleSnapshot),
    /// CREDIT — Unified credit score snapshot for a symbol.
    CreditSnapshotMsg(String, crate::core::research::CreditSnapshot),
    // ── ──
    /// GROWM — GARP composite snapshot for a symbol.
    GrowmSnapshotMsg(String, crate::core::research::GrowmSnapshot),
    /// FLOW — Insider + institutional flow snapshot for a symbol.
    FlowSnapshotMsg(String, crate::core::research::FlowSnapshot),
    /// REGIME — Market regime classifier snapshot for a symbol.
    RegimeSnapshotMsg(String, crate::core::research::RegimeSnapshot),
    /// RELVOL — Relative volume snapshot for a symbol.
    RelvolSnapshotMsg(String, crate::core::research::RelVolSnapshot),
    /// MARGINS — Margin trajectory snapshot for a symbol.
    MarginsSnapshotMsg(String, crate::core::research::MarginsSnapshot),
    // ── ──
    /// VAL — Value-factor composite snapshot for a symbol.
    ValSnapshotMsg(String, crate::core::research::ValueSnapshot),
    /// QUAL — Quality-factor composite snapshot for a symbol.
    QualSnapshotMsg(String, crate::core::research::QualitySnapshot),
    /// RISK — Risk-factor composite snapshot for a symbol.
    RiskSnapshotMsg(String, crate::core::research::RiskSnapshot),
    /// INSSTRK — Insider streak detector snapshot for a symbol.
    InsstrkSnapshotMsg(String, crate::core::research::InsiderStreakSnapshot),
    /// COVG — Analyst coverage breadth + churn snapshot for a symbol.
    CovgSnapshotMsg(String, crate::core::research::CoverageSnapshot),
    // ── ──
    /// VRK — Value Rank vs sector peers snapshot for a symbol.
    VrkSnapshotMsg(String, crate::core::research::ValueRankSnapshot),
    /// QRK — Quality Rank vs sector peers snapshot for a symbol.
    QrkSnapshotMsg(String, crate::core::research::QualityRankSnapshot),
    /// RRK — Risk Rank vs sector peers snapshot for a symbol.
    RrkSnapshotMsg(String, crate::core::research::RiskRankSnapshot),
    /// RELEPSGR — Relative 3y EPS CAGR vs sector median snapshot for a symbol.
    RelepsgrSnapshotMsg(String, crate::core::research::RelativeEpsGrowthSnapshot),
    /// PEAD — Post-earnings-announcement drift snapshot for a symbol.
    PeadSnapshotMsg(String, crate::core::research::PeadSnapshot),
    // ── ──
    /// SIZEF — Size factor rank vs sector snapshot for a symbol.
    SizefSnapshotMsg(String, crate::core::research::SizeFactorSnapshot),
    /// MOMF — Momentum factor rank snapshot for a symbol.
    MomfSnapshotMsg(String, crate::core::research::MomentumRankSnapshot),
    /// PEADRANK — PEAD drift rank vs sector peers snapshot for a symbol.
    PeadrankSnapshotMsg(String, crate::core::research::PeadRankSnapshot),
    /// FQM — Fundamental quality meter snapshot for a symbol.
    FqmSnapshotMsg(
        String,
        crate::core::research::FundamentalQualityMeterSnapshot,
    ),
    /// REVRANK — Relative 3y revenue CAGR snapshot for a symbol.
    RevrankSnapshotMsg(String, crate::core::research::RevenueGrowthRankSnapshot),
    // ── ──
    /// LEVRANK — Leverage rank vs sector peers snapshot for a symbol.
    LevrankSnapshotMsg(String, crate::core::research::LeverageRankSnapshot),
    /// OPERANK — Operating quality rank vs sector peers snapshot for a symbol.
    OperankSnapshotMsg(String, crate::core::research::OperatingQualityRankSnapshot),
    /// FQMRANK — FQM rank vs sector peers snapshot for a symbol.
    FqmrankSnapshotMsg(String, crate::core::research::FqmRankSnapshot),
    /// LIQRANK — Liquidity rank vs sector peers snapshot for a symbol.
    LiqrankSnapshotMsg(String, crate::core::research::LiquidityRankSnapshot),
    /// SURPSTK — Earnings surprise streak stat for a symbol.
    SurpstkSnapshotMsg(
        String,
        crate::core::research::EarningsSurpriseStreakSnapshot,
    ),
    // ── ──
    /// DVDRANK — Dividend growth rank vs sector peers snapshot for a symbol.
    DvdrankSnapshotMsg(String, crate::core::research::DividendGrowthRankSnapshot),
    /// EARMRANK — Earnings momentum rank vs sector peers snapshot for a symbol.
    EarmrankSnapshotMsg(String, crate::core::research::EarningsMomentumRankSnapshot),
    /// UPDGRANK — Upgrade/downgrade rank vs sector peers snapshot for a symbol.
    UpdgrankSnapshotMsg(String, crate::core::research::UpgradeDowngradeRankSnapshot),
    /// GY — Gap yearly stat for a symbol.
    GySnapshotMsg(String, crate::core::research::GapYearlySnapshot),
    /// DES — Daily event streak stat for a symbol.
    DesSnapshotMsg(String, crate::core::research::DailyEventStreakSnapshot),
    // ── ──
    /// DVDYIELDRANK — Dividend yield rank vs sector peers snapshot for a symbol.
    DvdyieldrankSnapshotMsg(String, crate::core::research::DividendYieldRankSnapshot),
    /// SHRANK — Short interest rank vs sector peers snapshot for a symbol.
    ShrankSnapshotMsg(String, crate::core::research::ShortInterestRankSnapshot),
    /// SHORTRANK_DELTA — short-interest trend rank snapshot for a symbol.
    ShortrankDeltaSnapshotMsg(
        String,
        crate::core::research::ShortInterestDeltaRankSnapshot,
    ),
    /// INSIDERCONC — insider ownership concentration snapshot for a symbol.
    InsiderconcSnapshotMsg(String, crate::core::research::InsiderConcentrationSnapshot),
    /// ATRANN — Annualized ATR volatility regime snapshot for a symbol.
    AtrannSnapshotMsg(String, crate::core::research::AnnualizedAtrSnapshot),
    /// DDHIST — Drawdown history snapshot for a symbol.
    DdhistSnapshotMsg(String, crate::core::research::DrawdownHistorySnapshot),
    /// PRICEPERF — Multi-horizon price performance snapshot for a symbol.
    PriceperfSnapshotMsg(String, crate::core::research::PricePerformanceSnapshot),
    /// MOMRANK_MULTI — sector-relative PRICEPERF rank snapshot for a symbol.
    MomrankMultiSnapshotMsg(String, crate::core::research::MomentumRankMultiSnapshot),
    // ── ──
    /// BETARANK — Beta rank vs sector peers snapshot for a symbol.
    BetarankSnapshotMsg(String, crate::core::research::BetaRankSnapshot),
    /// PEGRANK — PEG ratio rank vs sector peers snapshot for a symbol.
    PegrankSnapshotMsg(String, crate::core::research::PegRankSnapshot),
    /// FHIGHLOW — 52-week high/low distance snapshot for a symbol.
    FhighlowSnapshotMsg(String, crate::core::research::FiftyTwoWeekHighLowSnapshot),
    /// RVCONE — Multi-horizon realized vol cone snapshot for a symbol.
    RvconeSnapshotMsg(String, crate::core::research::RealizedVolConeSnapshot),
    /// CALPB — Calendar period breakdown snapshot for a symbol.
    CalpbSnapshotMsg(
        String,
        crate::core::research::CalendarPeriodBreakdownSnapshot,
    ),
    /// CORRSTK — rolling benchmark correlation snapshot for a symbol.
    CorrstkSnapshotMsg(String, crate::core::research::CorrStkSnapshot),
    /// TLRANK — trailing 30d liquidity rank snapshot for a symbol.
    TlrankSnapshotMsg(
        String,
        crate::core::research::ThirtyDayLiquidityRankSnapshot,
    ),
    /// CORRRANK — benchmark linkage rank snapshot for a symbol.
    CorrrankSnapshotMsg(String, crate::core::research::CorrelationRankSnapshot),
    /// OPERANK_DELTA — operating-margin trend rank snapshot for a symbol.
    OperankDeltaSnapshotMsg(
        String,
        crate::core::research::OperatingMarginDeltaRankSnapshot,
    ),
    /// DIVACC — dividend growth acceleration snapshot for a symbol.
    DivaccSnapshotMsg(String, crate::core::research::DividendAccelerationSnapshot),
    /// EPSACC — EPS acceleration snapshot for a symbol.
    EpsaccSnapshotMsg(String, crate::core::research::EpsAccelerationSnapshot),
    /// VRP — implied-vs-realized volatility premium snapshot for a symbol.
    VrpSnapshotMsg(String, crate::core::research::VolRiskPremiumSnapshot),
    // ── ──
    /// RETSKEW — Return distribution skewness snapshot for a symbol.
    RetskewSnapshotMsg(String, crate::core::research::ReturnSkewnessSnapshot),
    /// RETKURT — Return distribution excess kurtosis snapshot for a symbol.
    RetkurtSnapshotMsg(String, crate::core::research::ReturnKurtosisSnapshot),
    /// TAILR — Tail ratio snapshot for a symbol.
    TailrSnapshotMsg(String, crate::core::research::TailRatioSnapshot),
    /// RUNLEN — Up/down day run length snapshot for a symbol.
    RunlenSnapshotMsg(String, crate::core::research::RunLengthSnapshot),
    /// DAYRANGE — Daily range analysis snapshot for a symbol.
    DayrangeSnapshotMsg(String, crate::core::research::DailyRangeSnapshot),
    // ── ──
    /// AUTOCOR — Autocorrelation snapshot for a symbol.
    AutocorSnapshotMsg(String, crate::core::research::AutocorrelationSnapshot),
    /// HURST — Hurst exponent snapshot for a symbol.
    HurstSnapshotMsg(String, crate::core::research::HurstSnapshot),
    /// HITRATE — Multi-horizon hit rate snapshot for a symbol.
    HitrateSnapshotMsg(String, crate::core::research::HitRateSnapshot),
    /// GLASYM — Gain/loss asymmetry snapshot for a symbol.
    GlasymSnapshotMsg(String, crate::core::research::GainLossAsymmetrySnapshot),
    /// VOLRATIO — Up/down volume ratio snapshot for a symbol.
    VolratioSnapshotMsg(String, crate::core::research::VolumeRatioSnapshot),
    // ── ──
    /// DRAWUP — Rally history snapshot for a symbol.
    DrawupSnapshotMsg(String, crate::core::research::DrawupHistorySnapshot),
    /// GAPSTATS — Overnight gap statistics snapshot for a symbol.
    GapstatsSnapshotMsg(String, crate::core::research::GapStatsSnapshot),
    /// VOLCLUSTER — Volatility clustering snapshot for a symbol.
    VolclusterSnapshotMsg(String, crate::core::research::VolClusterSnapshot),
    /// CLOSEPLC — Close placement snapshot for a symbol.
    CloseplcSnapshotMsg(String, crate::core::research::ClosePlacementSnapshot),
    /// MRHL — Mean-reversion half-life snapshot for a symbol.
    MrhlSnapshotMsg(String, crate::core::research::MeanReversionHalfLifeSnapshot),
    // ── ──
    /// DOWNVOL — Downside deviation + Sortino snapshot for a symbol.
    DownvolSnapshotMsg(String, crate::core::research::DownsideVolSnapshot),
    /// SHARPR — Sharpe ratio snapshot for a symbol.
    SharprSnapshotMsg(String, crate::core::research::SharpeRatioSnapshot),
    /// EFFRATIO — Kaufman efficiency ratio snapshot for a symbol.
    EffratioSnapshotMsg(String, crate::core::research::EfficiencyRatioSnapshot),
    /// WICKBIAS — Upper vs lower wick asymmetry snapshot for a symbol.
    WickbiasSnapshotMsg(String, crate::core::research::WickBiasSnapshot),
    /// VOLOFVOL — Vol of rolling 20d realized vol snapshot for a symbol.
    VolofvolSnapshotMsg(String, crate::core::research::VolOfVolSnapshot),
    // ── Research section ──
    CalmarSnapshotMsg(String, crate::core::research::CalmarRatioSnapshot),
    UlcerSnapshotMsg(String, crate::core::research::UlcerIndexSnapshot),
    VarratioSnapshotMsg(String, crate::core::research::VarianceRatioSnapshot),
    AmihudSnapshotMsg(String, crate::core::research::AmihudIlliqSnapshot),
    JbnormSnapshotMsg(String, crate::core::research::JarqueBeraSnapshot),
    // ── Research section ──
    OmegaSnapshotMsg(String, crate::core::research::OmegaRatioSnapshot),
    DfaSnapshotMsg(String, crate::core::research::DetrendedFluctuationSnapshot),
    BurkeSnapshotMsg(String, crate::core::research::BurkeRatioSnapshot),
    MonthseasSnapshotMsg(String, crate::core::research::MonthlySeasonalitySnapshot),
    RollsprdSnapshotMsg(String, crate::core::research::RollSpreadSnapshot),
    // ── Research section ──
    ParkinsonSnapshotMsg(String, crate::core::research::ParkinsonVolSnapshot),
    GkvolSnapshotMsg(String, crate::core::research::GarmanKlassVolSnapshot),
    RsvolSnapshotMsg(String, crate::core::research::RogersSatchellVolSnapshot),
    CvarSnapshotMsg(String, crate::core::research::CVaRSnapshot),
    DoweffectSnapshotMsg(String, crate::core::research::DayOfWeekEffectSnapshot),
    // ── Research section ──
    SterlingSnapshotMsg(String, crate::core::research::SterlingRatioSnapshot),
    KellyfSnapshotMsg(String, crate::core::research::KellyFractionSnapshot),
    LjungbSnapshotMsg(String, crate::core::research::LjungBoxSnapshot),
    RunstestSnapshotMsg(String, crate::core::research::RunsTestSnapshot),
    ZeroretSnapshotMsg(String, crate::core::research::ZeroReturnSnapshot),
    // ── Research section ──
    PsrSnapshotMsg(String, crate::core::research::ProbabilisticSharpeSnapshot),
    AdfSnapshotMsg(String, crate::core::research::DickeyFullerSnapshot),
    MnkendallSnapshotMsg(String, crate::core::research::MannKendallSnapshot),
    BipowerSnapshotMsg(String, crate::core::research::BipowerVariationSnapshot),
    DddurSnapshotMsg(String, crate::core::research::DrawdownDurationSnapshot),
    // ── Research section ──
    HilltailSnapshotMsg(String, crate::core::research::HillTailSnapshot),
    ArchlmSnapshotMsg(String, crate::core::research::ArchLmSnapshot),
    PainratioSnapshotMsg(String, crate::core::research::PainRatioSnapshot),
    CusumSnapshotMsg(String, crate::core::research::CusumBreakSnapshot),
    CfvarSnapshotMsg(String, crate::core::research::CornishFisherSnapshot),
    // ── Research section ──
    EntropySnapshotMsg(String, crate::core::research::EntropySnapshot),
    RachevSnapshotMsg(String, crate::core::research::RachevSnapshot),
    GprSnapshotMsg(String, crate::core::research::GprSnapshot),
    PacfSnapshotMsg(String, crate::core::research::PacfSnapshot),
    ApenSnapshotMsg(String, crate::core::research::ApenSnapshot),
    // ── Research section ──
    UprSnapshotMsg(String, crate::core::research::UprSnapshot),
    LevereffSnapshotMsg(String, crate::core::research::LeverEffSnapshot),
    DrawdarSnapshotMsg(String, crate::core::research::DrawDaRSnapshot),
    VarhalfSnapshotMsg(String, crate::core::research::VarHalfSnapshot),
    GiniSnapshotMsg(String, crate::core::research::GiniSnapshot),
    // ── Research section ──
    SampenSnapshotMsg(String, crate::core::research::SampenSnapshot),
    PermenSnapshotMsg(String, crate::core::research::PermenSnapshot),
    RecfactSnapshotMsg(String, crate::core::research::RecfactSnapshot),
    KpssSnapshotMsg(String, crate::core::research::KpssSnapshot),
    SpecentSnapshotMsg(String, crate::core::research::SpecentSnapshot),
    // ── Research section ──
    RobvolSnapshotMsg(String, crate::core::research::RobVolSnapshot),
    RenyientSnapshotMsg(String, crate::core::research::RenyientSnapshot),
    RetquantSnapshotMsg(String, crate::core::research::RetquantSnapshot),
    MsentSnapshotMsg(String, crate::core::research::MsentSnapshot),
    EwmavolSnapshotMsg(String, crate::core::research::EwmaVolSnapshot),
    // ── Research section ──
    KsnormSnapshotMsg(String, crate::core::research::KsnormSnapshot),
    AdtestSnapshotMsg(String, crate::core::research::AdtestSnapshot),
    LmomSnapshotMsg(String, crate::core::research::LmomSnapshot),
    KylelamSnapshotMsg(String, crate::core::research::KylelamSnapshot),
    PeakoverSnapshotMsg(String, crate::core::research::PeakoverSnapshot),
    // ── Research section ──
    HiguchiSnapshotMsg(String, crate::core::research::HiguchiSnapshot),
    PickandsSnapshotMsg(String, crate::core::research::PickandsSnapshot),
    Kappa3SnapshotMsg(String, crate::core::research::Kappa3Snapshot),
    LyapunovSnapshotMsg(String, crate::core::research::LyapunovSnapshot),
    RankacSnapshotMsg(String, crate::core::research::RankacSnapshot),
    // ── Research section ──
    BnsjumpSnapshotMsg(String, crate::core::research::BnsjumpSnapshot),
    PprootSnapshotMsg(String, crate::core::research::PprootSnapshot),
    MfdfaSnapshotMsg(String, crate::core::research::MfdfaSnapshot),
    HillksSnapshotMsg(String, crate::core::research::HillksSnapshot),
    TsiSnapshotMsg(String, crate::core::research::TsiSnapshot),
    // ── Research section ──
    Garch11SnapshotMsg(String, crate::core::research::Garch11Snapshot),
    SadfSnapshotMsg(String, crate::core::research::SadfSnapshot),
    CordimSnapshotMsg(String, crate::core::research::CordimSnapshot),
    SkspecSnapshotMsg(String, crate::core::research::SkspecSnapshot),
    AutomiSnapshotMsg(String, crate::core::research::AutomiSnapshot),
    // ── Research section ──
    DurbinWatsonSnapshotMsg(String, crate::core::research::DurbinWatsonSnapshot),
    BdsTestSnapshotMsg(String, crate::core::research::BdsTestSnapshot),
    BreuschPaganSnapshotMsg(String, crate::core::research::BreuschPaganSnapshot),
    TurnPtsSnapshotMsg(String, crate::core::research::TurnPtsSnapshot),
    PeriodogramSnapshotMsg(String, crate::core::research::PeriodogramSnapshot),
    // ── Research section ──
    McLeodLiSnapshotMsg(String, crate::core::research::McLeodLiSnapshot),
    OuFitSnapshotMsg(String, crate::core::research::OuFitSnapshot),
    GphSnapshotMsg(String, crate::core::research::GphSnapshot),
    BurgSpecSnapshotMsg(String, crate::core::research::BurgSpecSnapshot),
    KendallTauSnapshotMsg(String, crate::core::research::KendallTauSnapshot),
    // ── Research section ──
    SqueezeSnapshotMsg(String, crate::core::research::SqueezeSnapshot),
    SqueezeRankSnapshotMsg(String, crate::core::research::SqueezeRankSnapshot),
    SqueezeWatchlistLoaded(Vec<crate::core::research::SqueezeSnapshot>),
    BbsqueezeSnapshotMsg(String, crate::core::research::BbsqueezeSnapshot),
    DonchianSnapshotMsg(String, crate::core::research::DonchianSnapshot),
    KamaSnapshotMsg(String, crate::core::research::KamaSnapshot),
    // ── Research section ──
    IchimokuSnapshotMsg(String, crate::core::research::IchimokuSnapshot),
    SupertrendSnapshotMsg(String, crate::core::research::SupertrendSnapshot),
    KeltnerSnapshotMsg(String, crate::core::research::KeltnerSnapshot),
    FisherSnapshotMsg(String, crate::core::research::FisherSnapshot),
    AroonSnapshotMsg(String, crate::core::research::AroonSnapshot),
    // ── Research section ──
    AdxSnapshotMsg(String, crate::core::research::AdxSnapshot),
    CciSnapshotMsg(String, crate::core::research::CciSnapshot),
    CmfSnapshotMsg(String, crate::core::research::CmfSnapshot),
    MfiSnapshotMsg(String, crate::core::research::MfiSnapshot),
    PsarSnapshotMsg(String, crate::core::research::PsarSnapshot),
    // ── Research section ──
    VortexSnapshotMsg(String, crate::core::research::VortexSnapshot),
    ChopSnapshotMsg(String, crate::core::research::ChopSnapshot),
    ObvSnapshotMsg(String, crate::core::research::ObvSnapshot),
    TrixSnapshotMsg(String, crate::core::research::TrixSnapshot),
    HmaSnapshotMsg(String, crate::core::research::HmaSnapshot),
    // ── Research section ──
    PpoSnapshotMsg(String, crate::core::research::PpoSnapshot),
    DpoSnapshotMsg(String, crate::core::research::DpoSnapshot),
    KstSnapshotMsg(String, crate::core::research::KstSnapshot),
    UltoscSnapshotMsg(String, crate::core::research::UltoscSnapshot),
    WillrSnapshotMsg(String, crate::core::research::WillrSnapshot),
    // ── Research section ──
    MassSnapshotMsg(String, crate::core::research::MassSnapshot),
    ChaikoscSnapshotMsg(String, crate::core::research::ChaikoscSnapshot),
    KlingerSnapshotMsg(String, crate::core::research::KlingerSnapshot),
    StochRsiSnapshotMsg(String, crate::core::research::StochRsiSnapshot),
    AwesomeSnapshotMsg(String, crate::core::research::AwesomeSnapshot),
    // ── Research section ──
    EfiSnapshotMsg(String, crate::core::research::EfiSnapshot),
    EmvSnapshotMsg(String, crate::core::research::EmvSnapshot),
    NviSnapshotMsg(String, crate::core::research::NviSnapshot),
    PviSnapshotMsg(String, crate::core::research::PviSnapshot),
    CoppockSnapshotMsg(String, crate::core::research::CoppockSnapshot),
    // ── Research section ──
    CmoSnapshotMsg(String, crate::core::research::CmoSnapshot),
    QstickSnapshotMsg(String, crate::core::research::QstickSnapshot),
    DisparitySnapshotMsg(String, crate::core::research::DisparitySnapshot),
    BopSnapshotMsg(String, crate::core::research::BopSnapshot),
    SchaffSnapshotMsg(String, crate::core::research::SchaffSnapshot),
    // ── Research moving-average palette aliases ──
    StochSnapshotMsg(String, crate::core::research::StochSnapshot),
    MacdSnapshotMsg(String, crate::core::research::MacdSnapshot),
    VwapSnapshotMsg(String, crate::core::research::VwapSnapshot),
    McgdSnapshotMsg(String, crate::core::research::McgdSnapshot),
    RwiSnapshotMsg(String, crate::core::research::RwiSnapshot),
    // ── Research section ──
    DemaSnapshotMsg(String, crate::core::research::DemaSnapshot),
    TemaSnapshotMsg(String, crate::core::research::TemaSnapshot),
    LinregSnapshotMsg(String, crate::core::research::LinregSnapshot),
    PivotsSnapshotMsg(String, crate::core::research::PivotsSnapshot),
    HeikinSnapshotMsg(String, crate::core::research::HeikinSnapshot),
    // ── Research section ──
    AlmaSnapshotMsg(String, crate::core::research::AlmaSnapshot),
    ZlemaSnapshotMsg(String, crate::core::research::ZlemaSnapshot),
    ElderRaySnapshotMsg(String, crate::core::research::ElderRaySnapshot),
    TsfSnapshotMsg(String, crate::core::research::TsfSnapshot),
    RviSnapshotMsg(String, crate::core::research::RviSnapshot),
    // ── Research section ──
    TrimaSnapshotMsg(String, crate::core::research::TrimaSnapshot),
    T3SnapshotMsg(String, crate::core::research::T3Snapshot),
    VidyaSnapshotMsg(String, crate::core::research::VidyaSnapshot),
    SmiSnapshotMsg(String, crate::core::research::SmiSnapshot),
    PvtSnapshotMsg(String, crate::core::research::PvtSnapshot),
    // ── Research section ──
    AcSnapshotMsg(String, crate::core::research::AcSnapshot),
    ChvolSnapshotMsg(String, crate::core::research::ChvolSnapshot),
    BbwidthSnapshotMsg(String, crate::core::research::BbwidthSnapshot),
    ElderImpSnapshotMsg(String, crate::core::research::ElderImpulseSnapshot),
    RmiSnapshotMsg(String, crate::core::research::RmiSnapshot),
    // ── ──
    SymbolExpirationsMsg(String, crate::core::research::SymbolExpirationsSnapshot),
    // ── Research section ──
    SmmaSnapshotMsg(String, crate::core::research::SmmaSnapshot),
    AlligatorSnapshotMsg(String, crate::core::research::AlligatorSnapshot),
    CrsiSnapshotMsg(String, crate::core::research::CrsiSnapshot),
    SebSnapshotMsg(String, crate::core::research::SebSnapshot),
    ImiSnapshotMsg(String, crate::core::research::ImiSnapshot),
    // ── Research section ──
    GmmaSnapshotMsg(String, crate::core::research::GmmaSnapshot),
    MaenvSnapshotMsg(String, crate::core::research::MaenvSnapshot),
    AdlSnapshotMsg(String, crate::core::research::AdlSnapshot),
    VhfSnapshotMsg(String, crate::core::research::VhfSnapshot),
    VrocSnapshotMsg(String, crate::core::research::VrocSnapshot),
    // ── Research section ──
    KdjSnapshotMsg(String, crate::core::research::KdjSnapshot),
    QqeSnapshotMsg(String, crate::core::research::QqeSnapshot),
    PmoSnapshotMsg(String, crate::core::research::PmoSnapshot),
    CfoSnapshotMsg(String, crate::core::research::CfoSnapshot),
    TmfSnapshotMsg(String, crate::core::research::TmfSnapshot),
    // ── Research section ──
    FractalsSnapshotMsg(String, crate::core::research::FractalsSnapshot),
    IftRsiSnapshotMsg(String, crate::core::research::IftRsiSnapshot),
    MamaSnapshotMsg(String, crate::core::research::MamaSnapshot),
    CogSnapshotMsg(String, crate::core::research::CogSnapshot),
    DidiSnapshotMsg(String, crate::core::research::DidiSnapshot),
    // ── Research section ──
    DemarkerSnapshotMsg(String, crate::core::research::DemarkerSnapshot),
    GatorSnapshotMsg(String, crate::core::research::GatorSnapshot),
    BwMfiSnapshotMsg(String, crate::core::research::BwMfiSnapshot),
    VwmaSnapshotMsg(String, crate::core::research::VwmaSnapshot),
    StddevSnapshotMsg(String, crate::core::research::StddevSnapshot),
    // ── Research section ──
    WmaSnapshotMsg(String, crate::core::research::WmaSnapshot),
    RainbowSnapshotMsg(String, crate::core::research::RainbowSnapshot),
    MesaSineSnapshotMsg(String, crate::core::research::MesaSineSnapshot),
    FramaSnapshotMsg(String, crate::core::research::FramaSnapshot),
    IbsSnapshotMsg(String, crate::core::research::IbsSnapshot),
    // ── Research section ──
    LaguerreRsiSnapshotMsg(String, crate::core::research::LaguerreRsiSnapshot),
    ZigzagSnapshotMsg(String, crate::core::research::ZigzagSnapshot),
    PgoSnapshotMsg(String, crate::core::research::PgoSnapshot),
    HtTrendlineSnapshotMsg(String, crate::core::research::HtTrendlineSnapshot),
    MidpointSnapshotMsg(String, crate::core::research::MidpointSnapshot),
    // ── Research section ──
    MassIndexSnapshotMsg(String, crate::core::research::MassIndexSnapshot),
    NatrSnapshotMsg(String, crate::core::research::NatrSnapshot),
    TtmSqueezeSnapshotMsg(String, crate::core::research::TtmSqueezeSnapshot),
    ForceIndexSnapshotMsg(String, crate::core::research::ForceIndexSnapshot),
    TrangeSnapshotMsg(String, crate::core::research::TrangeSnapshot),
    // ── Research section ──
    LinearregSlopeSnapshotMsg(String, crate::core::research::LinearregSlopeSnapshot),
    HtDcperiodSnapshotMsg(String, crate::core::research::HtDcperiodSnapshot),
    HtTrendmodeSnapshotMsg(String, crate::core::research::HtTrendmodeSnapshot),
    AccbandsSnapshotMsg(String, crate::core::research::AccbandsSnapshot),
    StochfSnapshotMsg(String, crate::core::research::StochfSnapshot),
    // ── Research section ──
    LinearregSnapshotMsg(String, crate::core::research::LinearregSnapshot),
    LinearregAngleSnapshotMsg(String, crate::core::research::LinearregAngleSnapshot),
    HtDcphaseSnapshotMsg(String, crate::core::research::HtDcphaseSnapshot),
    HtSineSnapshotMsg(String, crate::core::research::HtSineSnapshot),
    HtPhasorSnapshotMsg(String, crate::core::research::HtPhasorSnapshot),
    // ── Research section ──
    MidpriceSnapshotMsg(String, crate::core::research::MidpriceSnapshot),
    ApoSnapshotMsg(String, crate::core::research::ApoSnapshot),
    MomSnapshotMsg(String, crate::core::research::MomSnapshot),
    SarextSnapshotMsg(String, crate::core::research::SarextSnapshot),
    AdxrSnapshotMsg(String, crate::core::research::AdxrSnapshot),
    // ── Research section ──
    AvgpriceSnapshotMsg(String, crate::core::research::AvgpriceSnapshot),
    MedpriceSnapshotMsg(String, crate::core::research::MedpriceSnapshot),
    TypPriceSnapshotMsg(String, crate::core::research::TypPriceSnapshot),
    WclPriceSnapshotMsg(String, crate::core::research::WclPriceSnapshot),
    VarianceSnapshotMsg(String, crate::core::research::VarianceSnapshot),
    // ── Research section ──
    PlusDiSnapshotMsg(String, crate::core::research::PlusDiSnapshot),
    MinusDiSnapshotMsg(String, crate::core::research::MinusDiSnapshot),
    PlusDmSnapshotMsg(String, crate::core::research::PlusDmSnapshot),
    MinusDmSnapshotMsg(String, crate::core::research::MinusDmSnapshot),
    DxSnapshotMsg(String, crate::core::research::DxSnapshot),
    // ── Research section ──
    RocSnapshotMsg(String, crate::core::research::RocSnapshot),
    RocpSnapshotMsg(String, crate::core::research::RocpSnapshot),
    RocrSnapshotMsg(String, crate::core::research::RocrSnapshot),
    Rocr100SnapshotMsg(String, crate::core::research::Rocr100Snapshot),
    CorrelSnapshotMsg(String, crate::core::research::CorrelSnapshot),
    // ── Research section ──
    MinSnapshotMsg(String, crate::core::research::MinSnapshot),
    MaxSnapshotMsg(String, crate::core::research::MaxSnapshot),
    MinMaxSnapshotMsg(String, crate::core::research::MinMaxSnapshot),
    MinIndexSnapshotMsg(String, crate::core::research::MinIndexSnapshot),
    MaxIndexSnapshotMsg(String, crate::core::research::MaxIndexSnapshot),
    // ── Research section ──
    BbandsSnapshotMsg(String, crate::core::research::BbandsSnapshot),
    AdSnapshotMsg(String, crate::core::research::AdSnapshot),
    AdoscSnapshotMsg(String, crate::core::research::AdoscSnapshot),
    SumSnapshotMsg(String, crate::core::research::SumSnapshot),
    LinearRegInterceptSnapshotMsg(String, crate::core::research::LinearRegInterceptSnapshot),
    // ── Research section ──
    AroonoscSnapshotMsg(String, crate::core::research::AroonoscSnapshot),
    MinMaxIndexSnapshotMsg(String, crate::core::research::MinMaxIndexSnapshot),
    MacdextSnapshotMsg(String, crate::core::research::MacdextSnapshot),
    MacdfixSnapshotMsg(String, crate::core::research::MacdfixSnapshot),
    MavpSnapshotMsg(String, crate::core::research::MavpSnapshot),
    // ── Research section ──
    CdlDojiSnapshotMsg(String, crate::core::research::CdlDojiSnapshot),
    CdlHammerSnapshotMsg(String, crate::core::research::CdlHammerSnapshot),
    CdlShootingStarSnapshotMsg(String, crate::core::research::CdlShootingStarSnapshot),
    CdlEngulfingSnapshotMsg(String, crate::core::research::CdlEngulfingSnapshot),
    CdlHaramiSnapshotMsg(String, crate::core::research::CdlHaramiSnapshot),
    // ── Research section ──
    CdlMorningStarSnapshotMsg(String, crate::core::research::CdlMorningStarSnapshot),
    CdlEveningStarSnapshotMsg(String, crate::core::research::CdlEveningStarSnapshot),
    CdlThreeBlackCrowsSnapshotMsg(String, crate::core::research::CdlThreeBlackCrowsSnapshot),
    CdlThreeWhiteSoldiersSnapshotMsg(String, crate::core::research::CdlThreeWhiteSoldiersSnapshot),
    CdlDarkCloudCoverSnapshotMsg(String, crate::core::research::CdlDarkCloudCoverSnapshot),
    // ── Research section ──
    CdlPiercingSnapshotMsg(String, crate::core::research::CdlPiercingSnapshot),
    CdlDragonflyDojiSnapshotMsg(String, crate::core::research::CdlDragonflyDojiSnapshot),
    CdlGravestoneDojiSnapshotMsg(String, crate::core::research::CdlGravestoneDojiSnapshot),
    CdlHangingManSnapshotMsg(String, crate::core::research::CdlHangingManSnapshot),
    CdlInvertedHammerSnapshotMsg(String, crate::core::research::CdlInvertedHammerSnapshot),
    // ── Research section ──
    CdlHaramiCrossSnapshotMsg(String, crate::core::research::CdlHaramiCrossSnapshot),
    CdlLongLeggedDojiSnapshotMsg(String, crate::core::research::CdlLongLeggedDojiSnapshot),
    CdlMarubozuSnapshotMsg(String, crate::core::research::CdlMarubozuSnapshot),
    CdlSpinningTopSnapshotMsg(String, crate::core::research::CdlSpinningTopSnapshot),
    CdlTristarSnapshotMsg(String, crate::core::research::CdlTristarSnapshot),
    // ── Research section ──
    CdlDojiStarSnapshotMsg(String, crate::core::research::CdlDojiStarSnapshot),
    CdlMorningDojiStarSnapshotMsg(String, crate::core::research::CdlMorningDojiStarSnapshot),
    CdlEveningDojiStarSnapshotMsg(String, crate::core::research::CdlEveningDojiStarSnapshot),
    CdlAbandonedBabySnapshotMsg(String, crate::core::research::CdlAbandonedBabySnapshot),
    CdlThreeInsideSnapshotMsg(String, crate::core::research::CdlThreeInsideSnapshot),
    // ── Research section ──
    CdlBeltHoldSnapshotMsg(String, crate::core::research::CdlBeltHoldSnapshot),
    CdlClosingMarubozuSnapshotMsg(String, crate::core::research::CdlClosingMarubozuSnapshot),
    CdlHighWaveSnapshotMsg(String, crate::core::research::CdlHighWaveSnapshot),
    CdlLongLineSnapshotMsg(String, crate::core::research::CdlLongLineSnapshot),
    CdlShortLineSnapshotMsg(String, crate::core::research::CdlShortLineSnapshot),
    // ── Research section ──
    CdlCounterattackSnapshotMsg(String, crate::core::research::CdlCounterattackSnapshot),
    CdlHomingPigeonSnapshotMsg(String, crate::core::research::CdlHomingPigeonSnapshot),
    CdlInNeckSnapshotMsg(String, crate::core::research::CdlInNeckSnapshot),
    CdlOnNeckSnapshotMsg(String, crate::core::research::CdlOnNeckSnapshot),
    CdlThrustingSnapshotMsg(String, crate::core::research::CdlThrustingSnapshot),
    // ── Research section ──
    CdlTwoCrowsSnapshotMsg(String, crate::core::research::CdlTwoCrowsSnapshot),
    CdlThreeLineStrikeSnapshotMsg(String, crate::core::research::CdlThreeLineStrikeSnapshot),
    CdlThreeOutsideSnapshotMsg(String, crate::core::research::CdlThreeOutsideSnapshot),
    CdlMatchingLowSnapshotMsg(String, crate::core::research::CdlMatchingLowSnapshot),
    CdlSeparatingLinesSnapshotMsg(String, crate::core::research::CdlSeparatingLinesSnapshot),
    CdlStickSandwichSnapshotMsg(String, crate::core::research::CdlStickSandwichSnapshot),
    CdlRickshawManSnapshotMsg(String, crate::core::research::CdlRickshawManSnapshot),
    CdlTakuriSnapshotMsg(String, crate::core::research::CdlTakuriSnapshot),
    // ── Research section ──
    CdlThreeStarsInSouthSnapshotMsg(String, crate::core::research::CdlThreeStarsInSouthSnapshot),
    CdlIdenticalThreeCrowsSnapshotMsg(
        String,
        crate::core::research::CdlIdenticalThreeCrowsSnapshot,
    ),
    CdlKickingSnapshotMsg(String, crate::core::research::CdlKickingSnapshot),
    CdlKickingByLengthSnapshotMsg(String, crate::core::research::CdlKickingByLengthSnapshot),
    CdlLadderBottomSnapshotMsg(String, crate::core::research::CdlLadderBottomSnapshot),
    CdlUniqueThreeRiverSnapshotMsg(String, crate::core::research::CdlUniqueThreeRiverSnapshot),
    // ── Research section ──
    CdlAdvanceBlockSnapshotMsg(String, crate::core::research::CdlAdvanceBlockSnapshot),
    CdlBreakawaySnapshotMsg(String, crate::core::research::CdlBreakawaySnapshot),
    CdlGapSideSideWhiteSnapshotMsg(String, crate::core::research::CdlGapSideSideWhiteSnapshot),
    CdlUpsideGapTwoCrowsSnapshotMsg(String, crate::core::research::CdlUpsideGapTwoCrowsSnapshot),
    CdlXSideGapThreeMethodsSnapshotMsg(
        String,
        crate::core::research::CdlXSideGapThreeMethodsSnapshot,
    ),
    CdlConcealBabySwallowSnapshotMsg(String, crate::core::research::CdlConcealBabySwallowSnapshot),
    // ── Research section ──
    CdlHikkakeSnapshotMsg(String, crate::core::research::CdlHikkakeSnapshot),
    CdlHikkakeModSnapshotMsg(String, crate::core::research::CdlHikkakeModSnapshot),
    CdlMatHoldSnapshotMsg(String, crate::core::research::CdlMatHoldSnapshot),
    CdlRiseFallThreeMethodsSnapshotMsg(
        String,
        crate::core::research::CdlRiseFallThreeMethodsSnapshot,
    ),
    // ── Research section ──
    CdlStalledPatternSnapshotMsg(String, crate::core::research::CdlStalledPatternSnapshot),
    CdlTasukiGapSnapshotMsg(String, crate::core::research::CdlTasukiGapSnapshot),
    // ── (Quant Stats) ──
    ModSharpeSnapshotMsg(String, crate::core::research::ModSharpeSnapshot),
    HsiehTestSnapshotMsg(String, crate::core::research::HsiehTestSnapshot),
    ChowBreakSnapshotMsg(String, crate::core::research::ChowBreakSnapshot),
    DriftBurstSnapshotMsg(String, crate::core::research::DriftBurstSnapshot),
    HlvClustSnapshotMsg(String, crate::core::research::HlvClustSnapshot),
    // ── (Quant Stats) ──
    YangZhangSnapshotMsg(String, crate::core::research::YangZhangVolSnapshot),
    KuiperSnapshotMsg(String, crate::core::research::KuiperSnapshot),
    DagostinoSnapshotMsg(String, crate::core::research::DagostinoSnapshot),
    BaiPerronSnapshotMsg(String, crate::core::research::BaiPerronSnapshot),
    KupiecPofSnapshotMsg(String, crate::core::research::KupiecPofSnapshot),
    // ── ──
    /// Result of an INGEST_RESEARCH operation: per-symbol counts of
    /// newly-added articles plus any parser/write errors encountered.
    IngestResearchResult {
        per_symbol_added: Vec<(String, usize, usize)>, // (symbol, added, total)
        errors: Vec<String>,
    },
    /// Multi-source news articles loaded (from cache + fresh fetch) for a symbol.
    NewsArticlesLoaded {
        symbol: String,
        articles: Vec<crate::core::news::NewsArticle>,
    },
    /// Total article rows in the news DB, computed broker-side and pushed to
    /// the UI header ("· N in DB"). Replaces the old render-thread
    /// `count_all_articles` poll, which grabbed the write mutex behind bulk
    /// bar-sync writers and caused the 10–17s News-window frame stalls.
    NewsDbTotal(i64),
}

pub fn should_emit_fundamentals_scrape_progress(processed: usize, total: usize) -> bool {
    processed <= 10 || processed == total || processed.is_multiple_of(100)
}

pub fn format_news_scope_scrape_start(tickers: &[String]) -> String {
    pub const MAX_INLINE_SYMBOLS: usize = 24;
    let shown: Vec<&str> = tickers
        .iter()
        .take(MAX_INLINE_SYMBOLS)
        .map(String::as_str)
        .collect();
    let suffix = tickers
        .len()
        .checked_sub(MAX_INLINE_SYMBOLS)
        .filter(|remaining| *remaining > 0)
        .map(|remaining| format!(" … +{remaining} more"))
        .unwrap_or_default();
    format!(
        "News scrape: starting for {} symbol(s): {}{}",
        tickers.len(),
        shown.join(", "),
        suffix
    )
}

pub fn is_fundamentals_provider_coverage_gap(error: &str) -> bool {
    error.contains("404")
        || error.contains("Not Found")
        || error.contains("No Yahoo data")
        || error.contains("Yahoo returned 400")
        || error.contains("HTTP 400")
}

pub fn normalize_fundamentals_scrape_symbol(symbol: &str) -> Option<String> {
    let mut symbol = symbol.trim().to_ascii_uppercase();
    if symbol.is_empty() || symbol.starts_with("__") || symbol.contains('/') {
        return None;
    }
    if let Some(stripped) = symbol.strip_suffix(".EQ") {
        symbol = stripped.to_string();
    } else if let Some(stripped) = symbol.strip_suffix(".X") {
        symbol = stripped.to_string();
    }
    if symbol.is_empty() || crate::core::news::is_crypto_symbol(&symbol) {
        return None;
    }
    Some(symbol)
}
