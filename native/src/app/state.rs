//! Extracted from app.rs: state helpers.

use super::*;

// ─── application state ───────────────────────────────────────────────────────

/// Watchlist row data (TradingView-style).
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct KrakenEquityQuoteMeta {
    pub(crate) received_at_ms: i64,
    pub(crate) quote_time_ms: i64,
    pub(crate) delayed: bool,
    pub(crate) price: f64,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct WatchlistRow {
    /// Display symbol name (e.g. "BTCUSD", "SLV", "CC").
    pub(crate) symbol: String,
    /// Full cache key for loading.
    pub(crate) cache_key: String,
    /// Last close price.
    pub(crate) last: f64,
    /// Previous close (for change calculation).
    pub(crate) prev_close: f64,
    /// Current-day regular-session close (authoritative daily close, e.g.
    /// Alpaca `dailyBar.c` / Yahoo `regularMarketPrice`). Timeframe-independent,
    /// unlike a chart's own last-bar close, which differs between H1/H4/W1.
    /// `0.0` when unknown. Used to drive the extended-hours "Daily Close" badge.
    #[serde(default)]
    pub(crate) regular_close: f64,
    /// Absolute change.
    pub(crate) change: f64,
    /// Percentage change.
    pub(crate) change_pct: f64,
    /// Last bar volume.
    pub(crate) volume: f64,
    /// Extended hours change % (pre/post market).
    pub(crate) ext_change_pct: f64,
    /// True if the symbol is currently on the Nasdaq Reg SHO threshold list.
    #[serde(default)]
    pub(crate) reg_sho: bool,
    /// Live bid from WS (0.0 when none or stale >30s).
    #[serde(default, skip)]
    pub(crate) live_bid: f64,
    /// Live ask from WS (0.0 when none or stale >30s).
    #[serde(default, skip)]
    pub(crate) live_ask: f64,
    /// When the live quote arrived (for freshness check, same rule as charts).
    #[serde(default, skip)]
    pub(crate) live_quote_at: Option<std::time::Instant>,
}

pub(crate) fn watchlist_row_from_raw_bars(
    symbol: &str,
    cache_key: &str,
    raw: &[(i64, f64, f64, f64, f64, f64)],
) -> Option<WatchlistRow> {
    let mut valid = raw.iter().filter(|(ts, o, h, l, c, _v)| {
        *ts > 0
            && *o > 0.0
            && *h > 0.0
            && *l > 0.0
            && *c > 0.0
            && o.is_finite()
            && h.is_finite()
            && l.is_finite()
            && c.is_finite()
            && *h >= *l
    });
    let last_bar = valid.next_back()?;
    let prev_bar = valid.next_back().unwrap_or(last_bar);
    let change = last_bar.4 - prev_bar.4;
    let change_pct = if prev_bar.4 > 0.0 {
        change / prev_bar.4 * 100.0
    } else {
        0.0
    };
    Some(WatchlistRow {
        symbol: symbol.to_string(),
        cache_key: cache_key.to_string(),
        last: last_bar.4,
        prev_close: prev_bar.4,
        // Offline cache fallback has no separate regular-session close.
        regular_close: 0.0,
        change,
        change_pct,
        volume: last_bar.5,
        ext_change_pct: 0.0,
        live_bid: 0.0,
        live_ask: 0.0,
        live_quote_at: None,
        reg_sho: false,
    })
}

pub(crate) fn empty_watchlist_row(symbol: &str) -> WatchlistRow {
    WatchlistRow {
        symbol: symbol.to_string(),
        cache_key: symbol.to_string(),
        last: 0.0,
        prev_close: 0.0,
        regular_close: 0.0,
        reg_sho: false,
        change: 0.0,
        change_pct: 0.0,
        volume: 0.0,
        ext_change_pct: 0.0,
        live_bid: 0.0,
        live_ask: 0.0,
        live_quote_at: None,
    }
}

pub(crate) fn watchlist_cache_fallback_sources(symbol: &str) -> &'static [&'static str] {
    let symbol = symbol.trim().replace('/', "").to_ascii_uppercase();
    let equity_like = !symbol.contains('/')
        && !(symbol.ends_with("USD") && symbol.len() > 5)
        && !symbol.ends_with("USDT")
        && !symbol.ends_with("USDC");
    if equity_like {
        &["kraken-equities", "alpaca", "default"]
    } else {
        &["kraken", "kraken-futures", "default"]
    }
}

pub(crate) fn yahoo_market_state_allows_extended_quote(market_state: &str) -> bool {
    matches!(
        market_state.trim().to_ascii_uppercase().as_str(),
        "PRE" | "PREPRE" | "POST" | "POSTPOST"
    )
}

pub(crate) fn yahoo_extended_quote_time_is_fresh(ext_time: i64, regular_time: i64) -> bool {
    ext_time > 0 && (regular_time <= 0 || ext_time >= regular_time)
}

/// Upcoming event source filter for the Event Calendar window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum EventSource {
    All,
    Alpaca,
    Kraken,
    Positions,
}

/// Upcoming event type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EventKind {
    Earnings,
    ExDividend,
    DividendPayment,
}

impl EventKind {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::Earnings => "Earnings",
            Self::ExDividend => "Ex-Div",
            Self::DividendPayment => "Div Pay",
        }
    }
}

/// Single upcoming event row — used by the Event Calendar window.
#[derive(Debug, Clone)]
pub(crate) struct EventRow {
    pub(crate) symbol: String, // bare ticker (e.g. AAPL)
    pub(crate) company: String,
    pub(crate) date: String,    // YYYY-MM-DD
    pub(crate) days_until: i64, // days from today (negative = past)
    pub(crate) kind: EventKind,
    pub(crate) detail: String, // yield%, previous EPS, etc.
    pub(crate) in_alpaca: bool,
    pub(crate) in_kraken: bool,
}

/// Background-computed data — populated by background thread, read by render thread.
/// This eliminates SQLite queries from the render loop.
#[derive(Default, Clone)]
pub(crate) struct BgData {
    pub(crate) cache_stats: Option<(i64, i64, i64)>,
    pub(crate) sec_filings: Vec<sec_filing::SecFiling>,
    pub(crate) sec_alerts: Vec<sec_filing::FilingAlert>,
    pub(crate) detailed_stats: Vec<(String, i64, i64)>,
    /// Per-key compressed blob size in bytes. Populated alongside `detailed_stats`
    /// from the same BG query so Storage Manager can show per-row KB/MB/GB
    /// without re-scanning the DB. Separate HashMap instead of widening
    /// `detailed_stats` to a 4-tuple so the ~25 existing consumers stay untouched.
    pub(crate) cache_blob_sizes: std::collections::HashMap<String, i64>,
    /// Cached first/last bar timestamps per key + the write_ts snapshot we
    /// extracted it under (so staleness can be detected without re-hashing
    /// the whole blob). Value tuple = (first_ms, last_ms, cached_for_write_ts).
    /// Populated incrementally by the BG thread from the TTBR blob headers;
    /// rate-limited to avoid blowing the 3 s cycle budget on cold startup
    /// when thousands of entries need decompression. Storage Manager and the
    /// crypto backfill window both read directly from this map.
    pub(crate) bar_ts_cache: std::collections::HashMap<String, (i64, i64, i64)>,

    // ── SEC / Insider ──
    pub(crate) insider_trades: std::collections::HashMap<String, Vec<sec_filing::InsiderTrade>>,
    pub(crate) sec_content_stats: (usize, usize), // (total_filings, indexed_content)

    // ── Fundamentals (cached from background thread) ──
    pub(crate) all_fundamentals: Vec<fundamentals::Fundamentals>,
    pub(crate) upcoming_earnings: Vec<(String, String, String)>,
    pub(crate) upcoming_dividends: Vec<(String, String, String, Option<f64>)>,
    /// Active symbol-level regulatory warnings keyed by normalized ticker.
    /// Populated by the background thread from cached public outlier lists
    /// (currently NasdaqTrader Reg SHO threshold securities).
    pub(crate) regulatory_alerts_by_symbol:
        std::collections::HashMap<String, Vec<regulatory_alerts::RegulatoryAlert>>,
}

/// Bottom panel mode.
#[derive(PartialEq)]
pub(crate) enum BottomTab {
    Log,
}

/// FA window — which statement is currently shown.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum FinancialsView {
    Income,
    Balance,
    CashFlow,
}

/// FA window — annual vs quarterly reporting period.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum FinancialsPeriod {
    Annual,
    Quarterly,
}

/// RESEARCH_PACKET viewer tree node — one heading row in the
/// left-hand navigation of the packet viewer window. Depth maps to
/// markdown header level: 2 = `## `, 3 = `### `, 4 = `#### `.
#[derive(Clone, Debug, Default)]
pub(crate) struct PacketTreeNode {
    pub(crate) title: String,
    pub(crate) depth: u8,
    pub(crate) byte_offset: usize, // offset into packet_viewer_text where the header starts
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct ImportedResearchArtifact {
    pub(crate) symbol: String,
    pub(crate) report_date: String,
    pub(crate) filename: String,
    pub(crate) source_path: String,
    pub(crate) imported_at: String,
    pub(crate) content: String,
}

/// Right panel section tabs (matching old WebKit layout).
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum RightTab {
    Trading,
    Positions,
    Orders,
    Watchlist,
    Risk,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RightPanelSectionId {
    Trading,
    Positions,
    RecentFills,
    Orders,
    Watchlist,
    Risk,
    News,
    MtfGrid,
}

impl RightPanelSectionId {
    pub(crate) const DEFAULT_ORDER: [Self; 8] = [
        Self::Trading,
        Self::Positions,
        Self::RecentFills,
        Self::Orders,
        Self::Watchlist,
        Self::Risk,
        Self::News,
        Self::MtfGrid,
    ];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Trading => "trading",
            Self::Positions => "positions",
            Self::RecentFills => "recent_fills",
            Self::Orders => "orders",
            Self::Watchlist => "watchlist",
            Self::Risk => "risk",
            Self::News => "news",
            Self::MtfGrid => "mtf_grid",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Trading => "Trading",
            Self::Positions => "Positions",
            Self::RecentFills => "Recent Fills",
            Self::Orders => "Orders",
            Self::Watchlist => "Watchlist",
            Self::Risk => "Risk & Account",
            Self::News => "News",
            Self::MtfGrid => "MTF Grid",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "trading" => Some(Self::Trading),
            "positions" => Some(Self::Positions),
            "recent_fills" => Some(Self::RecentFills),
            "orders" => Some(Self::Orders),
            "watchlist" => Some(Self::Watchlist),
            "risk" => Some(Self::Risk),
            "news" => Some(Self::News),
            "mtf_grid" => Some(Self::MtfGrid),
            _ => None,
        }
    }
}

pub(crate) const KRAKEN_TRADE_HISTORY_CAP: usize = 20_000;

/// Risk sizing mode (old app had dropdown).
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum RiskMode {
    VaR,
    Standard,
    Fixed,
    Dynamic,
    KrakenPro,
}

impl RiskMode {
    pub(crate) fn label(self) -> &'static str {
        match self {
            RiskMode::VaR => "VaR",
            RiskMode::Standard => "Standard",
            RiskMode::Fixed => "Fixed",
            RiskMode::Dynamic => "Dynamic",
            RiskMode::KrakenPro => "KrakenPro",
        }
    }
}

/// Which broker to route orders to.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum OrderBroker {
    Alpaca,
    Kraken,
}

impl OrderBroker {
    pub(crate) fn label(self) -> &'static str {
        match self {
            OrderBroker::Alpaca => "Alpaca",
            OrderBroker::Kraken => "Kraken",
        }
    }
}

