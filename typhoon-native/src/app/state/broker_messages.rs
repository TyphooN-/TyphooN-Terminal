use super::*;

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
    // Company events, sentiment, transcripts, commodities, and tape research results
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