pub(crate) struct QuickTradePlan {
    pub(crate) symbol: String,
    pub(crate) last_price: f64,
    pub(crate) sl: f64,
    pub(crate) tp: f64,
    pub(crate) side_idx: usize,
    pub(crate) qty: f64,
    pub(crate) risk_dollars: f64,
    pub(crate) risk_pct: Option<f64>,
    pub(crate) reward_dollars: f64,
    pub(crate) rr: Option<f64>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct TradeAccountSnapshot {
    pub(crate) broker: &'static str,
    pub(crate) balance: f64,
    pub(crate) equity: f64,
    pub(crate) buying_power: f64,
    pub(crate) margin_used: f64,
}

/// Messages sent from UI → async broker task.
#[allow(dead_code)] // All variants are handled in broker task. Some lack dedicated UI buttons but are
// accessible via console commands or research windows.
pub(crate) enum BrokerCmd {
    Connect {
        api_key: String,
        secret: String,
        paper: bool,
        bar_requests_per_minute: u32,
        fetch_permits: usize,
    },
    ConfigureAlpacaSync {
        bar_requests_per_minute: u32,
        fetch_permits: usize,
    },
    GetAccount,
    GetPositions,
    GetOrders,
    CloseAll,
    ClosePosition {
        symbol: String,
        qty: Option<f64>,
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
    AlpacaFetchBarsBatch {
        symbols: Vec<String>,
        timeframe: String,
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
    /// Bulk research scrape — profile/peers/earnings/press/sentiment/transcripts (ADR-107).
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
    // ── Godel parity: research commands (ADR-107) ──
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
    // ── Godel Parity Round 2 ──
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
    // ── Godel Parity Round 3 ──
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
    // ── Godel Parity Round 4 ──
    /// FMP historical stock split events for a symbol.
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
    // ── Godel Parity Round 5 ──
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
    // ── Godel Parity Round 6 ──
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
    // ── Godel Parity Round 7 ──
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
    // ── Godel Parity Round 8 ──
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
    // ── Godel Parity Round 9 ──
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
    // ── Godel Parity Round 10 ──
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
    // ── Godel Parity Round 11 ──
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
    // ── Godel Parity Round 12 ──
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
    // ── Godel Parity Round 13 ──
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
    // ── Round 17 ──
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
    // ── Round 18 ──
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
    // ── Round 19 ──
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
    // ── Round 20 ──
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
    // ── Round 21 ──
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
    // ── Round 22 ──
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
    // ── Round 23 ──
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
    // ── Round 24 ──
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
    // ── Round 25 ──
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
    // ── Round 26 ──
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
    // ── Round 27 ──
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
    // ── Round 28 ──
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
    // ── Round 29 ──
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
    // ── Round 30 ──
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
    // ── Round 31 ──
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
    // ── Round 32 ──
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
    // ── Round 33 ──
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
    // ── Round 34 ──
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
    // ── Round 35 ──
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
    // ── Round 36 ──
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
    // ── Round 37 ──
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
    // ── Round 38 ──
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
    // ── Round 39 ──
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
    // ── Round 40 ──
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
    // ── Round 41 ──
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
    // ── Round 42 ──
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
    // ── Round 43 ──
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
    // ── Round 44 ──
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
    // ── Round 45 ──
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
    // ── Round 46 ──
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
    // ── Round 47 ──
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
    // ── Round 48 ──
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
    // ── Round 49 ──
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
    // ── Round 50 ──
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
    // ── Round 51 ──
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
    // ── Round 52 ──
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
    // ── Round 54 ──
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
    // ── Round 55 ──
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
    // ── Round 56 ──
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
    // ── Round 57 ──
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
    // ── Round 58 ──
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
    // ── Round 59 ──
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
    // ── Round 60 ──
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
    // ── Round 61 ──
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
    // ── Round 62 ──
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
    // ── Round 63 ──
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
    // ── Round 64 ──
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
    // ── Round 65 ──
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
    // ── Round 66 ──
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
    // ── Round 67 ──
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
    // ── Round 68 ──
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
    // ── Round 69 ──
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
    // ── Round 70 — BBANDS / AD / ADOSC / SUM / LINEARREG_INTERCEPT ──
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
    // ── Round 71 — AROONOSC / MINMAXINDEX / MACDEXT / MACDFIX / MAVP ──
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
    // ── Round 72 — CDLDOJI / CDLHAMMER / CDLSHOOTINGSTAR / CDLENGULFING / CDLHARAMI ──
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
    // ── Round 73 — CDLMORNINGSTAR / CDLEVENINGSTAR / CDL3BLACKCROWS / CDL3WHITESOLDIERS / CDLDARKCLOUDCOVER ──
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
    // ── Round 74 — CDLPIERCING / CDLDRAGONFLYDOJI / CDLGRAVESTONEDOJI / CDLHANGINGMAN / CDLINVERTEDHAMMER ──
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
    // ── Round 75 — CDLHARAMICROSS / CDLLONGLEGGEDDOJI / CDLMARUBOZU / CDLSPINNINGTOP / CDLTRISTAR ──
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
    // ── Round 76 — CDLDOJISTAR / CDLMORNINGDOJISTAR / CDLEVENINGDOJISTAR / CDLABANDONEDBABY / CDL3INSIDE ──
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
    // ── Round 77 — CDLBELTHOLD / CDLCLOSINGMARUBOZU / CDLHIGHWAVE / CDLLONGLINE / CDLSHORTLINE ──
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
    // ── Round 78 — CDLCOUNTERATTACK / CDLHOMINGPIGEON / CDLINNECK / CDLONNECK / CDLTHRUSTING ──
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
    // ── Round 79 — CDL2CROWS / CDL3LINESTRIKE / CDL3OUTSIDE / CDLMATCHINGLOW ──
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
    // ── Round 80 — CDLSEPARATINGLINES / CDLSTICKSANDWICH / CDLRICKSHAWMAN / CDLTAKURI ──
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
    // ── Round 81/82 — harder CDL* parity pack ──
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
    // ── Round 83/84 — additional multi-bar CDL* parity pack ──
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
    // ── Round 85/86 — stateful CDL* parity pack ──
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
    // ── Round 87/88 — final CDL* parity pack ──
    /// CDLSTALLEDPATTERN — three advancing green candles where the third stalls with a small body and upper shadow.
    ComputeCdlStalledPatternSnapshot {
        symbol: String,
    },
    /// CDLTASUKIGAP — gap continuation pattern with an opposite-colour retracement candle.
    ComputeCdlTasukiGapSnapshot {
        symbol: String,
    },
    // ── Round 76 (Quant Stats) ──
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
    // ── Round 77 — YANGZHANG / KUIPER / DAGOSTINO / BAIPERRON / KUPIECPOF ──
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
    /// Connect to Kraken crypto exchange.
    KrakenConnect {
        api_key: String,
        api_secret: String,
        ws_api_key: String,
        ws_api_secret: String,
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
        order: typhoon_engine::broker::kraken::KrakenOrderRequest,
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
pub(crate) enum BrokerMsg {
    Connected(String),
    Error(String),
    Account(AccountInfo),
    Positions(Vec<PositionInfo>),
    Orders(Vec<OrderInfo>),
    OrderResult(String),
    KrakenTrades(Vec<typhoon_engine::broker::kraken::KrakenTrade>),
    KrakenLiveTrade(typhoon_engine::broker::kraken::KrakenTrade),
    KrakenOpenOrders(Vec<typhoon_engine::broker::kraken::KrakenOrder>),
    KrakenWsStatus {
        status: String,
        message: String,
    },
    KrakenOrderbookUpdate(String),
    KrakenBookQuoteTick {
        symbol: String,
        bid: f64,
        ask: f64,
    },
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
    KrakenEquityQuote(typhoon_engine::broker::kraken::KrakenEquityTicker),
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
    KrakenEquityUniverse(Vec<typhoon_engine::broker::kraken::KrakenEquityMarket>),
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
    FredData(
        Vec<typhoon_engine::core::fred::FredSeries>,
        Vec<(String, f64)>,
    ),
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
    // ── Godel parity: research results (ADR-107) ──
    /// Finnhub company profile (symbol + profile).
    CompanyProfile(typhoon_engine::core::research::CompanyProfile),
    /// Finnhub peer list for a symbol.
    StockPeers(String, Vec<String>),
    /// Finnhub earnings history rows (symbol, rows).
    EarningsHistory(String, Vec<typhoon_engine::core::research::EarningRow>),
    /// Finnhub IPO calendar rows.
    IpoCalendar(Vec<typhoon_engine::core::research::IpoEvent>),
    /// Finnhub press releases for a symbol.
    PressReleases(String, Vec<typhoon_engine::core::research::PressRelease>),
    /// Finnhub social sentiment rows for a symbol.
    SocialSentiment(
        String,
        Vec<typhoon_engine::core::research::SocialSentimentRow>,
    ),
    /// FMP transcript metadata list.
    TranscriptList(String, Vec<typhoon_engine::core::research::TranscriptMeta>),
    /// FMP full transcript body.
    TranscriptBody(typhoon_engine::core::research::Transcript),
    /// Commodities quote batch.
    CommoditiesQuotes(Vec<typhoon_engine::core::research::CommodityQuote>),
    // ── ──
    /// Dividend payment history for a symbol.
    DividendHistory(String, Vec<typhoon_engine::core::research::DividendRecord>),
    /// Forward earnings estimates for a symbol.
    EarningsEstimates(
        String,
        Vec<typhoon_engine::core::research::EarningsEstimate>,
    ),
    /// Analyst rating change feed for a symbol.
    RatingChanges(String, Vec<typhoon_engine::core::research::RatingChange>),
    /// Treasury yield curve snapshot.
    TreasuryYields(Vec<typhoon_engine::core::research::TreasuryYield>),
    // ── ──
    /// Full FA bundle for a symbol (income + balance + cash flow × annual/quarterly).
    FinancialStatementsMsg(String, typhoon_engine::core::research::FinancialStatements),
    /// Company officers + compensation feed for a symbol.
    Executives(String, Vec<typhoon_engine::core::research::Executive>),
    /// CFTC COT weekly snapshot.
    CotReports(Vec<typhoon_engine::core::research::CotReport>),
    // ── ──
    /// Stock split history for a symbol.
    StockSplitsMsg(String, Vec<typhoon_engine::core::research::StockSplit>),
    /// ETF holdings (constituents) for an ETF ticker.
    EtfHoldingsMsg(String, Vec<typhoon_engine::core::research::EtfHolding>),
    /// Analyst recommendation buckets (monthly trend) for a symbol.
    AnalystRecsMsg(
        String,
        Vec<typhoon_engine::core::research::AnalystRecommendation>,
    ),
    /// Consensus price target snapshot for a symbol.
    PriceTargetMsg(String, typhoon_engine::core::research::PriceTarget),
    /// ESG score history for a symbol.
    EsgScoresMsg(String, Vec<typhoon_engine::core::research::EsgScore>),
    /// Index members (constituents) for an index code.
    IndexMembersMsg(String, Vec<typhoon_engine::core::research::IndexMember>),
    // ── ──
    /// Insider trade filings (Form 4) for a symbol.
    InsiderTradesMsg(String, Vec<typhoon_engine::core::research::InsiderTrade>),
    /// Institutional holders (13F-derived) for a symbol.
    InstitutionalHoldersMsg(
        String,
        Vec<typhoon_engine::core::research::InstitutionalHolder>,
    ),
    /// Shares float + outstanding snapshot for a symbol.
    SharesFloatMsg(String, typhoon_engine::core::research::SharesFloat),
    /// Historical price table (daily OHLCV rows) for a symbol.
    HistoricalPriceMsg(
        String,
        Vec<typhoon_engine::core::research::HistoricalPriceRow>,
    ),
    /// Earnings surprise rows (quarterly EPS actual vs estimate) for a symbol.
    EarningsSurpriseMsg(
        String,
        Vec<typhoon_engine::core::research::EarningsSurprise>,
    ),
    // ── ──
    /// World equity index quotes (global WEI dashboard).
    WorldIndicesMsg(Vec<typhoon_engine::core::research::WorldIndex>),
    /// Market movers bundle (gainers + losers + actives).
    MarketMoversMsg(typhoon_engine::core::research::MarketMovers),
    /// Sector performance snapshot (GICS sector ETF % change).
    SectorPerformanceMsg(Vec<typhoon_engine::core::research::SectorPerformance>),
    /// WACC (weighted-average cost of capital) snapshot for a symbol.
    WaccSnapshotMsg(String, typhoon_engine::core::research::WaccSnapshot),
    // ── ──
    /// World currency rates bundle (FX majors + crosses + EM).
    CurrencyRatesMsg(Vec<typhoon_engine::core::research::CurrencyRate>),
    /// Rolling beta snapshot (1Y/3Y/5Y vs SPY) for a symbol.
    BetaSnapshotMsg(String, typhoon_engine::core::research::BetaSnapshot),
    /// Gordon Growth dividend-discount-model snapshot for a symbol.
    DdmSnapshotMsg(String, typhoon_engine::core::research::DdmSnapshot),
    /// Relative-valuation peer matrix for a symbol.
    RelativeValuationMsg(String, typhoon_engine::core::research::RelativeValuation),
    /// OpenFIGI identifier mapping for a symbol.
    FigiSnapshotMsg(String, typhoon_engine::core::research::FigiSnapshot),
    // ── ──
    /// Historical return / risk analysis snapshot for a symbol.
    HraSnapshotMsg(String, typhoon_engine::core::research::HraSnapshot),
    /// Discounted cash flow (FCFF) fair-value snapshot for a symbol.
    DcfSnapshotMsg(String, typhoon_engine::core::research::DcfSnapshot),
    /// Stock valuation model synthesis (DDM + DCF + peer multiples).
    SvmSnapshotMsg(String, typhoon_engine::core::research::SvmSnapshot),
    /// Yahoo options chain snapshot for a symbol.
    OptionsChainMsg(String, typhoon_engine::core::research::OptionsChainSnapshot),
    /// Implied-vol rank / percentile snapshot for a symbol.
    IvolSnapshotMsg(String, typhoon_engine::core::research::IvolSnapshot),
    // ── ──
    /// SEAG — monthly/dow seasonality snapshot for a symbol.
    SeasonalitySnapshotMsg(String, typhoon_engine::core::research::SeasonalitySnapshot),
    /// COR — pairwise correlation matrix snapshot for a symbol.
    CorrelationMatrixMsg(String, typhoon_engine::core::research::CorrelationMatrix),
    /// TRA — total-return windows (price + dividend yield) for a symbol.
    TotalReturnSnapshotMsg(String, typhoon_engine::core::research::TotalReturnSnapshot),
    /// TECH — technical indicators snapshot for a symbol.
    TechnicalsSnapshotMsg(String, typhoon_engine::core::research::TechnicalSnapshot),
    /// SKEW — implied-volatility smile/skew snapshot for a symbol.
    VolSkewSnapshotMsg(String, typhoon_engine::core::research::VolatilitySkew),
    // ── ──
    /// LEV — debt leverage / coverage ratios snapshot for a symbol.
    LeverageSnapshotMsg(String, typhoon_engine::core::research::LeverageSnapshot),
    /// ACRL — earnings quality (NI vs FCF) snapshot for a symbol.
    AccrualsSnapshotMsg(String, typhoon_engine::core::research::AccrualsSnapshot),
    /// RVOL — realized volatility cone snapshot for a symbol.
    RealizedVolSnapshotMsg(String, typhoon_engine::core::research::RealizedVolSnapshot),
    /// FCFY — FCF yield / payout / dividend sustainability snapshot for a symbol.
    FcfYieldSnapshotMsg(String, typhoon_engine::core::research::FcfYieldSnapshot),
    /// SHRT — short interest / days-to-cover snapshot for a symbol.
    ShortInterestSnapshotMsg(
        String,
        typhoon_engine::core::research::ShortInterestSnapshot,
    ),
    // ── ──
    /// ALTZ — Altman Z-score snapshot for a symbol.
    AltmanZSnapshotMsg(String, typhoon_engine::core::research::AltmanZSnapshot),
    /// PTFS — Piotroski F-score snapshot for a symbol.
    PiotroskiSnapshotMsg(String, typhoon_engine::core::research::PiotroskiSnapshot),
    /// VOLE — OHLC volatility estimators snapshot for a symbol.
    OhlcVolSnapshotMsg(String, typhoon_engine::core::research::OhlcVolSnapshot),
    /// EPSB — EPS beat streak snapshot for a symbol.
    EpsBeatSnapshotMsg(String, typhoon_engine::core::research::EpsBeatSnapshot),
    /// PTD — Price target dispersion snapshot for a symbol.
    PriceTargetDispersionSnapshotMsg(
        String,
        typhoon_engine::core::research::PriceTargetDispersion,
    ),
    // ── ──
    /// MNGR — Insider activity bias snapshot for a symbol.
    InsiderActivitySnapshotMsg(
        String,
        typhoon_engine::core::research::InsiderActivitySnapshot,
    ),
    /// DIVG — Dividend growth analysis snapshot for a symbol.
    DivgSnapshotMsg(String, typhoon_engine::core::research::DivgSnapshot),
    /// EARM — Earnings momentum trend snapshot for a symbol.
    EarmSnapshotMsg(String, typhoon_engine::core::research::EarmSnapshot),
    /// SECTR — Sector rotation strength snapshot for a symbol.
    SectorRotationSnapshotMsg(
        String,
        typhoon_engine::core::research::SectorRotationSnapshot,
    ),
    /// UPDM — Upgrade/downgrade momentum snapshot for a symbol.
    UpdmSnapshotMsg(String, typhoon_engine::core::research::UpdmSnapshot),
    // ── ──
    /// MOM — 12-1 month momentum snapshot for a symbol.
    MomentumSnapshotMsg(String, typhoon_engine::core::research::MomentumSnapshot),
    /// LIQ — Liquidity profile snapshot for a symbol.
    LiquiditySnapshotMsg(String, typhoon_engine::core::research::LiquiditySnapshot),
    /// BREAK — Breakout proximity snapshot for a symbol.
    BreakoutSnapshotMsg(String, typhoon_engine::core::research::BreakoutSnapshot),
    /// CCRL — Cash conversion cycle snapshot for a symbol.
    CashCycleSnapshotMsg(String, typhoon_engine::core::research::CashCycleSnapshot),
    /// CREDIT — Unified credit score snapshot for a symbol.
    CreditSnapshotMsg(String, typhoon_engine::core::research::CreditSnapshot),
    // ── ──
    /// GROWM — GARP composite snapshot for a symbol.
    GrowmSnapshotMsg(String, typhoon_engine::core::research::GrowmSnapshot),
    /// FLOW — Insider + institutional flow snapshot for a symbol.
    FlowSnapshotMsg(String, typhoon_engine::core::research::FlowSnapshot),
    /// REGIME — Market regime classifier snapshot for a symbol.
    RegimeSnapshotMsg(String, typhoon_engine::core::research::RegimeSnapshot),
    /// RELVOL — Relative volume snapshot for a symbol.
    RelvolSnapshotMsg(String, typhoon_engine::core::research::RelVolSnapshot),
    /// MARGINS — Margin trajectory snapshot for a symbol.
    MarginsSnapshotMsg(String, typhoon_engine::core::research::MarginsSnapshot),
    // ── ──
    /// VAL — Value-factor composite snapshot for a symbol.
    ValSnapshotMsg(String, typhoon_engine::core::research::ValueSnapshot),
    /// QUAL — Quality-factor composite snapshot for a symbol.
    QualSnapshotMsg(String, typhoon_engine::core::research::QualitySnapshot),
    /// RISK — Risk-factor composite snapshot for a symbol.
    RiskSnapshotMsg(String, typhoon_engine::core::research::RiskSnapshot),
    /// INSSTRK — Insider streak detector snapshot for a symbol.
    InsstrkSnapshotMsg(
        String,
        typhoon_engine::core::research::InsiderStreakSnapshot,
    ),
    /// COVG — Analyst coverage breadth + churn snapshot for a symbol.
    CovgSnapshotMsg(String, typhoon_engine::core::research::CoverageSnapshot),
    // ── ──
    /// VRK — Value Rank vs sector peers snapshot for a symbol.
    VrkSnapshotMsg(String, typhoon_engine::core::research::ValueRankSnapshot),
    /// QRK — Quality Rank vs sector peers snapshot for a symbol.
    QrkSnapshotMsg(String, typhoon_engine::core::research::QualityRankSnapshot),
    /// RRK — Risk Rank vs sector peers snapshot for a symbol.
    RrkSnapshotMsg(String, typhoon_engine::core::research::RiskRankSnapshot),
    /// RELEPSGR — Relative 3y EPS CAGR vs sector median snapshot for a symbol.
    RelepsgrSnapshotMsg(
        String,
        typhoon_engine::core::research::RelativeEpsGrowthSnapshot,
    ),
    /// PEAD — Post-earnings-announcement drift snapshot for a symbol.
    PeadSnapshotMsg(String, typhoon_engine::core::research::PeadSnapshot),
    // ── ──
    /// SIZEF — Size factor rank vs sector snapshot for a symbol.
    SizefSnapshotMsg(String, typhoon_engine::core::research::SizeFactorSnapshot),
    /// MOMF — Momentum factor rank snapshot for a symbol.
    MomfSnapshotMsg(String, typhoon_engine::core::research::MomentumRankSnapshot),
    /// PEADRANK — PEAD drift rank vs sector peers snapshot for a symbol.
    PeadrankSnapshotMsg(String, typhoon_engine::core::research::PeadRankSnapshot),
    /// FQM — Fundamental quality meter snapshot for a symbol.
    FqmSnapshotMsg(
        String,
        typhoon_engine::core::research::FundamentalQualityMeterSnapshot,
    ),
    /// REVRANK — Relative 3y revenue CAGR snapshot for a symbol.
    RevrankSnapshotMsg(
        String,
        typhoon_engine::core::research::RevenueGrowthRankSnapshot,
    ),
    // ── ──
    /// LEVRANK — Leverage rank vs sector peers snapshot for a symbol.
    LevrankSnapshotMsg(String, typhoon_engine::core::research::LeverageRankSnapshot),
    /// OPERANK — Operating quality rank vs sector peers snapshot for a symbol.
    OperankSnapshotMsg(
        String,
        typhoon_engine::core::research::OperatingQualityRankSnapshot,
    ),
    /// FQMRANK — FQM rank vs sector peers snapshot for a symbol.
    FqmrankSnapshotMsg(String, typhoon_engine::core::research::FqmRankSnapshot),
    /// LIQRANK — Liquidity rank vs sector peers snapshot for a symbol.
    LiqrankSnapshotMsg(
        String,
        typhoon_engine::core::research::LiquidityRankSnapshot,
    ),
    /// SURPSTK — Earnings surprise streak stat for a symbol.
    SurpstkSnapshotMsg(
        String,
        typhoon_engine::core::research::EarningsSurpriseStreakSnapshot,
    ),
    // ── ──
    /// DVDRANK — Dividend growth rank vs sector peers snapshot for a symbol.
    DvdrankSnapshotMsg(
        String,
        typhoon_engine::core::research::DividendGrowthRankSnapshot,
    ),
    /// EARMRANK — Earnings momentum rank vs sector peers snapshot for a symbol.
    EarmrankSnapshotMsg(
        String,
        typhoon_engine::core::research::EarningsMomentumRankSnapshot,
    ),
    /// UPDGRANK — Upgrade/downgrade rank vs sector peers snapshot for a symbol.
    UpdgrankSnapshotMsg(
        String,
        typhoon_engine::core::research::UpgradeDowngradeRankSnapshot,
    ),
    /// GY — Gap yearly stat for a symbol.
    GySnapshotMsg(String, typhoon_engine::core::research::GapYearlySnapshot),
    /// DES — Daily event streak stat for a symbol.
    DesSnapshotMsg(
        String,
        typhoon_engine::core::research::DailyEventStreakSnapshot,
    ),
    // ── ──
    /// DVDYIELDRANK — Dividend yield rank vs sector peers snapshot for a symbol.
    DvdyieldrankSnapshotMsg(
        String,
        typhoon_engine::core::research::DividendYieldRankSnapshot,
    ),
    /// SHRANK — Short interest rank vs sector peers snapshot for a symbol.
    ShrankSnapshotMsg(
        String,
        typhoon_engine::core::research::ShortInterestRankSnapshot,
    ),
    /// SHORTRANK_DELTA — short-interest trend rank snapshot for a symbol.
    ShortrankDeltaSnapshotMsg(
        String,
        typhoon_engine::core::research::ShortInterestDeltaRankSnapshot,
    ),
    /// INSIDERCONC — insider ownership concentration snapshot for a symbol.
    InsiderconcSnapshotMsg(
        String,
        typhoon_engine::core::research::InsiderConcentrationSnapshot,
    ),
    /// ATRANN — Annualized ATR volatility regime snapshot for a symbol.
    AtrannSnapshotMsg(
        String,
        typhoon_engine::core::research::AnnualizedAtrSnapshot,
    ),
    /// DDHIST — Drawdown history snapshot for a symbol.
    DdhistSnapshotMsg(
        String,
        typhoon_engine::core::research::DrawdownHistorySnapshot,
    ),
    /// PRICEPERF — Multi-horizon price performance snapshot for a symbol.
    PriceperfSnapshotMsg(
        String,
        typhoon_engine::core::research::PricePerformanceSnapshot,
    ),
    /// MOMRANK_MULTI — sector-relative PRICEPERF rank snapshot for a symbol.
    MomrankMultiSnapshotMsg(
        String,
        typhoon_engine::core::research::MomentumRankMultiSnapshot,
    ),
    // ── ──
    /// BETARANK — Beta rank vs sector peers snapshot for a symbol.
    BetarankSnapshotMsg(String, typhoon_engine::core::research::BetaRankSnapshot),
    /// PEGRANK — PEG ratio rank vs sector peers snapshot for a symbol.
    PegrankSnapshotMsg(String, typhoon_engine::core::research::PegRankSnapshot),
    /// FHIGHLOW — 52-week high/low distance snapshot for a symbol.
    FhighlowSnapshotMsg(
        String,
        typhoon_engine::core::research::FiftyTwoWeekHighLowSnapshot,
    ),
    /// RVCONE — Multi-horizon realized vol cone snapshot for a symbol.
    RvconeSnapshotMsg(
        String,
        typhoon_engine::core::research::RealizedVolConeSnapshot,
    ),
    /// CALPB — Calendar period breakdown snapshot for a symbol.
    CalpbSnapshotMsg(
        String,
        typhoon_engine::core::research::CalendarPeriodBreakdownSnapshot,
    ),
    /// CORRSTK — rolling benchmark correlation snapshot for a symbol.
    CorrstkSnapshotMsg(String, typhoon_engine::core::research::CorrStkSnapshot),
    /// TLRANK — trailing 30d liquidity rank snapshot for a symbol.
    TlrankSnapshotMsg(
        String,
        typhoon_engine::core::research::ThirtyDayLiquidityRankSnapshot,
    ),
    /// CORRRANK — benchmark linkage rank snapshot for a symbol.
    CorrrankSnapshotMsg(
        String,
        typhoon_engine::core::research::CorrelationRankSnapshot,
    ),
    /// OPERANK_DELTA — operating-margin trend rank snapshot for a symbol.
    OperankDeltaSnapshotMsg(
        String,
        typhoon_engine::core::research::OperatingMarginDeltaRankSnapshot,
    ),
    /// DIVACC — dividend growth acceleration snapshot for a symbol.
    DivaccSnapshotMsg(
        String,
        typhoon_engine::core::research::DividendAccelerationSnapshot,
    ),
    /// EPSACC — EPS acceleration snapshot for a symbol.
    EpsaccSnapshotMsg(
        String,
        typhoon_engine::core::research::EpsAccelerationSnapshot,
    ),
    /// VRP — implied-vs-realized volatility premium snapshot for a symbol.
    VrpSnapshotMsg(
        String,
        typhoon_engine::core::research::VolRiskPremiumSnapshot,
    ),
    // ── ──
    /// RETSKEW — Return distribution skewness snapshot for a symbol.
    RetskewSnapshotMsg(
        String,
        typhoon_engine::core::research::ReturnSkewnessSnapshot,
    ),
    /// RETKURT — Return distribution excess kurtosis snapshot for a symbol.
    RetkurtSnapshotMsg(
        String,
        typhoon_engine::core::research::ReturnKurtosisSnapshot,
    ),
    /// TAILR — Tail ratio snapshot for a symbol.
    TailrSnapshotMsg(String, typhoon_engine::core::research::TailRatioSnapshot),
    /// RUNLEN — Up/down day run length snapshot for a symbol.
    RunlenSnapshotMsg(String, typhoon_engine::core::research::RunLengthSnapshot),
    /// DAYRANGE — Daily range analysis snapshot for a symbol.
    DayrangeSnapshotMsg(String, typhoon_engine::core::research::DailyRangeSnapshot),
    // ── ──
    /// AUTOCOR — Autocorrelation snapshot for a symbol.
    AutocorSnapshotMsg(
        String,
        typhoon_engine::core::research::AutocorrelationSnapshot,
    ),
    /// HURST — Hurst exponent snapshot for a symbol.
    HurstSnapshotMsg(String, typhoon_engine::core::research::HurstSnapshot),
    /// HITRATE — Multi-horizon hit rate snapshot for a symbol.
    HitrateSnapshotMsg(String, typhoon_engine::core::research::HitRateSnapshot),
    /// GLASYM — Gain/loss asymmetry snapshot for a symbol.
    GlasymSnapshotMsg(
        String,
        typhoon_engine::core::research::GainLossAsymmetrySnapshot,
    ),
    /// VOLRATIO — Up/down volume ratio snapshot for a symbol.
    VolratioSnapshotMsg(String, typhoon_engine::core::research::VolumeRatioSnapshot),
    // ── ──
    /// DRAWUP — Rally history snapshot for a symbol.
    DrawupSnapshotMsg(
        String,
        typhoon_engine::core::research::DrawupHistorySnapshot,
    ),
    /// GAPSTATS — Overnight gap statistics snapshot for a symbol.
    GapstatsSnapshotMsg(String, typhoon_engine::core::research::GapStatsSnapshot),
    /// VOLCLUSTER — Volatility clustering snapshot for a symbol.
    VolclusterSnapshotMsg(String, typhoon_engine::core::research::VolClusterSnapshot),
    /// CLOSEPLC — Close placement snapshot for a symbol.
    CloseplcSnapshotMsg(
        String,
        typhoon_engine::core::research::ClosePlacementSnapshot,
    ),
    /// MRHL — Mean-reversion half-life snapshot for a symbol.
    MrhlSnapshotMsg(
        String,
        typhoon_engine::core::research::MeanReversionHalfLifeSnapshot,
    ),
    // ── ──
    /// DOWNVOL — Downside deviation + Sortino snapshot for a symbol.
    DownvolSnapshotMsg(String, typhoon_engine::core::research::DownsideVolSnapshot),
    /// SHARPR — Sharpe ratio snapshot for a symbol.
    SharprSnapshotMsg(String, typhoon_engine::core::research::SharpeRatioSnapshot),
    /// EFFRATIO — Kaufman efficiency ratio snapshot for a symbol.
    EffratioSnapshotMsg(
        String,
        typhoon_engine::core::research::EfficiencyRatioSnapshot,
    ),
    /// WICKBIAS — Upper vs lower wick asymmetry snapshot for a symbol.
    WickbiasSnapshotMsg(String, typhoon_engine::core::research::WickBiasSnapshot),
    /// VOLOFVOL — Vol of rolling 20d realized vol snapshot for a symbol.
    VolofvolSnapshotMsg(String, typhoon_engine::core::research::VolOfVolSnapshot),
    // ── Round 26 ──
    CalmarSnapshotMsg(String, typhoon_engine::core::research::CalmarRatioSnapshot),
    UlcerSnapshotMsg(String, typhoon_engine::core::research::UlcerIndexSnapshot),
    VarratioSnapshotMsg(
        String,
        typhoon_engine::core::research::VarianceRatioSnapshot,
    ),
    AmihudSnapshotMsg(String, typhoon_engine::core::research::AmihudIlliqSnapshot),
    JbnormSnapshotMsg(String, typhoon_engine::core::research::JarqueBeraSnapshot),
    // ── Round 27 ──
    OmegaSnapshotMsg(String, typhoon_engine::core::research::OmegaRatioSnapshot),
    DfaSnapshotMsg(
        String,
        typhoon_engine::core::research::DetrendedFluctuationSnapshot,
    ),
    BurkeSnapshotMsg(String, typhoon_engine::core::research::BurkeRatioSnapshot),
    MonthseasSnapshotMsg(
        String,
        typhoon_engine::core::research::MonthlySeasonalitySnapshot,
    ),
    RollsprdSnapshotMsg(String, typhoon_engine::core::research::RollSpreadSnapshot),
    // ── Round 28 ──
    ParkinsonSnapshotMsg(String, typhoon_engine::core::research::ParkinsonVolSnapshot),
    GkvolSnapshotMsg(
        String,
        typhoon_engine::core::research::GarmanKlassVolSnapshot,
    ),
    RsvolSnapshotMsg(
        String,
        typhoon_engine::core::research::RogersSatchellVolSnapshot,
    ),
    CvarSnapshotMsg(String, typhoon_engine::core::research::CVaRSnapshot),
    DoweffectSnapshotMsg(
        String,
        typhoon_engine::core::research::DayOfWeekEffectSnapshot,
    ),
    // ── Round 29 ──
    SterlingSnapshotMsg(
        String,
        typhoon_engine::core::research::SterlingRatioSnapshot,
    ),
    KellyfSnapshotMsg(
        String,
        typhoon_engine::core::research::KellyFractionSnapshot,
    ),
    LjungbSnapshotMsg(String, typhoon_engine::core::research::LjungBoxSnapshot),
    RunstestSnapshotMsg(String, typhoon_engine::core::research::RunsTestSnapshot),
    ZeroretSnapshotMsg(String, typhoon_engine::core::research::ZeroReturnSnapshot),
    // ── Round 30 ──
    PsrSnapshotMsg(
        String,
        typhoon_engine::core::research::ProbabilisticSharpeSnapshot,
    ),
    AdfSnapshotMsg(String, typhoon_engine::core::research::DickeyFullerSnapshot),
    MnkendallSnapshotMsg(String, typhoon_engine::core::research::MannKendallSnapshot),
    BipowerSnapshotMsg(
        String,
        typhoon_engine::core::research::BipowerVariationSnapshot,
    ),
    DddurSnapshotMsg(
        String,
        typhoon_engine::core::research::DrawdownDurationSnapshot,
    ),
    // ── Round 31 ──
    HilltailSnapshotMsg(String, typhoon_engine::core::research::HillTailSnapshot),
    ArchlmSnapshotMsg(String, typhoon_engine::core::research::ArchLmSnapshot),
    PainratioSnapshotMsg(String, typhoon_engine::core::research::PainRatioSnapshot),
    CusumSnapshotMsg(String, typhoon_engine::core::research::CusumBreakSnapshot),
    CfvarSnapshotMsg(
        String,
        typhoon_engine::core::research::CornishFisherSnapshot,
    ),
    // ── Round 32 ──
    EntropySnapshotMsg(String, typhoon_engine::core::research::EntropySnapshot),
    RachevSnapshotMsg(String, typhoon_engine::core::research::RachevSnapshot),
    GprSnapshotMsg(String, typhoon_engine::core::research::GprSnapshot),
    PacfSnapshotMsg(String, typhoon_engine::core::research::PacfSnapshot),
    ApenSnapshotMsg(String, typhoon_engine::core::research::ApenSnapshot),
    // ── Round 33 ──
    UprSnapshotMsg(String, typhoon_engine::core::research::UprSnapshot),
    LevereffSnapshotMsg(String, typhoon_engine::core::research::LeverEffSnapshot),
    DrawdarSnapshotMsg(String, typhoon_engine::core::research::DrawDaRSnapshot),
    VarhalfSnapshotMsg(String, typhoon_engine::core::research::VarHalfSnapshot),
    GiniSnapshotMsg(String, typhoon_engine::core::research::GiniSnapshot),
    // ── Round 34 ──
    SampenSnapshotMsg(String, typhoon_engine::core::research::SampenSnapshot),
    PermenSnapshotMsg(String, typhoon_engine::core::research::PermenSnapshot),
    RecfactSnapshotMsg(String, typhoon_engine::core::research::RecfactSnapshot),
    KpssSnapshotMsg(String, typhoon_engine::core::research::KpssSnapshot),
    SpecentSnapshotMsg(String, typhoon_engine::core::research::SpecentSnapshot),
    // ── Round 35 ──
    RobvolSnapshotMsg(String, typhoon_engine::core::research::RobVolSnapshot),
    RenyientSnapshotMsg(String, typhoon_engine::core::research::RenyientSnapshot),
    RetquantSnapshotMsg(String, typhoon_engine::core::research::RetquantSnapshot),
    MsentSnapshotMsg(String, typhoon_engine::core::research::MsentSnapshot),
    EwmavolSnapshotMsg(String, typhoon_engine::core::research::EwmaVolSnapshot),
    // ── Round 36 ──
    KsnormSnapshotMsg(String, typhoon_engine::core::research::KsnormSnapshot),
    AdtestSnapshotMsg(String, typhoon_engine::core::research::AdtestSnapshot),
    LmomSnapshotMsg(String, typhoon_engine::core::research::LmomSnapshot),
    KylelamSnapshotMsg(String, typhoon_engine::core::research::KylelamSnapshot),
    PeakoverSnapshotMsg(String, typhoon_engine::core::research::PeakoverSnapshot),
    // ── Round 37 ──
    HiguchiSnapshotMsg(String, typhoon_engine::core::research::HiguchiSnapshot),
    PickandsSnapshotMsg(String, typhoon_engine::core::research::PickandsSnapshot),
    Kappa3SnapshotMsg(String, typhoon_engine::core::research::Kappa3Snapshot),
    LyapunovSnapshotMsg(String, typhoon_engine::core::research::LyapunovSnapshot),
    RankacSnapshotMsg(String, typhoon_engine::core::research::RankacSnapshot),
    // ── Round 38 ──
    BnsjumpSnapshotMsg(String, typhoon_engine::core::research::BnsjumpSnapshot),
    PprootSnapshotMsg(String, typhoon_engine::core::research::PprootSnapshot),
    MfdfaSnapshotMsg(String, typhoon_engine::core::research::MfdfaSnapshot),
    HillksSnapshotMsg(String, typhoon_engine::core::research::HillksSnapshot),
    TsiSnapshotMsg(String, typhoon_engine::core::research::TsiSnapshot),
    // ── Round 39 ──
    Garch11SnapshotMsg(String, typhoon_engine::core::research::Garch11Snapshot),
    SadfSnapshotMsg(String, typhoon_engine::core::research::SadfSnapshot),
    CordimSnapshotMsg(String, typhoon_engine::core::research::CordimSnapshot),
    SkspecSnapshotMsg(String, typhoon_engine::core::research::SkspecSnapshot),
    AutomiSnapshotMsg(String, typhoon_engine::core::research::AutomiSnapshot),
    // ── Round 40 ──
    DurbinWatsonSnapshotMsg(String, typhoon_engine::core::research::DurbinWatsonSnapshot),
    BdsTestSnapshotMsg(String, typhoon_engine::core::research::BdsTestSnapshot),
    BreuschPaganSnapshotMsg(String, typhoon_engine::core::research::BreuschPaganSnapshot),
    TurnPtsSnapshotMsg(String, typhoon_engine::core::research::TurnPtsSnapshot),
    PeriodogramSnapshotMsg(String, typhoon_engine::core::research::PeriodogramSnapshot),
    // ── Round 41 ──
    McLeodLiSnapshotMsg(String, typhoon_engine::core::research::McLeodLiSnapshot),
    OuFitSnapshotMsg(String, typhoon_engine::core::research::OuFitSnapshot),
    GphSnapshotMsg(String, typhoon_engine::core::research::GphSnapshot),
    BurgSpecSnapshotMsg(String, typhoon_engine::core::research::BurgSpecSnapshot),
    KendallTauSnapshotMsg(String, typhoon_engine::core::research::KendallTauSnapshot),
    // ── Round 42 ──
    SqueezeSnapshotMsg(String, typhoon_engine::core::research::SqueezeSnapshot),
    SqueezeRankSnapshotMsg(String, typhoon_engine::core::research::SqueezeRankSnapshot),
    SqueezeWatchlistLoaded(Vec<typhoon_engine::core::research::SqueezeSnapshot>),
    BbsqueezeSnapshotMsg(String, typhoon_engine::core::research::BbsqueezeSnapshot),
    DonchianSnapshotMsg(String, typhoon_engine::core::research::DonchianSnapshot),
    KamaSnapshotMsg(String, typhoon_engine::core::research::KamaSnapshot),
    // ── Round 43 ──
    IchimokuSnapshotMsg(String, typhoon_engine::core::research::IchimokuSnapshot),
    SupertrendSnapshotMsg(String, typhoon_engine::core::research::SupertrendSnapshot),
    KeltnerSnapshotMsg(String, typhoon_engine::core::research::KeltnerSnapshot),
    FisherSnapshotMsg(String, typhoon_engine::core::research::FisherSnapshot),
    AroonSnapshotMsg(String, typhoon_engine::core::research::AroonSnapshot),
    // ── Round 44 ──
    AdxSnapshotMsg(String, typhoon_engine::core::research::AdxSnapshot),
    CciSnapshotMsg(String, typhoon_engine::core::research::CciSnapshot),
    CmfSnapshotMsg(String, typhoon_engine::core::research::CmfSnapshot),
    MfiSnapshotMsg(String, typhoon_engine::core::research::MfiSnapshot),
    PsarSnapshotMsg(String, typhoon_engine::core::research::PsarSnapshot),
    // ── Round 45 ──
    VortexSnapshotMsg(String, typhoon_engine::core::research::VortexSnapshot),
    ChopSnapshotMsg(String, typhoon_engine::core::research::ChopSnapshot),
    ObvSnapshotMsg(String, typhoon_engine::core::research::ObvSnapshot),
    TrixSnapshotMsg(String, typhoon_engine::core::research::TrixSnapshot),
    HmaSnapshotMsg(String, typhoon_engine::core::research::HmaSnapshot),
    // ── Round 46 ──
    PpoSnapshotMsg(String, typhoon_engine::core::research::PpoSnapshot),
    DpoSnapshotMsg(String, typhoon_engine::core::research::DpoSnapshot),
    KstSnapshotMsg(String, typhoon_engine::core::research::KstSnapshot),
    UltoscSnapshotMsg(String, typhoon_engine::core::research::UltoscSnapshot),
    WillrSnapshotMsg(String, typhoon_engine::core::research::WillrSnapshot),
    // ── Round 47 ──
    MassSnapshotMsg(String, typhoon_engine::core::research::MassSnapshot),
    ChaikoscSnapshotMsg(String, typhoon_engine::core::research::ChaikoscSnapshot),
    KlingerSnapshotMsg(String, typhoon_engine::core::research::KlingerSnapshot),
    StochRsiSnapshotMsg(String, typhoon_engine::core::research::StochRsiSnapshot),
    AwesomeSnapshotMsg(String, typhoon_engine::core::research::AwesomeSnapshot),
    // ── Round 48 ──
    EfiSnapshotMsg(String, typhoon_engine::core::research::EfiSnapshot),
    EmvSnapshotMsg(String, typhoon_engine::core::research::EmvSnapshot),
    NviSnapshotMsg(String, typhoon_engine::core::research::NviSnapshot),
    PviSnapshotMsg(String, typhoon_engine::core::research::PviSnapshot),
    CoppockSnapshotMsg(String, typhoon_engine::core::research::CoppockSnapshot),
    // ── Round 49 ──
    CmoSnapshotMsg(String, typhoon_engine::core::research::CmoSnapshot),
    QstickSnapshotMsg(String, typhoon_engine::core::research::QstickSnapshot),
    DisparitySnapshotMsg(String, typhoon_engine::core::research::DisparitySnapshot),
    BopSnapshotMsg(String, typhoon_engine::core::research::BopSnapshot),
    SchaffSnapshotMsg(String, typhoon_engine::core::research::SchaffSnapshot),
    // ── Round 50 ──
    StochSnapshotMsg(String, typhoon_engine::core::research::StochSnapshot),
    MacdSnapshotMsg(String, typhoon_engine::core::research::MacdSnapshot),
    VwapSnapshotMsg(String, typhoon_engine::core::research::VwapSnapshot),
    McgdSnapshotMsg(String, typhoon_engine::core::research::McgdSnapshot),
    RwiSnapshotMsg(String, typhoon_engine::core::research::RwiSnapshot),
    // ── Round 51 ──
    DemaSnapshotMsg(String, typhoon_engine::core::research::DemaSnapshot),
    TemaSnapshotMsg(String, typhoon_engine::core::research::TemaSnapshot),
    LinregSnapshotMsg(String, typhoon_engine::core::research::LinregSnapshot),
    PivotsSnapshotMsg(String, typhoon_engine::core::research::PivotsSnapshot),
    HeikinSnapshotMsg(String, typhoon_engine::core::research::HeikinSnapshot),
    // ── Round 52 ──
    AlmaSnapshotMsg(String, typhoon_engine::core::research::AlmaSnapshot),
    ZlemaSnapshotMsg(String, typhoon_engine::core::research::ZlemaSnapshot),
    ElderRaySnapshotMsg(String, typhoon_engine::core::research::ElderRaySnapshot),
    TsfSnapshotMsg(String, typhoon_engine::core::research::TsfSnapshot),
    RviSnapshotMsg(String, typhoon_engine::core::research::RviSnapshot),
    // ── Round 53 ──
    TrimaSnapshotMsg(String, typhoon_engine::core::research::TrimaSnapshot),
    T3SnapshotMsg(String, typhoon_engine::core::research::T3Snapshot),
    VidyaSnapshotMsg(String, typhoon_engine::core::research::VidyaSnapshot),
    SmiSnapshotMsg(String, typhoon_engine::core::research::SmiSnapshot),
    PvtSnapshotMsg(String, typhoon_engine::core::research::PvtSnapshot),
    // ── Round 54 ──
    AcSnapshotMsg(String, typhoon_engine::core::research::AcSnapshot),
    ChvolSnapshotMsg(String, typhoon_engine::core::research::ChvolSnapshot),
    BbwidthSnapshotMsg(String, typhoon_engine::core::research::BbwidthSnapshot),
    ElderImpSnapshotMsg(String, typhoon_engine::core::research::ElderImpulseSnapshot),
    RmiSnapshotMsg(String, typhoon_engine::core::research::RmiSnapshot),
    // ── ──
    SymbolExpirationsMsg(
        String,
        typhoon_engine::core::research::SymbolExpirationsSnapshot,
    ),
    // ── Round 55 ──
    SmmaSnapshotMsg(String, typhoon_engine::core::research::SmmaSnapshot),
    AlligatorSnapshotMsg(String, typhoon_engine::core::research::AlligatorSnapshot),
    CrsiSnapshotMsg(String, typhoon_engine::core::research::CrsiSnapshot),
    SebSnapshotMsg(String, typhoon_engine::core::research::SebSnapshot),
    ImiSnapshotMsg(String, typhoon_engine::core::research::ImiSnapshot),
    // ── Round 56 ──
    GmmaSnapshotMsg(String, typhoon_engine::core::research::GmmaSnapshot),
    MaenvSnapshotMsg(String, typhoon_engine::core::research::MaenvSnapshot),
    AdlSnapshotMsg(String, typhoon_engine::core::research::AdlSnapshot),
    VhfSnapshotMsg(String, typhoon_engine::core::research::VhfSnapshot),
    VrocSnapshotMsg(String, typhoon_engine::core::research::VrocSnapshot),
    // ── Round 57 ──
    KdjSnapshotMsg(String, typhoon_engine::core::research::KdjSnapshot),
    QqeSnapshotMsg(String, typhoon_engine::core::research::QqeSnapshot),
    PmoSnapshotMsg(String, typhoon_engine::core::research::PmoSnapshot),
    CfoSnapshotMsg(String, typhoon_engine::core::research::CfoSnapshot),
    TmfSnapshotMsg(String, typhoon_engine::core::research::TmfSnapshot),
    // ── Round 58 ──
    FractalsSnapshotMsg(String, typhoon_engine::core::research::FractalsSnapshot),
    IftRsiSnapshotMsg(String, typhoon_engine::core::research::IftRsiSnapshot),
    MamaSnapshotMsg(String, typhoon_engine::core::research::MamaSnapshot),
    CogSnapshotMsg(String, typhoon_engine::core::research::CogSnapshot),
    DidiSnapshotMsg(String, typhoon_engine::core::research::DidiSnapshot),
    // ── Round 59 ──
    DemarkerSnapshotMsg(String, typhoon_engine::core::research::DemarkerSnapshot),
    GatorSnapshotMsg(String, typhoon_engine::core::research::GatorSnapshot),
    BwMfiSnapshotMsg(String, typhoon_engine::core::research::BwMfiSnapshot),
    VwmaSnapshotMsg(String, typhoon_engine::core::research::VwmaSnapshot),
    StddevSnapshotMsg(String, typhoon_engine::core::research::StddevSnapshot),
    // ── Round 60 ──
    WmaSnapshotMsg(String, typhoon_engine::core::research::WmaSnapshot),
    RainbowSnapshotMsg(String, typhoon_engine::core::research::RainbowSnapshot),
    MesaSineSnapshotMsg(String, typhoon_engine::core::research::MesaSineSnapshot),
    FramaSnapshotMsg(String, typhoon_engine::core::research::FramaSnapshot),
    IbsSnapshotMsg(String, typhoon_engine::core::research::IbsSnapshot),
    // ── Round 61 ──
    LaguerreRsiSnapshotMsg(String, typhoon_engine::core::research::LaguerreRsiSnapshot),
    ZigzagSnapshotMsg(String, typhoon_engine::core::research::ZigzagSnapshot),
    PgoSnapshotMsg(String, typhoon_engine::core::research::PgoSnapshot),
    HtTrendlineSnapshotMsg(String, typhoon_engine::core::research::HtTrendlineSnapshot),
    MidpointSnapshotMsg(String, typhoon_engine::core::research::MidpointSnapshot),
    // ── Round 62 ──
    MassIndexSnapshotMsg(String, typhoon_engine::core::research::MassIndexSnapshot),
    NatrSnapshotMsg(String, typhoon_engine::core::research::NatrSnapshot),
    TtmSqueezeSnapshotMsg(String, typhoon_engine::core::research::TtmSqueezeSnapshot),
    ForceIndexSnapshotMsg(String, typhoon_engine::core::research::ForceIndexSnapshot),
    TrangeSnapshotMsg(String, typhoon_engine::core::research::TrangeSnapshot),
    // ── Round 63 ──
    LinearregSlopeSnapshotMsg(
        String,
        typhoon_engine::core::research::LinearregSlopeSnapshot,
    ),
    HtDcperiodSnapshotMsg(String, typhoon_engine::core::research::HtDcperiodSnapshot),
    HtTrendmodeSnapshotMsg(String, typhoon_engine::core::research::HtTrendmodeSnapshot),
    AccbandsSnapshotMsg(String, typhoon_engine::core::research::AccbandsSnapshot),
    StochfSnapshotMsg(String, typhoon_engine::core::research::StochfSnapshot),
    // ── Round 64 ──
    LinearregSnapshotMsg(String, typhoon_engine::core::research::LinearregSnapshot),
    LinearregAngleSnapshotMsg(
        String,
        typhoon_engine::core::research::LinearregAngleSnapshot,
    ),
    HtDcphaseSnapshotMsg(String, typhoon_engine::core::research::HtDcphaseSnapshot),
    HtSineSnapshotMsg(String, typhoon_engine::core::research::HtSineSnapshot),
    HtPhasorSnapshotMsg(String, typhoon_engine::core::research::HtPhasorSnapshot),
    // ── Round 65 ──
    MidpriceSnapshotMsg(String, typhoon_engine::core::research::MidpriceSnapshot),
    ApoSnapshotMsg(String, typhoon_engine::core::research::ApoSnapshot),
    MomSnapshotMsg(String, typhoon_engine::core::research::MomSnapshot),
    SarextSnapshotMsg(String, typhoon_engine::core::research::SarextSnapshot),
    AdxrSnapshotMsg(String, typhoon_engine::core::research::AdxrSnapshot),
    // ── Round 66 ──
    AvgpriceSnapshotMsg(String, typhoon_engine::core::research::AvgpriceSnapshot),
    MedpriceSnapshotMsg(String, typhoon_engine::core::research::MedpriceSnapshot),
    TypPriceSnapshotMsg(String, typhoon_engine::core::research::TypPriceSnapshot),
    WclPriceSnapshotMsg(String, typhoon_engine::core::research::WclPriceSnapshot),
    VarianceSnapshotMsg(String, typhoon_engine::core::research::VarianceSnapshot),
    // ── Round 67 ──
    PlusDiSnapshotMsg(String, typhoon_engine::core::research::PlusDiSnapshot),
    MinusDiSnapshotMsg(String, typhoon_engine::core::research::MinusDiSnapshot),
    PlusDmSnapshotMsg(String, typhoon_engine::core::research::PlusDmSnapshot),
    MinusDmSnapshotMsg(String, typhoon_engine::core::research::MinusDmSnapshot),
    DxSnapshotMsg(String, typhoon_engine::core::research::DxSnapshot),
    // ── Round 68 ──
    RocSnapshotMsg(String, typhoon_engine::core::research::RocSnapshot),
    RocpSnapshotMsg(String, typhoon_engine::core::research::RocpSnapshot),
    RocrSnapshotMsg(String, typhoon_engine::core::research::RocrSnapshot),
    Rocr100SnapshotMsg(String, typhoon_engine::core::research::Rocr100Snapshot),
    CorrelSnapshotMsg(String, typhoon_engine::core::research::CorrelSnapshot),
    // ── Round 69 ──
    MinSnapshotMsg(String, typhoon_engine::core::research::MinSnapshot),
    MaxSnapshotMsg(String, typhoon_engine::core::research::MaxSnapshot),
    MinMaxSnapshotMsg(String, typhoon_engine::core::research::MinMaxSnapshot),
    MinIndexSnapshotMsg(String, typhoon_engine::core::research::MinIndexSnapshot),
    MaxIndexSnapshotMsg(String, typhoon_engine::core::research::MaxIndexSnapshot),
    // ── Round 70 ──
    BbandsSnapshotMsg(String, typhoon_engine::core::research::BbandsSnapshot),
    AdSnapshotMsg(String, typhoon_engine::core::research::AdSnapshot),
    AdoscSnapshotMsg(String, typhoon_engine::core::research::AdoscSnapshot),
    SumSnapshotMsg(String, typhoon_engine::core::research::SumSnapshot),
    LinearRegInterceptSnapshotMsg(
        String,
        typhoon_engine::core::research::LinearRegInterceptSnapshot,
    ),
    // ── Round 71 ──
    AroonoscSnapshotMsg(String, typhoon_engine::core::research::AroonoscSnapshot),
    MinMaxIndexSnapshotMsg(String, typhoon_engine::core::research::MinMaxIndexSnapshot),
    MacdextSnapshotMsg(String, typhoon_engine::core::research::MacdextSnapshot),
    MacdfixSnapshotMsg(String, typhoon_engine::core::research::MacdfixSnapshot),
    MavpSnapshotMsg(String, typhoon_engine::core::research::MavpSnapshot),
    // ── Round 72 ──
    CdlDojiSnapshotMsg(String, typhoon_engine::core::research::CdlDojiSnapshot),
    CdlHammerSnapshotMsg(String, typhoon_engine::core::research::CdlHammerSnapshot),
    CdlShootingStarSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlShootingStarSnapshot,
    ),
    CdlEngulfingSnapshotMsg(String, typhoon_engine::core::research::CdlEngulfingSnapshot),
    CdlHaramiSnapshotMsg(String, typhoon_engine::core::research::CdlHaramiSnapshot),
    // ── Round 73 ──
    CdlMorningStarSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlMorningStarSnapshot,
    ),
    CdlEveningStarSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlEveningStarSnapshot,
    ),
    CdlThreeBlackCrowsSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlThreeBlackCrowsSnapshot,
    ),
    CdlThreeWhiteSoldiersSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlThreeWhiteSoldiersSnapshot,
    ),
    CdlDarkCloudCoverSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlDarkCloudCoverSnapshot,
    ),
    // ── Round 74 ──
    CdlPiercingSnapshotMsg(String, typhoon_engine::core::research::CdlPiercingSnapshot),
    CdlDragonflyDojiSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlDragonflyDojiSnapshot,
    ),
    CdlGravestoneDojiSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlGravestoneDojiSnapshot,
    ),
    CdlHangingManSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlHangingManSnapshot,
    ),
    CdlInvertedHammerSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlInvertedHammerSnapshot,
    ),
    // ── Round 75 ──
    CdlHaramiCrossSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlHaramiCrossSnapshot,
    ),
    CdlLongLeggedDojiSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlLongLeggedDojiSnapshot,
    ),
    CdlMarubozuSnapshotMsg(String, typhoon_engine::core::research::CdlMarubozuSnapshot),
    CdlSpinningTopSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlSpinningTopSnapshot,
    ),
    CdlTristarSnapshotMsg(String, typhoon_engine::core::research::CdlTristarSnapshot),
    // ── Round 76 ──
    CdlDojiStarSnapshotMsg(String, typhoon_engine::core::research::CdlDojiStarSnapshot),
    CdlMorningDojiStarSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlMorningDojiStarSnapshot,
    ),
    CdlEveningDojiStarSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlEveningDojiStarSnapshot,
    ),
    CdlAbandonedBabySnapshotMsg(
        String,
        typhoon_engine::core::research::CdlAbandonedBabySnapshot,
    ),
    CdlThreeInsideSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlThreeInsideSnapshot,
    ),
    // ── Round 77 ──
    CdlBeltHoldSnapshotMsg(String, typhoon_engine::core::research::CdlBeltHoldSnapshot),
    CdlClosingMarubozuSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlClosingMarubozuSnapshot,
    ),
    CdlHighWaveSnapshotMsg(String, typhoon_engine::core::research::CdlHighWaveSnapshot),
    CdlLongLineSnapshotMsg(String, typhoon_engine::core::research::CdlLongLineSnapshot),
    CdlShortLineSnapshotMsg(String, typhoon_engine::core::research::CdlShortLineSnapshot),
    // ── Round 78 ──
    CdlCounterattackSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlCounterattackSnapshot,
    ),
    CdlHomingPigeonSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlHomingPigeonSnapshot,
    ),
    CdlInNeckSnapshotMsg(String, typhoon_engine::core::research::CdlInNeckSnapshot),
    CdlOnNeckSnapshotMsg(String, typhoon_engine::core::research::CdlOnNeckSnapshot),
    CdlThrustingSnapshotMsg(String, typhoon_engine::core::research::CdlThrustingSnapshot),
    // ── Round 79/80 ──
    CdlTwoCrowsSnapshotMsg(String, typhoon_engine::core::research::CdlTwoCrowsSnapshot),
    CdlThreeLineStrikeSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlThreeLineStrikeSnapshot,
    ),
    CdlThreeOutsideSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlThreeOutsideSnapshot,
    ),
    CdlMatchingLowSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlMatchingLowSnapshot,
    ),
    CdlSeparatingLinesSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlSeparatingLinesSnapshot,
    ),
    CdlStickSandwichSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlStickSandwichSnapshot,
    ),
    CdlRickshawManSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlRickshawManSnapshot,
    ),
    CdlTakuriSnapshotMsg(String, typhoon_engine::core::research::CdlTakuriSnapshot),
    // ── Round 81/82 ──
    CdlThreeStarsInSouthSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlThreeStarsInSouthSnapshot,
    ),
    CdlIdenticalThreeCrowsSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlIdenticalThreeCrowsSnapshot,
    ),
    CdlKickingSnapshotMsg(String, typhoon_engine::core::research::CdlKickingSnapshot),
    CdlKickingByLengthSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlKickingByLengthSnapshot,
    ),
    CdlLadderBottomSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlLadderBottomSnapshot,
    ),
    CdlUniqueThreeRiverSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlUniqueThreeRiverSnapshot,
    ),
    // ── Round 83/84 ──
    CdlAdvanceBlockSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlAdvanceBlockSnapshot,
    ),
    CdlBreakawaySnapshotMsg(String, typhoon_engine::core::research::CdlBreakawaySnapshot),
    CdlGapSideSideWhiteSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlGapSideSideWhiteSnapshot,
    ),
    CdlUpsideGapTwoCrowsSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlUpsideGapTwoCrowsSnapshot,
    ),
    CdlXSideGapThreeMethodsSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlXSideGapThreeMethodsSnapshot,
    ),
    CdlConcealBabySwallowSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlConcealBabySwallowSnapshot,
    ),
    // ── Round 85/86 ──
    CdlHikkakeSnapshotMsg(String, typhoon_engine::core::research::CdlHikkakeSnapshot),
    CdlHikkakeModSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlHikkakeModSnapshot,
    ),
    CdlMatHoldSnapshotMsg(String, typhoon_engine::core::research::CdlMatHoldSnapshot),
    CdlRiseFallThreeMethodsSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlRiseFallThreeMethodsSnapshot,
    ),
    // ── Round 87/88 ──
    CdlStalledPatternSnapshotMsg(
        String,
        typhoon_engine::core::research::CdlStalledPatternSnapshot,
    ),
    CdlTasukiGapSnapshotMsg(String, typhoon_engine::core::research::CdlTasukiGapSnapshot),
    // ── Round 76 (Quant Stats) ──
    ModSharpeSnapshotMsg(String, typhoon_engine::core::research::ModSharpeSnapshot),
    HsiehTestSnapshotMsg(String, typhoon_engine::core::research::HsiehTestSnapshot),
    ChowBreakSnapshotMsg(String, typhoon_engine::core::research::ChowBreakSnapshot),
    DriftBurstSnapshotMsg(String, typhoon_engine::core::research::DriftBurstSnapshot),
    HlvClustSnapshotMsg(String, typhoon_engine::core::research::HlvClustSnapshot),
    // ── Round 77 (Quant Stats) ──
    YangZhangSnapshotMsg(String, typhoon_engine::core::research::YangZhangVolSnapshot),
    KuiperSnapshotMsg(String, typhoon_engine::core::research::KuiperSnapshot),
    DagostinoSnapshotMsg(String, typhoon_engine::core::research::DagostinoSnapshot),
    BaiPerronSnapshotMsg(String, typhoon_engine::core::research::BaiPerronSnapshot),
    KupiecPofSnapshotMsg(String, typhoon_engine::core::research::KupiecPofSnapshot),
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
        articles: Vec<typhoon_engine::core::news::NewsArticle>,
    },
    /// Total article rows in the news DB, computed broker-side and pushed to
    /// the UI header ("· N in DB"). Replaces the old render-thread
    /// `count_all_articles` poll, which grabbed the write mutex behind bulk
    /// bar-sync writers and caused the 10–17s News-window frame stalls.
    NewsDbTotal(i64),
}

pub(crate) fn should_emit_fundamentals_scrape_progress(processed: usize, total: usize) -> bool {
    processed <= 10 || processed == total || processed.is_multiple_of(100)
}

pub(crate) fn format_news_scope_scrape_start(tickers: &[String]) -> String {
    pub(crate) const MAX_INLINE_SYMBOLS: usize = 24;
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

pub(crate) fn is_fundamentals_provider_coverage_gap(error: &str) -> bool {
    error.contains("404")
        || error.contains("Not Found")
        || error.contains("No Yahoo data")
        || error.contains("Yahoo returned 400")
        || error.contains("HTTP 400")
}

pub(crate) fn normalize_fundamentals_scrape_symbol(symbol: &str) -> Option<String> {
    let mut symbol = symbol.trim().to_ascii_uppercase();
    if symbol.is_empty() || symbol.starts_with("__") || symbol.contains('/') {
        return None;
    }
    if let Some(stripped) = symbol.strip_suffix(".EQ") {
        symbol = stripped.to_string();
    } else if let Some(stripped) = symbol.strip_suffix(".X") {
        symbol = stripped.to_string();
    }
    if symbol.is_empty() || typhoon_engine::core::news::is_crypto_symbol(&symbol) {
        return None;
    }
    Some(symbol)
}

/// Reusable sort state for clickable column headers.
#[derive(Clone, Default)]
pub(crate) struct SortState {
    pub(crate) column: usize,   // which column is sorted (0-indexed)
    pub(crate) ascending: bool, // true = ascending, false = descending
}

impl SortState {
    pub(crate) fn toggle(&mut self, col: usize) {
        if self.column == col {
            self.ascending = !self.ascending;
        } else {
            self.column = col;
            self.ascending = true;
        }
    }

    /// Render a clickable header label. Returns true if clicked.
    pub(crate) fn header(ui: &mut egui::Ui, label: &str, col: usize, state: &SortState) -> bool {
        let arrow = if state.column == col {
            if state.ascending {
                " \u{25B2}"
            } else {
                " \u{25BC}"
            }
        } else {
            ""
        };
        let text = format!("{}{}", label, arrow);
        let color = if state.column == col {
            egui::Color32::WHITE
        } else {
            egui::Color32::from_rgb(120, 120, 140)
        };
        ui.add(
            egui::Label::new(egui::RichText::new(text).color(color).small().strong())
                .sense(egui::Sense::click()),
        )
        .clicked()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct BookmapWindowState {
    pub(crate) symbol: String,
    pub(crate) open: bool,
}

pub struct TyphooNApp {
    /// Shared cache handle — opened once at startup.
    pub(crate) cache: Option<Arc<SqliteCache>>,
    /// Receiver for async cache open (delivered on first frame).
    pub(crate) cache_rx: Option<std::sync::mpsc::Receiver<Arc<SqliteCache>>>,
    /// Whether initial chart load has been done after cache arrived.
    pub(crate) cache_loaded: bool,
    /// Cache open error (shown in log if set).
    pub(crate) cache_err: Option<String>,

    /// Symbol input text in the toolbar.
    pub(crate) symbol_input: String,

    /// Primary chart (or charts[0] in grid mode).
    pub(crate) charts: Vec<ChartState>,
    /// MTF grid: how many columns to show.
    pub(crate) mtf_cols: usize,
    /// MTF grid enabled flag.
    pub(crate) mtf_enabled: bool,
    /// Base zstd level for foreground bar-cache writes. Auto-compact can promote rows to zstd-22 later.
    pub(crate) bar_zstd_level: i32,
    /// Auto-compact (zstd-22) scheduler — opt-out toggle. ADR-089.
    pub(crate) auto_compact_enabled: bool,
    /// User-configured cadence/window/row threshold for the auto-compact gate.
    pub(crate) auto_compact_schedule: auto_compact::Schedule,
    /// UTC ms timestamp of the last successful automated or manual compact run.
    /// Drives the cadence gate so weekly runs don't pile up.
    pub(crate) auto_compact_last_run_ms: i64,
    /// Set when a compact pass is in flight (manual or auto). Cleared by the
    /// "Compact complete:" handler in the OrderResult dispatch.
    pub(crate) auto_compact_in_progress: bool,
    /// UTC ms timestamp when the current compact run was dispatched.
    pub(crate) auto_compact_started_ms: i64,
    /// Latest skip reason (for Storage Manager readout).
    pub(crate) auto_compact_last_skip: Option<String>,
    /// Last frame-time we saw user input — used by the auto-compact idle gate.
    pub(crate) auto_compact_last_input_at: std::time::Instant,
    /// Throttle for the gate evaluation: only re-evaluate every ~minute so we
    /// don't burn CPU running the gate every frame.
    pub(crate) auto_compact_next_check_at: std::time::Instant,
    /// Canonical base TFs allowed for automated scrape/sync flows.
    /// Stored as cache suffixes (`1Min` .. `1Month`) so all broker backfill
    /// paths read the same config.
    pub(crate) enabled_sync_timeframes: std::collections::BTreeSet<String>,
    /// Optional startup hint for Alpaca historical data RPM. `0` means auto:
    /// start at Basic cadence, then upgrade when Alpaca rate-limit headers
    /// reveal a higher-tier plan.
    pub(crate) alpaca_historical_rpm_hint: u32,
    /// Runtime-only RPM observed from Alpaca `X-RateLimit-Limit` headers.
    /// Not persisted; a fresh session re-learns it unless the user pins a hint.
    pub(crate) alpaca_historical_rpm_observed: u32,
    /// Which chart cell is focused in MTF grid (click to select).
    pub(crate) mtf_focused: Option<usize>,
    /// Which tabs are visible in MTF grid (true = shown, per chart index).
    pub(crate) mtf_visible: Vec<bool>,
    /// MTF Grid indicator status: (tf_label, close, sma200, kama, fisher, fisher_signal) per TF.
    /// Computed on symbol load for the MTF Grid panel. Lightweight — no full ChartState needed.
    pub(crate) mtf_grid_status: Vec<(
        &'static str,
        Option<f64>,
        Option<f64>,
        Option<f64>,
        Option<f64>,
        Option<f64>,
    )>,
    /// Receiver for async MTF grid status results (computed in background thread).
    pub(crate) mtf_grid_rx: Option<
        std::sync::mpsc::Receiver<
            Vec<(
                &'static str,
                Option<f64>,
                Option<f64>,
                Option<f64>,
                Option<f64>,
                Option<f64>,
            )>,
        >,
    >,
    /// Symbol that `mtf_grid_status` was last computed for. Lets the grid panel
    /// keep the cache-loaded all-timeframe status in sync with the active symbol
    /// so non-open timeframes still render their values (grid is not limited to
    /// whichever chart tabs happen to be open).
    pub(crate) mtf_grid_status_symbol: String,
    /// Deferred chart loads: indices of charts to load, one per frame (avoids startup freeze).
    pub(crate) deferred_chart_loads: VecDeque<usize>,
    /// Side index for O(1) duplicate suppression in `deferred_chart_loads`.
    pub(crate) deferred_chart_load_set: HashSet<usize>,
    /// Last time a deferred chart was synchronously loaded. Used to pace expensive
    /// cache reads + indicator/MTF recomputes so restored MTF grids don't monopolize
    /// consecutive UI frames during broad market-data sync.
    pub(crate) deferred_chart_last_load_at: std::time::Instant,

    /// Command palette open state.
    pub(crate) command_open: bool,
    /// Raw user input in the command palette.
    pub(crate) command_input: String,
    /// Currently highlighted command in console (arrow key navigation).
    pub(crate) console_selected: usize,
    /// ADR-092: Recent commands (MRU, up to 10, shown when palette filter is empty).
    pub(crate) recent_commands: VecDeque<String>,
    /// ADR-092: Compact mode — hides indicators and sub-panes for minimal execution view.
    pub(crate) compact_mode: bool,
    /// Session persistence is armed after we've attempted an initial restore.
    pub(crate) session_state_ready: bool,
    /// Last persisted session snapshot used for debounced incremental saves.
    pub(crate) session_last_saved_json: String,
    /// Timestamp when the current unsaved session diff was first observed.
    pub(crate) session_dirty_since: Option<std::time::Instant>,
    /// Last time we scanned session state for incremental persistence.
    pub(crate) session_last_scan_at: std::time::Instant,
    /// Consecutive incremental-save scans that found no change. Drives an
    /// adaptive scan backoff (500ms→2s) so an idle terminal stops rebuilding and
    /// diffing the session JSON twice a second; reset to 0 on any detected change.
    pub(crate) session_idle_scans: u32,
    /// Monotonic write sequence for session persistence. Bumped on the UI thread
    /// each time a snapshot is issued to disk; paired with `session_write_gate`
    /// so a late background autosave can never clobber a newer synchronous save.
    pub(crate) session_save_seq: u64,
    /// Highest session-write sequence already persisted to disk. Shared with the
    /// off-thread autosave worker to keep disk writes strictly newest-wins.
    pub(crate) session_write_gate: Arc<std::sync::Mutex<u64>>,
    /// True while an off-thread session autosave is running; coalesces the
    /// per-frame incremental save so redundant writers don't pile up.
    pub(crate) session_save_in_flight: Arc<std::sync::atomic::AtomicBool>,

    // ── indicator overlay toggles ────────────────────────────────────────
    pub(crate) show_sma200: bool,
    pub(crate) show_sma100: bool,
    pub(crate) show_kama: bool,
    pub(crate) show_ema21: bool,
    pub(crate) show_bollinger: bool,
    pub(crate) show_rsi: bool,
    pub(crate) show_fisher: bool,
    pub(crate) show_macd: bool,
    pub(crate) show_volume_pane: bool,
    pub(crate) show_stochastic: bool,
    pub(crate) show_adx: bool,
    pub(crate) show_ichimoku: bool,
    pub(crate) show_wma: bool,
    pub(crate) show_hma: bool,
    pub(crate) show_psar: bool,
    pub(crate) show_atr_proj: bool,
    pub(crate) show_prev_levels: bool,
    pub(crate) show_pivots: bool,
    pub(crate) show_fractals: bool,
    pub(crate) show_harmonics: bool,
    pub(crate) show_auto_fib: bool,
    pub(crate) show_supply_demand: bool,
    pub(crate) show_ehlers_ss: bool,
    pub(crate) show_ehlers_decycler: bool,
    pub(crate) show_ehlers_itl: bool,
    pub(crate) show_ehlers_mama: bool,
    pub(crate) show_ehlers_ebsw: bool,
    pub(crate) show_ehlers_cyber: bool,
    pub(crate) show_ehlers_cg: bool,
    pub(crate) show_ehlers_roof: bool,
    pub(crate) show_cci: bool,
    pub(crate) show_williams_r: bool,
    pub(crate) show_obv: bool,
    pub(crate) show_momentum: bool,
    pub(crate) show_cmo: bool,
    pub(crate) show_qstick: bool,
    pub(crate) show_disparity: bool,
    pub(crate) show_bop: bool,
    pub(crate) show_stddev: bool,
    pub(crate) show_mfi: bool,
    pub(crate) show_trix: bool,
    pub(crate) show_ppo: bool,
    pub(crate) show_ultosc: bool,
    pub(crate) show_stochrsi: bool,
    pub(crate) show_var_oscillator: bool,
    pub(crate) show_better_volume: bool,
    pub(crate) show_sessions: bool,
    pub(crate) show_vol_heatmap: bool,
    pub(crate) show_vwap: bool,
    pub(crate) show_price_histogram: bool,
    pub(crate) show_supertrend: bool,
    pub(crate) show_donchian: bool,
    pub(crate) show_keltner: bool,
    pub(crate) show_regression: bool,
    pub(crate) show_squeeze: bool,
    pub(crate) show_fvg: bool,
    pub(crate) show_order_blocks: bool,

    /// Saved chart templates: name → indicator JSON snapshot.
    pub(crate) chart_templates: std::collections::HashMap<String, serde_json::Value>,

    /// Drawing interaction mode.
    pub(crate) draw_mode: DrawMode,
    /// Current drawing style (applied to new drawings).
    pub(crate) draw_width: f32,
    pub(crate) draw_line_style: LineStyle,
    /// Pre-placement color for new drawings (TradingView: choose before placing).
    pub(crate) draw_color: egui::Color32,
    /// OHLC snap (magnet) toggle.
    pub(crate) snap_enabled: bool,
    /// Sync drawings across all charts with the same symbol (cross-timeframe).
    pub(crate) drawings_cross_tf: bool,
    /// Cross-timeframe drawings: sync drawings across charts with same symbol.
    pub(crate) cross_tf_drawings: bool,
    /// Auto-scroll to latest bar when new data arrives. Toggle with FOLLOW command.
    pub(crate) follow_latest: bool,
    /// In-progress polyline points (used during PlacingPolyline mode).
    pub(crate) polyline_points: Vec<(usize, f64)>,
    /// In-progress Elliott Wave / ABC / H&S / XABCD multi-click points.
    pub(crate) multi_click_points: Vec<(usize, f64)>,
    /// In-progress brush/freehand points (accumulated while mouse held down).
    pub(crate) brush_points: Vec<(usize, f64)>,

    /// ADR-038 Phase 2: Pluggable data source manager.
    pub(crate) data_sources: typhoon_engine::core::data_source::DataSourceManager,

    /// Broker connection fields (Alpaca).
    pub(crate) broker_api_key: String,
    pub(crate) broker_secret: String,
    pub(crate) broker_paper: bool,
    /// Full bar-sync controls are deliberately separate from broker login.
    /// Off = light mode: account/trading plus targeted fetches for open charts,
    /// owned positions, open-order symbols, and the user's watchlist.
    pub(crate) alpaca_full_bar_sync_enabled: bool,
    pub(crate) alpaca_enabled: bool,

    /// Broker connection fields (Kraken).
    pub(crate) kraken_full_bar_sync_enabled: bool,
    pub(crate) kraken_api_key: String,
    pub(crate) kraken_api_secret: String,
    pub(crate) kraken_ws_api_key: String,
    pub(crate) kraken_ws_api_secret: String,
    pub(crate) kraken_enabled: bool,
    pub(crate) kraken_connected: bool,
    pub(crate) kraken_pairs_requested: bool,
    pub(crate) kraken_futures_requested: bool,
    pub(crate) kraken_balances: Vec<(String, f64)>,
    pub(crate) show_kraken_spot_sell_dialog: bool,
    pub(crate) kraken_spot_sell_asset: String,
    pub(crate) kraken_spot_sell_pair: String,
    pub(crate) kraken_spot_sell_available: f64,
    pub(crate) kraken_spot_sell_qty: f64,
    pub(crate) kraken_spot_sell_pct: f32,
    pub(crate) kraken_spot_buy_pct: f32,
    pub(crate) kraken_spot_buy_qty: f64,
    pub(crate) kraken_trades: VecDeque<typhoon_engine::broker::kraken::KrakenTrade>,
    pub(crate) kraken_trade_keys: std::collections::HashSet<String>,
    pub(crate) kraken_cost_basis: std::collections::HashMap<String, KrakenCostBasis>,
    pub(crate) kraken_open_orders: Vec<typhoon_engine::broker::kraken::KrakenOrder>,
    /// (symbol, tf_label) → epoch-ms anchor for "WS pushed a bar this recent
    /// for this key". The REST sync scheduler consults this to skip refetch
    /// while the WS feed is keeping the cache current. O(1) insert and
    /// lookup; entries are not actively pruned because the per-key check
    /// already age-bounds them with the TF period.
    pub(crate) kraken_ws_fresh_until: std::collections::HashMap<(String, String), i64>,
    pub(crate) kraken_pairs: Vec<(String, String)>,
    /// Normalized pair/display symbols cached as a set so
    /// `kraken_spot_symbol_in_loaded_pairs` is O(1) — the previous linear
    /// `kraken_pairs.iter().any(...)` ran `normalize_pair_symbol` (allocating)
    /// twice per element on every sync-symbol audit, multiplying with O(n)
    /// `kraken_spot_symbol_scrape_enabled` callers.
    pub(crate) kraken_pairs_normalized: std::collections::HashSet<String>,
    pub(crate) kraken_futures_symbols: Vec<String>,
    /// Kraken public market-data scrape universe switches. These gate automated
    /// public OHLC/Futures scheduling so the scrape budget stays on instruments
    /// the user can actually trade.
    pub(crate) kraken_scrape_xstocks: bool,
    pub(crate) kraken_scrape_usd_crypto: bool,
    pub(crate) kraken_scrape_fiat_crypto: bool,
    pub(crate) kraken_scrape_crypto_crosses: bool,
    pub(crate) kraken_scrape_futures: bool,
    /// Backfill provider switches. These are source-specific fallbacks, not
    /// broker universe toggles: they supplement native broker bars without
    /// changing broker/account connectivity.
    pub(crate) backfill_alpaca_kraken_equities_enabled: bool,
    pub(crate) backfill_yahoo_chart_enabled: bool,
    /// Stream Kraken bar updates via WS v2 in addition to the REST scheduler.
    /// Subscribes to every spot pair across 1Min/5Min/15Min/30Min/1Hour/4Hour/
    /// 1Day/1Week so low-timeframe bars stay current without burning REST
    /// budget. On by default whenever Kraken is enabled — the OHLC channel
    /// is on Kraken's public WS endpoint (no auth needed) and is strictly
    /// better than REST alone for the low timeframes that REST can't keep
    /// up with. Toggle persists across sessions; flip it off only if you
    /// need to suppress the 8 TCP connections for testing or footprint.
    pub(crate) kraken_ws_ohlc_enabled: bool,
    /// Set once the WS OHLC pipeline has been kicked off this session so we
    /// don't re-spawn streamers if the broker thread emits more lifecycle
    /// events. Resets to false when the user toggles the setting off.
    pub(crate) kraken_ws_ohlc_started: bool,
    /// Exact WS v2 subscribe symbols already handed to the streamer pipeline.
    /// Lets Spot start from AssetPairs and xStocks join later from the
    /// instrument universe without duplicate subscriptions.
    pub(crate) kraken_ws_ohlc_streamed_pairs: std::collections::HashSet<String>,
    /// Rotating cursor for the one-batch-at-a-time Kraken xStocks OHLC snapshot sweep.
    pub(crate) kraken_ws_ohlc_snapshot_sweep_cursor: usize,
    pub(crate) kraken_ws_ohlc_snapshot_sweep_last_schedule: std::time::Instant,
    pub(crate) kraken_ws_ohlc_snapshot_sweep_in_flight: bool,
    pub(crate) crypto_fiat_quote_usd: bool,
    pub(crate) crypto_fiat_quote_usdt: bool,
    pub(crate) crypto_fiat_quote_usdc: bool,
    pub(crate) crypto_fiat_quote_usdg: bool,
    pub(crate) crypto_fiat_quote_eur: bool,
    pub(crate) crypto_fiat_quote_gbp: bool,
    pub(crate) crypto_fiat_quote_cad: bool,
    pub(crate) crypto_fiat_quote_aud: bool,
    pub(crate) crypto_fiat_quote_jpy: bool,
    pub(crate) crypto_fiat_quote_chf: bool,

    /// Finnhub API key.
    pub(crate) finnhub_key: String,
    /// FRED (Federal Reserve Economic Data) API key.
    pub(crate) fred_key: String,
    /// Notification config: Discord webhook, Pushover token/user, ntfy topic.
    pub(crate) discord_webhook: String,
    pub(crate) pushover_token: String,
    pub(crate) pushover_user: String,
    pub(crate) ntfy_topic: String,
    /// AI chat (Anthropic Claude / OpenAI GPT).
    pub(crate) anthropic_key: String,
    pub(crate) openai_key: String,
    pub(crate) gemini_key: String,
    pub(crate) xai_key: String, // Grok (xAI)
    pub(crate) mistral_key: String,
    pub(crate) perplexity_key: String,
    pub(crate) show_ai_chat: bool,
    /// Claude Code CLI chat window.
    pub(crate) show_claude_code: bool,
    pub(crate) claude_code_input: String,
    pub(crate) claude_code_history: Vec<(bool, String)>, // (is_user, message)
    pub(crate) claude_code_rx: Option<std::sync::mpsc::Receiver<String>>,
    /// Research packet stored verbatim so follow-ups in the chat window still see
    /// the TyphooN fundamentals — not just the `[Research packet: AAPL]` placeholder.
    pub(crate) claude_code_packet: Option<String>,
    /// Per-session UUID (reused across Send clicks so Claude CLI resumes the same thread).
    pub(crate) claude_code_session_id: Option<String>,
    /// Picked model alias for Claude CLI: "opus" | "sonnet" | "haiku".
    pub(crate) claude_model: String,
    /// Extended-thinking trigger prepended to prompts. Stored as the literal
    /// trigger phrase ("ultrathink" | "think harder" | "think hard" | "think" | "")
    /// so we can just inject it verbatim in `build_claude_prompt`.
    pub(crate) claude_effort: String,
    /// Gemini CLI chat window.
    pub(crate) show_gemini_cli: bool,
    pub(crate) gemini_cli_input: String,
    pub(crate) gemini_cli_history: Vec<(bool, String)>,
    pub(crate) gemini_cli_rx: Option<std::sync::mpsc::Receiver<String>>,
    pub(crate) gemini_cli_packet: Option<String>,
    pub(crate) gemini_model: String,
    /// Codex CLI chat window (local `codex` binary — OpenAI Codex).
    pub(crate) show_codex_cli: bool,
    pub(crate) codex_cli_input: String,
    pub(crate) codex_cli_history: Vec<(bool, String)>,
    pub(crate) codex_cli_rx: Option<std::sync::mpsc::Receiver<String>>,
    pub(crate) codex_cli_packet: Option<String>,
    pub(crate) codex_model: String,
    pub(crate) codex_reasoning_effort: String,
    /// Hermes Agent CLI chat window (local `hermes` binary).
    pub(crate) show_hermes_cli: bool,
    pub(crate) hermes_cli_input: String,
    pub(crate) hermes_cli_history: Vec<(bool, String)>,
    pub(crate) hermes_cli_rx: Option<std::sync::mpsc::Receiver<String>>,
    pub(crate) hermes_cli_packet: Option<String>,
    pub(crate) hermes_cli_session_id: String,
    /// Optional Hermes model override. Empty means use Hermes' configured default.
    pub(crate) hermes_model: String,
    /// Optional Hermes provider override. Empty means use Hermes' configured default.
    pub(crate) hermes_provider: String,
    /// Grok Build CLI chat window (local `grok` binary).
    pub(crate) show_grok_cli: bool,
    pub(crate) grok_cli_input: String,
    pub(crate) grok_cli_history: Vec<(bool, String)>,
    pub(crate) grok_cli_rx: Option<std::sync::mpsc::Receiver<String>>,
    pub(crate) grok_cli_packet: Option<String>,
    pub(crate) grok_cli_session_id: String,
    pub(crate) grok_model: String,
    pub(crate) grok_effort: String,
    pub(crate) ai_chat_history: Vec<(bool, String)>, // (is_user, message)
    pub(crate) ai_chat_input: String,
    pub(crate) ai_chat_packet: Option<String>,
    /// Currently selected model name for the ai_provider picker. Resets to the
    /// provider default when the user switches providers.
    pub(crate) ai_model: String,
    pub(crate) ai_provider: usize, // 0=Claude, 1=GPT
    // ── AI session persistence ──
    /// Stable per-conversation UUIDs. Empty until the first turn, then reused
    /// for every subsequent save of the same conversation. Claude reuses the
    /// existing `claude_code_session_id` (a UUID) as both the --resume id and
    /// the kv-cache session id.
    pub(crate) ai_chat_session_id: String,
    pub(crate) gemini_cli_session_id: String,
    pub(crate) codex_cli_session_id: String,
    /// AI Sessions history browser.
    pub(crate) show_ai_sessions: bool,
    pub(crate) ai_sessions_index: Vec<typhoon_engine::core::ai_sessions::SessionIndexEntry>,
    pub(crate) ai_sessions_viewing: Option<typhoon_engine::core::ai_sessions::AiSessionRecord>,
    pub(crate) ai_sessions_last_refresh: i64,
    /// AI response cache stats window.
    pub(crate) show_ai_cache: bool,
    pub(crate) ai_cache_stats: typhoon_engine::core::ai_response_cache::AiResponseCacheStats,
    pub(crate) ai_cache_recent: Vec<typhoon_engine::core::ai_response_cache::AiResponseCacheEntry>,
    pub(crate) ai_cache_last_refresh: i64,
    /// Reddit WSB posts.
    pub(crate) show_reddit: bool,
    /// BARDATA sync progress tracking.
    pub(crate) show_bardata: bool,
    pub(crate) bardata_total: usize,
    pub(crate) bardata_queued: usize,
    pub(crate) bardata_completed: usize,
    pub(crate) bardata_skipped: usize,
    pub(crate) bardata_log: VecDeque<String>,
    pub(crate) bardata_active: bool,
    pub(crate) reddit_posts: Vec<(String, String, u64, u64)>, // (title, url, score, comments)
    /// Matrix chat (community chat room — send + receive).
    pub(crate) show_matrix_chat: bool,
    pub(crate) matrix_room: String,
    pub(crate) matrix_messages: Vec<(String, String, String)>, // (sender, timestamp, body)
    pub(crate) matrix_input: String,
    pub(crate) matrix_access_token: String,
    pub(crate) matrix_user_id: String,
    pub(crate) matrix_last_fetch: std::time::Instant,
    /// Legacy cached Finnhub news tuples (headline, source, datetime) — used by the
    /// compact "News" side-pane and the WASM web mirror. Retained for backward compat.
    pub(crate) news_articles: Vec<(String, String, String)>,
    /// Rich multi-source news for the two-pane NEWS reader.
    pub(crate) news_full_articles: Vec<typhoon_engine::core::news::NewsArticle>,
    /// Index into `news_full_articles` currently open in the right pane.
    pub(crate) news_selected: Option<usize>,
    /// Symbol currently loaded into the news window (drives refresh button).
    pub(crate) news_symbol_filter: String,
    /// Full-text search query for the news reader.
    pub(crate) news_search_query: String,
    /// URL hash of the article selected before the most recent reload/session restore.
    pub(crate) news_selected_url_hash: String,
    /// UI state flag while a fetch/cached-load is in flight.
    pub(crate) news_loading: bool,
    /// Watchdog start time for news_loading so a lost broker result cannot keep heavy-sync mode latched forever.
    pub(crate) news_loading_started_at: Option<std::time::Instant>,
    /// Content hash of the current news_full_articles set (used for cache guard / re-filter decisions, mirroring the pattern requested for fundamentals).
    pub(crate) news_input_hash: u64,
    /// Latches once the News window has triggered its initial
    /// `LoadCachedNews` for the session. Prevents auto-load from
    /// firing every frame the window stays open, while still
    /// re-triggering on the next restart.
    pub(crate) news_initial_load_done: bool,
    /// Total rows in the `research_news` SQLite table, pushed from the broker
    /// via `BrokerMsg::NewsDbTotal` after each cache load / fresh fetch / scope
    /// scrape so the header shows "· N in DB" even when the in-memory list is
    /// empty. `None` until the first push arrives. The render thread never
    /// queries this itself (the old poll blocked on the write mutex).
    pub(crate) news_db_total: Option<i64>,
    /// User-entered Marketaux API key (free tier 100/day).
    pub(crate) marketaux_key: String,
    /// User-entered Alpha Vantage API key (free tier 25/day).
    pub(crate) alpha_vantage_key: String,
    /// User-entered FMP API key (free tier 250/day, also used for transcripts).
    pub(crate) fmp_key: String,
    /// User-entered CryptoPanic API token (free public tier, per-currency filter).
    pub(crate) cryptopanic_key: String,

    /// SL/TP planning lines (visual, pre-broker).
    pub(crate) sl_price: Option<f64>,
    pub(crate) tp_price: Option<f64>,
    /// True while user is dragging the SL line on the chart.
    pub(crate) dragging_sl: bool,
    /// True while user is dragging the TP line on the chart.
    pub(crate) dragging_tp: bool,

    // ── risk calculator state ────────────────────────────────────────────
    pub(crate) rc_equity: String,
    pub(crate) rc_risk_pct: String,
    pub(crate) rc_entry: String,
    pub(crate) rc_sl: String,
    pub(crate) rc_tp: String,
    pub(crate) rc_tick_value: String,
    pub(crate) rc_tick_size: String,
    pub(crate) rc_result: String,

    // ── backtest state ───────────────────────────────────────────────────
    pub(crate) bt_strategy: usize,
    pub(crate) bt_fast_period: String,
    pub(crate) bt_slow_period: String,
    pub(crate) bt_equity: String,
    pub(crate) bt_result: Option<backtest::TradeReport>,
    pub(crate) bt_trades: Vec<backtest::Trade>,
    pub(crate) bt_equity_curve: Vec<f64>,

    // ── optimizer state ──────────────────────────────────────────────────
    pub(crate) opt_fast_range: String,
    pub(crate) opt_slow_range: String,
    pub(crate) opt_results: Vec<backtest::OptimizationResult>,
    // Walk-forward analysis state
    pub(crate) wf_result: Option<backtest::WalkForwardResult>,
    pub(crate) wf_windows_count: String,
    // GPU optimizer state
    pub(crate) opt_rsi_range: String,
    pub(crate) opt_atr_sl_range: String,
    pub(crate) opt_atr_tp_range: String,
    pub(crate) gpu_opt_results: Vec<gpu_compute::BacktestResult>,
    pub(crate) gpu_opt_combos: Vec<gpu_compute::ParamCombo>,
    pub(crate) gpu_backtester: Option<gpu_compute::GpuBacktester>,

    // ── margin monitor state ─────────────────────────────────────────────
    pub(crate) mm_equity: String,
    pub(crate) mm_margin: String,
    pub(crate) mm_margin_per_lot: String,
    pub(crate) mm_trim_pct: String,
    pub(crate) mm_result: String,

    // ── tab bar ──────────────────────────────────────────────────────────
    /// Index of the active tab (into `charts`).
    pub(crate) active_tab: usize,

    // ── watchlist ────────────────────────────────────────────────────────
    /// Rich watchlist data: symbol name, last, prev_close, change, change_pct, volume, cache_key.
    pub(crate) watchlist_rows: Vec<WatchlistRow>,
    /// Unix timestamp of last successful watchlist quote refresh — drives staleness badge.
    pub(crate) watchlist_last_update_ts: i64,
    /// Last time an automatic watchlist quote refresh was dispatched. `None` until
    /// the first dispatch, so the watchlist populates on startup rather than only
    /// when the user manually adds a symbol. See the periodic refresh in `update`.
    pub(crate) watchlist_auto_refresh_at: Option<std::time::Instant>,
    /// Watchlist length at the last quote fetch. A change forces an immediate
    /// refresh even on the slow weekend cadence (new symbol added / removed).
    pub(crate) watchlist_quotes_fetched_count: usize,
    /// Unix timestamp of last successful positions refresh.
    pub(crate) positions_last_update_ts: i64,
    /// Last automatic positions/orders snapshot dispatch. Separate from successful
    /// update timestamp so a slow broker response does not cause per-frame request spam.
    pub(crate) positions_auto_refresh_at: Option<std::time::Instant>,
    /// Unix timestamp of last successful orders refresh.
    pub(crate) orders_last_update_ts: i64,
    /// User-managed watchlist symbols (persisted in session).
    pub(crate) user_watchlist: Vec<String>,
    /// Fallback prices from Yahoo (or other sources) when primary broker has no data.
    /// Key = symbol, Value = (price, source, timestamp)
    pub(crate) watchlist_fallback_prices:
        std::collections::HashMap<String, (f64, String, std::time::Instant)>,
    /// Input field for adding symbols to watchlist.
    pub(crate) watchlist_input: String,

    // ── floating window visibility ───────────────────────────────────────
    pub(crate) show_settings: bool,
    pub(crate) was_settings_open: bool,
    pub(crate) show_risk_calc: bool,
    pub(crate) show_compound_calc: bool,
    pub(crate) ci_principal: String,
    pub(crate) ci_rate: String,
    pub(crate) ci_years: String,
    pub(crate) ci_compounds: String,
    pub(crate) ci_contribution: String,
    pub(crate) ci_result: Vec<(f64, f64, f64)>,
    pub(crate) show_backtest: bool,
    pub(crate) show_screener: bool,
    pub(crate) screener_filter: String,
    pub(crate) screener_sort_col: usize,
    pub(crate) screener_sort_asc: bool,
    pub(crate) show_symbols: bool,
    pub(crate) symbols_filter: String,
    pub(crate) symbols_expanded: std::collections::HashSet<String>,
    /// Full asset list from broker (symbol, name, asset_class) for Symbol Explorer.
    pub(crate) all_broker_assets: Vec<(String, String, String)>,
    pub(crate) all_broker_assets_fetched: bool,
    pub(crate) show_optimizer: bool,
    pub(crate) show_news: bool,
    pub(crate) show_calendar: bool,
    pub(crate) show_sec: bool,
    pub(crate) sec_selected_filing: Option<usize>,
    pub(crate) sec_tab: usize, // 0=Filings, 1=Alerts, 2=Insiders, 3=Timeline
    pub(crate) sec_search_query: String, // text search filter for filings
    pub(crate) sec_keyword_input: String, // keyword watchlist input
    pub(crate) sec_keywords: Vec<String>, // cached keyword list
    pub(crate) earnings_active_only: bool, // filter earnings calendar to active symbols
    pub(crate) dividends_active_only: bool, // filter dividend calendar to active symbols
    pub(crate) ev_active_only: bool, // filter EV scanner to active symbols
    /// Per-symbol visibility toggles in the Fundamentals window.
    pub(crate) fundamentals_hidden_symbols: std::collections::HashSet<String>,
    pub(crate) congress_active_only: bool, // filter congressional trades to active symbols
    pub(crate) volume_active_only: bool,   // filter unusual volume to active symbols
    pub(crate) sec_filing_content: String, // cached filing document text
    pub(crate) sec_filing_content_for: String, // accession number this content belongs to (for sticky display)
    pub(crate) sec_filing_pinned: bool, // pin document viewer (don't clear when navigating filings)
    pub(crate) sec_filing_loading: bool,
    pub(crate) sec_filing_summary: Option<sec_filing::FilingSummary>,
    pub(crate) sec_filing_summary_for: String,
    pub(crate) show_insider: bool,
    pub(crate) show_fundamentals: bool,
    pub(crate) show_analyst: bool,
    pub(crate) analyst_result: String, // last fetched Finnhub recommendations JSON
    pub(crate) show_holders: bool,
    pub(crate) holders_result: String, // last fetched SEC EDGAR 13F JSON
    pub(crate) show_orderbook_window: bool,
    pub(crate) orderbook_result: String, // last fetched L2 orderbook JSON
    pub(crate) kraken_orderbook_ws_symbol: String,
    pub(crate) kraken_chart_l2_ws_symbol: String,
    pub(crate) kraken_chart_l2_last_start_attempt: std::time::Instant,
    pub(crate) market_clock_status: String,
    pub(crate) show_symbol_overlap: bool,
    pub(crate) show_correlation: bool,
    pub(crate) show_seasonals: bool,
    pub(crate) show_montecarlo: bool,
    pub(crate) show_stress_test: bool,
    pub(crate) show_volume_profile: bool,
    pub(crate) show_hv_cone: bool,
    pub(crate) show_sector_heatmap: bool,
    pub(crate) show_dividends: bool,
    pub(crate) show_company_info_window: bool,
    pub(crate) company_info_symbol: String,
    pub(crate) company_info_text: String,
    /// Global broker scope filter applied to all fundamental-based commands
    /// (OUTLIERS, EVOUTLIERS, DIVSCREEN, SECTOR_HEATMAP, HV_CONE, EV viewer, etc.).
    /// All = no filter. Use `SCOPE [ALL|ALPACA|KRAKEN]` command to change.
    pub(crate) broker_scope: EventSource,
    /// Cached broker scope HashSet. Invalidated by `(bg_rev, broker_scope)` pair —
    /// only recomputed when fundamentals/specs load (bg_rev bumped) or scope changes.
    /// O(1) reads for the 5+ windows that need scope filtering.
    pub(crate) cached_scope_syms: Option<std::collections::HashSet<String>>,
    pub(crate) cached_scope_key: Option<(u64, EventSource)>,
    /// Monotonic counter bumped each time `self.bg` is replaced from the BG thread.
    /// Used as a dirty-flag for any cache derived from `bg.*` state.
    pub(crate) bg_rev: u64,
    /// UX6: One-shot flag to auto-scroll outlier table to first EXTREME tier on next render.
    pub(crate) outlier_scroll_pending: bool,
    /// UX4: Named workspace presets — maps name → JSON snapshot of show_* flags.
    pub(crate) workspaces: std::collections::HashMap<String, String>,
    /// UX7: Sparkline cache — last 30 daily closes per symbol, lazy-fetched on first render.
    /// PERF: values wrapped in Arc so `get_sparkline` returns O(1) clones instead of copying
    /// the whole Vec<f64> on every cache hit (called for every visible window row per frame).
    pub(crate) sparkline_cache: std::collections::HashMap<String, std::sync::Arc<Vec<f64>>>,
    /// UX3: Deferred symbol action from right-panel context menus (applied at end of update()).
    pub(crate) deferred_symbol_action: SymbolAction,
    /// PERF: Cached active symbols list. Recomputed when chart/position/watchlist inputs change.
    pub(crate) cached_active_symbols: Vec<String>,
    pub(crate) cached_active_symbols_key: Option<u64>,
    /// PERF: HashSet of cached_active_symbols for O(1) lookup.
    pub(crate) cached_active_symbols_set: std::collections::HashSet<String>,
    /// Cached scoped fundamentals (filtered by broker_scope). Rebuilt only when
    /// `(bg_rev, broker_scope)` changes — not per frame. Used by Sector Heatmap,
    /// Dividend Yield Screener, Outlier Scanner.
    pub(crate) cached_scoped_fundamentals: Vec<typhoon_engine::core::fundamentals::Fundamentals>,
    pub(crate) cached_scoped_fundamentals_key: Option<(u64, EventSource)>,
    /// Cached Alpaca bar-state map (symbol, timeframe) -> sync metadata.
    /// Rebuilt only when `bg_rev` changes so the sync scheduler doesn't rescan
    /// `bg.detailed_stats` every rotation.
    pub(super) cached_alpaca_sync_state:
        std::collections::HashMap<(String, String), SyncCacheState>,
    pub(crate) cached_alpaca_sync_state_rev: Option<u64>,
    /// Cached Kraken bar-state map keyed by normalized `(symbol, timeframe)`.
    pub(super) cached_kraken_sync_state:
        std::collections::HashMap<(String, String), SyncCacheState>,
    pub(crate) cached_kraken_sync_state_rev: Option<u64>,
    /// Cached Kraken Futures bar-state map keyed by normalized `(symbol, timeframe)`.
    pub(super) cached_kraken_futures_sync_state:
        std::collections::HashMap<(String, String), SyncCacheState>,
    pub(crate) cached_kraken_futures_sync_state_rev: Option<u64>,
    /// Cached Kraken internal equities bar-state map keyed by normalized `(symbol, timeframe)`.
    pub(super) cached_kraken_equities_sync_state:
        std::collections::HashMap<(String, String), SyncCacheState>,
    pub(crate) cached_kraken_equities_sync_state_rev: Option<u64>,
    /// Cached Yahoo Chart assist bar-state map keyed by normalized `(symbol, timeframe)`.
    pub(super) cached_yahoo_chart_sync_state:
        std::collections::HashMap<(String, String), SyncCacheState>,
    pub(crate) cached_yahoo_chart_sync_state_rev: Option<u64>,
    /// Cached Sync Status rows. The window is informational; recomputing the
    /// whole broker/timeframe matrix on every repaint during full sync is pure
    /// render-thread waste.
    pub(super) cached_bar_sync_rows: Vec<SyncStatsRow>,
    pub(crate) cached_bar_sync_rows_last: std::time::Instant,
    /// Receiver for an in-flight bar-sync matrix recompute running on a blocking
    /// worker. The full xStocks/Merged scan is hundreds of ms of CPU on a 12k
    /// universe, so it is never run on the render thread; `Some` means a compute
    /// is in flight and a new one must not be dispatched.
    pub(crate) bar_sync_compute_rx:
        Option<std::sync::mpsc::Receiver<super::sync_status::BarSyncResult>>,
    /// Coverage % across live brokers from the most recent Sync Status snapshot.
    /// Refreshed on every `compute_bar_sync_rows` call (cached, ≤1Hz).
    /// `auto_full_tilt_until_caught_up` consults it to keep request pressure
    /// high while the catch-up is in progress and let the scheduler drop back
    /// to balanced cadence once it crosses the healthy threshold.
    pub(crate) cached_bar_sync_overall_pct: f32,
    /// Latched "catch-up in progress" flag. Engages when overall coverage drops
    /// below the engage threshold and only releases once we cross the higher
    /// release threshold — hysteresis to keep a row flipping at the edge from
    /// rapidly cycling full-tilt mode on/off.
    pub(crate) auto_full_tilt_active: bool,
    /// Full tradable Kraken Securities/equities symbol universe from the internal public catalog.
    pub(crate) kraken_equity_universe_symbols: Vec<String>,
    /// Lightweight ticker → company name map populated from the Kraken iapi equity
    /// catalog at universe load time. Used as a fast fallback for chart headers and
    /// the symbol picker when the full `all_fundamentals` table is empty (the normal
    /// case for the 12 k xStock universe because the heavy scrape is intentionally
    /// deferred).
    pub(crate) kraken_equity_names: std::collections::HashMap<String, String>,
    /// WS-tokenized xStock subset of the universe (the `{SYM}x/USD` pairs that
    /// actually exist on Kraken's public WS v2 OHLC channel, ~150 of the ~12k).
    /// Scopes the WS OHLC snapshot sweep; the full catalog still syncs via the
    /// Alpaca/Yahoo breadth lanes + demand-scoped iapi.
    pub(crate) kraken_equity_tokenized_symbols: Vec<String>,
    /// Equity symbols whose iapi `overnight_trading_support` is disabled — they
    /// trade pre/core/after only (CLOSED 8 PM–4 AM ET), not the full 24/5 cycle.
    /// Absence ⇒ overnight-enabled (the common case); drives the session label.
    pub(crate) kraken_equity_no_overnight: std::collections::HashSet<String>,
    pub(crate) kraken_equity_universe_requested: bool,
    pub(crate) show_reg_sho_window: bool,
    pub(crate) kraken_equity_universe_retry_after_ts: i64,
    pub(crate) kraken_equities_sync_pause_until_ts: i64,
    pub(crate) kraken_equities_sync_pause_reason: String,
    pub(crate) yahoo_chart_sync_pause_until_ts: i64,
    pub(crate) yahoo_chart_sync_pause_reason: String,
    pub(crate) heavy_sync_in_progress: bool,
    /// SEC window caches — all keyed off `(bg_rev, broker_scope, ...)` so the heavy
    /// dedup/filter/sort work only runs when state actually changes, not every frame.
    /// Keys are u64 hashes for zero-alloc per-frame comparison.
    /// Filings tab: sorted indices into `bg.sec_filings` after dedup+scope+filter+search.
    pub(crate) sec_cache_filings: Vec<usize>,
    pub(crate) sec_cache_filings_key: Option<u64>,
    /// User-controlled filter/search/sort key for the filings cache. Kept
    /// separate from the SEC data key so checkbox/search changes can rebuild
    /// the visible table even while a broad EDGAR scrape is publishing rows.
    pub(crate) sec_cache_filings_controls_key: Option<u64>,
    /// Insiders tab: (ticker, trade index) tuples for cross-symbol rendering.
    pub(crate) sec_cache_insiders: Vec<(String, usize)>,
    pub(crate) sec_cache_insiders_clusters: Vec<(String, usize)>,
    pub(crate) sec_cache_insiders_key: Option<u64>,
    /// Timeline tab: (month, count, "type:count type:count..." breakdown) per month, newest first.
    pub(crate) sec_cache_timeline: Vec<(String, usize, String)>,
    pub(crate) sec_cache_timeline_key: Option<u64>,
    /// Tab count strings — `(scoped_filings, alerts, insider_total)`.
    pub(crate) sec_cache_tab_counts: (usize, usize, usize),
    pub(crate) sec_cache_tab_counts_key: Option<u64>,
    /// Last time SEC window caches performed O(N) rebuild work on the UI thread.
    pub(crate) sec_cache_last_rebuild: std::time::Instant,
    pub(crate) show_event_calendar: bool,
    pub(crate) event_calendar_rows: Vec<EventRow>,
    pub(crate) event_filter_source: EventSource,
    pub(crate) event_filter_earnings: bool,
    pub(crate) event_filter_exdiv: bool,
    pub(crate) event_filter_divpay: bool,
    pub(crate) show_confluence: bool,
    pub(crate) show_stat_arb: bool,
    pub(crate) show_risk_budget: bool,
    pub(crate) show_order_flow: bool,
    pub(crate) show_bookmap: bool,
    pub(crate) bookmap_windows: Vec<BookmapWindowState>,
    pub(crate) show_outliers: bool,
    pub(crate) outliers: Vec<typhoon_engine::core::var::OutlierResult>,
    pub(crate) sector_stats: Vec<typhoon_engine::core::var::SectorStats>,
    pub(crate) multi_outliers: Vec<typhoon_engine::core::var::MultiOutlierResult>,
    pub(crate) show_option_chain: bool,
    pub(crate) option_chain_sym: String, // symbol last fetched
    // MQL5/PineScript/…/transpile compiler
    pub(crate) show_indicator_compiler: bool,
    pub(crate) compiler_source: String,  // source code input
    pub(crate) compiler_language: usize, // see COMPILER_LANGS below
    pub(crate) compiler_transpile_target: usize, // target language index for transpile dropdown
    pub(crate) compiler_transpiled: Option<String>, // transpiled source output
    pub(crate) compiler_diagnostics: VecDeque<String>,
    pub(crate) compiler_metadata: Option<mql5_compiler::CompileResult>,
    pub(crate) show_journal: bool,
    pub(crate) show_object_list: bool,
    pub(crate) journal_entries: Vec<JournalEntry>,
    pub(crate) show_var_mult: bool,
    pub(crate) show_margin_monitor: bool,
    pub(crate) show_cache_stats: bool,
    pub(crate) show_storage: bool,
    pub(crate) storage_filter: String,
    pub(crate) storage_delete_confirm: Option<String>,
    pub(crate) storage_delete_filtered_confirm: bool,
    pub(crate) storage_prune_disabled_kraken_quotes_confirm: bool,
    pub(crate) storage_purge_bars_confirm: bool,
    pub(crate) storage_purge_broker_confirm: Option<String>,
    pub(crate) storage_purge_timeframe_confirm: bool,
    /// Slider position for the news age-purge tool (index into the
    /// NEWS_PURGE_AGE_DAYS notch array — 7/30/90/180/365/730/1825).
    /// Default 4 = 1 year, which is the most common "I want to free
    /// some space but keep recent context" pick.
    pub(crate) storage_purge_news_age_idx: usize,
    /// 2-step confirmation latch for the news purge button.
    pub(crate) storage_purge_news_confirm: bool,
    pub(crate) storage_page: usize,
    pub(crate) storage_sort_col: usize,
    pub(crate) storage_sort_asc: bool,
    /// Cached filtered/sorted Storage Manager rows. Rebuilding this every frame
    /// against a multi-million-row cache summary causes hard UI stalls.
    pub(crate) storage_filtered_rows_cache: Vec<(String, i64, i64)>,
    pub(crate) storage_filtered_rows_cache_key: Option<u64>,
    pub(crate) storage_disabled_kraken_quote_keys_cache: Vec<String>,
    pub(crate) storage_disabled_kraken_quote_keys_cache_rev: Option<u64>,
    /// Broader "out-of-scope" Kraken purge: confirm latch + cached key set
    /// (keys the current sync config would not fetch — disabled sector or no
    /// longer in Kraken's loaded universe). Mirrors the disabled-quote prune.
    pub(crate) storage_prune_out_of_scope_kraken_confirm: bool,
    pub(crate) storage_out_of_scope_kraken_keys_cache: Vec<String>,
    pub(crate) storage_out_of_scope_kraken_keys_cache_rev: Option<u64>,
    pub(crate) cache_stats_sort_col: usize,
    pub(crate) cache_stats_sort_asc: bool,
    /// Canonical cache suffix selected for "delete this timeframe across all brokers".
    pub(crate) storage_delete_timeframe: String,
    /// Text buffer for the "new cache location" input in Storage Manager. Holds
    /// the path the user is typing BEFORE they click Save/Copy. Seeded with
    /// the currently-configured custom dir (if any) when Storage Manager opens.
    pub(crate) storage_cache_path_input: String,
    /// Last result from the Save / Copy / Reset cache-location action. Shown
    /// inline in Storage Manager. `(success, message)`.
    pub(crate) storage_cache_move_result: Option<(bool, String)>,
    /// Live async result channel for "Copy cache to new location (VACUUM INTO)".
    /// Produced by a worker thread, consumed once in the render loop to
    /// populate `storage_cache_move_result`. `None` = no op in flight.
    pub(crate) storage_cache_move_rx: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
    /// Bar-sync % per broker/TF window. Counted from `bg.detailed_stats`,
    /// with freshness derived from `bg.bar_ts_cache` when available.
    pub(crate) show_sync_status: bool,
    pub(crate) show_help: bool,
    pub(crate) help_filter: String,
    /// Count of alerts that have fired since the user last dismissed the badge.
    /// Rendered as a red breach counter on the top bar so the trader can't miss it.
    pub(crate) alert_breach_count: u32,
    /// Unix timestamp of the most recent alert breach (for tooltip display).
    pub(crate) alert_last_breach_ts: i64,
    /// Message of the most recent alert (shown in the breach tooltip).
    pub(crate) alert_last_breach_msg: String,
    /// Deduplicate broker bar fetches per source. Keys are normalized `SYMBOL:TF`.
    pub(crate) pending_alpaca_fetches: std::collections::HashSet<String>,
    pub(crate) pending_kraken_fetches: std::collections::HashSet<String>,
    pub(crate) pending_kraken_futures_fetches: std::collections::HashSet<String>,
    pub(crate) pending_yahoo_chart_fetches: std::collections::HashSet<String>,
    /// Per-key cooldown for broker bar re-queues. The in-flight HashSet only
    /// dedups while a fetch is pending; once it completes we'd previously
    /// re-queue immediately on the next sync tick, which (during a closed
    /// market) hit the same SYMBOL:TF every minute without producing new
    /// bars. Keys are `{source}:{SYMBOL}:{TF}` and values are the unix-second
    /// timestamp of the last queued fetch. A re-queue within ~half the TF
    /// period is skipped.
    pub(crate) fetch_last_queued_ts: std::collections::HashMap<String, i64>,
    /// Cursor-limited broad sync rotation. Each refill scans only a bounded slice
    /// of the broker universe in high-timeframe-first order, while the pending
    /// fetch sets keep foreground/manual and background requests deduplicated.
    ///
    /// Keep independent cursors for native Kraken Equities/iapi and the fast
    /// Alpaca/Yahoo assist lanes. Sharing the iapi cursor made Cloudflare-bound
    /// native repair, Alpaca batches, and Yahoo requests advance one another's
    /// symbol windows, slowing broad Merged coverage convergence.
    pub(crate) alpaca_sync_cursor: usize,
    pub(crate) kraken_spot_sync_cursors: [usize; 4],
    pub(crate) kraken_equities_sync_cursor: usize,
    pub(crate) kraken_equities_alpaca_sync_cursor: usize,
    pub(crate) yahoo_chart_sync_cursor: usize,
    pub(crate) kraken_futures_sync_cursors: [usize; 4],
    /// Alpaca retry queue — persisted across restarts via cache KV at `alpaca:retry_queue`.
    /// Entries are (symbol, timeframe) pairs that 429'd or partially completed; the
    /// `poll_alpaca_retry_queue()` tick re-dispatches due entries with exponential backoff.
    pub(crate) alpaca_retry_queue: Vec<AlpacaRetry>,
    /// Unix seconds of the last retry-queue poll tick (throttled to 10s intervals).
    pub(crate) alpaca_retry_last_poll: i64,
    /// Set once after startup to trigger the initial KV load into `alpaca_retry_queue`.
    pub(crate) alpaca_retry_loaded: bool,
    /// First time the persisted Alpaca retry queue diverged from memory.
    /// Flushed in coarse batches; never write this KV on every worker result.
    pub(crate) alpaca_retry_dirty_since: Option<std::time::Instant>,
    /// Definitive no-data tombstones for Alpaca symbol/timeframe pairs.
    /// Persisted via cache KV at `alpaca:no_data_pairs` and consulted by all
    /// automated scheduling paths before dispatch.
    pub(super) alpaca_no_data_pairs: std::collections::HashMap<String, AlpacaNoDataPair>,
    pub unresolvable_pairs: std::collections::HashMap<String, UnresolvablePair>,
    /// Per-broker normalized `SYMBOL:TF` tombstone indexes for O(1) scheduler
    /// membership checks without rebuilding a filtered HashSet on every refill.
    pub(crate) unresolvable_fetch_keys_by_broker:
        std::collections::HashMap<String, std::collections::HashSet<String>>,
    pub(crate) alpaca_no_data_loaded: bool,
    /// First time the persisted Alpaca no-data tombstone set diverged from memory.
    pub(crate) alpaca_no_data_dirty_since: Option<std::time::Instant>,
    /// First time the persisted unresolvable-pair set diverged from memory.
    pub(crate) unresolvable_dirty_since: Option<std::time::Instant>,
    /// Persisted "bounded full-history fetch already exhausted available
    /// Alpaca bars for this pair" markers. Only suppresses repeat Backfill
    /// scheduling; Missing/Stale sync still proceeds normally.
    pub(super) alpaca_backfill_complete_pairs:
        std::collections::HashMap<String, AlpacaBackfillCompletePair>,
    pub(crate) alpaca_backfill_complete_loaded: bool,
    pub(crate) alpaca_backfill_complete_dirty_since: Option<std::time::Instant>,
    pub(super) kraken_backfill_complete_pairs:
        std::collections::HashMap<String, AlpacaBackfillCompletePair>,
    pub(crate) kraken_backfill_complete_loaded: bool,
    pub(crate) kraken_backfill_complete_dirty_since: Option<std::time::Instant>,
    pub(super) kraken_futures_backfill_complete_pairs:
        std::collections::HashMap<String, AlpacaBackfillCompletePair>,
    pub(crate) kraken_futures_backfill_complete_loaded: bool,
    pub(crate) kraken_futures_backfill_complete_dirty_since: Option<std::time::Instant>,
    pub(crate) show_connect: bool,
    pub(crate) show_indicators_panel: bool,
    pub(crate) show_data_window: bool,
    pub(crate) show_alerts: bool,
    // Fundamentals symbol source settings
    pub(crate) fund_source_alpaca: bool,
    pub(crate) fund_source_kraken: bool,
    /// ADR-094: SCOPE popup window with source checkboxes.
    pub(crate) show_scope_window: bool,
    // Scrape status dashboard
    pub(crate) show_scrape_status: bool,
    pub(crate) scrape_fund_running: bool,
    pub(crate) scrape_fund_started_at: Option<std::time::Instant>,
    pub(crate) scrape_fund_ok: usize,
    pub(crate) scrape_fund_fail: usize,
    pub(crate) scrape_fund_skipped: usize,
    pub(crate) scrape_fund_total: usize,
    pub(crate) scrape_fund_last_msg: String,
    /// Startup auto fundamentals was deferred because selected source universes
    /// were not loaded yet. Re-fired once the universe symbols arrive.
    pub(crate) auto_fundamentals_deferred: bool,
    pub(crate) auto_fundamentals_started: bool,
    pub(crate) scrape_sec_running: bool,
    pub(crate) scrape_sec_started_at: Option<std::time::Instant>,
    pub(crate) scrape_sec_last_msg: String,
    /// Startup auto SEC scrape was deferred because Scope had no symbols yet.
    pub(crate) auto_sec_scrape_deferred: bool,
    /// Hash-based dedup for broker KV writes — skip put_kv if content unchanged
    pub(crate) kv_write_hashes: std::collections::HashMap<&'static str, u64>,
    /// Throttle: last write time per KV key (max once per 30s even if content changes)
    pub(crate) kv_write_times: std::collections::HashMap<&'static str, std::time::Instant>,
    // Fundamentals windows
    pub(crate) show_ev_scanner: bool,
    pub(crate) show_earnings_calendar: bool,
    pub(crate) show_dividend_calendar: bool,
    // Unusual Whales / Godel Terminal features
    pub(crate) show_unusual_volume: bool,
    pub(crate) unusual_volume_results: Vec<(String, f64, f64, f64)>, // (symbol, today_vol, avg_vol, ratio)
    pub(crate) show_sector_rotation: bool,
    pub(crate) show_fred: bool,
    pub(crate) fred_data: Vec<typhoon_engine::core::fred::FredSeries>,
    pub(crate) fred_yield_curve: Vec<(String, f64)>,
    pub(crate) show_econ_calendar: bool,
    pub(crate) econ_events: Vec<(String, String, String, String, String)>, // (date, country, event, impact, actual)
    // Calendar filters — persisted across window open/close.
    pub(crate) econ_filter_high: bool,
    pub(crate) econ_filter_medium: bool,
    pub(crate) econ_filter_low: bool,
    pub(crate) econ_filter_holiday: bool,
    /// Currency/country filter text. Empty = all. Comma-separated (e.g. "USD,EUR,GBP").
    pub(crate) econ_filter_currencies: String,
    /// Unix timestamp of last successful econ calendar fetch, for staleness badge.
    pub(crate) econ_last_fetch_ts: i64,
    pub(crate) show_congress: bool,
    pub(crate) congress_trades: Vec<(String, String, String, String, String, String)>, // (date, rep, ticker, type, amount, party)
    /// SEC filing type filters [Form 4, 13F, DEF 14A, S-1, 10-K, 10-Q, 8-K].
    pub(crate) sec_filters: [bool; 7],
    /// SEC filings pagination (0-indexed page number).
    pub(crate) sec_page: usize,

    /// Sort states for data tables.
    pub(crate) ev_sort: SortState,
    pub(crate) sec_sort: SortState,
    pub(crate) insider_sort: SortState,
    pub(crate) outlier_sort: SortState,
    /// Sort state for the single-metric outlier table (VAROUTLIER/EVOUTLIER/ATROUTLIER).
    pub(crate) outlier_single_sort: SortState,
    pub(crate) watchlist_sort: SortState,
    /// Whether we've already tried populating watchlist from cache (avoid repeated DB scans).
    pub(crate) watchlist_cache_tried: bool,

    /// Price alerts.
    pub(crate) alerts: Vec<(f64, String)>,
    pub(crate) alert_price_input: String,
    // Indicator-based alert engine
    pub(crate) indicator_alerts: Vec<IndicatorAlert>,
    pub(crate) show_alert_builder: bool,
    pub(crate) alert_symbol: String,
    pub(crate) alert_indicator: usize, // index into ALERT_INDICATORS
    pub(crate) alert_condition: usize, // 0=crosses above, 1=crosses below, 2=greater than, 3=less than
    pub(crate) alert_threshold: String,
    // Risk-of-Ruin
    pub(crate) show_risk_ruin: bool,
    pub(crate) ruin_win_rate: String,
    pub(crate) ruin_avg_win: String,
    pub(crate) ruin_avg_loss: String,
    pub(crate) ruin_risk_pct: String,
    pub(crate) ruin_results: Vec<f32>, // final equity per simulation
    // Replay mode
    pub(crate) replay_active: bool,
    pub(crate) replay_bar_idx: usize,
    pub(crate) replay_playing: bool,
    pub(crate) replay_speed: f32,
    pub(crate) replay_timer: f32,
    // Symbol autocomplete
    pub(crate) symbol_suggestions: Vec<(String, String, String)>, // (symbol, company, sector)
    pub(crate) symbol_ac_selected: usize,
    pub(crate) symbol_ac_visible: bool,
    pub(crate) alert_label_input: String,

    /// Order entry form.
    pub(crate) order_symbol: String,
    pub(crate) order_qty: String,
    pub(crate) order_side: usize, // 0=buy, 1=sell

    // ── Fear & Greed Index ───────────────────────────────────────────────
    pub(crate) show_fear_greed: bool,
    pub(crate) fear_greed_value: u32,    // 0-100
    pub(crate) fear_greed_label: String, // "Extreme Fear", "Fear", "Neutral", "Greed", "Extreme Greed"

    // ── World Indices Dashboard ─────────────────────────────────────────
    pub(crate) show_world_indices: bool,
    pub(crate) world_indices_data: Vec<WatchlistRow>,

    // ── Crypto Top 50 ───────────────────────────────────────────────────
    pub(crate) show_crypto_top50: bool,
    pub(crate) crypto_top50: Vec<(String, f64, f64, f64)>, // (name, price, change_24h%, market_cap)

    // ── Forex Major Pairs ───────────────────────────────────────────────
    pub(crate) show_forex_matrix: bool,
    pub(crate) forex_pairs_data: Vec<WatchlistRow>,

    // ── Godel parity research windows (ADR-107) ─────────────────────────
    /// DES command — comprehensive company overview.
    pub(crate) show_company_desc: bool,
    pub(crate) desc_symbol: String,
    pub(crate) desc_profile: Option<typhoon_engine::core::research::CompanyProfile>,
    pub(crate) desc_peers: Vec<String>,
    pub(crate) desc_earnings: Vec<typhoon_engine::core::research::EarningRow>,
    pub(crate) desc_press: Vec<typhoon_engine::core::research::PressRelease>,
    pub(crate) desc_loading: bool,

    /// IPO command — upcoming IPO calendar.
    pub(crate) show_ipo_calendar: bool,
    pub(crate) ipo_events: Vec<typhoon_engine::core::research::IpoEvent>,
    pub(crate) ipo_loading: bool,
    pub(crate) ipo_sort_col: usize,
    pub(crate) ipo_sort_asc: bool,

    /// EARNINGS command — historical actuals vs estimates.
    pub(crate) show_earnings_history: bool,
    pub(crate) earnings_history_symbol: String,
    pub(crate) earnings_history_rows: Vec<typhoon_engine::core::research::EarningRow>,
    pub(crate) earnings_history_loading: bool,
    pub(crate) earnings_history_sort_col: usize,
    pub(crate) earnings_history_sort_asc: bool,

    /// PEERS command — related tickers.
    pub(crate) show_peers: bool,
    pub(crate) peers_symbol: String,
    pub(crate) peers_list: Vec<String>,
    pub(crate) peers_loading: bool,

    /// PRESS command — company press releases.
    pub(crate) show_press_releases: bool,
    pub(crate) press_symbol: String,
    pub(crate) press_releases_list: Vec<typhoon_engine::core::research::PressRelease>,
    pub(crate) press_loading: bool,

    /// SENTIMENT command — Reddit + Twitter social sentiment.
    pub(crate) show_sentiment: bool,
    pub(crate) sentiment_symbol: String,
    pub(crate) sentiment_rows: Vec<typhoon_engine::core::research::SocialSentimentRow>,
    pub(crate) sentiment_loading: bool,
    pub(crate) sentiment_sort_col: usize,
    pub(crate) sentiment_sort_asc: bool,

    /// TRANSCRIPTS command — earnings call transcripts.
    pub(crate) show_transcripts: bool,
    pub(crate) transcripts_symbol: String,
    pub(crate) transcripts_list: Vec<typhoon_engine::core::research::TranscriptMeta>,
    pub(crate) transcripts_selected: Option<usize>,
    pub(crate) transcripts_body: Option<typhoon_engine::core::research::Transcript>,
    pub(crate) transcripts_loading_list: bool,
    pub(crate) transcripts_loading_body: bool,
    #[allow(dead_code)]
    pub(crate) transcripts_summary: Option<typhoon_engine::core::sec_filing::FilingSummary>,
    #[allow(dead_code)]
    pub(crate) transcripts_summary_for: (String, i32, i32),

    /// GLCO command — global commodities futures dashboard.
    pub(crate) show_commodities: bool,
    pub(crate) commodities_quotes: Vec<typhoon_engine::core::research::CommodityQuote>,
    pub(crate) commodities_last_fetch: Option<std::time::Instant>,
    pub(crate) commodities_loading: bool,

    /// TAS command — live Time & Sales tape for the active chart symbol.
    /// (symbol, price, size, side, timestamp) — most recent at front.
    pub(crate) show_tas: bool,
    pub(crate) tas_symbol: String,
    pub(crate) tas_rows: VecDeque<(String, f64, f64, String, String)>,
    pub(crate) tas_paused: bool,

    // ── Godel Parity Round 2 ──────────────────────────────────
    /// DVD — per-symbol dividend history.
    pub(crate) show_dividend_history: bool,
    pub(crate) dividend_history_symbol: String,
    pub(crate) dividend_history: Vec<typhoon_engine::core::research::DividendRecord>,
    pub(crate) dividend_history_loading: bool,

    /// EEB — forward earnings estimates.
    pub(crate) show_earnings_estimates: bool,
    pub(crate) earnings_estimates_symbol: String,
    pub(crate) earnings_estimates: Vec<typhoon_engine::core::research::EarningsEstimate>,
    pub(crate) earnings_estimates_loading: bool,

    /// UPDG — analyst rating change feed (upgrades/downgrades).
    pub(crate) show_rating_changes: bool,
    pub(crate) rating_changes_symbol: String,
    pub(crate) rating_changes: Vec<typhoon_engine::core::research::RatingChange>,
    pub(crate) rating_changes_loading: bool,

    /// GY — US Treasury yield curve snapshot.
    pub(crate) show_treasury_curve: bool,
    pub(crate) treasury_yields: Vec<typhoon_engine::core::research::TreasuryYield>,
    pub(crate) treasury_yields_last_fetch: Option<std::time::Instant>,
    pub(crate) treasury_yields_loading: bool,

    // ── Godel Parity Round 3 ──────────────────────────────────
    /// FA — full financial statements bundle (Income / Balance / Cash Flow).
    pub(crate) show_financials: bool,
    pub(crate) financials_symbol: String,
    pub(crate) financials: typhoon_engine::core::research::FinancialStatements,
    pub(crate) financials_loading: bool,
    pub(crate) financials_view: FinancialsView,
    pub(crate) financials_period: FinancialsPeriod,

    /// MGMT — company officers + compensation.
    pub(crate) show_executives: bool,
    pub(crate) executives_symbol: String,
    pub(crate) executives: Vec<typhoon_engine::core::research::Executive>,
    pub(crate) executives_loading: bool,

    /// COT — CFTC Commitments of Traders (weekly, global).
    pub(crate) show_cot: bool,
    pub(crate) cot_reports: Vec<typhoon_engine::core::research::CotReport>,
    pub(crate) cot_loading: bool,
    pub(crate) cot_last_fetch: Option<std::time::Instant>,
    pub(crate) cot_filter: String,

    // ── Godel Parity Round 4 ──────────────────────────────────
    /// SPLT — historical stock split events.
    pub(crate) show_splits: bool,
    pub(crate) splits_symbol: String,
    pub(crate) splits_list: Vec<typhoon_engine::core::research::StockSplit>,
    pub(crate) splits_loading: bool,

    /// ETF — exchange-traded fund holdings (constituents).
    pub(crate) show_etf_holdings: bool,
    pub(crate) etf_symbol: String,
    pub(crate) etf_holdings: Vec<typhoon_engine::core::research::EtfHolding>,
    pub(crate) etf_loading: bool,

    /// ANR — analyst recommendation buckets + consensus price target.
    pub(crate) show_analyst_recs: bool,
    pub(crate) anr_symbol: String,
    pub(crate) analyst_recs: Vec<typhoon_engine::core::research::AnalystRecommendation>,
    pub(crate) price_target: typhoon_engine::core::research::PriceTarget,
    pub(crate) anr_loading: bool,

    /// ESG — environmental / social / governance scores by year.
    pub(crate) show_esg: bool,
    pub(crate) esg_symbol: String,
    pub(crate) esg_rows: Vec<typhoon_engine::core::research::EsgScore>,
    pub(crate) esg_loading: bool,

    /// MEMB — equity index constituents (global, cached by index code).
    pub(crate) show_index_members: bool,
    pub(crate) index_code: String,
    pub(crate) index_members: Vec<typhoon_engine::core::research::IndexMember>,
    pub(crate) memb_loading: bool,
    pub(crate) memb_filter: String,

    // ── Godel Parity Round 5 ──────────────────────────────────
    /// INS — SEC Form-4 insider trades.
    pub(crate) show_insider_trades: bool,
    pub(crate) insider_symbol: String,
    pub(crate) insider_trades: Vec<typhoon_engine::core::research::InsiderTrade>,
    pub(crate) insider_loading: bool,

    /// HDS — 13F-derived institutional holders.
    pub(crate) show_inst_holders: bool,
    pub(crate) inst_holders_symbol: String,
    pub(crate) institutional_holders: Vec<typhoon_engine::core::research::InstitutionalHolder>,
    pub(crate) inst_holders_loading: bool,

    /// FLOAT — shares float + outstanding snapshot.
    pub(crate) show_shares_float: bool,
    pub(crate) float_symbol: String,
    pub(crate) shares_float: typhoon_engine::core::research::SharesFloat,
    pub(crate) float_loading: bool,

    /// HP — historical price table (daily OHLCV).
    pub(crate) show_hist_price: bool,
    pub(crate) hp_symbol: String,
    pub(crate) hp_rows: Vec<typhoon_engine::core::research::HistoricalPriceRow>,
    pub(crate) hp_loading: bool,
    pub(crate) hp_limit: usize,

    /// EPS — quarterly earnings surprise history.
    pub(crate) show_eps_surprise: bool,
    pub(crate) eps_symbol: String,
    pub(crate) eps_surprises: Vec<typhoon_engine::core::research::EarningsSurprise>,
    pub(crate) eps_loading: bool,

    // ── Godel Parity Round 6 ──────────────────────────────────
    /// WEI — world equity indices dashboard (Yahoo index tickers, separate
    /// from the legacy ETF-based "World Indices" dashboard above).
    pub(crate) show_wei: bool,
    pub(crate) wei_indices: Vec<typhoon_engine::core::research::WorldIndex>,
    pub(crate) wei_loading: bool,
    pub(crate) wei_region_filter: String, // "" | "Americas" | "EMEA" | "Asia-Pacific"

    /// MOV — market movers (gainers / losers / actives).
    pub(crate) show_market_movers: bool,
    pub(crate) market_movers: typhoon_engine::core::research::MarketMovers,
    pub(crate) mov_loading: bool,

    /// INDU — sector performance snapshot.
    pub(crate) show_sector_perf: bool,
    pub(crate) sector_perf: Vec<typhoon_engine::core::research::SectorPerformance>,
    pub(crate) indu_loading: bool,

    /// CACS — corporate-actions calendar aggregator (UI-only, reuses cached
    /// splits / dividends / earnings / IPO data).
    pub(crate) show_cacs: bool,
    pub(crate) cacs_symbol: String,

    /// WACC — derived cost-of-capital snapshot (per symbol).
    pub(crate) show_wacc: bool,
    pub(crate) wacc_symbol: String,
    pub(crate) wacc_snapshot: typhoon_engine::core::research::WaccSnapshot,
    pub(crate) wacc_loading: bool,

    // ── Godel Parity Round 7 ──────────────────────────────────
    /// WCR — world currency rates (FX majors + crosses + EM), Yahoo-sourced
    /// single-row snapshot. Separate state from the legacy FOREX_MATRIX
    /// dashboard which is broker-sourced.
    pub(crate) show_wcr: bool,
    pub(crate) wcr_rates: Vec<typhoon_engine::core::research::CurrencyRate>,
    pub(crate) wcr_loading: bool,
    pub(crate) wcr_region_filter: String, // "" | "Majors" | "Crosses" | "EM"

    /// BETA — rolling beta history vs SPY (1Y/3Y/5Y windows).
    pub(crate) show_beta: bool,
    pub(crate) beta_symbol: String,
    pub(crate) beta_snapshot: typhoon_engine::core::research::BetaSnapshot,
    pub(crate) beta_loading: bool,

    /// DDM — Gordon Growth dividend-discount-model snapshot.
    pub(crate) show_ddm: bool,
    pub(crate) ddm_symbol: String,
    pub(crate) ddm_snapshot: typhoon_engine::core::research::DdmSnapshot,
    pub(crate) ddm_loading: bool,

    /// RV — relative valuation peer matrix (zero-fetch, pure compute).
    pub(crate) show_rv: bool,
    pub(crate) rv_symbol: String,
    pub(crate) rv_snapshot: typhoon_engine::core::research::RelativeValuation,
    pub(crate) rv_loading: bool,

    /// FIGI — OpenFIGI identifier mapping.
    pub(crate) show_figi: bool,
    pub(crate) figi_symbol: String,
    pub(crate) figi_snapshot: typhoon_engine::core::research::FigiSnapshot,
    pub(crate) figi_loading: bool,

    // ── Godel Parity Round 8 ──────────────────────────────────
    /// HRA — historical return / risk analysis (vol, Sharpe, Sortino, drawdowns).
    pub(crate) show_hra: bool,
    pub(crate) hra_symbol: String,
    pub(crate) hra_snapshot: typhoon_engine::core::research::HraSnapshot,
    pub(crate) hra_loading: bool,

    /// DCF — discounted cash flow fair value (FCFF model).
    pub(crate) show_dcf: bool,
    pub(crate) dcf_symbol: String,
    pub(crate) dcf_snapshot: typhoon_engine::core::research::DcfSnapshot,
    pub(crate) dcf_growth_pct: f64,
    pub(crate) dcf_terminal_growth_pct: f64,
    pub(crate) dcf_projection_years: usize,
    pub(crate) dcf_loading: bool,

    /// SVM — stock valuation model synthesis (DDM + DCF + peer multiples).
    pub(crate) show_svm: bool,
    pub(crate) svm_symbol: String,
    pub(crate) svm_snapshot: typhoon_engine::core::research::SvmSnapshot,
    pub(crate) svm_loading: bool,

    /// OMON — Yahoo options chain monitor.
    pub(crate) show_omon: bool,
    pub(crate) omon_symbol: String,
    pub(crate) omon_snapshot: typhoon_engine::core::research::OptionsChainSnapshot,
    pub(crate) omon_loading: bool,

    /// IVOL — implied-vol rank / percentile from cached OMON history.
    pub(crate) show_ivol: bool,
    pub(crate) ivol_symbol: String,
    pub(crate) ivol_snapshot: typhoon_engine::core::research::IvolSnapshot,
    pub(crate) ivol_loading: bool,

    // ── Godel Parity Round 9 ──────────────────────────────────
    /// SEAG — monthly + day-of-week seasonality over cached HP.
    pub(crate) show_seag: bool,
    pub(crate) seag_symbol: String,
    pub(crate) seag_snapshot: typhoon_engine::core::research::SeasonalitySnapshot,
    pub(crate) seag_loading: bool,

    /// COR — correlation matrix vs peers.
    pub(crate) show_cor: bool,
    pub(crate) cor_symbol: String,
    pub(crate) cor_snapshot: typhoon_engine::core::research::CorrelationMatrix,
    pub(crate) cor_window_days: usize,
    pub(crate) cor_loading: bool,

    /// TRA — total return (price + dividends) snapshot.
    pub(crate) show_tra: bool,
    pub(crate) tra_symbol: String,
    pub(crate) tra_snapshot: typhoon_engine::core::research::TotalReturnSnapshot,
    pub(crate) tra_loading: bool,

    /// TECH — technical-indicator dashboard.
    pub(crate) show_tech: bool,
    pub(crate) tech_symbol: String,
    pub(crate) tech_snapshot: typhoon_engine::core::research::TechnicalSnapshot,
    pub(crate) tech_loading: bool,

    /// SKEW — volatility-skew/smile over cached OMON.
    pub(crate) show_skew: bool,
    pub(crate) skew_symbol: String,
    pub(crate) skew_snapshot: typhoon_engine::core::research::VolatilitySkew,
    pub(crate) skew_loading: bool,

    // ── Godel Parity Round 10 ──
    /// LEV — debt leverage & coverage ratios from cached Financials + Fundamentals.
    pub(crate) show_lev: bool,
    pub(crate) lev_symbol: String,
    pub(crate) lev_snapshot: typhoon_engine::core::research::LeverageSnapshot,
    pub(crate) lev_loading: bool,

    /// ACRL — earnings quality (NI vs FCF) from cached quarterly Financials.
    pub(crate) show_acrl: bool,
    pub(crate) acrl_symbol: String,
    pub(crate) acrl_snapshot: typhoon_engine::core::research::AccrualsSnapshot,
    pub(crate) acrl_loading: bool,

    /// RVOL — realized volatility cone from cached HP bars.
    pub(crate) show_rvol: bool,
    pub(crate) rvol_symbol: String,
    pub(crate) rvol_snapshot: typhoon_engine::core::research::RealizedVolSnapshot,
    pub(crate) rvol_loading: bool,

    /// FCFY — FCF yield, payout ratios, dividend sustainability from cached Financials.
    pub(crate) show_fcfy: bool,
    pub(crate) fcfy_symbol: String,
    pub(crate) fcfy_snapshot: typhoon_engine::core::research::FcfYieldSnapshot,
    pub(crate) fcfy_loading: bool,

    /// SHRT — short interest / days-to-cover from cached SharesFloat + HP bars.
    pub(crate) show_shrt: bool,
    pub(crate) shrt_symbol: String,
    pub(crate) shrt_snapshot: typhoon_engine::core::research::ShortInterestSnapshot,
    pub(crate) shrt_loading: bool,

    // ── Godel Parity Round 11 ──
    /// ALTZ — classic Altman Z-score from cached Financials + Fundamentals.
    pub(crate) show_altz: bool,
    pub(crate) altz_symbol: String,
    pub(crate) altz_snapshot: typhoon_engine::core::research::AltmanZSnapshot,
    pub(crate) altz_loading: bool,

    /// PTFS — Piotroski F-score over 2 annual periods from cached Financials.
    pub(crate) show_ptfs: bool,
    pub(crate) ptfs_symbol: String,
    pub(crate) ptfs_snapshot: typhoon_engine::core::research::PiotroskiSnapshot,
    pub(crate) ptfs_loading: bool,

    /// VOLE — OHLC volatility estimators (CtC / Parkinson / GK / RS / YZ) from cached HP bars.
    pub(crate) show_vole: bool,
    pub(crate) vole_symbol: String,
    pub(crate) vole_snapshot: typhoon_engine::core::research::OhlcVolSnapshot,
    pub(crate) vole_loading: bool,

    /// EPSB — EPS beat streak & surprise analysis from cached earnings surprise history.
    pub(crate) show_epsb: bool,
    pub(crate) epsb_symbol: String,
    pub(crate) epsb_snapshot: typhoon_engine::core::research::EpsBeatSnapshot,
    pub(crate) epsb_loading: bool,

    /// PTD — Price target dispersion & implied return from cached PriceTarget + current price.
    pub(crate) show_ptd: bool,
    pub(crate) ptd_symbol: String,
    pub(crate) ptd_snapshot: typhoon_engine::core::research::PriceTargetDispersion,
    pub(crate) ptd_loading: bool,

    // ── Godel Parity Round 12 ──
    /// MNGR — Insider activity bias over cached INS form-4 trades in a lookback window.
    pub(crate) show_mngr: bool,
    pub(crate) mngr_symbol: String,
    pub(crate) mngr_window_days: i32,
    pub(crate) mngr_snapshot: typhoon_engine::core::research::InsiderActivitySnapshot,
    pub(crate) mngr_loading: bool,

    /// DIVG — Dividend growth analysis (CAGR, consistency) from cached DVD history.
    pub(crate) show_divg: bool,
    pub(crate) divg_symbol: String,
    pub(crate) divg_snapshot: typhoon_engine::core::research::DivgSnapshot,
    pub(crate) divg_loading: bool,

    /// EARM — Earnings momentum trend from cached FA + EPS surprises.
    pub(crate) show_earm: bool,
    pub(crate) earm_symbol: String,
    pub(crate) earm_snapshot: typhoon_engine::core::research::EarmSnapshot,
    pub(crate) earm_loading: bool,

    /// SECTR — Sector rotation strength from cached INDU + Fundamentals.sector.
    pub(crate) show_sectr: bool,
    pub(crate) sectr_symbol: String,
    pub(crate) sectr_snapshot: typhoon_engine::core::research::SectorRotationSnapshot,
    pub(crate) sectr_loading: bool,

    /// UPDM — Upgrade/downgrade momentum from cached UPDG rating changes.
    pub(crate) show_updm: bool,
    pub(crate) updm_symbol: String,
    pub(crate) updm_snapshot: typhoon_engine::core::research::UpdmSnapshot,
    pub(crate) updm_loading: bool,

    // ── Godel Parity Round 13 ──
    /// MOM — 12-1 month momentum score from cached HP bars.
    pub(crate) show_mom: bool,
    pub(crate) mom_symbol: String,
    pub(crate) mom_snapshot: typhoon_engine::core::research::MomentumSnapshot,
    pub(crate) mom_loading: bool,

    /// LIQ — Liquidity profile from cached HP bars + Fundamentals.
    pub(crate) show_liq: bool,
    pub(crate) liq_symbol: String,
    pub(crate) liq_window_days: i32,
    pub(crate) liq_snapshot: typhoon_engine::core::research::LiquiditySnapshot,
    pub(crate) liq_loading: bool,

    /// BREAK — Breakout proximity from cached HP bars.
    pub(crate) show_break: bool,
    pub(crate) break_symbol: String,
    pub(crate) break_snapshot: typhoon_engine::core::research::BreakoutSnapshot,
    pub(crate) break_loading: bool,

    /// CCRL — Cash conversion cycle from cached FA statements.
    pub(crate) show_ccrl: bool,
    pub(crate) ccrl_symbol: String,
    pub(crate) ccrl_snapshot: typhoon_engine::core::research::CashCycleSnapshot,
    pub(crate) ccrl_loading: bool,

    /// CREDIT — Unified credit score fusing cached ALTZ + PTFS + LEV + ACRL.
    pub(crate) show_credit: bool,
    pub(crate) credit_symbol: String,
    pub(crate) credit_snapshot: typhoon_engine::core::research::CreditSnapshot,
    pub(crate) credit_loading: bool,

    // ── Godel Parity Round 14 ──
    /// GROWM — GARP composite fusing cached MOM + EARM + DIVG.
    pub(crate) show_growm: bool,
    pub(crate) growm_symbol: String,
    pub(crate) growm_snapshot: typhoon_engine::core::research::GrowmSnapshot,
    pub(crate) growm_loading: bool,

    /// FLOW — Insider + institutional flow score.
    pub(crate) show_flow: bool,
    pub(crate) flow_symbol: String,
    pub(crate) flow_window_days: i32,
    pub(crate) flow_snapshot: typhoon_engine::core::research::FlowSnapshot,
    pub(crate) flow_loading: bool,

    /// REGIME — Market regime classifier fusing VOLE + TECH + HRA.
    pub(crate) show_regime: bool,
    pub(crate) regime_symbol: String,
    pub(crate) regime_snapshot: typhoon_engine::core::research::RegimeSnapshot,
    pub(crate) regime_loading: bool,

    /// RELVOL — Relative volume vs trailing averages.
    pub(crate) show_relvol: bool,
    pub(crate) relvol_symbol: String,
    pub(crate) relvol_snapshot: typhoon_engine::core::research::RelVolSnapshot,
    pub(crate) relvol_loading: bool,

    /// MARGINS — Margin trajectory (gross/op/net) over cached FA statements.
    pub(crate) show_margins: bool,
    pub(crate) margins_symbol: String,
    pub(crate) margins_snapshot: typhoon_engine::core::research::MarginsSnapshot,
    pub(crate) margins_loading: bool,

    // ── Godel Parity Round 15 ─────────────────────────────────
    /// VAL — Value-factor composite vs sector peers.
    pub(crate) show_val: bool,
    pub(crate) val_symbol: String,
    pub(crate) val_snapshot: typhoon_engine::core::research::ValueSnapshot,
    pub(crate) val_loading: bool,

    /// QUAL — Quality-factor composite fusing PTFS + MARGINS + ACRL + LEV.
    pub(crate) show_qual: bool,
    pub(crate) qual_symbol: String,
    pub(crate) qual_snapshot: typhoon_engine::core::research::QualitySnapshot,
    pub(crate) qual_loading: bool,

    /// RISK — Risk-factor composite fusing VOLE + BETA + LIQ + SHRT + ALTZ.
    pub(crate) show_risk: bool,
    pub(crate) risk_symbol: String,
    pub(crate) risk_snapshot: typhoon_engine::core::research::RiskSnapshot,
    pub(crate) risk_loading: bool,

    /// INSSTRK — Insider streak detector from cached Form 4 trades.
    pub(crate) show_insstrk: bool,
    pub(crate) insstrk_symbol: String,
    pub(crate) insstrk_window_days: i32,
    pub(crate) insstrk_snapshot: typhoon_engine::core::research::InsiderStreakSnapshot,
    pub(crate) insstrk_loading: bool,

    /// COVG — Analyst coverage breadth + churn snapshot.
    pub(crate) show_covg: bool,
    pub(crate) covg_symbol: String,
    pub(crate) covg_snapshot: typhoon_engine::core::research::CoverageSnapshot,
    pub(crate) covg_loading: bool,

    // ── Godel Parity Round 16 ─────────────────────────────────
    /// VRK — Value Rank vs sector peers.
    pub(crate) show_vrk: bool,
    pub(crate) vrk_symbol: String,
    pub(crate) vrk_snapshot: typhoon_engine::core::research::ValueRankSnapshot,
    pub(crate) vrk_loading: bool,

    /// QRK — Quality Rank vs sector peers.
    pub(crate) show_qrk: bool,
    pub(crate) qrk_symbol: String,
    pub(crate) qrk_snapshot: typhoon_engine::core::research::QualityRankSnapshot,
    pub(crate) qrk_loading: bool,

    /// RRK — Risk Rank vs sector peers (inverted — higher rank = safer).
    pub(crate) show_rrk: bool,
    pub(crate) rrk_symbol: String,
    pub(crate) rrk_snapshot: typhoon_engine::core::research::RiskRankSnapshot,
    pub(crate) rrk_loading: bool,

    /// RELEPSGR — Relative 3y EPS CAGR vs sector median.
    pub(crate) show_relepsgr: bool,
    pub(crate) relepsgr_symbol: String,
    pub(crate) relepsgr_snapshot: typhoon_engine::core::research::RelativeEpsGrowthSnapshot,
    pub(crate) relepsgr_loading: bool,

    /// PEAD — Post-Earnings-Announcement Drift.
    pub(crate) show_pead: bool,
    pub(crate) pead_symbol: String,
    pub(crate) pead_snapshot: typhoon_engine::core::research::PeadSnapshot,
    pub(crate) pead_loading: bool,

    // ── Godel Parity Round 17 ─────────────────────────────────
    /// SIZEF — Size factor rank vs sector peers.
    pub(crate) show_sizef: bool,
    pub(crate) sizef_symbol: String,
    pub(crate) sizef_snapshot: typhoon_engine::core::research::SizeFactorSnapshot,
    pub(crate) sizef_loading: bool,

    /// MOMF — Momentum factor rank vs sector peers.
    pub(crate) show_momf: bool,
    pub(crate) momf_symbol: String,
    pub(crate) momf_snapshot: typhoon_engine::core::research::MomentumRankSnapshot,
    pub(crate) momf_loading: bool,

    /// PEADRANK — PEAD drift rank vs sector peers.
    pub(crate) show_peadrank: bool,
    pub(crate) peadrank_symbol: String,
    pub(crate) peadrank_snapshot: typhoon_engine::core::research::PeadRankSnapshot,
    pub(crate) peadrank_loading: bool,

    /// FQM — Fundamental Quality Meter (Piotroski + margins + accruals).
    pub(crate) show_fqm: bool,
    pub(crate) fqm_symbol: String,
    pub(crate) fqm_snapshot: typhoon_engine::core::research::FundamentalQualityMeterSnapshot,
    pub(crate) fqm_loading: bool,

    /// REVRANK — Relative 3y revenue CAGR vs sector median.
    pub(crate) show_revrank: bool,
    pub(crate) revrank_symbol: String,
    pub(crate) revrank_snapshot: typhoon_engine::core::research::RevenueGrowthRankSnapshot,
    pub(crate) revrank_loading: bool,

    // ── Godel Parity Round 18 ─────────────────────────────────
    /// LEVRANK — Leverage rank vs sector peers (D/E percentile, inverted).
    pub(crate) show_levrank: bool,
    pub(crate) levrank_symbol: String,
    pub(crate) levrank_snapshot: typhoon_engine::core::research::LeverageRankSnapshot,
    pub(crate) levrank_loading: bool,

    /// OPERANK — Operating Quality rank vs sector peers.
    pub(crate) show_operank: bool,
    pub(crate) operank_symbol: String,
    pub(crate) operank_snapshot: typhoon_engine::core::research::OperatingQualityRankSnapshot,
    pub(crate) operank_loading: bool,

    /// FQMRANK — Fundamental Quality Meter rank vs sector peers.
    pub(crate) show_fqmrank: bool,
    pub(crate) fqmrank_symbol: String,
    pub(crate) fqmrank_snapshot: typhoon_engine::core::research::FqmRankSnapshot,
    pub(crate) fqmrank_loading: bool,

    /// LIQRANK — Liquidity rank vs sector peers.
    pub(crate) show_liqrank: bool,
    pub(crate) liqrank_symbol: String,
    pub(crate) liqrank_snapshot: typhoon_engine::core::research::LiquidityRankSnapshot,
    pub(crate) liqrank_loading: bool,

    /// SURPSTK — Earnings surprise streak stat.
    pub(crate) show_surpstk: bool,
    pub(crate) surpstk_symbol: String,
    pub(crate) surpstk_snapshot: typhoon_engine::core::research::EarningsSurpriseStreakSnapshot,
    pub(crate) surpstk_loading: bool,

    /// DVDRANK — Dividend growth rank vs sector peers.
    pub(crate) show_dvdrank: bool,
    pub(crate) dvdrank_symbol: String,
    pub(crate) dvdrank_snapshot: typhoon_engine::core::research::DividendGrowthRankSnapshot,
    pub(crate) dvdrank_loading: bool,

    /// EARMRANK — Earnings momentum rank vs sector peers.
    pub(crate) show_earmrank: bool,
    pub(crate) earmrank_symbol: String,
    pub(crate) earmrank_snapshot: typhoon_engine::core::research::EarningsMomentumRankSnapshot,
    pub(crate) earmrank_loading: bool,

    /// UPDGRANK — Upgrade/downgrade rank vs sector peers.
    pub(crate) show_updgrank: bool,
    pub(crate) updgrank_symbol: String,
    pub(crate) updgrank_snapshot: typhoon_engine::core::research::UpgradeDowngradeRankSnapshot,
    pub(crate) updgrank_loading: bool,

    /// GY — Gap yearly stat.
    pub(crate) show_gy: bool,
    pub(crate) gy_symbol: String,
    pub(crate) gy_snapshot: typhoon_engine::core::research::GapYearlySnapshot,
    pub(crate) gy_loading: bool,

    /// DES — Daily event streak stat.
    pub(crate) show_des: bool,
    pub(crate) des_symbol: String,
    pub(crate) des_snapshot: typhoon_engine::core::research::DailyEventStreakSnapshot,
    pub(crate) des_loading: bool,

    /// DVDYIELDRANK — Dividend yield rank vs sector peers.
    pub(crate) show_dvdyieldrank: bool,
    pub(crate) dvdyieldrank_symbol: String,
    pub(crate) dvdyieldrank_snapshot: typhoon_engine::core::research::DividendYieldRankSnapshot,
    pub(crate) dvdyieldrank_loading: bool,

    /// SHRANK — Short interest rank vs sector peers (risk-inverted).
    pub(crate) show_shrank: bool,
    pub(crate) shrank_symbol: String,
    pub(crate) shrank_snapshot: typhoon_engine::core::research::ShortInterestRankSnapshot,
    pub(crate) shrank_loading: bool,

    /// SHORTRANK_DELTA — short-interest trend rank vs sector peers.
    pub(crate) show_shortrank_delta: bool,
    pub(crate) shortrank_delta_symbol: String,
    pub(crate) shortrank_delta_snapshot:
        typhoon_engine::core::research::ShortInterestDeltaRankSnapshot,
    pub(crate) shortrank_delta_loading: bool,

    /// INSIDERCONC — insider ownership concentration vs sector peers.
    pub(crate) show_insiderconc: bool,
    pub(crate) insiderconc_symbol: String,
    pub(crate) insiderconc_snapshot: typhoon_engine::core::research::InsiderConcentrationSnapshot,
    pub(crate) insiderconc_loading: bool,

    /// ATRANN — Annualized ATR volatility regime.
    pub(crate) show_atrann: bool,
    pub(crate) atrann_symbol: String,
    pub(crate) atrann_snapshot: typhoon_engine::core::research::AnnualizedAtrSnapshot,
    pub(crate) atrann_loading: bool,

    /// DDHIST — Drawdown history.
    pub(crate) show_ddhist: bool,
    pub(crate) ddhist_symbol: String,
    pub(crate) ddhist_snapshot: typhoon_engine::core::research::DrawdownHistorySnapshot,
    pub(crate) ddhist_loading: bool,

    /// PRICEPERF — Multi-horizon price performance.
    pub(crate) show_priceperf: bool,
    pub(crate) priceperf_symbol: String,
    pub(crate) priceperf_snapshot: typhoon_engine::core::research::PricePerformanceSnapshot,
    pub(crate) priceperf_loading: bool,

    /// MOMRANK_MULTI — sector-relative rank of PRICEPERF horizons.
    pub(crate) show_momrank_multi: bool,
    pub(crate) momrank_multi_symbol: String,
    pub(crate) momrank_multi_snapshot: typhoon_engine::core::research::MomentumRankMultiSnapshot,
    pub(crate) momrank_multi_loading: bool,

    /// BETARANK — Beta rank vs sector peers (risk-inverted).
    pub(crate) show_betarank: bool,
    pub(crate) betarank_symbol: String,
    pub(crate) betarank_snapshot: typhoon_engine::core::research::BetaRankSnapshot,
    pub(crate) betarank_loading: bool,

    /// PEGRANK — PEG ratio rank vs sector peers.
    pub(crate) show_pegrank: bool,
    pub(crate) pegrank_symbol: String,
    pub(crate) pegrank_snapshot: typhoon_engine::core::research::PegRankSnapshot,
    pub(crate) pegrank_loading: bool,

    /// FHIGHLOW — 52-week high/low distance + proximity band.
    pub(crate) show_fhighlow: bool,
    pub(crate) fhighlow_symbol: String,
    pub(crate) fhighlow_snapshot: typhoon_engine::core::research::FiftyTwoWeekHighLowSnapshot,
    pub(crate) fhighlow_loading: bool,

    /// RVCONE — Realized volatility cone (multi-horizon).
    pub(crate) show_rvcone: bool,
    pub(crate) rvcone_symbol: String,
    pub(crate) rvcone_snapshot: typhoon_engine::core::research::RealizedVolConeSnapshot,
    pub(crate) rvcone_loading: bool,

    /// CALPB — Calendar period breakdowns.
    pub(crate) show_calpb: bool,
    pub(crate) calpb_symbol: String,
    pub(crate) calpb_snapshot: typhoon_engine::core::research::CalendarPeriodBreakdownSnapshot,
    pub(crate) calpb_loading: bool,

    /// CORRSTK — rolling correlation vs SPY / sector ETF benchmark.
    pub(crate) show_corrstk: bool,
    pub(crate) corrstk_symbol: String,
    pub(crate) corrstk_snapshot: typhoon_engine::core::research::CorrStkSnapshot,
    pub(crate) corrstk_loading: bool,

    /// TLRANK — trailing 30d liquidity rank vs sector peers.
    pub(crate) show_tlrank: bool,
    pub(crate) tlrank_symbol: String,
    pub(crate) tlrank_snapshot: typhoon_engine::core::research::ThirtyDayLiquidityRankSnapshot,
    pub(crate) tlrank_loading: bool,

    /// CORRRANK — benchmark-linkage rank vs sector peers.
    pub(crate) show_corrrank: bool,
    pub(crate) corrrank_symbol: String,
    pub(crate) corrrank_snapshot: typhoon_engine::core::research::CorrelationRankSnapshot,
    pub(crate) corrrank_loading: bool,

    /// OPERANK_DELTA — operating-margin trend rank vs sector peers.
    pub(crate) show_operank_delta: bool,
    pub(crate) operank_delta_symbol: String,
    pub(crate) operank_delta_snapshot:
        typhoon_engine::core::research::OperatingMarginDeltaRankSnapshot,
    pub(crate) operank_delta_loading: bool,

    /// DIVACC — dividend growth acceleration.
    pub(crate) show_divacc: bool,
    pub(crate) divacc_symbol: String,
    pub(crate) divacc_snapshot: typhoon_engine::core::research::DividendAccelerationSnapshot,
    pub(crate) divacc_loading: bool,

    /// EPSACC — EPS acceleration from cached quarterly financials.
    pub(crate) show_epsacc: bool,
    pub(crate) epsacc_symbol: String,
    pub(crate) epsacc_snapshot: typhoon_engine::core::research::EpsAccelerationSnapshot,
    pub(crate) epsacc_loading: bool,

    /// VRP — implied-vs-realized vol premium.
    pub(crate) show_vrp: bool,
    pub(crate) vrp_symbol: String,
    pub(crate) vrp_snapshot: typhoon_engine::core::research::VolRiskPremiumSnapshot,
    pub(crate) vrp_loading: bool,

    /// RETSKEW — Return distribution skewness.
    pub(crate) show_retskew: bool,
    pub(crate) retskew_symbol: String,
    pub(crate) retskew_snapshot: typhoon_engine::core::research::ReturnSkewnessSnapshot,
    pub(crate) retskew_loading: bool,

    /// RETKURT — Return distribution excess kurtosis.
    pub(crate) show_retkurt: bool,
    pub(crate) retkurt_symbol: String,
    pub(crate) retkurt_snapshot: typhoon_engine::core::research::ReturnKurtosisSnapshot,
    pub(crate) retkurt_loading: bool,

    /// TAILR — Tail ratio.
    pub(crate) show_tailr: bool,
    pub(crate) tailr_symbol: String,
    pub(crate) tailr_snapshot: typhoon_engine::core::research::TailRatioSnapshot,
    pub(crate) tailr_loading: bool,

    /// RUNLEN — Up/down day run length stats.
    pub(crate) show_runlen: bool,
    pub(crate) runlen_symbol: String,
    pub(crate) runlen_snapshot: typhoon_engine::core::research::RunLengthSnapshot,
    pub(crate) runlen_loading: bool,

    /// DAYRANGE — Daily range analysis.
    pub(crate) show_dayrange: bool,
    pub(crate) dayrange_symbol: String,
    pub(crate) dayrange_snapshot: typhoon_engine::core::research::DailyRangeSnapshot,
    pub(crate) dayrange_loading: bool,

    // ── Round 23 ──
    /// AUTOCOR — Autocorrelation at multiple lags.
    pub(crate) show_autocor: bool,
    pub(crate) autocor_symbol: String,
    pub(crate) autocor_snapshot: typhoon_engine::core::research::AutocorrelationSnapshot,
    pub(crate) autocor_loading: bool,

    /// HURST — Hurst exponent via R/S analysis.
    pub(crate) show_hurst: bool,
    pub(crate) hurst_symbol: String,
    pub(crate) hurst_snapshot: typhoon_engine::core::research::HurstSnapshot,
    pub(crate) hurst_loading: bool,

    /// HITRATE — Multi-horizon hit rate.
    pub(crate) show_hitrate: bool,
    pub(crate) hitrate_symbol: String,
    pub(crate) hitrate_snapshot: typhoon_engine::core::research::HitRateSnapshot,
    pub(crate) hitrate_loading: bool,

    /// GLASYM — Gain/loss asymmetry.
    pub(crate) show_glasym: bool,
    pub(crate) glasym_symbol: String,
    pub(crate) glasym_snapshot: typhoon_engine::core::research::GainLossAsymmetrySnapshot,
    pub(crate) glasym_loading: bool,

    /// VOLRATIO — Up vs down volume ratio.
    pub(crate) show_volratio: bool,
    pub(crate) volratio_symbol: String,
    pub(crate) volratio_snapshot: typhoon_engine::core::research::VolumeRatioSnapshot,
    pub(crate) volratio_loading: bool,

    // ── Round 24 ──
    /// DRAWUP — Upside rally history (mirror of DDHIST).
    pub(crate) show_drawup: bool,
    pub(crate) drawup_symbol: String,
    pub(crate) drawup_snapshot: typhoon_engine::core::research::DrawupHistorySnapshot,
    pub(crate) drawup_loading: bool,

    /// GAPSTATS — Overnight gap statistics.
    pub(crate) show_gapstats: bool,
    pub(crate) gapstats_symbol: String,
    pub(crate) gapstats_snapshot: typhoon_engine::core::research::GapStatsSnapshot,
    pub(crate) gapstats_loading: bool,

    /// VOLCLUSTER — Volatility clustering autocorrelation.
    pub(crate) show_volcluster: bool,
    pub(crate) volcluster_symbol: String,
    pub(crate) volcluster_snapshot: typhoon_engine::core::research::VolClusterSnapshot,
    pub(crate) volcluster_loading: bool,

    /// CLOSEPLC — Close placement within daily range.
    pub(crate) show_closeplc: bool,
    pub(crate) closeplc_symbol: String,
    pub(crate) closeplc_snapshot: typhoon_engine::core::research::ClosePlacementSnapshot,
    pub(crate) closeplc_loading: bool,

    /// MRHL — AR(1) mean-reversion half-life.
    pub(crate) show_mrhl: bool,
    pub(crate) mrhl_symbol: String,
    pub(crate) mrhl_snapshot: typhoon_engine::core::research::MeanReversionHalfLifeSnapshot,
    pub(crate) mrhl_loading: bool,

    // ── Round 25 ──
    /// DOWNVOL — Downside deviation + Sortino ratio.
    pub(crate) show_downvol: bool,
    pub(crate) downvol_symbol: String,
    pub(crate) downvol_snapshot: typhoon_engine::core::research::DownsideVolSnapshot,
    pub(crate) downvol_loading: bool,

    /// SHARPR — Sharpe ratio snapshot.
    pub(crate) show_sharpr: bool,
    pub(crate) sharpr_symbol: String,
    pub(crate) sharpr_snapshot: typhoon_engine::core::research::SharpeRatioSnapshot,
    pub(crate) sharpr_loading: bool,

    /// EFFRATIO — Kaufman's efficiency ratio.
    pub(crate) show_effratio: bool,
    pub(crate) effratio_symbol: String,
    pub(crate) effratio_snapshot: typhoon_engine::core::research::EfficiencyRatioSnapshot,
    pub(crate) effratio_loading: bool,

    /// WICKBIAS — Upper vs lower wick asymmetry.
    pub(crate) show_wickbias: bool,
    pub(crate) wickbias_symbol: String,
    pub(crate) wickbias_snapshot: typhoon_engine::core::research::WickBiasSnapshot,
    pub(crate) wickbias_loading: bool,

    /// VOLOFVOL — Stdev of rolling 20d realized vol.
    pub(crate) show_volofvol: bool,
    pub(crate) volofvol_symbol: String,
    pub(crate) volofvol_snapshot: typhoon_engine::core::research::VolOfVolSnapshot,
    pub(crate) volofvol_loading: bool,

    // ── Round 26 ──
    pub(crate) show_calmar: bool,
    pub(crate) calmar_symbol: String,
    pub(crate) calmar_snapshot: typhoon_engine::core::research::CalmarRatioSnapshot,
    pub(crate) calmar_loading: bool,
    pub(crate) show_ulcer: bool,
    pub(crate) ulcer_symbol: String,
    pub(crate) ulcer_snapshot: typhoon_engine::core::research::UlcerIndexSnapshot,
    pub(crate) ulcer_loading: bool,
    pub(crate) show_varratio: bool,
    pub(crate) varratio_symbol: String,
    pub(crate) varratio_snapshot: typhoon_engine::core::research::VarianceRatioSnapshot,
    pub(crate) varratio_loading: bool,
    pub(crate) show_amihud: bool,
    pub(crate) amihud_symbol: String,
    pub(crate) amihud_snapshot: typhoon_engine::core::research::AmihudIlliqSnapshot,
    pub(crate) amihud_loading: bool,
    pub(crate) show_jbnorm: bool,
    pub(crate) jbnorm_symbol: String,
    pub(crate) jbnorm_snapshot: typhoon_engine::core::research::JarqueBeraSnapshot,
    pub(crate) jbnorm_loading: bool,

    // ── Round 27 ──
    pub(crate) show_omega: bool,
    pub(crate) omega_symbol: String,
    pub(crate) omega_snapshot: typhoon_engine::core::research::OmegaRatioSnapshot,
    pub(crate) omega_loading: bool,
    pub(crate) show_dfa: bool,
    pub(crate) dfa_symbol: String,
    pub(crate) dfa_snapshot: typhoon_engine::core::research::DetrendedFluctuationSnapshot,
    pub(crate) dfa_loading: bool,
    pub(crate) show_burke: bool,
    pub(crate) burke_symbol: String,
    pub(crate) burke_snapshot: typhoon_engine::core::research::BurkeRatioSnapshot,
    pub(crate) burke_loading: bool,
    pub(crate) show_monthseas: bool,
    pub(crate) monthseas_symbol: String,
    pub(crate) monthseas_snapshot: typhoon_engine::core::research::MonthlySeasonalitySnapshot,
    pub(crate) monthseas_loading: bool,
    pub(crate) show_rollsprd: bool,
    pub(crate) rollsprd_symbol: String,
    pub(crate) rollsprd_snapshot: typhoon_engine::core::research::RollSpreadSnapshot,
    pub(crate) rollsprd_loading: bool,

    // ── Round 28 ──
    pub(crate) show_parkinson: bool,
    pub(crate) parkinson_symbol: String,
    pub(crate) parkinson_snapshot: typhoon_engine::core::research::ParkinsonVolSnapshot,
    pub(crate) parkinson_loading: bool,
    pub(crate) show_gkvol: bool,
    pub(crate) gkvol_symbol: String,
    pub(crate) gkvol_snapshot: typhoon_engine::core::research::GarmanKlassVolSnapshot,
    pub(crate) gkvol_loading: bool,
    pub(crate) show_rsvol: bool,
    pub(crate) rsvol_symbol: String,
    pub(crate) rsvol_snapshot: typhoon_engine::core::research::RogersSatchellVolSnapshot,
    pub(crate) rsvol_loading: bool,
    pub(crate) show_cvar: bool,
    pub(crate) cvar_symbol: String,
    pub(crate) cvar_snapshot: typhoon_engine::core::research::CVaRSnapshot,
    pub(crate) cvar_loading: bool,
    pub(crate) show_doweffect: bool,
    pub(crate) doweffect_symbol: String,
    pub(crate) doweffect_snapshot: typhoon_engine::core::research::DayOfWeekEffectSnapshot,
    pub(crate) doweffect_loading: bool,

    // ── Round 29 ──
    pub(crate) show_sterling: bool,
    pub(crate) sterling_symbol: String,
    pub(crate) sterling_snapshot: typhoon_engine::core::research::SterlingRatioSnapshot,
    pub(crate) sterling_loading: bool,
    pub(crate) show_kellyf: bool,
    pub(crate) kellyf_symbol: String,
    pub(crate) kellyf_snapshot: typhoon_engine::core::research::KellyFractionSnapshot,
    pub(crate) kellyf_loading: bool,
    pub(crate) show_ljungb: bool,
    pub(crate) ljungb_symbol: String,
    pub(crate) ljungb_snapshot: typhoon_engine::core::research::LjungBoxSnapshot,
    pub(crate) ljungb_loading: bool,
    pub(crate) show_runstest: bool,
    pub(crate) runstest_symbol: String,
    pub(crate) runstest_snapshot: typhoon_engine::core::research::RunsTestSnapshot,
    pub(crate) runstest_loading: bool,
    pub(crate) show_zeroret: bool,
    pub(crate) zeroret_symbol: String,
    pub(crate) zeroret_snapshot: typhoon_engine::core::research::ZeroReturnSnapshot,
    pub(crate) zeroret_loading: bool,

    // ── Round 30 ──
    pub(crate) show_psr: bool,
    pub(crate) psr_symbol: String,
    pub(crate) psr_snapshot: typhoon_engine::core::research::ProbabilisticSharpeSnapshot,
    pub(crate) psr_loading: bool,
    pub(crate) show_adf: bool,
    pub(crate) adf_symbol: String,
    pub(crate) adf_snapshot: typhoon_engine::core::research::DickeyFullerSnapshot,
    pub(crate) adf_loading: bool,
    pub(crate) show_mnkendall: bool,
    pub(crate) mnkendall_symbol: String,
    pub(crate) mnkendall_snapshot: typhoon_engine::core::research::MannKendallSnapshot,
    pub(crate) mnkendall_loading: bool,
    pub(crate) show_bipower: bool,
    pub(crate) bipower_symbol: String,
    pub(crate) bipower_snapshot: typhoon_engine::core::research::BipowerVariationSnapshot,
    pub(crate) bipower_loading: bool,
    pub(crate) show_dddur: bool,
    pub(crate) dddur_symbol: String,
    pub(crate) dddur_snapshot: typhoon_engine::core::research::DrawdownDurationSnapshot,
    pub(crate) dddur_loading: bool,

    // ── Round 31 ──
    pub(crate) show_hilltail: bool,
    pub(crate) hilltail_symbol: String,
    pub(crate) hilltail_snapshot: typhoon_engine::core::research::HillTailSnapshot,
    pub(crate) hilltail_loading: bool,
    pub(crate) show_archlm: bool,
    pub(crate) archlm_symbol: String,
    pub(crate) archlm_snapshot: typhoon_engine::core::research::ArchLmSnapshot,
    pub(crate) archlm_loading: bool,
    pub(crate) show_painratio: bool,
    pub(crate) painratio_symbol: String,
    pub(crate) painratio_snapshot: typhoon_engine::core::research::PainRatioSnapshot,
    pub(crate) painratio_loading: bool,
    pub(crate) show_cusum: bool,
    pub(crate) cusum_symbol: String,
    pub(crate) cusum_snapshot: typhoon_engine::core::research::CusumBreakSnapshot,
    pub(crate) cusum_loading: bool,
    pub(crate) show_cfvar: bool,
    pub(crate) cfvar_symbol: String,
    pub(crate) cfvar_snapshot: typhoon_engine::core::research::CornishFisherSnapshot,
    pub(crate) cfvar_loading: bool,

    // ── Round 32 ──
    pub(crate) show_entropy: bool,
    pub(crate) entropy_symbol: String,
    pub(crate) entropy_snapshot: typhoon_engine::core::research::EntropySnapshot,
    pub(crate) entropy_loading: bool,
    pub(crate) show_rachev: bool,
    pub(crate) rachev_symbol: String,
    pub(crate) rachev_snapshot: typhoon_engine::core::research::RachevSnapshot,
    pub(crate) rachev_loading: bool,
    pub(crate) show_gpr: bool,
    pub(crate) gpr_symbol: String,
    pub(crate) gpr_snapshot: typhoon_engine::core::research::GprSnapshot,
    pub(crate) gpr_loading: bool,
    pub(crate) show_pacf: bool,
    pub(crate) pacf_symbol: String,
    pub(crate) pacf_snapshot: typhoon_engine::core::research::PacfSnapshot,
    pub(crate) pacf_loading: bool,
    pub(crate) show_apen: bool,
    pub(crate) apen_symbol: String,
    pub(crate) apen_snapshot: typhoon_engine::core::research::ApenSnapshot,
    pub(crate) apen_loading: bool,

    // ── Round 33 ──
    pub(crate) show_upr: bool,
    pub(crate) upr_symbol: String,
    pub(crate) upr_snapshot: typhoon_engine::core::research::UprSnapshot,
    pub(crate) upr_loading: bool,
    pub(crate) show_levereff: bool,
    pub(crate) levereff_symbol: String,
    pub(crate) levereff_snapshot: typhoon_engine::core::research::LeverEffSnapshot,
    pub(crate) levereff_loading: bool,
    pub(crate) show_drawdar: bool,
    pub(crate) drawdar_symbol: String,
    pub(crate) drawdar_snapshot: typhoon_engine::core::research::DrawDaRSnapshot,
    pub(crate) drawdar_loading: bool,
    pub(crate) show_varhalf: bool,
    pub(crate) varhalf_symbol: String,
    pub(crate) varhalf_snapshot: typhoon_engine::core::research::VarHalfSnapshot,
    pub(crate) varhalf_loading: bool,
    pub(crate) show_gini: bool,
    pub(crate) gini_symbol: String,
    pub(crate) gini_snapshot: typhoon_engine::core::research::GiniSnapshot,
    pub(crate) gini_loading: bool,
    // ── Round 34 ──
    pub(crate) show_sampen: bool,
    pub(crate) sampen_symbol: String,
    pub(crate) sampen_snapshot: typhoon_engine::core::research::SampenSnapshot,
    pub(crate) sampen_loading: bool,
    pub(crate) show_permen: bool,
    pub(crate) permen_symbol: String,
    pub(crate) permen_snapshot: typhoon_engine::core::research::PermenSnapshot,
    pub(crate) permen_loading: bool,
    pub(crate) show_recfact: bool,
    pub(crate) recfact_symbol: String,
    pub(crate) recfact_snapshot: typhoon_engine::core::research::RecfactSnapshot,
    pub(crate) recfact_loading: bool,
    pub(crate) show_kpss: bool,
    pub(crate) kpss_symbol: String,
    pub(crate) kpss_snapshot: typhoon_engine::core::research::KpssSnapshot,
    pub(crate) kpss_loading: bool,
    pub(crate) show_specent: bool,
    pub(crate) specent_symbol: String,
    pub(crate) specent_snapshot: typhoon_engine::core::research::SpecentSnapshot,
    pub(crate) specent_loading: bool,
    // ── Round 35 ──
    pub(crate) show_robvol: bool,
    pub(crate) robvol_symbol: String,
    pub(crate) robvol_snapshot: typhoon_engine::core::research::RobVolSnapshot,
    pub(crate) robvol_loading: bool,
    pub(crate) show_renyient: bool,
    pub(crate) renyient_symbol: String,
    pub(crate) renyient_snapshot: typhoon_engine::core::research::RenyientSnapshot,
    pub(crate) renyient_loading: bool,
    pub(crate) show_retquant: bool,
    pub(crate) retquant_symbol: String,
    pub(crate) retquant_snapshot: typhoon_engine::core::research::RetquantSnapshot,
    pub(crate) retquant_loading: bool,
    pub(crate) show_msent: bool,
    pub(crate) msent_symbol: String,
    pub(crate) msent_snapshot: typhoon_engine::core::research::MsentSnapshot,
    pub(crate) msent_loading: bool,
    pub(crate) show_ewmavol: bool,
    pub(crate) ewmavol_symbol: String,
    pub(crate) ewmavol_snapshot: typhoon_engine::core::research::EwmaVolSnapshot,
    pub(crate) ewmavol_loading: bool,
    // ── Round 36 ──
    pub(crate) show_ksnorm: bool,
    pub(crate) ksnorm_symbol: String,
    pub(crate) ksnorm_snapshot: typhoon_engine::core::research::KsnormSnapshot,
    pub(crate) ksnorm_loading: bool,
    pub(crate) show_adtest: bool,
    pub(crate) adtest_symbol: String,
    pub(crate) adtest_snapshot: typhoon_engine::core::research::AdtestSnapshot,
    pub(crate) adtest_loading: bool,
    pub(crate) show_lmom: bool,
    pub(crate) lmom_symbol: String,
    pub(crate) lmom_snapshot: typhoon_engine::core::research::LmomSnapshot,
    pub(crate) lmom_loading: bool,
    pub(crate) show_kylelam: bool,
    pub(crate) kylelam_symbol: String,
    pub(crate) kylelam_snapshot: typhoon_engine::core::research::KylelamSnapshot,
    pub(crate) kylelam_loading: bool,
    pub(crate) show_peakover: bool,
    pub(crate) peakover_symbol: String,
    pub(crate) peakover_snapshot: typhoon_engine::core::research::PeakoverSnapshot,
    pub(crate) peakover_loading: bool,
    // ── Round 37 ──
    pub(crate) show_higuchi: bool,
    pub(crate) higuchi_symbol: String,
    pub(crate) higuchi_snapshot: typhoon_engine::core::research::HiguchiSnapshot,
    pub(crate) higuchi_loading: bool,
    pub(crate) show_pickands: bool,
    pub(crate) pickands_symbol: String,
    pub(crate) pickands_snapshot: typhoon_engine::core::research::PickandsSnapshot,
    pub(crate) pickands_loading: bool,
    pub(crate) show_kappa3: bool,
    pub(crate) kappa3_symbol: String,
    pub(crate) kappa3_snapshot: typhoon_engine::core::research::Kappa3Snapshot,
    pub(crate) kappa3_loading: bool,
    pub(crate) show_lyapunov: bool,
    pub(crate) lyapunov_symbol: String,
    pub(crate) lyapunov_snapshot: typhoon_engine::core::research::LyapunovSnapshot,
    pub(crate) lyapunov_loading: bool,
    pub(crate) show_rankac: bool,
    pub(crate) rankac_symbol: String,
    pub(crate) rankac_snapshot: typhoon_engine::core::research::RankacSnapshot,
    pub(crate) rankac_loading: bool,
    // ── Round 38 ──
    pub(crate) show_bnsjump: bool,
    pub(crate) bnsjump_symbol: String,
    pub(crate) bnsjump_snapshot: typhoon_engine::core::research::BnsjumpSnapshot,
    pub(crate) bnsjump_loading: bool,
    pub(crate) show_pproot: bool,
    pub(crate) pproot_symbol: String,
    pub(crate) pproot_snapshot: typhoon_engine::core::research::PprootSnapshot,
    pub(crate) pproot_loading: bool,
    pub(crate) show_mfdfa: bool,
    pub(crate) mfdfa_symbol: String,
    pub(crate) mfdfa_snapshot: typhoon_engine::core::research::MfdfaSnapshot,
    pub(crate) mfdfa_loading: bool,
    pub(crate) show_hillks: bool,
    pub(crate) hillks_symbol: String,
    pub(crate) hillks_snapshot: typhoon_engine::core::research::HillksSnapshot,
    pub(crate) hillks_loading: bool,
    pub(crate) show_tsi: bool,
    pub(crate) tsi_symbol: String,
    pub(crate) tsi_snapshot: typhoon_engine::core::research::TsiSnapshot,
    pub(crate) tsi_loading: bool,
    // ── Round 39 ──
    pub(crate) show_garch11: bool,
    pub(crate) garch11_symbol: String,
    pub(crate) garch11_snapshot: typhoon_engine::core::research::Garch11Snapshot,
    pub(crate) garch11_loading: bool,
    pub(crate) show_sadf: bool,
    pub(crate) sadf_symbol: String,
    pub(crate) sadf_snapshot: typhoon_engine::core::research::SadfSnapshot,
    pub(crate) sadf_loading: bool,
    pub(crate) show_cordim: bool,
    pub(crate) cordim_symbol: String,
    pub(crate) cordim_snapshot: typhoon_engine::core::research::CordimSnapshot,
    pub(crate) cordim_loading: bool,
    pub(crate) show_skspec: bool,
    pub(crate) skspec_symbol: String,
    pub(crate) skspec_snapshot: typhoon_engine::core::research::SkspecSnapshot,
    pub(crate) skspec_loading: bool,
    pub(crate) show_automi: bool,
    pub(crate) automi_symbol: String,
    pub(crate) automi_snapshot: typhoon_engine::core::research::AutomiSnapshot,
    pub(crate) automi_loading: bool,
    // ── Round 40 ──
    pub(crate) show_durbinwatson: bool,
    pub(crate) durbinwatson_symbol: String,
    pub(crate) durbinwatson_snapshot: typhoon_engine::core::research::DurbinWatsonSnapshot,
    pub(crate) durbinwatson_loading: bool,
    pub(crate) show_bdstest: bool,
    pub(crate) bdstest_symbol: String,
    pub(crate) bdstest_snapshot: typhoon_engine::core::research::BdsTestSnapshot,
    pub(crate) bdstest_loading: bool,
    pub(crate) show_breuschpagan: bool,
    pub(crate) breuschpagan_symbol: String,
    pub(crate) breuschpagan_snapshot: typhoon_engine::core::research::BreuschPaganSnapshot,
    pub(crate) breuschpagan_loading: bool,
    pub(crate) show_turnpts: bool,
    pub(crate) turnpts_symbol: String,
    pub(crate) turnpts_snapshot: typhoon_engine::core::research::TurnPtsSnapshot,
    pub(crate) turnpts_loading: bool,
    pub(crate) show_periodogram: bool,
    pub(crate) periodogram_symbol: String,
    pub(crate) periodogram_snapshot: typhoon_engine::core::research::PeriodogramSnapshot,
    pub(crate) periodogram_loading: bool,
    // ── Round 41 ──
    pub(crate) show_mcleodli: bool,
    pub(crate) mcleodli_symbol: String,
    pub(crate) mcleodli_snapshot: typhoon_engine::core::research::McLeodLiSnapshot,
    pub(crate) mcleodli_loading: bool,
    pub(crate) show_oufit: bool,
    pub(crate) oufit_symbol: String,
    pub(crate) oufit_snapshot: typhoon_engine::core::research::OuFitSnapshot,
    pub(crate) oufit_loading: bool,
    pub(crate) show_gph: bool,
    pub(crate) gph_symbol: String,
    pub(crate) gph_snapshot: typhoon_engine::core::research::GphSnapshot,
    pub(crate) gph_loading: bool,
    pub(crate) show_burgspec: bool,
    pub(crate) burgspec_symbol: String,
    pub(crate) burgspec_snapshot: typhoon_engine::core::research::BurgSpecSnapshot,
    pub(crate) burgspec_loading: bool,
    pub(crate) show_kendalltau: bool,
    pub(crate) kendalltau_symbol: String,
    pub(crate) kendalltau_snapshot: typhoon_engine::core::research::KendallTauSnapshot,
    pub(crate) kendalltau_loading: bool,

    // ── Round 42 ──
    pub(crate) show_squeeze_win: bool,
    pub(crate) squeeze_win_symbol: String,
    pub(crate) squeeze_win_snapshot: typhoon_engine::core::research::SqueezeSnapshot,
    pub(crate) squeeze_win_loading: bool,
    pub(crate) show_squeezerank: bool,
    pub(crate) squeezerank_symbol: String,
    pub(crate) squeezerank_snapshot: typhoon_engine::core::research::SqueezeRankSnapshot,
    pub(crate) squeezerank_loading: bool,
    pub(crate) show_squeeze_watchlist: bool,
    pub(crate) squeeze_watchlist_rows: Vec<typhoon_engine::core::research::SqueezeSnapshot>,
    pub(crate) squeeze_watchlist_loading: bool,
    pub(crate) show_bbsqueeze: bool,
    pub(crate) bbsqueeze_symbol: String,
    pub(crate) bbsqueeze_snapshot: typhoon_engine::core::research::BbsqueezeSnapshot,
    pub(crate) bbsqueeze_loading: bool,
    pub(crate) show_donchian_win: bool,
    pub(crate) donchian_win_symbol: String,
    pub(crate) donchian_win_snapshot: typhoon_engine::core::research::DonchianSnapshot,
    pub(crate) donchian_win_loading: bool,
    pub(crate) show_kama_win: bool,
    pub(crate) kama_win_symbol: String,
    pub(crate) kama_win_snapshot: typhoon_engine::core::research::KamaSnapshot,
    pub(crate) kama_win_loading: bool,
    // ── Round 43 ──
    pub(crate) show_ichimoku_win: bool,
    pub(crate) ichimoku_win_symbol: String,
    pub(crate) ichimoku_win_snapshot: typhoon_engine::core::research::IchimokuSnapshot,
    pub(crate) ichimoku_win_loading: bool,
    pub(crate) show_supertrend_win: bool,
    pub(crate) supertrend_win_symbol: String,
    pub(crate) supertrend_win_snapshot: typhoon_engine::core::research::SupertrendSnapshot,
    pub(crate) supertrend_win_loading: bool,
    pub(crate) show_keltner_win: bool,
    pub(crate) keltner_win_symbol: String,
    pub(crate) keltner_win_snapshot: typhoon_engine::core::research::KeltnerSnapshot,
    pub(crate) keltner_win_loading: bool,
    pub(crate) show_fisher_win: bool,
    pub(crate) fisher_win_symbol: String,
    pub(crate) fisher_win_snapshot: typhoon_engine::core::research::FisherSnapshot,
    pub(crate) fisher_win_loading: bool,
    pub(crate) show_aroon_win: bool,
    pub(crate) aroon_win_symbol: String,
    pub(crate) aroon_win_snapshot: typhoon_engine::core::research::AroonSnapshot,
    pub(crate) aroon_win_loading: bool,
    // ── Round 44 ──
    pub(crate) show_adx_win: bool,
    pub(crate) adx_win_symbol: String,
    pub(crate) adx_win_snapshot: typhoon_engine::core::research::AdxSnapshot,
    pub(crate) adx_win_loading: bool,
    pub(crate) show_cci_win: bool,
    pub(crate) cci_win_symbol: String,
    pub(crate) cci_win_snapshot: typhoon_engine::core::research::CciSnapshot,
    pub(crate) cci_win_loading: bool,
    pub(crate) show_cmf_win: bool,
    pub(crate) cmf_win_symbol: String,
    pub(crate) cmf_win_snapshot: typhoon_engine::core::research::CmfSnapshot,
    pub(crate) cmf_win_loading: bool,
    pub(crate) show_mfi_win: bool,
    pub(crate) mfi_win_symbol: String,
    pub(crate) mfi_win_snapshot: typhoon_engine::core::research::MfiSnapshot,
    pub(crate) mfi_win_loading: bool,
    pub(crate) show_psar_win: bool,
    pub(crate) psar_win_symbol: String,
    pub(crate) psar_win_snapshot: typhoon_engine::core::research::PsarSnapshot,
    pub(crate) psar_win_loading: bool,
    // ── Round 45 ──
    pub(crate) show_vortex_win: bool,
    pub(crate) vortex_win_symbol: String,
    pub(crate) vortex_win_snapshot: typhoon_engine::core::research::VortexSnapshot,
    pub(crate) vortex_win_loading: bool,
    pub(crate) show_chop_win: bool,
    pub(crate) chop_win_symbol: String,
    pub(crate) chop_win_snapshot: typhoon_engine::core::research::ChopSnapshot,
    pub(crate) chop_win_loading: bool,
    pub(crate) show_obv_win: bool,
    pub(crate) obv_win_symbol: String,
    pub(crate) obv_win_snapshot: typhoon_engine::core::research::ObvSnapshot,
    pub(crate) obv_win_loading: bool,
    pub(crate) show_trix_win: bool,
    pub(crate) trix_win_symbol: String,
    pub(crate) trix_win_snapshot: typhoon_engine::core::research::TrixSnapshot,
    pub(crate) trix_win_loading: bool,
    pub(crate) show_hma_win: bool,
    pub(crate) hma_win_symbol: String,
    pub(crate) hma_win_snapshot: typhoon_engine::core::research::HmaSnapshot,
    pub(crate) hma_win_loading: bool,
    // ── Round 46 ──
    pub(crate) show_ppo_win: bool,
    pub(crate) ppo_win_symbol: String,
    pub(crate) ppo_win_snapshot: typhoon_engine::core::research::PpoSnapshot,
    pub(crate) ppo_win_loading: bool,
    pub(crate) show_dpo_win: bool,
    pub(crate) dpo_win_symbol: String,
    pub(crate) dpo_win_snapshot: typhoon_engine::core::research::DpoSnapshot,
    pub(crate) dpo_win_loading: bool,
    pub(crate) show_kst_win: bool,
    pub(crate) kst_win_symbol: String,
    pub(crate) kst_win_snapshot: typhoon_engine::core::research::KstSnapshot,
    pub(crate) kst_win_loading: bool,
    pub(crate) show_ultosc_win: bool,
    pub(crate) ultosc_win_symbol: String,
    pub(crate) ultosc_win_snapshot: typhoon_engine::core::research::UltoscSnapshot,
    pub(crate) ultosc_win_loading: bool,
    pub(crate) show_willr_win: bool,
    pub(crate) willr_win_symbol: String,
    pub(crate) willr_win_snapshot: typhoon_engine::core::research::WillrSnapshot,
    pub(crate) willr_win_loading: bool,
    // ── Round 47 ──
    pub(crate) show_mass_win: bool,
    pub(crate) mass_win_symbol: String,
    pub(crate) mass_win_snapshot: typhoon_engine::core::research::MassSnapshot,
    pub(crate) mass_win_loading: bool,
    pub(crate) show_chaikosc_win: bool,
    pub(crate) chaikosc_win_symbol: String,
    pub(crate) chaikosc_win_snapshot: typhoon_engine::core::research::ChaikoscSnapshot,
    pub(crate) chaikosc_win_loading: bool,
    pub(crate) show_klinger_win: bool,
    pub(crate) klinger_win_symbol: String,
    pub(crate) klinger_win_snapshot: typhoon_engine::core::research::KlingerSnapshot,
    pub(crate) klinger_win_loading: bool,
    pub(crate) show_stochrsi_win: bool,
    pub(crate) stochrsi_win_symbol: String,
    pub(crate) stochrsi_win_snapshot: typhoon_engine::core::research::StochRsiSnapshot,
    pub(crate) stochrsi_win_loading: bool,
    pub(crate) show_awesome_win: bool,
    pub(crate) awesome_win_symbol: String,
    pub(crate) awesome_win_snapshot: typhoon_engine::core::research::AwesomeSnapshot,
    pub(crate) awesome_win_loading: bool,
    // ── Round 48 windows ──
    pub(crate) show_efi_win: bool,
    pub(crate) efi_win_symbol: String,
    pub(crate) efi_win_snapshot: typhoon_engine::core::research::EfiSnapshot,
    pub(crate) efi_win_loading: bool,
    pub(crate) show_emv_win: bool,
    pub(crate) emv_win_symbol: String,
    pub(crate) emv_win_snapshot: typhoon_engine::core::research::EmvSnapshot,
    pub(crate) emv_win_loading: bool,
    pub(crate) show_nvi_win: bool,
    pub(crate) nvi_win_symbol: String,
    pub(crate) nvi_win_snapshot: typhoon_engine::core::research::NviSnapshot,
    pub(crate) nvi_win_loading: bool,
    pub(crate) show_pvi_win: bool,
    pub(crate) pvi_win_symbol: String,
    pub(crate) pvi_win_snapshot: typhoon_engine::core::research::PviSnapshot,
    pub(crate) pvi_win_loading: bool,
    pub(crate) show_coppock_win: bool,
    pub(crate) coppock_win_symbol: String,
    pub(crate) coppock_win_snapshot: typhoon_engine::core::research::CoppockSnapshot,
    pub(crate) coppock_win_loading: bool,
    // ── Round 49 windows ──
    pub(crate) show_cmo_win: bool,
    pub(crate) cmo_win_symbol: String,
    pub(crate) cmo_win_snapshot: typhoon_engine::core::research::CmoSnapshot,
    pub(crate) cmo_win_loading: bool,
    pub(crate) show_qstick_win: bool,
    pub(crate) qstick_win_symbol: String,
    pub(crate) qstick_win_snapshot: typhoon_engine::core::research::QstickSnapshot,
    pub(crate) qstick_win_loading: bool,
    pub(crate) show_disparity_win: bool,
    pub(crate) disparity_win_symbol: String,
    pub(crate) disparity_win_snapshot: typhoon_engine::core::research::DisparitySnapshot,
    pub(crate) disparity_win_loading: bool,
    pub(crate) show_bop_win: bool,
    pub(crate) bop_win_symbol: String,
    pub(crate) bop_win_snapshot: typhoon_engine::core::research::BopSnapshot,
    pub(crate) bop_win_loading: bool,
    pub(crate) show_schaff_win: bool,
    pub(crate) schaff_win_symbol: String,
    pub(crate) schaff_win_snapshot: typhoon_engine::core::research::SchaffSnapshot,
    pub(crate) schaff_win_loading: bool,
    // ── Round 50 windows ──
    pub(crate) show_stoch_win: bool,
    pub(crate) stoch_win_symbol: String,
    pub(crate) stoch_win_snapshot: typhoon_engine::core::research::StochSnapshot,
    pub(crate) stoch_win_loading: bool,
    pub(crate) show_macd_win: bool,
    pub(crate) macd_win_symbol: String,
    pub(crate) macd_win_snapshot: typhoon_engine::core::research::MacdSnapshot,
    pub(crate) macd_win_loading: bool,
    pub(crate) show_vwap_win: bool,
    pub(crate) vwap_win_symbol: String,
    pub(crate) vwap_win_snapshot: typhoon_engine::core::research::VwapSnapshot,
    pub(crate) vwap_win_loading: bool,
    pub(crate) show_mcgd_win: bool,
    pub(crate) mcgd_win_symbol: String,
    pub(crate) mcgd_win_snapshot: typhoon_engine::core::research::McgdSnapshot,
    pub(crate) mcgd_win_loading: bool,
    pub(crate) show_rwi_win: bool,
    pub(crate) rwi_win_symbol: String,
    pub(crate) rwi_win_snapshot: typhoon_engine::core::research::RwiSnapshot,
    pub(crate) rwi_win_loading: bool,
    // ── Round 51 windows ──
    pub(crate) show_dema_win: bool,
    pub(crate) dema_win_symbol: String,
    pub(crate) dema_win_snapshot: typhoon_engine::core::research::DemaSnapshot,
    pub(crate) dema_win_loading: bool,
    pub(crate) show_tema_win: bool,
    pub(crate) tema_win_symbol: String,
    pub(crate) tema_win_snapshot: typhoon_engine::core::research::TemaSnapshot,
    pub(crate) tema_win_loading: bool,
    pub(crate) show_linreg_win: bool,
    pub(crate) linreg_win_symbol: String,
    pub(crate) linreg_win_snapshot: typhoon_engine::core::research::LinregSnapshot,
    pub(crate) linreg_win_loading: bool,
    pub(crate) show_pivots_win: bool,
    pub(crate) pivots_win_symbol: String,
    pub(crate) pivots_win_snapshot: typhoon_engine::core::research::PivotsSnapshot,
    pub(crate) pivots_win_loading: bool,
    pub(crate) show_heikin_win: bool,
    pub(crate) heikin_win_symbol: String,
    pub(crate) heikin_win_snapshot: typhoon_engine::core::research::HeikinSnapshot,
    pub(crate) heikin_win_loading: bool,
    // ── Round 52 windows ──
    pub(crate) show_alma_win: bool,
    pub(crate) alma_win_symbol: String,
    pub(crate) alma_win_snapshot: typhoon_engine::core::research::AlmaSnapshot,
    pub(crate) alma_win_loading: bool,
    pub(crate) show_zlema_win: bool,
    pub(crate) zlema_win_symbol: String,
    pub(crate) zlema_win_snapshot: typhoon_engine::core::research::ZlemaSnapshot,
    pub(crate) zlema_win_loading: bool,
    pub(crate) show_elderray_win: bool,
    pub(crate) elderray_win_symbol: String,
    pub(crate) elderray_win_snapshot: typhoon_engine::core::research::ElderRaySnapshot,
    pub(crate) elderray_win_loading: bool,
    pub(crate) show_tsf_win: bool,
    pub(crate) tsf_win_symbol: String,
    pub(crate) tsf_win_snapshot: typhoon_engine::core::research::TsfSnapshot,
    pub(crate) tsf_win_loading: bool,
    pub(crate) show_rvi_win: bool,
    pub(crate) rvi_win_symbol: String,
    pub(crate) rvi_win_snapshot: typhoon_engine::core::research::RviSnapshot,
    pub(crate) rvi_win_loading: bool,
    // ── Round 53 windows ──
    pub(crate) show_trima_win: bool,
    pub(crate) trima_win_symbol: String,
    pub(crate) trima_win_snapshot: typhoon_engine::core::research::TrimaSnapshot,
    pub(crate) trima_win_loading: bool,
    pub(crate) show_t3_win: bool,
    pub(crate) t3_win_symbol: String,
    pub(crate) t3_win_snapshot: typhoon_engine::core::research::T3Snapshot,
    pub(crate) t3_win_loading: bool,
    pub(crate) show_vidya_win: bool,
    pub(crate) vidya_win_symbol: String,
    pub(crate) vidya_win_snapshot: typhoon_engine::core::research::VidyaSnapshot,
    pub(crate) vidya_win_loading: bool,
    pub(crate) show_smi_win: bool,
    pub(crate) smi_win_symbol: String,
    pub(crate) smi_win_snapshot: typhoon_engine::core::research::SmiSnapshot,
    pub(crate) smi_win_loading: bool,
    pub(crate) show_pvt_win: bool,
    pub(crate) pvt_win_symbol: String,
    pub(crate) pvt_win_snapshot: typhoon_engine::core::research::PvtSnapshot,
    pub(crate) pvt_win_loading: bool,
    // ── Round 54 windows ──
    pub(crate) show_ac_win: bool,
    pub(crate) ac_win_symbol: String,
    pub(crate) ac_win_snapshot: typhoon_engine::core::research::AcSnapshot,
    pub(crate) ac_win_loading: bool,
    pub(crate) show_chvol_win: bool,
    pub(crate) chvol_win_symbol: String,
    pub(crate) chvol_win_snapshot: typhoon_engine::core::research::ChvolSnapshot,
    pub(crate) chvol_win_loading: bool,
    pub(crate) show_bbwidth_win: bool,
    pub(crate) bbwidth_win_symbol: String,
    pub(crate) bbwidth_win_snapshot: typhoon_engine::core::research::BbwidthSnapshot,
    pub(crate) bbwidth_win_loading: bool,
    pub(crate) show_elderimp_win: bool,
    pub(crate) elderimp_win_symbol: String,
    pub(crate) elderimp_win_snapshot: typhoon_engine::core::research::ElderImpulseSnapshot,
    pub(crate) elderimp_win_loading: bool,
    pub(crate) show_rmi_win: bool,
    pub(crate) rmi_win_symbol: String,
    pub(crate) rmi_win_snapshot: typhoon_engine::core::research::RmiSnapshot,
    pub(crate) rmi_win_loading: bool,

    // ── Options Expiration Calendar ──
    pub(crate) show_expcal_win: bool,
    pub(crate) expcal_win_symbol: String,
    pub(crate) expcal_win_snapshot: typhoon_engine::core::research::SymbolExpirationsSnapshot,
    pub(crate) expcal_win_loading: bool,
    pub(crate) expcal_win_tab: u8, // 0 = Tier 1 market calendar, 1 = Tier 2 symbol chain
    pub(crate) expcal_win_horizon_days: u32,
    pub(crate) expcal_win_calendar: Vec<typhoon_engine::core::research::CalendarExpiry>,

    // ── Round 55: SMMA / ALLIGATOR / CRSI / SEB / IMI ──
    pub(crate) show_smma_win: bool,
    pub(crate) smma_win_symbol: String,
    pub(crate) smma_win_snapshot: typhoon_engine::core::research::SmmaSnapshot,
    pub(crate) smma_win_loading: bool,
    pub(crate) show_alligator_win: bool,
    pub(crate) alligator_win_symbol: String,
    pub(crate) alligator_win_snapshot: typhoon_engine::core::research::AlligatorSnapshot,
    pub(crate) alligator_win_loading: bool,
    pub(crate) show_crsi_win: bool,
    pub(crate) crsi_win_symbol: String,
    pub(crate) crsi_win_snapshot: typhoon_engine::core::research::CrsiSnapshot,
    pub(crate) crsi_win_loading: bool,
    pub(crate) show_seb_win: bool,
    pub(crate) seb_win_symbol: String,
    pub(crate) seb_win_snapshot: typhoon_engine::core::research::SebSnapshot,
    pub(crate) seb_win_loading: bool,
    pub(crate) show_imi_win: bool,
    pub(crate) imi_win_symbol: String,
    pub(crate) imi_win_snapshot: typhoon_engine::core::research::ImiSnapshot,
    pub(crate) imi_win_loading: bool,

    // ── Round 56: GMMA / MAENV / ADL / VHF / VROC ──
    pub(crate) show_gmma_win: bool,
    pub(crate) gmma_win_symbol: String,
    pub(crate) gmma_win_snapshot: typhoon_engine::core::research::GmmaSnapshot,
    pub(crate) gmma_win_loading: bool,
    pub(crate) show_maenv_win: bool,
    pub(crate) maenv_win_symbol: String,
    pub(crate) maenv_win_snapshot: typhoon_engine::core::research::MaenvSnapshot,
    pub(crate) maenv_win_loading: bool,
    pub(crate) show_adl_win: bool,
    pub(crate) adl_win_symbol: String,
    pub(crate) adl_win_snapshot: typhoon_engine::core::research::AdlSnapshot,
    pub(crate) adl_win_loading: bool,
    pub(crate) show_vhf_win: bool,
    pub(crate) vhf_win_symbol: String,
    pub(crate) vhf_win_snapshot: typhoon_engine::core::research::VhfSnapshot,
    pub(crate) vhf_win_loading: bool,
    pub(crate) show_vroc_win: bool,
    pub(crate) vroc_win_symbol: String,
    pub(crate) vroc_win_snapshot: typhoon_engine::core::research::VrocSnapshot,
    pub(crate) vroc_win_loading: bool,

    // ── Round 57: KDJ / QQE / PMO / CFO / TMF ──
    pub(crate) show_kdj_win: bool,
    pub(crate) kdj_win_symbol: String,
    pub(crate) kdj_win_snapshot: typhoon_engine::core::research::KdjSnapshot,
    pub(crate) kdj_win_loading: bool,
    pub(crate) show_qqe_win: bool,
    pub(crate) qqe_win_symbol: String,
    pub(crate) qqe_win_snapshot: typhoon_engine::core::research::QqeSnapshot,
    pub(crate) qqe_win_loading: bool,
    pub(crate) show_pmo_win: bool,
    pub(crate) pmo_win_symbol: String,
    pub(crate) pmo_win_snapshot: typhoon_engine::core::research::PmoSnapshot,
    pub(crate) pmo_win_loading: bool,
    pub(crate) show_cfo_win: bool,
    pub(crate) cfo_win_symbol: String,
    pub(crate) cfo_win_snapshot: typhoon_engine::core::research::CfoSnapshot,
    pub(crate) cfo_win_loading: bool,
    pub(crate) show_tmf_win: bool,
    pub(crate) tmf_win_symbol: String,
    pub(crate) tmf_win_snapshot: typhoon_engine::core::research::TmfSnapshot,
    pub(crate) tmf_win_loading: bool,

    // ── Round 58: FRACTALS / IFT_RSI / MAMA / COG / DIDI ──
    pub(crate) show_fractals_win: bool,
    pub(crate) fractals_win_symbol: String,
    pub(crate) fractals_win_snapshot: typhoon_engine::core::research::FractalsSnapshot,
    pub(crate) fractals_win_loading: bool,
    pub(crate) show_ift_rsi_win: bool,
    pub(crate) ift_rsi_win_symbol: String,
    pub(crate) ift_rsi_win_snapshot: typhoon_engine::core::research::IftRsiSnapshot,
    pub(crate) ift_rsi_win_loading: bool,
    pub(crate) show_mama_win: bool,
    pub(crate) mama_win_symbol: String,
    pub(crate) mama_win_snapshot: typhoon_engine::core::research::MamaSnapshot,
    pub(crate) mama_win_loading: bool,
    pub(crate) show_cog_win: bool,
    pub(crate) cog_win_symbol: String,
    pub(crate) cog_win_snapshot: typhoon_engine::core::research::CogSnapshot,
    pub(crate) cog_win_loading: bool,
    pub(crate) show_didi_win: bool,
    pub(crate) didi_win_symbol: String,
    pub(crate) didi_win_snapshot: typhoon_engine::core::research::DidiSnapshot,
    pub(crate) didi_win_loading: bool,

    // ── Round 59: DEMARKER / GATOR / BW_MFI / VWMA / STDDEV ──
    pub(crate) show_demarker_win: bool,
    pub(crate) demarker_win_symbol: String,
    pub(crate) demarker_win_snapshot: typhoon_engine::core::research::DemarkerSnapshot,
    pub(crate) demarker_win_loading: bool,
    pub(crate) show_gator_win: bool,
    pub(crate) gator_win_symbol: String,
    pub(crate) gator_win_snapshot: typhoon_engine::core::research::GatorSnapshot,
    pub(crate) gator_win_loading: bool,
    pub(crate) show_bw_mfi_win: bool,
    pub(crate) bw_mfi_win_symbol: String,
    pub(crate) bw_mfi_win_snapshot: typhoon_engine::core::research::BwMfiSnapshot,
    pub(crate) bw_mfi_win_loading: bool,
    pub(crate) show_vwma_win: bool,
    pub(crate) vwma_win_symbol: String,
    pub(crate) vwma_win_snapshot: typhoon_engine::core::research::VwmaSnapshot,
    pub(crate) vwma_win_loading: bool,
    pub(crate) show_stddev_win: bool,
    pub(crate) stddev_win_symbol: String,
    pub(crate) stddev_win_snapshot: typhoon_engine::core::research::StddevSnapshot,
    pub(crate) stddev_win_loading: bool,

    // ── Round 60: WMA / RAINBOW / MESA_SINE / FRAMA / IBS ──
    pub(crate) show_wma_win: bool,
    pub(crate) wma_win_symbol: String,
    pub(crate) wma_win_snapshot: typhoon_engine::core::research::WmaSnapshot,
    pub(crate) wma_win_loading: bool,
    pub(crate) show_rainbow_win: bool,
    pub(crate) rainbow_win_symbol: String,
    pub(crate) rainbow_win_snapshot: typhoon_engine::core::research::RainbowSnapshot,
    pub(crate) rainbow_win_loading: bool,
    pub(crate) show_mesa_sine_win: bool,
    pub(crate) mesa_sine_win_symbol: String,
    pub(crate) mesa_sine_win_snapshot: typhoon_engine::core::research::MesaSineSnapshot,
    pub(crate) mesa_sine_win_loading: bool,
    pub(crate) show_frama_win: bool,
    pub(crate) frama_win_symbol: String,
    pub(crate) frama_win_snapshot: typhoon_engine::core::research::FramaSnapshot,
    pub(crate) frama_win_loading: bool,
    pub(crate) show_ibs_win: bool,
    pub(crate) ibs_win_symbol: String,
    pub(crate) ibs_win_snapshot: typhoon_engine::core::research::IbsSnapshot,
    pub(crate) ibs_win_loading: bool,

    // ── Round 61: LAGUERRE_RSI / ZIGZAG / PGO / HT_TRENDLINE / MIDPOINT ──
    pub(crate) show_laguerre_rsi_win: bool,
    pub(crate) laguerre_rsi_win_symbol: String,
    pub(crate) laguerre_rsi_win_snapshot: typhoon_engine::core::research::LaguerreRsiSnapshot,
    pub(crate) laguerre_rsi_win_loading: bool,
    pub(crate) show_zigzag_win: bool,
    pub(crate) zigzag_win_symbol: String,
    pub(crate) zigzag_win_snapshot: typhoon_engine::core::research::ZigzagSnapshot,
    pub(crate) zigzag_win_loading: bool,
    pub(crate) show_pgo_win: bool,
    pub(crate) pgo_win_symbol: String,
    pub(crate) pgo_win_snapshot: typhoon_engine::core::research::PgoSnapshot,
    pub(crate) pgo_win_loading: bool,
    pub(crate) show_ht_trendline_win: bool,
    pub(crate) ht_trendline_win_symbol: String,
    pub(crate) ht_trendline_win_snapshot: typhoon_engine::core::research::HtTrendlineSnapshot,
    pub(crate) ht_trendline_win_loading: bool,
    pub(crate) show_midpoint_win: bool,
    pub(crate) midpoint_win_symbol: String,
    pub(crate) midpoint_win_snapshot: typhoon_engine::core::research::MidpointSnapshot,
    pub(crate) midpoint_win_loading: bool,

    // ── Round 62: MASSINDEX / NATR / TTM_SQUEEZE / FORCE_INDEX / TRANGE ──
    pub(crate) show_mass_index_win: bool,
    pub(crate) mass_index_win_symbol: String,
    pub(crate) mass_index_win_snapshot: typhoon_engine::core::research::MassIndexSnapshot,
    pub(crate) mass_index_win_loading: bool,
    pub(crate) show_natr_win: bool,
    pub(crate) natr_win_symbol: String,
    pub(crate) natr_win_snapshot: typhoon_engine::core::research::NatrSnapshot,
    pub(crate) natr_win_loading: bool,
    pub(crate) show_ttm_squeeze_win: bool,
    pub(crate) ttm_squeeze_win_symbol: String,
    pub(crate) ttm_squeeze_win_snapshot: typhoon_engine::core::research::TtmSqueezeSnapshot,
    pub(crate) ttm_squeeze_win_loading: bool,
    pub(crate) show_force_index_win: bool,
    pub(crate) force_index_win_symbol: String,
    pub(crate) force_index_win_snapshot: typhoon_engine::core::research::ForceIndexSnapshot,
    pub(crate) force_index_win_loading: bool,
    pub(crate) show_trange_win: bool,
    pub(crate) trange_win_symbol: String,
    pub(crate) trange_win_snapshot: typhoon_engine::core::research::TrangeSnapshot,
    pub(crate) trange_win_loading: bool,

    // ── Round 63: LINEARREG_SLOPE / HT_DCPERIOD / HT_TRENDMODE / ACCBANDS / STOCHF ──
    pub(crate) show_linearreg_slope_win: bool,
    pub(crate) linearreg_slope_win_symbol: String,
    pub(crate) linearreg_slope_win_snapshot: typhoon_engine::core::research::LinearregSlopeSnapshot,
    pub(crate) linearreg_slope_win_loading: bool,
    pub(crate) show_ht_dcperiod_win: bool,
    pub(crate) ht_dcperiod_win_symbol: String,
    pub(crate) ht_dcperiod_win_snapshot: typhoon_engine::core::research::HtDcperiodSnapshot,
    pub(crate) ht_dcperiod_win_loading: bool,
    pub(crate) show_ht_trendmode_win: bool,
    pub(crate) ht_trendmode_win_symbol: String,
    pub(crate) ht_trendmode_win_snapshot: typhoon_engine::core::research::HtTrendmodeSnapshot,
    pub(crate) ht_trendmode_win_loading: bool,
    pub(crate) show_accbands_win: bool,
    pub(crate) accbands_win_symbol: String,
    pub(crate) accbands_win_snapshot: typhoon_engine::core::research::AccbandsSnapshot,
    pub(crate) accbands_win_loading: bool,
    pub(crate) show_stochf_win: bool,
    pub(crate) stochf_win_symbol: String,
    pub(crate) stochf_win_snapshot: typhoon_engine::core::research::StochfSnapshot,
    pub(crate) stochf_win_loading: bool,

    // ── Round 64: LINEARREG / LINEARREG_ANGLE / HT_DCPHASE / HT_SINE / HT_PHASOR ──
    pub(crate) show_linearreg_win: bool,
    pub(crate) linearreg_win_symbol: String,
    pub(crate) linearreg_win_snapshot: typhoon_engine::core::research::LinearregSnapshot,
    pub(crate) linearreg_win_loading: bool,
    pub(crate) show_linearreg_angle_win: bool,
    pub(crate) linearreg_angle_win_symbol: String,
    pub(crate) linearreg_angle_win_snapshot: typhoon_engine::core::research::LinearregAngleSnapshot,
    pub(crate) linearreg_angle_win_loading: bool,
    pub(crate) show_ht_dcphase_win: bool,
    pub(crate) ht_dcphase_win_symbol: String,
    pub(crate) ht_dcphase_win_snapshot: typhoon_engine::core::research::HtDcphaseSnapshot,
    pub(crate) ht_dcphase_win_loading: bool,
    pub(crate) show_ht_sine_win: bool,
    pub(crate) ht_sine_win_symbol: String,
    pub(crate) ht_sine_win_snapshot: typhoon_engine::core::research::HtSineSnapshot,
    pub(crate) ht_sine_win_loading: bool,
    pub(crate) show_ht_phasor_win: bool,
    pub(crate) ht_phasor_win_symbol: String,
    pub(crate) ht_phasor_win_snapshot: typhoon_engine::core::research::HtPhasorSnapshot,
    pub(crate) ht_phasor_win_loading: bool,

    // ── Round 65: MIDPRICE / APO / MOM / SAREXT / ADXR ──
    pub(crate) show_midprice_win: bool,
    pub(crate) midprice_win_symbol: String,
    pub(crate) midprice_win_snapshot: typhoon_engine::core::research::MidpriceSnapshot,
    pub(crate) midprice_win_loading: bool,
    pub(crate) show_apo_win: bool,
    pub(crate) apo_win_symbol: String,
    pub(crate) apo_win_snapshot: typhoon_engine::core::research::ApoSnapshot,
    pub(crate) apo_win_loading: bool,
    pub(crate) show_mom_win: bool,
    pub(crate) mom_win_symbol: String,
    pub(crate) mom_win_snapshot: typhoon_engine::core::research::MomSnapshot,
    pub(crate) mom_win_loading: bool,
    pub(crate) show_sarext_win: bool,
    pub(crate) sarext_win_symbol: String,
    pub(crate) sarext_win_snapshot: typhoon_engine::core::research::SarextSnapshot,
    pub(crate) sarext_win_loading: bool,
    pub(crate) show_adxr_win: bool,
    pub(crate) adxr_win_symbol: String,
    pub(crate) adxr_win_snapshot: typhoon_engine::core::research::AdxrSnapshot,
    pub(crate) adxr_win_loading: bool,

    // ── Round 66: AVGPRICE / MEDPRICE / TYPPRICE / WCLPRICE / VARIANCE ──
    pub(crate) show_avgprice_win: bool,
    pub(crate) avgprice_win_symbol: String,
    pub(crate) avgprice_win_snapshot: typhoon_engine::core::research::AvgpriceSnapshot,
    pub(crate) avgprice_win_loading: bool,
    pub(crate) show_medprice_win: bool,
    pub(crate) medprice_win_symbol: String,
    pub(crate) medprice_win_snapshot: typhoon_engine::core::research::MedpriceSnapshot,
    pub(crate) medprice_win_loading: bool,
    pub(crate) show_typprice_win: bool,
    pub(crate) typprice_win_symbol: String,
    pub(crate) typprice_win_snapshot: typhoon_engine::core::research::TypPriceSnapshot,
    pub(crate) typprice_win_loading: bool,
    pub(crate) show_wclprice_win: bool,
    pub(crate) wclprice_win_symbol: String,
    pub(crate) wclprice_win_snapshot: typhoon_engine::core::research::WclPriceSnapshot,
    pub(crate) wclprice_win_loading: bool,
    pub(crate) show_variance_win: bool,
    pub(crate) variance_win_symbol: String,
    pub(crate) variance_win_snapshot: typhoon_engine::core::research::VarianceSnapshot,
    pub(crate) variance_win_loading: bool,
    // ── Round 67 ──
    pub(crate) show_plus_di_win: bool,
    pub(crate) plus_di_win_symbol: String,
    pub(crate) plus_di_win_snapshot: typhoon_engine::core::research::PlusDiSnapshot,
    pub(crate) plus_di_win_loading: bool,
    pub(crate) show_minus_di_win: bool,
    pub(crate) minus_di_win_symbol: String,
    pub(crate) minus_di_win_snapshot: typhoon_engine::core::research::MinusDiSnapshot,
    pub(crate) minus_di_win_loading: bool,
    pub(crate) show_plus_dm_win: bool,
    pub(crate) plus_dm_win_symbol: String,
    pub(crate) plus_dm_win_snapshot: typhoon_engine::core::research::PlusDmSnapshot,
    pub(crate) plus_dm_win_loading: bool,
    pub(crate) show_minus_dm_win: bool,
    pub(crate) minus_dm_win_symbol: String,
    pub(crate) minus_dm_win_snapshot: typhoon_engine::core::research::MinusDmSnapshot,
    pub(crate) minus_dm_win_loading: bool,
    pub(crate) show_dx_win: bool,
    pub(crate) dx_win_symbol: String,
    pub(crate) dx_win_snapshot: typhoon_engine::core::research::DxSnapshot,
    pub(crate) dx_win_loading: bool,
    // ── Round 68 ──
    pub(crate) show_roc_win: bool,
    pub(crate) roc_win_symbol: String,
    pub(crate) roc_win_snapshot: typhoon_engine::core::research::RocSnapshot,
    pub(crate) roc_win_loading: bool,
    pub(crate) show_rocp_win: bool,
    pub(crate) rocp_win_symbol: String,
    pub(crate) rocp_win_snapshot: typhoon_engine::core::research::RocpSnapshot,
    pub(crate) rocp_win_loading: bool,
    pub(crate) show_rocr_win: bool,
    pub(crate) rocr_win_symbol: String,
    pub(crate) rocr_win_snapshot: typhoon_engine::core::research::RocrSnapshot,
    pub(crate) rocr_win_loading: bool,
    pub(crate) show_rocr100_win: bool,
    pub(crate) rocr100_win_symbol: String,
    pub(crate) rocr100_win_snapshot: typhoon_engine::core::research::Rocr100Snapshot,
    pub(crate) rocr100_win_loading: bool,
    pub(crate) show_correl_win: bool,
    pub(crate) correl_win_symbol: String,
    pub(crate) correl_win_snapshot: typhoon_engine::core::research::CorrelSnapshot,
    pub(crate) correl_win_loading: bool,
    // ── Round 69 ──
    pub(crate) show_min_win: bool,
    pub(crate) min_win_symbol: String,
    pub(crate) min_win_snapshot: typhoon_engine::core::research::MinSnapshot,
    pub(crate) min_win_loading: bool,
    pub(crate) show_max_win: bool,
    pub(crate) max_win_symbol: String,
    pub(crate) max_win_snapshot: typhoon_engine::core::research::MaxSnapshot,
    pub(crate) max_win_loading: bool,
    pub(crate) show_minmax_win: bool,
    pub(crate) minmax_win_symbol: String,
    pub(crate) minmax_win_snapshot: typhoon_engine::core::research::MinMaxSnapshot,
    pub(crate) minmax_win_loading: bool,
    pub(crate) show_minindex_win: bool,
    pub(crate) minindex_win_symbol: String,
    pub(crate) minindex_win_snapshot: typhoon_engine::core::research::MinIndexSnapshot,
    pub(crate) minindex_win_loading: bool,
    pub(crate) show_maxindex_win: bool,
    pub(crate) maxindex_win_symbol: String,
    pub(crate) maxindex_win_snapshot: typhoon_engine::core::research::MaxIndexSnapshot,
    pub(crate) maxindex_win_loading: bool,
    // ── Round 70 ──
    pub(crate) show_bbands_win: bool,
    pub(crate) bbands_win_symbol: String,
    pub(crate) bbands_win_snapshot: typhoon_engine::core::research::BbandsSnapshot,
    pub(crate) bbands_win_loading: bool,
    pub(crate) show_ad_win: bool,
    pub(crate) ad_win_symbol: String,
    pub(crate) ad_win_snapshot: typhoon_engine::core::research::AdSnapshot,
    pub(crate) ad_win_loading: bool,
    pub(crate) show_adosc_win: bool,
    pub(crate) adosc_win_symbol: String,
    pub(crate) adosc_win_snapshot: typhoon_engine::core::research::AdoscSnapshot,
    pub(crate) adosc_win_loading: bool,
    pub(crate) show_sum_win: bool,
    pub(crate) sum_win_symbol: String,
    pub(crate) sum_win_snapshot: typhoon_engine::core::research::SumSnapshot,
    pub(crate) sum_win_loading: bool,
    pub(crate) show_linreg_intercept_win: bool,
    pub(crate) linreg_intercept_win_symbol: String,
    pub(crate) linreg_intercept_win_snapshot:
        typhoon_engine::core::research::LinearRegInterceptSnapshot,
    pub(crate) linreg_intercept_win_loading: bool,
    // ── Round 71 — AROONOSC / MINMAXINDEX / MACDEXT / MACDFIX / MAVP ──
    pub(crate) show_aroonosc_win: bool,
    pub(crate) aroonosc_win_symbol: String,
    pub(crate) aroonosc_win_snapshot: typhoon_engine::core::research::AroonoscSnapshot,
    pub(crate) aroonosc_win_loading: bool,
    pub(crate) show_minmaxindex_win: bool,
    pub(crate) minmaxindex_win_symbol: String,
    pub(crate) minmaxindex_win_snapshot: typhoon_engine::core::research::MinMaxIndexSnapshot,
    pub(crate) minmaxindex_win_loading: bool,
    pub(crate) show_macdext_win: bool,
    pub(crate) macdext_win_symbol: String,
    pub(crate) macdext_win_snapshot: typhoon_engine::core::research::MacdextSnapshot,
    pub(crate) macdext_win_loading: bool,
    pub(crate) show_macdfix_win: bool,
    pub(crate) macdfix_win_symbol: String,
    pub(crate) macdfix_win_snapshot: typhoon_engine::core::research::MacdfixSnapshot,
    pub(crate) macdfix_win_loading: bool,
    pub(crate) show_mavp_win: bool,
    pub(crate) mavp_win_symbol: String,
    pub(crate) mavp_win_snapshot: typhoon_engine::core::research::MavpSnapshot,
    pub(crate) mavp_win_loading: bool,
    // ── Round 72 — CDL* candlestick patterns ──
    pub(crate) show_cdl_doji_win: bool,
    pub(crate) cdl_doji_win_symbol: String,
    pub(crate) cdl_doji_win_snapshot: typhoon_engine::core::research::CdlDojiSnapshot,
    pub(crate) cdl_doji_win_loading: bool,
    pub(crate) show_cdl_hammer_win: bool,
    pub(crate) cdl_hammer_win_symbol: String,
    pub(crate) cdl_hammer_win_snapshot: typhoon_engine::core::research::CdlHammerSnapshot,
    pub(crate) cdl_hammer_win_loading: bool,
    pub(crate) show_cdl_shooting_star_win: bool,
    pub(crate) cdl_shooting_star_win_symbol: String,
    pub(crate) cdl_shooting_star_win_snapshot:
        typhoon_engine::core::research::CdlShootingStarSnapshot,
    pub(crate) cdl_shooting_star_win_loading: bool,
    pub(crate) show_cdl_engulfing_win: bool,
    pub(crate) cdl_engulfing_win_symbol: String,
    pub(crate) cdl_engulfing_win_snapshot: typhoon_engine::core::research::CdlEngulfingSnapshot,
    pub(crate) cdl_engulfing_win_loading: bool,
    pub(crate) show_cdl_harami_win: bool,
    pub(crate) cdl_harami_win_symbol: String,
    pub(crate) cdl_harami_win_snapshot: typhoon_engine::core::research::CdlHaramiSnapshot,
    pub(crate) cdl_harami_win_loading: bool,
    // ── Round 73 — CDL* 3-bar / 2-bar patterns ──
    pub(crate) show_cdl_morning_star_win: bool,
    pub(crate) cdl_morning_star_win_symbol: String,
    pub(crate) cdl_morning_star_win_snapshot:
        typhoon_engine::core::research::CdlMorningStarSnapshot,
    pub(crate) cdl_morning_star_win_loading: bool,
    pub(crate) show_cdl_evening_star_win: bool,
    pub(crate) cdl_evening_star_win_symbol: String,
    pub(crate) cdl_evening_star_win_snapshot:
        typhoon_engine::core::research::CdlEveningStarSnapshot,
    pub(crate) cdl_evening_star_win_loading: bool,
    pub(crate) show_cdl_three_black_crows_win: bool,
    pub(crate) cdl_three_black_crows_win_symbol: String,
    pub(crate) cdl_three_black_crows_win_snapshot:
        typhoon_engine::core::research::CdlThreeBlackCrowsSnapshot,
    pub(crate) cdl_three_black_crows_win_loading: bool,
    pub(crate) show_cdl_three_white_soldiers_win: bool,
    pub(crate) cdl_three_white_soldiers_win_symbol: String,
    pub(crate) cdl_three_white_soldiers_win_snapshot:
        typhoon_engine::core::research::CdlThreeWhiteSoldiersSnapshot,
    pub(crate) cdl_three_white_soldiers_win_loading: bool,
    pub(crate) show_cdl_dark_cloud_cover_win: bool,
    pub(crate) cdl_dark_cloud_cover_win_symbol: String,
    pub(crate) cdl_dark_cloud_cover_win_snapshot:
        typhoon_engine::core::research::CdlDarkCloudCoverSnapshot,
    pub(crate) cdl_dark_cloud_cover_win_loading: bool,
    // ── Round 74 — CDL* piercing / doji variants / hammer mirrors ──
    pub(crate) show_cdl_piercing_win: bool,
    pub(crate) cdl_piercing_win_symbol: String,
    pub(crate) cdl_piercing_win_snapshot: typhoon_engine::core::research::CdlPiercingSnapshot,
    pub(crate) cdl_piercing_win_loading: bool,
    pub(crate) show_cdl_dragonfly_doji_win: bool,
    pub(crate) cdl_dragonfly_doji_win_symbol: String,
    pub(crate) cdl_dragonfly_doji_win_snapshot:
        typhoon_engine::core::research::CdlDragonflyDojiSnapshot,
    pub(crate) cdl_dragonfly_doji_win_loading: bool,
    pub(crate) show_cdl_gravestone_doji_win: bool,
    pub(crate) cdl_gravestone_doji_win_symbol: String,
    pub(crate) cdl_gravestone_doji_win_snapshot:
        typhoon_engine::core::research::CdlGravestoneDojiSnapshot,
    pub(crate) cdl_gravestone_doji_win_loading: bool,
    pub(crate) show_cdl_hanging_man_win: bool,
    pub(crate) cdl_hanging_man_win_symbol: String,
    pub(crate) cdl_hanging_man_win_snapshot: typhoon_engine::core::research::CdlHangingManSnapshot,
    pub(crate) cdl_hanging_man_win_loading: bool,
    pub(crate) show_cdl_inverted_hammer_win: bool,
    pub(crate) cdl_inverted_hammer_win_symbol: String,
    pub(crate) cdl_inverted_hammer_win_snapshot:
        typhoon_engine::core::research::CdlInvertedHammerSnapshot,
    pub(crate) cdl_inverted_hammer_win_loading: bool,
    // ── Round 75 — CDL* harami cross / long-legged doji / marubozu / spinning top / tristar ──
    pub(crate) show_cdl_harami_cross_win: bool,
    pub(crate) cdl_harami_cross_win_symbol: String,
    pub(crate) cdl_harami_cross_win_snapshot:
        typhoon_engine::core::research::CdlHaramiCrossSnapshot,
    pub(crate) cdl_harami_cross_win_loading: bool,
    pub(crate) show_cdl_long_legged_doji_win: bool,
    pub(crate) cdl_long_legged_doji_win_symbol: String,
    pub(crate) cdl_long_legged_doji_win_snapshot:
        typhoon_engine::core::research::CdlLongLeggedDojiSnapshot,
    pub(crate) cdl_long_legged_doji_win_loading: bool,
    pub(crate) show_cdl_marubozu_win: bool,
    pub(crate) cdl_marubozu_win_symbol: String,
    pub(crate) cdl_marubozu_win_snapshot: typhoon_engine::core::research::CdlMarubozuSnapshot,
    pub(crate) cdl_marubozu_win_loading: bool,
    pub(crate) show_cdl_spinning_top_win: bool,
    pub(crate) cdl_spinning_top_win_symbol: String,
    pub(crate) cdl_spinning_top_win_snapshot:
        typhoon_engine::core::research::CdlSpinningTopSnapshot,
    pub(crate) cdl_spinning_top_win_loading: bool,
    pub(crate) show_cdl_tristar_win: bool,
    pub(crate) cdl_tristar_win_symbol: String,
    pub(crate) cdl_tristar_win_snapshot: typhoon_engine::core::research::CdlTristarSnapshot,
    pub(crate) cdl_tristar_win_loading: bool,
    // ── Round 76 — CDL* doji star / morning doji star / evening doji star / abandoned baby / three inside ──
    pub(crate) show_cdl_doji_star_win: bool,
    pub(crate) cdl_doji_star_win_symbol: String,
    pub(crate) cdl_doji_star_win_snapshot: typhoon_engine::core::research::CdlDojiStarSnapshot,
    pub(crate) cdl_doji_star_win_loading: bool,
    pub(crate) show_cdl_morning_doji_star_win: bool,
    pub(crate) cdl_morning_doji_star_win_symbol: String,
    pub(crate) cdl_morning_doji_star_win_snapshot:
        typhoon_engine::core::research::CdlMorningDojiStarSnapshot,
    pub(crate) cdl_morning_doji_star_win_loading: bool,
    pub(crate) show_cdl_evening_doji_star_win: bool,
    pub(crate) cdl_evening_doji_star_win_symbol: String,
    pub(crate) cdl_evening_doji_star_win_snapshot:
        typhoon_engine::core::research::CdlEveningDojiStarSnapshot,
    pub(crate) cdl_evening_doji_star_win_loading: bool,
    pub(crate) show_cdl_abandoned_baby_win: bool,
    pub(crate) cdl_abandoned_baby_win_symbol: String,
    pub(crate) cdl_abandoned_baby_win_snapshot:
        typhoon_engine::core::research::CdlAbandonedBabySnapshot,
    pub(crate) cdl_abandoned_baby_win_loading: bool,
    pub(crate) show_cdl_three_inside_win: bool,
    pub(crate) cdl_three_inside_win_symbol: String,
    pub(crate) cdl_three_inside_win_snapshot:
        typhoon_engine::core::research::CdlThreeInsideSnapshot,
    pub(crate) cdl_three_inside_win_loading: bool,
    // ── Round 77 — CDL* belt hold / closing marubozu / high wave / long line / short line ──
    pub(crate) show_cdl_belt_hold_win: bool,
    pub(crate) cdl_belt_hold_win_symbol: String,
    pub(crate) cdl_belt_hold_win_snapshot: typhoon_engine::core::research::CdlBeltHoldSnapshot,
    pub(crate) cdl_belt_hold_win_loading: bool,
    pub(crate) show_cdl_closing_marubozu_win: bool,
    pub(crate) cdl_closing_marubozu_win_symbol: String,
    pub(crate) cdl_closing_marubozu_win_snapshot:
        typhoon_engine::core::research::CdlClosingMarubozuSnapshot,
    pub(crate) cdl_closing_marubozu_win_loading: bool,
    pub(crate) show_cdl_high_wave_win: bool,
    pub(crate) cdl_high_wave_win_symbol: String,
    pub(crate) cdl_high_wave_win_snapshot: typhoon_engine::core::research::CdlHighWaveSnapshot,
    pub(crate) cdl_high_wave_win_loading: bool,
    pub(crate) show_cdl_long_line_win: bool,
    pub(crate) cdl_long_line_win_symbol: String,
    pub(crate) cdl_long_line_win_snapshot: typhoon_engine::core::research::CdlLongLineSnapshot,
    pub(crate) cdl_long_line_win_loading: bool,
    pub(crate) show_cdl_short_line_win: bool,
    pub(crate) cdl_short_line_win_symbol: String,
    pub(crate) cdl_short_line_win_snapshot: typhoon_engine::core::research::CdlShortLineSnapshot,
    pub(crate) cdl_short_line_win_loading: bool,
    // ── Round 78 — CDL* counterattack / homing pigeon / in-neck / on-neck / thrusting ──
    pub(crate) show_cdl_counterattack_win: bool,
    pub(crate) cdl_counterattack_win_symbol: String,
    pub(crate) cdl_counterattack_win_snapshot:
        typhoon_engine::core::research::CdlCounterattackSnapshot,
    pub(crate) cdl_counterattack_win_loading: bool,
    pub(crate) show_cdl_homing_pigeon_win: bool,
    pub(crate) cdl_homing_pigeon_win_symbol: String,
    pub(crate) cdl_homing_pigeon_win_snapshot:
        typhoon_engine::core::research::CdlHomingPigeonSnapshot,
    pub(crate) cdl_homing_pigeon_win_loading: bool,
    pub(crate) show_cdl_in_neck_win: bool,
    pub(crate) cdl_in_neck_win_symbol: String,
    pub(crate) cdl_in_neck_win_snapshot: typhoon_engine::core::research::CdlInNeckSnapshot,
    pub(crate) cdl_in_neck_win_loading: bool,
    pub(crate) show_cdl_on_neck_win: bool,
    pub(crate) cdl_on_neck_win_symbol: String,
    pub(crate) cdl_on_neck_win_snapshot: typhoon_engine::core::research::CdlOnNeckSnapshot,
    pub(crate) cdl_on_neck_win_loading: bool,
    pub(crate) show_cdl_thrusting_win: bool,
    pub(crate) cdl_thrusting_win_symbol: String,
    pub(crate) cdl_thrusting_win_snapshot: typhoon_engine::core::research::CdlThrustingSnapshot,
    pub(crate) cdl_thrusting_win_loading: bool,
    // ── Round 79/80 — additional CDL* parity windows ──
    pub(crate) show_cdl_two_crows_win: bool,
    pub(crate) cdl_two_crows_win_symbol: String,
    pub(crate) cdl_two_crows_win_snapshot: typhoon_engine::core::research::CdlTwoCrowsSnapshot,
    pub(crate) cdl_two_crows_win_loading: bool,
    pub(crate) show_cdl_three_line_strike_win: bool,
    pub(crate) cdl_three_line_strike_win_symbol: String,
    pub(crate) cdl_three_line_strike_win_snapshot:
        typhoon_engine::core::research::CdlThreeLineStrikeSnapshot,
    pub(crate) cdl_three_line_strike_win_loading: bool,
    pub(crate) show_cdl_three_outside_win: bool,
    pub(crate) cdl_three_outside_win_symbol: String,
    pub(crate) cdl_three_outside_win_snapshot:
        typhoon_engine::core::research::CdlThreeOutsideSnapshot,
    pub(crate) cdl_three_outside_win_loading: bool,
    pub(crate) show_cdl_matching_low_win: bool,
    pub(crate) cdl_matching_low_win_symbol: String,
    pub(crate) cdl_matching_low_win_snapshot:
        typhoon_engine::core::research::CdlMatchingLowSnapshot,
    pub(crate) cdl_matching_low_win_loading: bool,
    pub(crate) show_cdl_separating_lines_win: bool,
    pub(crate) cdl_separating_lines_win_symbol: String,
    pub(crate) cdl_separating_lines_win_snapshot:
        typhoon_engine::core::research::CdlSeparatingLinesSnapshot,
    pub(crate) cdl_separating_lines_win_loading: bool,
    pub(crate) show_cdl_stick_sandwich_win: bool,
    pub(crate) cdl_stick_sandwich_win_symbol: String,
    pub(crate) cdl_stick_sandwich_win_snapshot:
        typhoon_engine::core::research::CdlStickSandwichSnapshot,
    pub(crate) cdl_stick_sandwich_win_loading: bool,
    pub(crate) show_cdl_rickshaw_man_win: bool,
    pub(crate) cdl_rickshaw_man_win_symbol: String,
    pub(crate) cdl_rickshaw_man_win_snapshot:
        typhoon_engine::core::research::CdlRickshawManSnapshot,
    pub(crate) cdl_rickshaw_man_win_loading: bool,
    pub(crate) show_cdl_takuri_win: bool,
    pub(crate) cdl_takuri_win_symbol: String,
    pub(crate) cdl_takuri_win_snapshot: typhoon_engine::core::research::CdlTakuriSnapshot,
    pub(crate) cdl_takuri_win_loading: bool,
    // ── Round 81/82 — harder CDL* parity windows ──
    pub(crate) show_cdl_three_stars_in_south_win: bool,
    pub(crate) cdl_three_stars_in_south_win_symbol: String,
    pub(crate) cdl_three_stars_in_south_win_snapshot:
        typhoon_engine::core::research::CdlThreeStarsInSouthSnapshot,
    pub(crate) cdl_three_stars_in_south_win_loading: bool,
    pub(crate) show_cdl_identical_three_crows_win: bool,
    pub(crate) cdl_identical_three_crows_win_symbol: String,
    pub(crate) cdl_identical_three_crows_win_snapshot:
        typhoon_engine::core::research::CdlIdenticalThreeCrowsSnapshot,
    pub(crate) cdl_identical_three_crows_win_loading: bool,
    pub(crate) show_cdl_kicking_win: bool,
    pub(crate) cdl_kicking_win_symbol: String,
    pub(crate) cdl_kicking_win_snapshot: typhoon_engine::core::research::CdlKickingSnapshot,
    pub(crate) cdl_kicking_win_loading: bool,
    pub(crate) show_cdl_kicking_by_length_win: bool,
    pub(crate) cdl_kicking_by_length_win_symbol: String,
    pub(crate) cdl_kicking_by_length_win_snapshot:
        typhoon_engine::core::research::CdlKickingByLengthSnapshot,
    pub(crate) cdl_kicking_by_length_win_loading: bool,
    pub(crate) show_cdl_ladder_bottom_win: bool,
    pub(crate) cdl_ladder_bottom_win_symbol: String,
    pub(crate) cdl_ladder_bottom_win_snapshot:
        typhoon_engine::core::research::CdlLadderBottomSnapshot,
    pub(crate) cdl_ladder_bottom_win_loading: bool,
    pub(crate) show_cdl_unique_three_river_win: bool,
    pub(crate) cdl_unique_three_river_win_symbol: String,
    pub(crate) cdl_unique_three_river_win_snapshot:
        typhoon_engine::core::research::CdlUniqueThreeRiverSnapshot,
    pub(crate) cdl_unique_three_river_win_loading: bool,
    // ── Round 83/84 — additional multi-bar CDL* parity windows ──
    pub(crate) show_cdl_advance_block_win: bool,
    pub(crate) cdl_advance_block_win_symbol: String,
    pub(crate) cdl_advance_block_win_snapshot:
        typhoon_engine::core::research::CdlAdvanceBlockSnapshot,
    pub(crate) cdl_advance_block_win_loading: bool,
    pub(crate) show_cdl_breakaway_win: bool,
    pub(crate) cdl_breakaway_win_symbol: String,
    pub(crate) cdl_breakaway_win_snapshot: typhoon_engine::core::research::CdlBreakawaySnapshot,
    pub(crate) cdl_breakaway_win_loading: bool,
    pub(crate) show_cdl_gap_side_side_white_win: bool,
    pub(crate) cdl_gap_side_side_white_win_symbol: String,
    pub(crate) cdl_gap_side_side_white_win_snapshot:
        typhoon_engine::core::research::CdlGapSideSideWhiteSnapshot,
    pub(crate) cdl_gap_side_side_white_win_loading: bool,
    pub(crate) show_cdl_upside_gap_two_crows_win: bool,
    pub(crate) cdl_upside_gap_two_crows_win_symbol: String,
    pub(crate) cdl_upside_gap_two_crows_win_snapshot:
        typhoon_engine::core::research::CdlUpsideGapTwoCrowsSnapshot,
    pub(crate) cdl_upside_gap_two_crows_win_loading: bool,
    pub(crate) show_cdl_xside_gap_three_methods_win: bool,
    pub(crate) cdl_xside_gap_three_methods_win_symbol: String,
    pub(crate) cdl_xside_gap_three_methods_win_snapshot:
        typhoon_engine::core::research::CdlXSideGapThreeMethodsSnapshot,
    pub(crate) cdl_xside_gap_three_methods_win_loading: bool,
    pub(crate) show_cdl_conceal_baby_swallow_win: bool,
    pub(crate) cdl_conceal_baby_swallow_win_symbol: String,
    pub(crate) cdl_conceal_baby_swallow_win_snapshot:
        typhoon_engine::core::research::CdlConcealBabySwallowSnapshot,
    pub(crate) cdl_conceal_baby_swallow_win_loading: bool,
    // ── Round 85/86 — stateful CDL* parity windows ──
    pub(crate) show_cdl_hikkake_win: bool,
    pub(crate) cdl_hikkake_win_symbol: String,
    pub(crate) cdl_hikkake_win_snapshot: typhoon_engine::core::research::CdlHikkakeSnapshot,
    pub(crate) cdl_hikkake_win_loading: bool,
    pub(crate) show_cdl_hikkake_mod_win: bool,
    pub(crate) cdl_hikkake_mod_win_symbol: String,
    pub(crate) cdl_hikkake_mod_win_snapshot: typhoon_engine::core::research::CdlHikkakeModSnapshot,
    pub(crate) cdl_hikkake_mod_win_loading: bool,
    pub(crate) show_cdl_mat_hold_win: bool,
    pub(crate) cdl_mat_hold_win_symbol: String,
    pub(crate) cdl_mat_hold_win_snapshot: typhoon_engine::core::research::CdlMatHoldSnapshot,
    pub(crate) cdl_mat_hold_win_loading: bool,
    pub(crate) show_cdl_rise_fall_three_methods_win: bool,
    pub(crate) cdl_rise_fall_three_methods_win_symbol: String,
    pub(crate) cdl_rise_fall_three_methods_win_snapshot:
        typhoon_engine::core::research::CdlRiseFallThreeMethodsSnapshot,
    pub(crate) cdl_rise_fall_three_methods_win_loading: bool,
    // ── Round 87/88 — final CDL* parity windows ──
    pub(crate) show_cdl_stalled_pattern_win: bool,
    pub(crate) cdl_stalled_pattern_win_symbol: String,
    pub(crate) cdl_stalled_pattern_win_snapshot:
        typhoon_engine::core::research::CdlStalledPatternSnapshot,
    pub(crate) cdl_stalled_pattern_win_loading: bool,
    pub(crate) show_cdl_tasuki_gap_win: bool,
    pub(crate) cdl_tasuki_gap_win_symbol: String,
    pub(crate) cdl_tasuki_gap_win_snapshot: typhoon_engine::core::research::CdlTasukiGapSnapshot,
    pub(crate) cdl_tasuki_gap_win_loading: bool,
    // ── Round 76 — Quant Stats (modsharpe / hsieh / chow / driftburst / hlvclust) ──
    pub(crate) show_modsharpe_win: bool,
    pub(crate) modsharpe_win_symbol: String,
    pub(crate) modsharpe_win_snapshot: typhoon_engine::core::research::ModSharpeSnapshot,
    pub(crate) modsharpe_win_loading: bool,
    pub(crate) show_hsiehtest_win: bool,
    pub(crate) hsiehtest_win_symbol: String,
    pub(crate) hsiehtest_win_snapshot: typhoon_engine::core::research::HsiehTestSnapshot,
    pub(crate) hsiehtest_win_loading: bool,
    pub(crate) show_chowbreak_win: bool,
    pub(crate) chowbreak_win_symbol: String,
    pub(crate) chowbreak_win_snapshot: typhoon_engine::core::research::ChowBreakSnapshot,
    pub(crate) chowbreak_win_loading: bool,
    pub(crate) show_driftburst_win: bool,
    pub(crate) driftburst_win_symbol: String,
    pub(crate) driftburst_win_snapshot: typhoon_engine::core::research::DriftBurstSnapshot,
    pub(crate) driftburst_win_loading: bool,
    pub(crate) show_hlvclust_win: bool,
    pub(crate) hlvclust_win_symbol: String,
    pub(crate) hlvclust_win_snapshot: typhoon_engine::core::research::HlvClustSnapshot,
    pub(crate) hlvclust_win_loading: bool,
    // ── Round 77 — Quant Stats (yangzhang / kuiper / dagostino / baiperron / kupiecpof) ──
    pub(crate) show_yangzhang_win: bool,
    pub(crate) yangzhang_win_symbol: String,
    pub(crate) yangzhang_win_snapshot: typhoon_engine::core::research::YangZhangVolSnapshot,
    pub(crate) yangzhang_win_loading: bool,
    pub(crate) show_kuiper_win: bool,
    pub(crate) kuiper_win_symbol: String,
    pub(crate) kuiper_win_snapshot: typhoon_engine::core::research::KuiperSnapshot,
    pub(crate) kuiper_win_loading: bool,
    pub(crate) show_dagostino_win: bool,
    pub(crate) dagostino_win_symbol: String,
    pub(crate) dagostino_win_snapshot: typhoon_engine::core::research::DagostinoSnapshot,
    pub(crate) dagostino_win_loading: bool,
    pub(crate) show_baiperron_win: bool,
    pub(crate) baiperron_win_symbol: String,
    pub(crate) baiperron_win_snapshot: typhoon_engine::core::research::BaiPerronSnapshot,
    pub(crate) baiperron_win_loading: bool,
    pub(crate) show_kupiecpof_win: bool,
    pub(crate) kupiecpof_win_symbol: String,
    pub(crate) kupiecpof_win_snapshot: typhoon_engine::core::research::KupiecPofSnapshot,
    pub(crate) kupiecpof_win_loading: bool,

    // ── Web article ingestion + packet viewer ──
    /// INGEST_RESEARCH — paste-in window where the user drops an AI
    /// agent reply that contains `===TYPHOON_INGEST===` blocks.
    pub(crate) show_ingest_research: bool,
    pub(crate) ingest_research_text: String,
    pub(crate) ingest_research_agent: String, // default tag applied to records missing an agent field
    pub(crate) ingest_research_status: String, // last status / result summary
    pub(crate) ingest_research_busy: bool,
    /// RESEARCH_PACKET — viewer window with tree nav + scrollable text.
    pub(crate) show_packet_viewer: bool,
    pub(crate) packet_viewer_symbol: String,
    pub(crate) packet_viewer_question: String,
    pub(crate) packet_viewer_text: String, // generated packet markdown
    pub(crate) packet_viewer_tree: Vec<PacketTreeNode>, // parsed H2/H3/H4 headers
    pub(crate) packet_viewer_scroll_target: Option<usize>, // byte offset in text to scroll to
    pub(crate) packet_viewer_selected: Option<usize>,

    /// Bottom panel tab.
    pub(crate) bottom_tab: BottomTab,

    /// Application log — max 500 entries, ring-buffer style.
    pub(crate) log: VecDeque<LogEntry>,
    /// ADR-094: Log level filter dropdown.
    pub(crate) log_filter: LogFilter,

    // ── ADR-094: UX Analytics Features ──────────────────────────────
    /// Active result card (rendered above log, auto-dismissed after 30s).
    pub(crate) result_card: Option<(ResultCard, std::time::Instant)>,
    /// Toast notification stack (top-right overlay).
    pub(crate) toasts: Vec<Toast>,
    /// Command palette context (set by right-click location).
    pub(crate) palette_context: PaletteContext,

    /// Crosshair position in screen coordinates (updated each frame).
    pub(crate) crosshair: Option<egui::Pos2>,

    /// Counter to avoid calling ctx.request_repaint in a tight loop.
    pub(crate) frame_count: u64,

    /// Tab being dragged (for drag-and-drop reordering).
    pub(crate) dragging_tab: Option<usize>,

    /// Last active tab index the tab strip handled — used to detect active-tab
    /// changes (clicking a tab, the + button, NEW_TAB, close adjustments) so the
    /// horizontally-scrollable tab bar can scroll the active tab into view.
    pub(crate) tab_bar_last_active: usize,

    // ── async broker ─────────────────────────────────────────────────────
    /// Tokio runtime handle for spawning async tasks.
    pub(crate) rt_handle: tokio::runtime::Handle,
    /// Send commands to broker task.
    pub(crate) broker_tx: mpsc::UnboundedSender<BrokerCmd>,
    /// Receive results from broker task.
    pub(crate) broker_rx: mpsc::UnboundedReceiver<BrokerMsg>,
    /// Whether broker is connected.
    pub(crate) broker_connected: bool,
    /// Live account info.
    pub(crate) live_account: Option<AccountInfo>,
    /// Live positions.
    pub(crate) live_positions: Vec<PositionInfo>,
    pub(crate) kr_positions: Vec<PositionInfo>,
    pub(crate) kraken_equity_quote_meta: std::collections::BTreeMap<String, KrakenEquityQuoteMeta>,
    /// Position visibility toggles (still synced, just hidden in UI)
    pub(crate) show_alpaca_positions: bool,
    pub(crate) show_kr_positions: bool,
    pub(crate) show_kraken_trade_history: bool,
    pub(crate) show_kraken_open_orders: bool,
    /// Live orders.
    pub(crate) live_orders: Vec<OrderInfo>,

    // ── right panel state (WebKit parity) ─────────────────────────────
    /// Active right panel tab (kept for session compat).
    pub(crate) right_tab: RightTab,
    /// Collapsible right panel sections (all visible, individually expandable).
    pub(crate) right_trading_open: bool,
    pub(crate) right_positions_open: bool,
    pub(crate) right_orders_open: bool,
    pub(crate) right_watchlist_open: bool,
    pub(crate) right_risk_open: bool,
    pub(crate) right_recent_fills_open: bool,
    pub(crate) right_news_open: bool,
    pub(crate) right_mtf_grid_open: bool,
    pub(crate) right_panel_order: Vec<RightPanelSectionId>,
    pub(crate) dragging_right_panel_section: Option<RightPanelSectionId>,
    /// Risk sizing mode dropdown.
    pub(crate) risk_mode: RiskMode,
    pub(crate) order_broker: OrderBroker,
    /// SL price input text.
    pub(crate) sl_input: String,
    /// TP price input text.
    pub(crate) tp_input: String,
    /// Standard mode risk % input.
    pub(crate) trade_risk_pct_input: String,
    /// Dynamic mode minimum balance floor.
    pub(crate) trade_min_balance_input: String,
    /// Dynamic mode losses-to-floor input.
    pub(crate) trade_losses_to_min_input: String,
    /// VaR mode risk % of equity.
    pub(crate) trade_var_risk_pct_input: String,
    /// Whether SL checkbox is enabled.
    pub(crate) sl_enabled: bool,
    /// Whether TP checkbox is enabled.
    pub(crate) tp_enabled: bool,
    /// Recent fills (symbol, side, qty, price, time).
    pub(crate) recent_fills: Vec<(String, String, f64, f64, String)>,

    /// Latest background-computed data. Updated by draining bg_rx each frame.
    pub(crate) bg: BgData,
    /// Receiver for background data snapshots.
    pub(crate) bg_rx: std::sync::mpsc::Receiver<BgData>,

    pub(crate) gpu_indicators: Option<gpu_compute::GpuCompute>,
    /// Set true when indicator periods change in the UI; cleared after recompute.
    pub(crate) indicators_dirty: bool,

    // ── Prometheus metrics ───────────────────────────────────────────────
    /// Shared metrics registry (updated periodically, served via HTTP).
    pub(crate) metrics_registry: Option<std::sync::Arc<crate::metrics::MetricsRegistry>>,
    /// App start time for uptime calculation.
    pub(crate) metrics_start: std::time::Instant,
    /// Wall-clock gates for periodic work. These must not be derived from frame_count:
    /// native-refresh rendering can run at 60/144/240Hz, while old code assumed 4fps idle.
    pub(crate) periodic_crypto_last_refresh: std::time::Instant,
    pub(crate) kraken_universe_last_schedule: std::time::Instant,
    /// Wall-clock anchor for the periodic re-evaluation of the WS OHLC spawn
    /// when focus was empty at startup. Once the streamers are up
    /// (`kraken_ws_ohlc_started == true`) this stops being read.
    pub(crate) kraken_ws_ohlc_last_spawn_retry: std::time::Instant,
    /// Wall-clock anchor for the background news-body hydration tick.
    /// Throttle is HYDRATE_INTERVAL_SECS (see `news_ingest`). One in-flight
    /// hydrate at a time is enforced by `news_body_hydrate_in_flight`.
    pub(crate) news_body_last_hydrate: std::time::Instant,
    pub(crate) news_body_hydrate_in_flight: bool,
    /// CommonMark renderer cache for the article-body pane (egui_commonmark
    /// needs a persistent cache so per-frame parse + image-handle lookup
    /// stays cheap). Hero images render via the egui image loader installed
    /// in `new` on construction.
    pub(crate) news_md_cache: egui_commonmark::CommonMarkCache,
    pub(crate) kraken_futures_universe_last_schedule: std::time::Instant,
    pub(crate) session_last_autosave: std::time::Instant,
    pub(crate) metrics_last_update: std::time::Instant,
    /// Last REST `TradesHistory` fetch. The `ownTrades` WebSocket already
    /// streams new trades live (see KrakenLiveTrade handler), so the REST
    /// pull is only needed at connect / reconnect / cold cache. A periodic
    /// dispatch on every KrakenBalances tick (~60 s) was burning a private
    /// REST counter slot and re-rendering the same history; this gate caps
    /// the cadence to KRAKEN_TRADES_REST_REFRESH_SECS.
    pub(crate) kraken_trades_last_fetch: std::time::Instant,
    pub(crate) weekend_crypto_last_sync: std::time::Instant,
    pub(crate) alpaca_rotation_last_sync: std::time::Instant,
    pub(crate) perf_last_report: std::time::Instant,
    pub(crate) perf_slow_frame_count: u32,
    pub(crate) perf_max_update_ms: f64,
    pub(crate) perf_broker_msgs_drained: u32,

    /// Screenshot requested via SCREENSHOT command (triggers ViewportCommand::Screenshot next frame).
    pub(crate) screenshot_requested: bool,
    /// Path to the last saved screenshot (for sharing to Matrix chat).
    pub(crate) last_screenshot_path: Option<std::path::PathBuf>,

    /// Artefact gallery: scanned list of on-disk screenshot files
    /// (path, mtime unix seconds, size bytes), sorted newest-first.
    pub(crate) screenshots_list: Vec<(std::path::PathBuf, i64, u64)>,
    pub(crate) screenshots_sort_col: usize,
    pub(crate) screenshots_sort_asc: bool,
    /// Wall-clock unix ts of last scan_screenshots() call; throttles
    /// redundant directory walks while the gallery window is open.
    pub(crate) screenshots_last_refresh: i64,
    /// Toggle for the Screenshots Gallery window (palette: SCREENSHOTS / GALLERY).
    pub(crate) show_screenshots_gallery: bool,
}

/// Alpaca retry-queue entry. Persisted as JSON under KV key `alpaca:retry_queue`
/// so 429'd symbol/TF pairs resume after app restart.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct AlpacaRetry {
    pub(crate) symbol: String,
    pub(crate) timeframe: String,
    pub(crate) last_attempt: i64, // unix seconds of last dispatch
    pub(crate) next_attempt: i64, // unix seconds — earliest the retry worker may re-fire
    pub(crate) retry_count: u32,
    pub(crate) last_error: String,
    /// True if a prior fetch returned some bars but was cut short by 429 —
    /// tells the coverage sweep this is genuinely incomplete, not "no history."
    pub(crate) partial: bool,
}
